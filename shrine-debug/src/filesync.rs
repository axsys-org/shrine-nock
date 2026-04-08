use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use async_trait::async_trait;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use shrine_core::Care;
use shrine_core::driver::{
    DriverAck, DriverCard, DriverID, DriverNote, DriverOwned, IntoTale, ShrineDriver, Subscription
};
use shrine_core::supervisor::{ActorHandle, ChildSpec, RestartPolicy};
use shrine_storage::store::Namespace;
use shrine_storage::{DiskPail, Gift, Mode, Observe, Tale};
use tokio::sync::mpsc;

/// Slot containing the unix filesystem path for a mount.
const SLOT_UNIX: &str = "/sys/slot/unix";
/// Slot containing the shrine namespace path for a mount.
const SLOT_SHRINE: &str = "/sys/slot/shrine";
/// Slot containing the projection slot name for a mount.
const SLOT_PROJECTION: &str = "/sys/slot/projection";
/// Slot containing the file extension for unix files (e.g. "hoon", "txt").
const SLOT_EXTENSION: &str = "/sys/slot/extension";
/// Slot containing a kook path to add as a Duct to each synced file.
const SLOT_KOOK: &str = "/sys/slot/kook";
/// Path where mount configurations are stored.
const CLAY_PATH: &str = "/drivers/clay";

#[derive(Debug, Clone)]
struct MountConfig {
    name: String,
    unix_path: PathBuf,
    shrine_path: String,
    projection_slot: String,
    file_ext: String,
    /// If set, each synced file gets a /sys/slot/kook Duct pointing to this path.
    kook_path: Option<String>,
}

/// IntoTale for unix->shrine file writes.
#[derive(Debug)]
struct FileSyncTale {
    slot: String,
    data: Vec<u8>,
    /// File extension for this entry (e.g. "hoon", "txt").
    extension: String,
    /// If set, add /sys/slot/kook as a Duct pointing to this path.
    kook_path: Option<String>,
}

impl IntoTale for FileSyncTale {
    fn into_tale(&self, namespace: Arc<Namespace>) -> Tale {
        let mut tale = Tale::empty();
        tale.insert(&self.slot, DiskPail::Text { data: self.data.clone() });
        if !self.extension.is_empty() {
            tale.insert(SLOT_EXTENSION, DiskPail::Text { data: self.extension.as_bytes().to_vec() });
        }
        if let Some(kook) = &self.kook_path {
            if let Ok(idx) = namespace.get_path_id(kook) {
                tale.insert("/sys/slot/kook", DiskPail::Duct { data: vec![idx] });
            }
        }
        tale
    }
}

#[derive(Debug)]
struct HelpTale {
    text: String,
}

impl IntoTale for HelpTale {
    fn into_tale(&self, _namespace: Arc<Namespace>) -> Tale {
        let mut tale = Tale::empty();
        tale.insert("/sys/slot/help", DiskPail::Text { data: self.text.as_bytes().to_vec() });
        tale
    }
}

#[derive(Debug)]
struct NullTale;

impl IntoTale for NullTale {
    fn into_tale(&self, _: Arc<Namespace>) -> Tale {
        Tale::empty()
    }
}

// ---------------------------------------------------------------------------
// FileSyncDriver — supervisor that manages per-mount child actors
// ---------------------------------------------------------------------------

pub struct FileSyncDriver {
    id: DriverID,
    namespace: Arc<Namespace>,
    tx: Option<mpsc::Sender<Vec<DriverCard>>>,
    handle: Option<ActorHandle>,
    /// Tracks which mount names we have spawned children for.
    active_mounts: Vec<String>,
    last_z_version: u32,
}

impl FileSyncDriver {
    pub fn new(id: DriverID, namespace: Arc<Namespace>) -> Self {
        Self {
            id,
            namespace,
            tx: None,
            handle: None,
            active_mounts: Vec::new(),
            last_z_version: 0,
        }
    }

    fn read_mounts(namespace: &Namespace) -> Vec<MountConfig> {
        let mut mounts = Vec::new();

        let epic = match namespace.look(Care::Y, CLAY_PATH) {
            Ok(Observe::Found(epic)) => epic,
            _ => return mounts,
        };

        for (path_idx, disk_saga) in epic.iter() {
            let path_str = match namespace.path_idx_to_str(path_idx) {
                Ok(Some(s)) => s,
                _ => continue,
            };

            if path_str == CLAY_PATH {
                continue;
            }

            let tale = disk_saga.tale();
            let unix = tale.get(SLOT_UNIX).and_then(|p| std::str::from_utf8(p.data()).ok());
            let shrine = tale.get(SLOT_SHRINE).and_then(|p| std::str::from_utf8(p.data()).ok());
            let proj = tale.get(SLOT_PROJECTION).and_then(|p| std::str::from_utf8(p.data()).ok());
            let ext = tale.get(SLOT_EXTENSION).and_then(|p| std::str::from_utf8(p.data()).ok()).unwrap_or("");
            let kook = tale.get(SLOT_KOOK).and_then(|p| std::str::from_utf8(p.data()).ok());

            if let (Some(unix), Some(shrine), Some(proj)) = (unix, shrine, proj) {
                let name = path_str.rsplit('/').next().unwrap_or(&path_str).to_string();
                mounts.push(MountConfig {
                    name,
                    unix_path: PathBuf::from(unix),
                    shrine_path: shrine.to_string(),
                    projection_slot: proj.to_string(),
                    file_ext: ext.to_string(),
                    kook_path: kook.map(|s| s.to_string()),
                });
            }
        }

        mounts
    }

    async fn reconcile_mounts(&mut self) -> Result<(), Box<dyn Error + Send>> {
        let mounts = Self::read_mounts(&self.namespace);
        let handle = match &self.handle {
            Some(h) => h.clone(),
            None => return Ok(()),
        };

        let new_names: Vec<String> = mounts.iter().map(|m| m.name.clone()).collect();

        // Terminate children for removed mounts.
        // (We don't have child DriverIDs stored, so we track by name.
        //  Termination by name would require a lookup. For now, we rely on
        //  the supervisor restarting children — removed mounts just won't be
        //  re-spawned. We'll clean up by spawning only for new mounts.)
        // TODO: store child DriverIDs for explicit termination

        let root_path = self.namespace.get_path_id("/").unwrap().raw();

        // Spawn children for new mounts.
        for mount in &mounts {
            if !self.active_mounts.contains(&mount.name) {
                let config = mount.clone();
                let spec = ChildSpec {
                    name: mount.name.clone(),
                    factory: Arc::new(move |id, ns| {
                        Box::new(MountWatcherDriver::new(id, ns, config.clone()))
                    }),
                    restart: RestartPolicy::Permanent,
                    subscription: Subscription { path: root_path, care: Care::Z },
                };
                println!("[filesync] spawning mount watcher for '{}'", mount.name);
                let _ = handle.spawn(spec).await;
            }
        }

        self.active_mounts = new_names;
        Ok(())
    }
}

#[async_trait]
impl ShrineDriver for FileSyncDriver {
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
        // Register as the clay driver.
        let tx = self.tx.clone().expect("tx must be set");
        tx.send(vec![DriverCard {
            id: 0,
            note: DriverNote {
                path: "/drivers/clay".to_string(),
                mode: Mode::Add,
                slots: Arc::new(DriverOwned::new(self.id)),
            },
        }]).await.ok();

        self.reconcile_mounts().await?;
        Ok(())
    }

    async fn on_dirty(&mut self, dirty: &[Gift]) -> Result<(), Box<dyn Error + Send>> {
        let z_version = dirty.iter()
            .find(|g| g.care == Care::Z)
            .map(|g| g.ever.z.data)
            .unwrap_or(0);

        if z_version != 0 && z_version == self.last_z_version {
            return Ok(());
        }
        self.last_z_version = z_version;

        self.reconcile_mounts().await?;
        Ok(())
    }

    async fn on_done(&mut self, _ack: &DriverAck) -> Result<(), Box<dyn Error + Send>> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// MountWatcherDriver — per-mount actor handling bidirectional sync
// ---------------------------------------------------------------------------

pub struct MountWatcherDriver {
    id: DriverID,
    namespace: Arc<Namespace>,
    tx: Option<mpsc::Sender<Vec<DriverCard>>>,
    config: MountConfig,
    watcher: Option<RecommendedWatcher>,
    last_z_version: u32,
    /// Tracks mtimes of files last written by shrine→unix to suppress echo.
    written_mtimes: Arc<Mutex<HashMap<PathBuf, SystemTime>>>,
}

impl MountWatcherDriver {
    fn new(id: DriverID, namespace: Arc<Namespace>, config: MountConfig) -> Self {
        Self {
            id,
            namespace,
            tx: None,
            config,
            watcher: None,
            last_z_version: 0,
            written_mtimes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn sync_shrine_to_unix(
        namespace: &Namespace,
        mount: &MountConfig,
        written_mtimes: &Mutex<HashMap<PathBuf, SystemTime>>,
    ) {
        if let Err(e) = fs::create_dir_all(&mount.unix_path) {
            println!("[mount:{}] mkdir failed: {}", mount.name, e);
            return;
        }

        let epic = match namespace.look(Care::Z, &mount.shrine_path) {
            Ok(Observe::Found(epic)) => epic,
            _ => return,
        };

        // Sort entries shallow-first by path depth.
        let mut epic_vec: Vec<_> = epic.iter().collect();
        epic_vec.sort_by_key(|(path_idx, _)| {
            namespace.path_idx_to_str(path_idx)
                .ok().flatten()
                .map(|s| s.matches('/').count())
                .unwrap_or(0)
        });

        for (path_idx, disk_saga) in epic_vec.iter() {
            let path_str = match namespace.path_idx_to_str(path_idx) {
                Ok(Some(s)) => s,
                _ => continue,
            };

            let rel = match path_str.strip_prefix(&mount.shrine_path) {
                Some(r) if !r.is_empty() => r.trim_start_matches('/'),
                _ => continue,
            };

            let tale = disk_saga.tale();
            if let Some(pail) = tale.get(&mount.projection_slot) {
                let ext = tale.get(SLOT_EXTENSION)
                    .and_then(|p| std::str::from_utf8(p.data()).ok())
                    .unwrap_or("");
                let filename = if ext.is_empty() {
                    rel.to_string()
                } else {
                    format!("{}.{}", rel, ext)
                };
                let unix_file = mount.unix_path.join(filename);
                let shrine_data = pail.data();
                // Skip write if unix file already has the same content.
                if let Ok(existing) = fs::read(&unix_file) {
                    if existing == shrine_data {
                        continue;
                    }
                }
                if let Some(parent) = unix_file.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                if let Err(e) = fs::write(&unix_file, shrine_data) {
                    println!("[mount:{}] write error: {}", mount.name, e);
                } else if let Ok(meta) = fs::metadata(&unix_file) {
                    if let Ok(mtime) = meta.modified() {
                        written_mtimes.lock().unwrap().insert(unix_file, mtime);
                    }
                }
            }
        }
    }

    async fn sync_unix_to_shrine(
        namespace: &Namespace,
        tx: &mpsc::Sender<Vec<DriverCard>>,
        driver_id: DriverID,
        mount: &MountConfig,
        written_mtimes: &Mutex<HashMap<PathBuf, SystemTime>>,
    ) {
        let files = walk_dir(&mount.unix_path);

        // Collect entries with their shrine paths, then sort shallow-first.
        let mut entries: Vec<(String, PathBuf)> = Vec::new();
        for file in files {
            let rel = match file.strip_prefix(&mount.unix_path) {
                Ok(r) => r.to_string_lossy(),
                Err(_) => continue,
            };

            let shrine_path = format!(
                "{}/{}",
                mount.shrine_path.trim_end_matches('/'),
                strip_extensions(&rel)
            );

            entries.push((shrine_path, file));
        }
        entries.sort_by_key(|(p, _)| p.matches('/').count());
        let mut cards = vec![];

        for (shrine_path, file) in entries {
            // Skip files whose mtime matches what we last wrote from shrine→unix.
            if let Ok(meta) = fs::metadata(&file) {
                if let Ok(mtime) = meta.modified() {
                    let map = written_mtimes.lock().unwrap();
                    if map.get(&file) == Some(&mtime) {
                        continue;
                    }
                }
            }

            let data = match fs::read(&file) {
                Ok(d) => d,
                Err(_) => continue,
            };

            let (_exists, already_current) = check_shrine_content(
                namespace, &shrine_path, &mount.projection_slot, &data,
            );
            if already_current {
                continue;
            }

            let tale = FileSyncTale {
                slot: mount.projection_slot.clone(),
                data,
                extension: mount.file_ext.clone(),
                kook_path: mount.kook_path.clone(),
            };
            let note = DriverNote::make(&shrine_path, tale);
            let card = DriverCard { id: driver_id, note };
            cards.push(card);
        }

        if tx.send(cards).await.is_err() {
            return;
        }

    }

    fn setup_watcher(&mut self) -> Result<(), Box<dyn Error>> {
        let tx = self.tx.clone().expect("tx must be set");
        let namespace = self.namespace.clone();
        let config = self.config.clone();
        let driver_id = self.id;
        let written_mtimes = self.written_mtimes.clone();

        let (fs_tx, mut fs_rx) = tokio::sync::mpsc::unbounded_channel::<notify::Event>();
        let fs_tx_clone = fs_tx.clone();

        let mut watcher =
            notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
                if let Ok(event) = res {
                    let _ = fs_tx_clone.send(event);
                }
            })?;

        if self.config.unix_path.exists() {
            watcher.watch(&self.config.unix_path, RecursiveMode::Recursive)?;
        }

        self.watcher = Some(watcher);

        tokio::spawn(async move {
            while let Some(event) = fs_rx.recv().await {
                let is_write = matches!(
                    event.kind,
                    notify::EventKind::Create(_) | notify::EventKind::Modify(_)
                );
                let is_remove = matches!(event.kind, notify::EventKind::Remove(_));

                if !is_write && !is_remove {
                    continue;
                }
                let mut cards = vec![];

                for path in &event.paths {
                    if path.is_dir() {
                        continue;
                    }

                    // Skip events for files we just wrote from shrine→unix.
                    if is_write {
                        if let Ok(meta) = fs::metadata(path) {
                            if let Ok(mtime) = meta.modified() {
                                let map = written_mtimes.lock().unwrap();
                                if map.get(path) == Some(&mtime) {
                                    continue;
                                }
                            }
                        }
                    }

                    let rel = match path.strip_prefix(&config.unix_path) {
                        Ok(r) => r.to_string_lossy(),
                        Err(_) => continue,
                    };

                    let shrine_path = format!(
                        "{}/{}",
                        config.shrine_path.trim_end_matches('/'),
                        strip_extensions(&rel)
                    );

                    if is_remove {
                        // Clear tracked mtime on removal.
                        written_mtimes.lock().unwrap().remove(path);
                        let note = DriverNote {
                            path: shrine_path,
                            mode: Mode::Del,
                            slots: Arc::new(NullTale),
                        };
                        let card = DriverCard { id: driver_id, note };
                        if tx.send(vec![card]).await.is_err() {
                            return;
                        }
                    } else {
                        let data = match fs::read(path) {
                            Ok(d) => d,
                            Err(_) => continue,
                        };

                        let (_exists, already_current) = check_shrine_content(
                            &namespace, &shrine_path, &config.projection_slot, &data,
                        );
                        if already_current {
                            continue;
                        }

                        let tale = FileSyncTale {
                            slot: config.projection_slot.clone(),
                            data,
                            extension: config.file_ext.clone(),
                            kook_path: config.kook_path.clone(),
                        };
                        let note = DriverNote::make(&shrine_path, tale);
                        let card = DriverCard { id: driver_id, note };
                        cards.push(card);

                    }

                }
                if let Err(e) = tx.send(cards).await {
                    tracing::error!("cards {e:?}");
                    break;
                }
            }
            return;
        });

        Ok(())
    }
}

#[async_trait]
impl ShrineDriver for MountWatcherDriver {
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

    async fn on_start(&mut self) -> Result<(), Box<dyn Error + Send>> {
        println!("[mount:{}] started (shrine={} unix={:?})",
            self.config.name, self.config.shrine_path, self.config.unix_path);

        // Initial shrine -> unix sync.
        Self::sync_shrine_to_unix(&self.namespace, &self.config, &self.written_mtimes);

        // Write root manifest before any children (reparenting is forbidden).
        {
            let tx = self.tx.clone().expect("tx must be set");
            let help = HelpTale {
                text: format!(
                    "filesync mount '{}'\nunix: {}\nshrine: {}\nprojection: {}\nextension: {}",
                    self.config.name,
                    self.config.unix_path.display(),
                    self.config.shrine_path,
                    self.config.projection_slot,
                    self.config.file_ext,
                ),
            };
            let note = DriverNote::make(&self.config.shrine_path, help);
            let card = DriverCard { id: self.id, note };
            tx.send(vec![card]).await.ok();
        }

        // Initial unix -> shrine sync (after root manifest).
        if self.config.unix_path.exists() {
            let tx = self.tx.clone().expect("tx must be set");
            Self::sync_unix_to_shrine(&self.namespace, &tx, self.id, &self.config, &self.written_mtimes).await;
        }

        // Set up filesystem watcher.
        self.setup_watcher().unwrap();

        Ok(())
    }

    async fn on_dirty(&mut self, dirty: &[Gift]) -> Result<(), Box<dyn Error + Send>> {
        let z_version = dirty.iter()
            .find(|g| g.care == Care::Z)
            .map(|g| g.ever.z.data)
            .unwrap_or(0);

        if z_version != 0 && z_version == self.last_z_version {
            return Ok(());
        }
        self.last_z_version = z_version;

        // Resync shrine -> unix.
        Self::sync_shrine_to_unix(&self.namespace, &self.config, &self.written_mtimes);

        // Resync unix -> shrine.
        if self.config.unix_path.exists() {
            let tx = self.tx.clone().expect("tx must be set");
            Self::sync_unix_to_shrine(&self.namespace, &tx, self.id, &self.config, &self.written_mtimes).await;
        }

        Ok(())
    }

    async fn on_done(&mut self, _ack: &DriverAck) -> Result<(), Box<dyn Error + Send>> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn strip_extensions(rel: &str) -> String {
    rel.split('/')
        .map(|seg| {
            match seg.rfind('.') {
                Some(i) if i > 0 => &seg[..i],
                _ => seg,
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn walk_dir(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(walk_dir(&path));
            } else {
                files.push(path);
            }
        }
    }
    files.sort_by(|a,b| {
        a.components().count().cmp(&b.components().count())
    });
    files
}

fn check_shrine_content(
    namespace: &Namespace,
    shrine_path: &str,
    projection_slot: &str,
    data: &[u8],
) -> (bool, bool) {
    match namespace.look(Care::X, shrine_path) {
        Ok(Observe::Found(epic)) => {
            let mut current = false;
            for (_, saga) in epic.iter() {
                if let Some(pail) = saga.tale().get(projection_slot) {
                    if pail.data() == data {
                        current = true;
                        break;
                    }
                }
            }
            (true, current)
        }
        _ => (false, false),
    }
}
