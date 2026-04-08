use std::{fmt::Debug, sync::Arc};

use async_trait::async_trait;
use shrine_storage::{
    DiskPail, Mode, Tale,
    core::types::{Care, Gift, Note},
    store::Namespace,
};
use tokio::sync::mpsc;

use crate::{
    supervisor::{ActorHandle, ExitReason, SupervisorCommand},
    types::Card,
};

pub type DriverID = u32;
type StdError = Box<dyn std::error::Error + Send>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Subscription {
    pub path: u32,
    pub care: Care,
}

#[derive(Debug, Clone)]
pub struct DriverCard {
    pub id: u32,
    pub note: DriverNote,
}

#[derive(Debug, Clone)]
pub enum DriverSign {
    Ack(DriverAck),
    Gift(Vec<Gift>),
    Quiet
}

#[derive(Debug)]
pub struct DriverOwned {
    pub id: DriverID,
}

impl DriverOwned {
    pub fn new(id: DriverID) -> Self {
        Self { id }
    }
}

impl IntoTale for DriverOwned {
    fn into_tale(&self, _namespace: Arc<Namespace>) -> Tale {
        let mut tale = Tale::empty();
        tale.insert(
            "/sys/slot/content",
            DiskPail::Atom {
                data: self.id.to_le_bytes().to_vec(),
            },
        );
        return tale;
    }
}

/// XX: must be called on the main thread
impl DriverNote {
    pub fn into_noun_note(self, namespace: Arc<Namespace>) -> Note {
        Note {
            path: self.path,
            mode: self.mode,
            slots: self.slots.into_tale(namespace),
        }
    }

    pub fn into_noun_card(self, namespace: Arc<Namespace>) -> Card {
        Card::Note(self.into_noun_note(namespace))
    }
}

pub trait IntoTale: Debug + Send + Sync {
    fn into_tale(&self, namespace: Arc<Namespace>) -> Tale;
}

#[allow(dead_code)]
impl EmptyTale {
    fn new() -> Self {
        Self {}
    }
}

#[derive(Clone, Debug)]
pub struct DriverNote {
    /// The path where this binding will be written.
    pub path: String,
    /// The mode of the write.
    pub mode: Mode,
    /// Named slots mapping to pails (type + data). Empty slots indicate a tombstone (deletion).
    /// This must use a trait because noun creation must happen
    /// on the main threads
    pub slots: Arc<dyn IntoTale + 'static>,
}

#[derive(Debug)]
pub struct EmptyTale {}

impl IntoTale for EmptyTale {
    fn into_tale(&self, _namespace: Arc<Namespace>) -> Tale {
        Tale::empty()
    }
}

impl DriverNote {
    pub fn poke<T: IntoTale + 'static>(path: &str, tale: T) -> Self {
        Self {
            path: path.to_string(),
            mode: Mode::Dif,
            slots: Arc::new(tale),
        }
    }

    pub fn make<T: IntoTale + 'static>(path: &str, tale: T) -> Self {
        Self {
            path: path.to_string(),
            mode: Mode::Add,
            slots: Arc::new(tale),
        }
    }

    pub fn cull(path: &str) -> Self {
        Self {
            path: path.to_string(),
            mode: Mode::Add,
            slots: Arc::new(EmptyTale {}),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DriverMove {
    pub driver: DriverID,
    pub card: DriverCard,
}

#[derive(Debug, Clone)]
pub struct DriverAck {
    pub ack: Option<String>,
    pub mov: DriverMove,
}

#[async_trait]
pub trait ShrineDriver: Send + 'static {
    fn id(&self) -> DriverID;
    fn subscribe(&self) -> Subscription;
    fn set_tx(&mut self, tx: mpsc::Sender<Vec<DriverCard>>);
    fn set_handle(&mut self, _handle: ActorHandle) {}
    async fn on_start(&mut self) -> Result<(), StdError>;

    async fn on_dirty(&mut self, dirty: &[Gift]) -> Result<(), StdError>;

    async fn on_done(&mut self, ack: &DriverAck) -> Result<(), StdError>;

    async fn on_quiet(&mut self) -> Result<(), StdError> {
        Ok(())
    }
}

pub async fn run_driver(
    mut driver: Box<dyn ShrineDriver>,
    mut rx: mpsc::Receiver<DriverSign>,
    tx: mpsc::Sender<Vec<DriverCard>>,
    handle: ActorHandle,
    supervisor_tx: mpsc::Sender<SupervisorCommand>,
) {
    let actor_id = handle.id;
    driver.set_tx(tx);
    driver.set_handle(handle);

    let reason = match run_driver_inner(&mut driver, &mut rx).await {
        Ok(()) => ExitReason::Normal,
        Err(e) => ExitReason::Error(e.to_string()),
    };

    let _ = supervisor_tx
        .send(SupervisorCommand::Exited {
            actor: actor_id,
            reason,
        })
        .await;
}

async fn run_driver_inner(
    driver: &mut Box<dyn ShrineDriver>,
    rx: &mut mpsc::Receiver<DriverSign>,
) -> Result<(), Box<dyn std::error::Error + Send>> {
    driver.on_start().await?;

    while let Some(sign) = rx.recv().await {
        match sign {
            DriverSign::Gift(gifts) => driver.on_dirty(&gifts).await?,
            DriverSign::Ack(ack) => driver.on_done(&ack).await?,
            DriverSign::Quiet => driver.on_quiet().await?,
        }
    }
    Ok(())
}
