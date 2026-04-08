use std::error::Error;
use std::sync::Arc;

use async_trait::async_trait;
use milton::cue;
use milton::traits::FromNoun;
use rand::Rng;
use shrine_storage::core::path::{DummyError, path_string_from_noun};
use shrine_storage::core::types::slot;
use shrine_storage::store::Namespace;
use shrine_storage::{Care, DiskPail, Gift, Mode, Observe, Tale};
use tokio::sync::mpsc;

use crate::driver::{
    DriverAck, DriverCard, DriverID, DriverNote, DriverOwned, IntoTale, ShrineDriver, Subscription,
};
use crate::supervisor::{ActorHandle, ChildSpec, RestartPolicy};

const JAM_SYNC_PATH: &str = "/drv/jam-sync";

// ---------------------------------------------------------------------------
// JamSyncDriver — root supervisor that watches /drv/jam-sync
// ---------------------------------------------------------------------------

pub struct JamSyncDriver {
    id: DriverID,
    namespace: Arc<Namespace>,
    tx: Option<mpsc::Sender<Vec<DriverCard>>>,
    handle: Option<ActorHandle>,
    active_requests: Vec<String>,
    last_y_version: u32,
}

impl JamSyncDriver {
    pub fn new(id: DriverID, namespace: Arc<Namespace>) -> Self {
        Self {
            id,
            namespace,
            tx: None,
            handle: None,
            active_requests: Vec::new(),
            last_y_version: 0,
        }
    }

    async fn reconcile_requests(&mut self) -> Result<(), Box<dyn Error + Send>> {
        let epic = match self.namespace.look(Care::Y, JAM_SYNC_PATH) {
            Ok(Observe::Found(epic)) => epic,
            _ => return Ok(()),
        };

        let handle = match &self.handle {
            Some(h) => h.clone(),
            None => return Ok(()),
        };

        let root_path = self.namespace.get_path_id("/").unwrap().raw();

        for (path_idx, disk_saga) in epic.iter() {
            let path_str = match self.namespace.path_idx_to_str(path_idx) {
                Ok(Some(s)) => s,
                _ => continue,
            };

            if path_str == JAM_SYNC_PATH {
                continue;
            }

            let tale = disk_saga.tale();

            // Already processed
            if tale.get(slot::RES).is_some() {
                continue;
            }

            // No request to process
            if tale.get(slot::REQ).is_none() {
                continue;
            }

            let name = path_str.rsplit('/').next().unwrap_or(&path_str).to_string();

            if self.active_requests.contains(&name) {
                continue;
            }

            let worker_path = path_str.clone();
            let spec = ChildSpec {
                name: name.clone(),
                factory: Arc::new(move |id, ns| {
                    Box::new(JamSyncWorkerDriver::new(id, ns, worker_path.clone()))
                }),
                restart: RestartPolicy::Temporary,
                subscription: Subscription { path: root_path, care: Care::X },
            };

            println!("[jam-sync] spawning worker for '{}'", name);
            let _ = handle.spawn(spec).await;
            self.active_requests.push(name);
        }

        Ok(())
    }
}

#[async_trait]
impl ShrineDriver for JamSyncDriver {
    fn id(&self) -> DriverID {
        self.id
    }

    fn subscribe(&self) -> Subscription {
        Subscription {
            path: self.namespace.get_path_id("/").unwrap().raw(),
            care: Care::Z,
        }
    }

    fn set_tx(&mut self, tx: mpsc::Sender<Vec<DriverCard>>) {
        self.tx = Some(tx);
    }

    fn set_handle(&mut self, handle: ActorHandle) {
        self.handle = Some(handle);
    }

    async fn on_start(&mut self) -> Result<(), Box<dyn Error + Send>> {
        let tx = self.tx.clone().expect("tx must be set");
        tx.send(vec![DriverCard {
            id: 0,
            note: DriverNote {
                path: JAM_SYNC_PATH.to_string(),
                mode: Mode::Add,
                slots: Arc::new(DriverOwned::new(self.id)),
            },
        }])
        .await
        .ok();

        self.reconcile_requests().await?;
        Ok(())
    }

    async fn on_dirty(&mut self, dirty: &[Gift]) -> Result<(), Box<dyn Error + Send>> {
        let y_version = dirty
            .iter()
            .find(|g| g.care == Care::Y)
            .map(|g| g.ever.y.data)
            .unwrap_or(0);

        if y_version != 0 && y_version == self.last_y_version {
            return Ok(());
        }
        self.last_y_version = y_version;

        self.reconcile_requests().await?;
        Ok(())
    }

    async fn on_done(&mut self, _ack: &DriverAck) -> Result<(), Box<dyn Error + Send>> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// JamSyncWorkerDriver — per-request worker
// ---------------------------------------------------------------------------

struct JamSyncWorkerDriver {
    id: DriverID,
    namespace: Arc<Namespace>,
    tx: Option<mpsc::Sender<Vec<DriverCard>>>,
    path: String,
}

impl JamSyncWorkerDriver {
    fn new(id: DriverID, namespace: Arc<Namespace>, path: String) -> Self {
        Self {
            id,
            namespace,
            tx: None,
            path,
        }
    }
}

#[async_trait]
impl ShrineDriver for JamSyncWorkerDriver {
    fn id(&self) -> DriverID {
        self.id
    }

    fn subscribe(&self) -> Subscription {
        Subscription {
            path: self.namespace.get_path_id(&self.path).unwrap().raw(),
            care: Care::X,
        }
    }

    fn set_tx(&mut self, tx: mpsc::Sender<Vec<DriverCard>>) {
        self.tx = Some(tx);
    }

    async fn on_start(&mut self) -> Result<(), Box<dyn Error + Send>> {
        let saga = match self.namespace.grab(&self.path) {
            Ok(Some(s)) => s,
            _ => {
                println!("[jam-sync:worker] no saga at {}", self.path);
                return Ok(());
            }
        };

        let req = match saga.tale().get(slot::REQ) {
            Some(p) => p.clone(),
            None => {
                println!("[jam-sync:worker] no req slot at {}", self.path);
                return Ok(());
            }
        };

        let tx = self.tx.clone().expect("tx must be set");

        match req {
            DiskPail::Jam { data, .. } => {
                // Export: jammed pith → grab content → jam → write file
                let note = DriverNote::poke(&self.path, ExportResultTale { req_data: data });
                tx.send(vec![DriverCard { id: self.id, note }]).await.ok();
            }
            DiskPail::Text { data } => {
                // Import: unix path → read file → cue → pail
                let unix_path = String::from_utf8_lossy(&data).to_string();
                let file_bytes = tokio::fs::read(&unix_path).await.map_err(|e| {
                    println!("[jam-sync:worker] failed to read {}: {}", unix_path, e);
                    DummyError("jam sync failed to read").boxed()
                })?;
                let note = DriverNote::poke(&self.path, ImportResultTale { file_bytes });
                tx.send(vec![DriverCard { id: self.id, note }]).await.ok();
            }
            _ => {
                println!("[jam-sync:worker] unexpected req type at {}", self.path);
            }
        }

        Ok(())
    }

    async fn on_dirty(&mut self, _dirty: &[Gift]) -> Result<(), Box<dyn Error + Send>> {
        Ok(())
    }

    async fn on_done(&mut self, _ack: &DriverAck) -> Result<(), Box<dyn Error + Send>> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// IntoTale implementations
// ---------------------------------------------------------------------------

/// Export: cues pith, grabs content from namespace, jams to file, returns path.
#[derive(Debug)]
struct ExportResultTale {
    req_data: Vec<u8>,
}

impl IntoTale for ExportResultTale {
    fn into_tale(&self, namespace: Arc<Namespace>) -> Tale {
        let mut tale = Tale::empty();

        let Some(path_noun) = cue(&self.req_data) else {
            println!("[jam-sync:export] failed to cue pith");
            return tale;
        };

        let path_str = path_string_from_noun(path_noun);

        let saga = match namespace.grab(&path_str) {
            Ok(Some(s)) => s,
            _ => {
                println!("[jam-sync:export] no saga at {}", path_str);
                return tale;
            }
        };

        let content = match saga.tale().get(slot::CONTENT) {
            Some(p) => p,
            None => {
                println!("[jam-sync:export] no content slot at {}", path_str);
                return tale;
            }
        };

        // For Jam pails, data is already jammed — write directly.
        // For other types, go through to_noun + jam.
        let jammed = match content {
            DiskPail::Jam { data, .. } => data.clone(),
            other => {
                let noun = match other.to_noun(&namespace) {
                    Ok(n) => n,
                    Err(e) => {
                        println!("[jam-sync:export] to_noun failed: {:?}", e);
                        return tale;
                    }
                };
                noun.jam()
            }
        };

        let hex: [u8; 8] = rand::rng().random();
        let hex_str: String = hex.iter().map(|b| format!("{:02x}", b)).collect();
        let filename = format!("shrine-jam-{}.jam", hex_str);

        if let Err(e) = std::fs::write(&filename, &jammed) {
            println!("[jam-sync:export] failed to write {}: {}", filename, e);
            return tale;
        }

        println!("[jam-sync:export] wrote {} ({} bytes)", filename, jammed.len());
        tale.insert(slot::RES, DiskPail::Text { data: filename.into_bytes() });
        tale
    }
}

/// Import: cues file bytes into a DiskPail, returns it as /sys/slot/res.
#[derive(Debug)]
struct ImportResultTale {
    file_bytes: Vec<u8>,
}

impl IntoTale for ImportResultTale {
    fn into_tale(&self, _namespace: Arc<Namespace>) -> Tale {
        let mut tale = Tale::empty();

        let Some(noun) = cue(&self.file_bytes) else {
            println!("[jam-sync:import] failed to cue file bytes");
            return tale;
        };

        let pail = match DiskPail::from_noun(noun) {
            Ok(p) => p,
            Err(e) => {
                println!("[jam-sync:import] failed to parse pail: {:?}", e);
                return tale;
            }
        };

        tale.insert(slot::RES, pail);
        tale
    }
}
