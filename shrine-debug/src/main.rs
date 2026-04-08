use std::{env, path::Path, sync::Arc, time::Duration};


mod filesync;
mod shell;
use clap::Parser;

use async_trait::async_trait;
use shrine_core::{
    DiskPail, Tale, docs::get_docs, driver::{DriverAck, DriverCard, DriverID, DriverNote, IntoTale, ShrineDriver, Subscription}, oneshot::OneShotDriver, shrine::{Germ, Shrine}
};
use shrine_storage::{Care, Gift, store::Namespace};
use tokio::{runtime::Builder, sync::mpsc, time::sleep};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Debug, Parser)]
pub struct Args {
    #[arg(short, long)]
    path: Option<String>,
    #[arg(short, default_value = "false")]
    fresh: bool,
    #[arg(short, long, default_value = "false")]
    stdout: bool,
}

pub struct DummyDriver {
    id: DriverID,
    namespace: Arc<Namespace>,
    tx: Option<mpsc::Sender<Vec<DriverCard>>>,
}

#[allow(dead_code)]
impl DummyDriver {
    fn new(id: DriverID, namespace: Arc<Namespace>) -> Self {
        Self {
            id,
            namespace,
            tx: None,
        }
    }
}

#[derive(Debug)]
pub struct DummyDriverTale {
    pub num: u32,
}

impl DummyDriverTale {
    fn new(num: u32) -> Self {
        Self { num }
    }
}

impl IntoTale for DummyDriverTale {
    fn into_tale(&self, _namespace: Arc<Namespace>) -> Tale {
        let pail = DiskPail::Atom { data: self.num.to_le_bytes().to_vec() };
        Tale::from_content(pail)
    }
}

#[async_trait]
impl ShrineDriver for DummyDriver {
    fn set_tx(&mut self, tx: mpsc::Sender<Vec<DriverCard>>) {
        self.tx = Some(tx);
    }
    fn id(&self) -> DriverID {
        self.id
    }
    fn subscribe(&self) -> Subscription {
        let path = self.namespace.get_path_id("/").unwrap();
        Subscription {
            path: path.raw(),
            care: Care::Z,
        }
    }

    async fn on_start(&mut self) -> Result<(), Box<dyn std::error::Error + Send>> {
        let tx = self.tx.clone().expect("tx must be set before on_start");

        // Spawn a background task that ticks every second and sends cards
        tokio::spawn(async move {
            let mut timer = tokio::time::interval(Duration::from_secs(1));
            let mut count = 0u32;
            let mut req_id = 0u32;

            loop {
                timer.tick().await;
                count += 1;
                tracing::debug!("ticked, count: {}", count);

                // XX: move noun creation into main thread
                // let pail = Pail::noun(Atom::from_u32(count).into_noun());
                // let tale = Tale::from_content(pail);
                let tale = DummyDriverTale::new(req_id);
                let card = DriverCard {
                    id: req_id,
                    note: DriverNote::make("/timer", tale),
                };
                req_id += 1;
                tokio::time::sleep(Duration::from_millis(10000)).await;

                if tx.send(vec![card]).await.is_err() {
                    // Channel closed, driver is shutting down
                    break;
                }
            }
        });

        Ok(())
    }

    async fn on_dirty(&mut self, dirty: &[Gift]) -> Result<(), Box<dyn std::error::Error + Send>> {
        tracing::info!("dirtied: {:?}", dirty);
        Ok(())
    }

    async fn on_done(&mut self, ack: &DriverAck) -> Result<(), Box<dyn std::error::Error + Send>> {
        println!("done: {:?}", ack);
        Ok(())
    }
}

use tracing_appender::{
    non_blocking::{self, NonBlockingBuilder},
    rolling,
};
use tracing_subscriber::{EnvFilter, fmt};

use crate::filesync::FileSyncDriver;
use crate::shell::ShellDriver;
use shrine_core::jam_sync::JamSyncDriver;
use shrine_core::http::HttpDriver;

static mut _GUARD: Option<non_blocking::WorkerGuard> = None;

fn init_tracing(stdout: bool) {
    let appender = rolling::daily("logs", "app.log");

    let builder = NonBlockingBuilder::default().buffered_lines_limit(1);
    let (writer, guard) = builder.finish(appender);
    let filter = EnvFilter::from_default_env();
    // keep guard alive
    unsafe {
        _GUARD = Some(guard);
    }

    let file_layer = fmt::Layer::default().with_writer(writer);

    let registry = tracing_subscriber::registry().with(filter).with(file_layer);

    let stdout_layer = if stdout {
        Some(fmt::layer().compact())
    } else {
        None
    };

    registry.with(stdout_layer).init();

    // let filter = EnvFilter::try_from_default_env()
    //     .unwrap_or_else(|_| EnvFilter::new("info"));

    // fmt()
    //     .with_env_filter(filter)
    //     .with_writer(writer)
    //     .with_target(false)
    //     .init();
}

fn main() {
    let rt = Builder::new_multi_thread().enable_all().max_blocking_threads(1).build().expect("Failed to build tokio runtime");
    // let rt = Builder::new_current_thread().enable_all().build().expect("Failed to build tokio runtime");
    rt.block_on(async {
        start().await;
    });
}
const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

async fn start() {
    let args = Args::parse();
    let path_str = args
        .path
        .unwrap_or_else(|| format!("{}/shrine.db", MANIFEST_DIR));
    let path = Path::new(&path_str);
    if args.fresh {
        std::fs::remove_dir_all(path).unwrap();
    }
    init_tracing(true);
    tracing::info!("Started");

    let exists = std::fs::exists(path).unwrap();
    let mut shrine = if exists {
        Shrine::resume(&path).unwrap()
    } else {
        Shrine::new(&path).unwrap()
    };
    tracing::warn!("exists");
    // let driver_id = shrine.next_driver_id();
    // let driver = DummyDriver::new(driver_id, shrine.namespace());
    let shell_id = shrine.next_driver_id();
    let shell = ShellDriver::new(shell_id, shrine.namespace());
    let filesync_id = shrine.next_driver_id();
    let filesync = FileSyncDriver::new(filesync_id, shrine.namespace());
    let jam_sync_id = shrine.next_driver_id();
    let jam_sync = JamSyncDriver::new(jam_sync_id, shrine.namespace());
    let http_id = shrine.next_driver_id();
    let http = HttpDriver::new(http_id, shrine.namespace());
    if !exists {
        let init_id = shrine.next_driver_id();
        let mut docs = get_docs();

        let init = OneShotDriver::new(init_id, shrine.namespace(), docs);
        shrine.add_driver(Box::new(init));

    }

    // shrine.add_driver(Box::new(driver));
    shrine.add_driver(Box::new(shell));
    // shrine.add_driver(Box::new(filesync));
    // shrine.add_driver(Box::new(jam_sync));
    // shrine.add_driver(Box::new(http));

    let germ = if exists { None } else { Some(Germ::default()) };

    // sleep(Duration::from_secs(10)).await;

    shrine.run(germ).await.unwrap();
}
