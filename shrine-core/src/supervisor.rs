use std::sync::Arc;
use std::time::Instant;

use tokio::sync::mpsc;

use crate::driver::{DriverID, ShrineDriver, Subscription};
use shrine_storage::store::Namespace;

/// How to restart a failed actor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestartPolicy {
    /// Always restart on exit.
    Permanent,
    /// Restart only on error, not normal exit.
    Transient,
    /// Never restart.
    Temporary,
}

/// Why an actor exited.
#[derive(Debug, Clone)]
pub enum ExitReason {
    Normal,
    Error(String),
    Panic(String),
}

impl ExitReason {
    pub fn is_error(&self) -> bool {
        matches!(self, ExitReason::Error(_) | ExitReason::Panic(_))
    }
}

/// Describes how to create and supervise a child actor.
pub struct ChildSpec {
    pub name: String,
    pub factory: Arc<dyn Fn(DriverID, Arc<Namespace>) -> Box<dyn ShrineDriver> + Send + Sync>,
    pub restart: RestartPolicy,
    pub subscription: Subscription,
}

impl std::fmt::Debug for ChildSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChildSpec")
            .field("name", &self.name)
            .field("restart", &self.restart)
            .field("subscription", &self.subscription)
            .finish()
    }
}

/// Commands from actors to the Shrine supervision system.
pub enum SupervisorCommand {
    /// Request to spawn a child actor.
    Spawn {
        parent: DriverID,
        spec: ChildSpec,
    },
    /// Request to terminate a child actor.
    Terminate {
        parent: DriverID,
        child: DriverID,
    },
    /// Notification that an actor has exited.
    Exited {
        actor: DriverID,
        reason: ExitReason,
    },
}

/// Handle given to drivers for spawning and managing children.
#[derive(Clone)]
pub struct ActorHandle {
    pub id: DriverID,
    supervisor_tx: mpsc::Sender<SupervisorCommand>,
}

impl ActorHandle {
    pub fn new(id: DriverID, supervisor_tx: mpsc::Sender<SupervisorCommand>) -> Self {
        Self { id, supervisor_tx }
    }

    /// Request the Shrine to spawn a child actor supervised by this actor.
    pub async fn spawn(&self, spec: ChildSpec) -> Result<(), mpsc::error::SendError<SupervisorCommand>> {
        self.supervisor_tx.send(SupervisorCommand::Spawn {
            parent: self.id,
            spec,
        }).await
    }

    /// Request the Shrine to terminate a child actor.
    pub async fn terminate(&self, child: DriverID) -> Result<(), mpsc::error::SendError<SupervisorCommand>> {
        self.supervisor_tx.send(SupervisorCommand::Terminate {
            parent: self.id,
            child,
        }).await
    }
}

impl std::fmt::Debug for ActorHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActorHandle")
            .field("id", &self.id)
            .finish()
    }
}

/// Tracks restart frequency to enforce intensity limits.
#[derive(Debug, Clone)]
pub struct RestartTracker {
    pub max_restarts: u32,
    pub within_secs: u64,
    history: Vec<Instant>,
}

impl RestartTracker {
    pub fn new(max_restarts: u32, within_secs: u64) -> Self {
        Self {
            max_restarts,
            within_secs,
            history: Vec::new(),
        }
    }

    /// Record a restart attempt. Returns true if within limits, false if exceeded.
    pub fn record_restart(&mut self) -> bool {
        let now = Instant::now();
        let cutoff = now - std::time::Duration::from_secs(self.within_secs);
        self.history.retain(|&t| t > cutoff);
        self.history.push(now);
        self.history.len() <= self.max_restarts as usize
    }
}

impl Default for RestartTracker {
    fn default() -> Self {
        Self::new(5, 10)
    }
}

/// Per-actor supervision state stored in Shrine.
pub struct ActorState {
    pub parent: Option<DriverID>,
    pub children: Vec<DriverID>,
    pub spec: Option<Arc<ChildSpec>>,
    pub restarts: RestartTracker,
}

impl ActorState {
    pub fn root() -> Self {
        Self {
            parent: None,
            children: Vec::new(),
            spec: None,
            restarts: RestartTracker::default(),
        }
    }

    pub fn child(parent: DriverID, spec: Arc<ChildSpec>) -> Self {
        Self {
            parent: Some(parent),
            children: Vec::new(),
            spec: Some(spec),
            restarts: RestartTracker::default(),
        }
    }
}
