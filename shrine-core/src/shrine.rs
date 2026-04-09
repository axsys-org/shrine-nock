use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::Path,
    sync::{Arc, OnceLock, atomic::AtomicBool},
    time::Instant,
};

use ::futures::future;
use milton::{
    Atom, IVORY_PILL, Noun, Runtime, RuntimeError, axal::Axal, debug_noun_print, jets::{self, map::merge}, traits::{FromNoun, IntoNoun, NounConversionError}, trel
};
use rand::Rng;
use shrine_storage::{
    NAMESPACE,
    CULL, Care, DiskPail, MAKE, Mode, Observe, POKE, Tale,
    core::{
        apollo::ApolloTime,
        path::{DummyError, PathIdx, path_noun_from_string, path_string_from_noun, undrive_case},
        types::{Gift, Note, slot},
    },
    overlay::Tray,
    store::{Namespace, NamespaceError, lmdb::saga::DiskSaga},
};
use thiserror::Error;
use tokio::{
    sync::mpsc::{self, Sender, error::TryRecvError},
    task::JoinHandle,
};
use tracing::{error, info, trace, warn};

use crate::{
    driver::{DriverCard, DriverID, DriverSign, ShrineDriver, run_driver},
    fate::Fate,
    react::{News, React, Reaction},
    supervisor::{ActorHandle, ActorState, ExitReason, RestartPolicy, SupervisorCommand},
    types::{Card, Ovum},
};

#[derive(Debug, Error)]
pub enum ShrineError {
    #[error("namespace error: {0}")]
    Namespace(#[from] NamespaceError),
    #[error("runtime error: {0}")]
    Runtime(#[from] RuntimeError),
    #[error("mpsc error: {0}")]
    Mpsc(#[from] TryRecvError),
    #[error("noun conversion error: {0}")]
    NounConv(#[from] NounConversionError),
    #[error("dummy error: {0}")]
    Dummy(#[from] DummyError),
}

type Result<T> = std::result::Result<T, ShrineError>;

pub struct DriverEntry {
    pub driver: DriverID,
    pub tx: Sender<DriverSign>,
    pub handle: JoinHandle<()>,
}

pub struct Germ {
  // pub root: Tale,
  // pub sys: Tale,
  pub reef: &'static [u8],
  pub rest: Vec<(String, Tale)>
}

impl Default for Germ {
    fn default() -> Self {
        Self {
            reef: IVORY_PILL,
            rest: vec![]
        }
    }
}


pub struct Shrine {
    runtime: Runtime,
    namespace: Arc<Namespace>,
    driver_id: DriverID,
    current: Axal,
    stop: Arc<AtomicBool>,
    react: React,
    queue: VecDeque<Ovum>,
    inbox: mpsc::Receiver<Vec<DriverCard>>,
    tx_handle: mpsc::Sender<Vec<DriverCard>>,
    drivers: HashMap<DriverID, DriverEntry>,
    pending_drivers: Vec<Box<dyn ShrineDriver>>,
    time: ApolloTime,
    supervisor_rx: mpsc::Receiver<SupervisorCommand>,
    supervisor_tx: mpsc::Sender<SupervisorCommand>,
    actor_states: HashMap<DriverID, ActorState>,
    booted: bool
}


unsafe extern "C" fn ffi_roof(case: u32, car: u32, pax: u32) -> u32 {
    unsafe {
        let _cas = Noun::from_raw(case);
        let _car_noun = Noun::from_raw(car);
        let pax_noun = Noun::from_raw(pax);
        let care = Care::from_ascii(car).unwrap();
        let namespace = NAMESPACE.get().unwrap().clone();
        let str = path_string_from_noun(pax_noun);

        let Ok(epic) = roof_inner(namespace.clone(), case, care, &str) else {
            return Noun::null().into_raw();
        };
        let desc = epic.desc();
        let noun = match epic {
            Observe::Found(res) => {
                let mut ax = jets::axal::empty();
                for (k, v) in res {
                    // truncate
                    let pax_noun = path_noun_from_string(&k[str.len()..]);
                    let saga = v.to_noun(namespace.clone().as_ref());
                    ax = jets::axal::insert(ax, pax_noun, saga);
                }
                ax.into_some()
            }
            Observe::Null => {
                let res = jets::axal::empty();
                res.gain();
                let r = res.into_some();
                r.gain();
                r
            }
            Observe::Unknown => {
                if case == 0 {
                    let res = jets::axal::empty();
                    res.gain();
                    let r = res.into_some();
                    r.gain();
                    r
                } else {
                    Noun::null()
                }
            }
        };




        return noun.into_raw();
    }
}

#[allow(improper_ctypes_definitions)]
unsafe extern "C" fn roof_inner(
    namespace: Arc<Namespace>,
    case: u32,
    car: Care,
    pax: &str,
) -> Result<Observe<HashMap<String, DiskSaga>>> {
    let start = Instant::now();

    // let spun = tracing::span!(tracing::Level::INFO, "roof", str = pax);
    // let _guard = spun.enter();

    // let namespace = unsafe { NAMESPACE.get().unwrap().clone() };

    if pax.starts_with("/over") {
        let (func_case, func_path, dat_path) = undrive_case(&pax)?;
        assert!(func_case != 0);
        let Some(func) = namespace.peek_item(func_path, func_case)? else {
            return Ok(Observe::Unknown);
        };

        let Some((tray, gat)) = func.tray() else {
            return Ok(Observe::Unknown);
        };
        let res = unsafe { roof_inner(namespace.clone(), case, car, dat_path) }?;
        let Observe::Found(mut epic) = res else {
            return Ok(res);
        };
        let mut new = HashMap::new();
        let prefix_start = pax.len() - dat_path.len();
        let prefix = &pax[prefix_start..];
        match tray {
            Tray::Dish => {
                for (k, v) in &mut epic {
                    let new_k = format!("{}{}", prefix, k);
                    let sam = v.to_noun(namespace.clone().as_ref());
                    let runtime = Runtime::get().expect("runtime init");
                    let Some(res) = runtime.soft_slam_opt(10000, gat.clone(), sam) else {
                        tracing::info!("overlay crashed skipping");
                        continue;
                    };
                    let new_tale = Tale::from_noun(res)?;
                    let new_saga = DiskSaga::from_raw(v.ever(), new_tale).unwrap();

                    new.insert(new_k, new_saga);
                }
                return Ok(Observe::Found(new));
            }
            Tray::Ewer => {
                println!("stubbed");
                return Ok(Observe::Unknown);
            }
        }
    }

    let res = if case == 0 {
        namespace.look(car, &pax).unwrap()
    } else {
        namespace.peek(car, &pax, case).unwrap()
    };
    let _res_str = match &res {
        Observe::Found(_) => "Found",
        Observe::Null => "Null",
        Observe::Unknown => "Unknown",
    };
    let noun = match res {
        Observe::Found(epic) => {
            // info!("got epic {epic:?}");
            let mut res = HashMap::new();

            for (key, saga) in epic.iter() {
                // let saga = saga.to_noun(&namespace);
                let path = namespace.path_idx_to_str(key).unwrap().unwrap();

                // trace!("a {path:?} {saga:?}");
                res.insert(path, saga.clone());
                // ax = jets::axal::insert(ax, path_noun_from_string(&path), saga);
            }
            Observe::Found(res)
            // ax.into_some()
        }
        Observe::Null => Observe::Null,
        Observe::Unknown => Observe::Unknown,
    };

    let end = Instant::now();
    info!("roof time: {:?}", end.duration_since(start));
    return Ok(noun);
}

impl Shrine {
    pub fn new(path: &Path) -> Result<Self> {
        let runtime = Runtime::new(1 << 31)?;
        let namespace = Namespace::new_at_path(path)?;
        let res = Self::new_(runtime, namespace)?;
        Ok(res)
    }

    fn new_(mut runtime: Runtime, namespace: Namespace) -> Result<Self> {
        let (tx, rx) = mpsc::channel(1024);
        let (sup_tx, sup_rx) = mpsc::channel(1024);
        let time = ApolloTime::now();
        // TODO: hydrate cache
        let cache = Axal::new();
        runtime.set_roof(ffi_roof);
        namespace.chow().fill(runtime.get_arvo_noun());
        let res = Self {
            runtime,
            namespace: Arc::new(namespace),
            react: React::new(),
            current: cache,
            inbox: rx,
            tx_handle: tx,
            driver_id: 0,
            drivers: HashMap::new(),
            queue: VecDeque::new(),
            pending_drivers: vec![],
            time,
            supervisor_rx: sup_rx,
            supervisor_tx: sup_tx,
            actor_states: HashMap::new(),
            stop: Arc::new(AtomicBool::new(false)),
            booted: false,
        };
        let _ = NAMESPACE.set(res.namespace.clone());
        Ok(res)
    }

    pub fn resume(path: &Path) -> Result<Self> {
        let runtime = Runtime::new(1 << 31)?;
        let namespace = Namespace::open_at_path(path)?;
        let res = Self::new_(runtime, namespace)?;
        Ok(res)
    }

    pub fn runtime(&self) -> &Runtime {
        &self.runtime
    }

    pub fn namespace(&self) -> Arc<Namespace> {
        self.namespace.clone()
    }

    pub fn next_driver_id(&mut self) -> DriverID {
        self.driver_id += 1;
        self.driver_id
    }

    pub fn tick_time(&mut self) -> ApolloTime {
        self.time = ApolloTime::now();
        self.time
    }

    fn spawn_actor(
        &mut self,
        driver: Box<dyn ShrineDriver>,
        parent: Option<DriverID>,
        spec: Option<Arc<crate::supervisor::ChildSpec>>,
    ) {
        let (tx, rx) = mpsc::channel(1024);
        let sub = driver.subscribe();
        let id = driver.id();

        self.react
            .add_reaction(sub.care, PathIdx::new(sub.path), Reaction::Driver(id));

        let handle_tx = self.tx_handle.clone();
        let sup_tx = self.supervisor_tx.clone();
        let handle = ActorHandle::new(id, self.supervisor_tx.clone());

        let join = tokio::spawn(async move {
            run_driver(driver, rx, handle_tx, handle, sup_tx).await;
        });

        let driver_entry = DriverEntry {
            driver: id,
            tx,
            handle: join,
        };
        self.drivers.insert(id, driver_entry);

        let state = match parent {
            Some(parent_id) => ActorState::child(
                parent_id,
                spec.unwrap_or_else(|| {
                    Arc::new(crate::supervisor::ChildSpec {
                        name: format!("driver-{}", id),
                        factory: Arc::new(|_, _| {
                            panic!("root actors cannot be restarted via factory")
                        }),
                        restart: RestartPolicy::Temporary,
                        subscription: crate::driver::Subscription {
                            path: 0,
                            care: Care::Z,
                        },
                    })
                }),
            ),
            None => ActorState::root(),
        };
        self.actor_states.insert(id, state);

        if let Some(parent_id) = parent {
            if let Some(parent_state) = self.actor_states.get_mut(&parent_id) {
                parent_state.children.push(id);
            }
        }
    }

    fn handle_supervisor_cmd(&mut self, cmd: SupervisorCommand) {
        match cmd {
            SupervisorCommand::Spawn { parent, spec } => {
                let id = self.next_driver_id();
                let ns = self.namespace.clone();
                let driver = (spec.factory)(id, ns);
                let spec = Arc::new(spec);
                self.spawn_actor(driver, Some(parent), Some(spec));
                println!("[supervisor] spawned child {} for parent {}", id, parent);
            }
            SupervisorCommand::Terminate { parent, child } => {
                self.terminate_actor(child);
                println!(
                    "[supervisor] terminated child {} (parent {})",
                    child, parent
                );
            }
            SupervisorCommand::Exited { actor, reason } => {
                self.handle_actor_exit(actor, reason);
            }
        }
    }

    fn handle_actor_exit(&mut self, actor: DriverID, reason: ExitReason) {
        let state = match self.actor_states.get(&actor) {
            Some(s) => s,
            None => return,
        };

        let parent = state.parent;
        let spec = state.spec.clone();
        let restart_policy = spec
            .as_ref()
            .map(|s| s.restart)
            .unwrap_or(RestartPolicy::Temporary);

        let should_restart = match (restart_policy, &reason) {
            (RestartPolicy::Permanent, _) => true,
            (RestartPolicy::Transient, r) if r.is_error() => true,
            _ => false,
        };

        println!(
            "[supervisor] actor {} exited: {:?}, restart={}",
            actor, reason, should_restart
        );

        // Clean up the dead actor
        self.cleanup_actor(actor);

        if should_restart {
            if let Some(spec) = spec {
                // Check restart intensity on the parent
                let intensity_ok = if let Some(parent_id) = parent {
                    if let Some(parent_state) = self.actor_states.get_mut(&parent_id) {
                        parent_state.restarts.record_restart()
                    } else {
                        false
                    }
                } else {
                    // Root actors: check their own tracker (but root actors are Temporary by default)
                    true
                };

                if intensity_ok {
                    let id = self.next_driver_id();
                    let ns = self.namespace.clone();
                    let driver = (spec.factory)(id, ns);
                    self.spawn_actor(driver, parent, Some(spec));
                    println!("[supervisor] restarted actor as {} (was {})", id, actor);
                } else {
                    // Exceeded restart intensity — escalate: kill the parent
                    if let Some(parent_id) = parent {
                        println!(
                            "[supervisor] restart intensity exceeded for parent {}, escalating",
                            parent_id
                        );
                        self.terminate_actor(parent_id);
                    }
                }
            }
        }
    }

    fn cleanup_actor(&mut self, actor: DriverID) {
        // Remove from parent's children list
        if let Some(state) = self.actor_states.get(&actor) {
            if let Some(parent_id) = state.parent {
                if let Some(parent_state) = self.actor_states.get_mut(&parent_id) {
                    parent_state.children.retain(|&c| c != actor);
                }
            }
        }

        // Remove React subscription
        if let Some(_entry) = self.drivers.get(&actor) {
            // We need the subscription info to remove it. Get it from actor_states.
            if let Some(state) = self.actor_states.get(&actor) {
                if let Some(spec) = &state.spec {
                    self.react.remove_reaction(
                        spec.subscription.care,
                        PathIdx::new(spec.subscription.path),
                        &Reaction::Driver(actor),
                    );
                }
            }
        }

        // Remove driver entry and actor state
        self.drivers.remove(&actor);
        self.actor_states.remove(&actor);
    }

    fn terminate_actor(&mut self, actor: DriverID) {
        // First terminate all children (top-down)
        let children: Vec<DriverID> = self
            .actor_states
            .get(&actor)
            .map(|s| s.children.clone())
            .unwrap_or_default();

        for child in children {
            self.terminate_actor(child);
        }

        // Abort the tokio task
        if let Some(entry) = self.drivers.get(&actor) {
            entry.handle.abort();
        }

        println!("[supervisor] terminated actor {}", actor);
        self.cleanup_actor(actor);
    }

    pub async fn step(&mut self, ovo: Ovum) -> Result<()> {
        let time = self.tick_time();
        let eny = rand::rng().random::<[u8; 16]>();
        let eny = Atom::from_bytes(&eny).as_noun().to_owned();
        if self.booted {
            eprintln!("ovo {ovo:?}");

        }
        let ovo = ovo.into_noun(&self.namespace).unwrap();
        let noun = trel(
            time.into_noun().unwrap(),
            eny,
            ovo.clone()
        )
        .into_noun();
        unsafe {
            if self.booted {
                debug_noun_print("test", ovo.clone());
            }
            // let cap_c = CString::new("cap").unwrap();
            // u3m_p(cap_c.as_ptr() as *const i8, noun.clone().into_raw());
        };
        let Some(fate) = self.runtime.poke(Some(100000), noun) else {
            println!("[runtime] crashed");
            error!("runtime crashed processing ovo");
            return Ok(());
        };
        let mut f = Fate::from_noun(fate.clone())?;

        self.handle_notes(fate, &mut f.notes).await;
        Ok(())
    }

    fn update_cache(&mut self, fate: Noun) {
        let (notes, _res) = fate.into_pair().unwrap();
        let mut curr = notes.clone();
        loop {
            match curr.into_pair() {
                Ok((hed, tel)) => {
                    curr = tel;
                    let (path, mode, slots) = hed.into_trel().unwrap();
                    let _next = match mode.as_raw() {
                        POKE => {
                            let old = self.current.get(path.clone()).unwrap_or(Noun::null());
                            self.current.insert(path.clone(), merge(old, slots));
                        }
                        MAKE => {
                            self.current.insert(path.clone(), slots.clone());
                        }
                        CULL => {
                            self.current.delete(path.clone());
                        }
                        _ => {
                            panic!("Weird mode");
                        }
                    };
                }
                Err(_n) => {
                    return;
                }
            };
        }
    }

    pub fn add_driver(&mut self, driver: Box<dyn ShrineDriver>) {
        self.pending_drivers.push(driver);
    }

    async fn handle_notes(&mut self, fate: Noun, notes: &mut [Note]) {
        self.update_cache(fate.clone());

        let gifts = self.namespace.write(fate, notes).unwrap();

        // Intercept crew slot changes before routing gifts
        // Also update live cache
        for gift in &gifts {
            if gift.mode == Some(Mode::Del) {
                self.react.clear_watches(gift.path);
            } else if gift.care == Care::X && gift.slots.contains(slot::CREW) {
                if let Some(saga) = self
                    .namespace
                    .grab(
                        &self
                            .namespace
                            .path_idx_to_str(&gift.path)
                            .unwrap()
                            .unwrap_or_default(),
                    )
                    .unwrap()
                {
                    if let Some(DiskPail::Mesh { data }) = saga.tale().get(slot::CREW) {
                        let targets: Vec<(Care, PathIdx)> =
                            data.values().map(|p| (Care::Z, *p)).collect();
                        self.react.set_watches(gift.path, targets);
                    } else {
                        self.react.clear_watches(gift.path);
                    }
                } else {
                    self.react.clear_watches(gift.path);
                }
            }
        }

        let mut news = News::new();
        let mut driver_gifts: HashMap<DriverID, Vec<Gift>> = HashMap::new();
        let mut sent: HashSet<(DriverID, usize)> = HashSet::new();

        for (i, gift) in gifts.iter().enumerate() {
            // 1. Exact match (existing behavior)
            let reacts = self.react.run(gift.care, gift.path);
            Self::collect(&reacts, gift, i, &mut sent, &mut news, &mut driver_gifts);

            // 2. Propagate X gifts to Y/Z subscribers on ancestors
            if gift.care == Care::X {
                if let Ok(ancestors) = self.namespace.ancestors_of(gift.path) {
                    if let Some(parent) = ancestors.last() {
                        let y_reacts = self.react.run(Care::Y, *parent);
                        Self::collect(&y_reacts, gift, i, &mut sent, &mut news, &mut driver_gifts);
                    }
                    for ancestor in &ancestors {
                        let z_reacts = self.react.run(Care::Z, *ancestor);
                        Self::collect(&z_reacts, gift, i, &mut sent, &mut news, &mut driver_gifts);
                    }
                }
            }
        }

        if !news.is_empty() {
            self.queue.push_back(Ovum::Hear(news));
        }

        let mut promises = vec![];
        for (driver_id, batch) in driver_gifts {
            if let Some(drv) = self.drivers.get(&driver_id) {
                info!("sending {} gifts to driver {:?}", batch.len(), driver_id);
                promises.push(drv.tx.send(DriverSign::Gift(batch)));
            }
        }

        let send_errs = future::join_all(promises).await;
        for err in send_errs {
            if let Err(e) = err {
                println!("[supervisor] warning: failed to send gift to driver: {}", e);
            }
        }
    }

    fn collect(
        reacts: &[Reaction],
        gift: &Gift,
        gift_idx: usize,
        sent: &mut HashSet<(DriverID, usize)>,
        news: &mut News,
        driver_gifts: &mut HashMap<DriverID, Vec<Gift>>,
    ) {
        for reaction in reacts {
            match reaction {
                Reaction::Driver(driver_id) => {
                    if sent.insert((*driver_id, gift_idx)) {
                        driver_gifts
                            .entry(*driver_id)
                            .or_default()
                            .push(gift.clone());
                    }
                }
                Reaction::Path(path) => {
                    news.add_gift(*path, gift.clone());
                }
            }
        }
    }

    fn rebuild_watch(&mut self, watches: &[(PathIdx, DiskPail)]) {
        for (watcher, pail) in watches {
                        if let DiskPail::Mesh { data } = pail {
                            let targets: Vec<(Care, PathIdx)> =
                                data.values().map(|p| (Care::Z, *p)).collect();
                            info!(
                                "restoring crew watches for {:?}: {} targets",
                                watcher,
                                targets.len()
                            );
                            self.react.set_watches(*watcher, targets);
                        }
                    }
    }

    pub async fn run(&mut self, init: Option<Germ>) -> Result<()> {
        let pending: Vec<_> = self.pending_drivers.drain(..).collect();
        if init.is_none() {
            // Startup scan: restore persistent crew slot watches
            match self.namespace.scan_slot(slot::CREW) {
                Ok(entries) => {  self.rebuild_watch(&entries) }
                Err(e) => panic!("failed to scan crew slots on startup: {}", e),
            }
        }


        if let Some(germ) = init {
            let root = Note {
                path: "/".to_string(),
                mode: Mode::Add,
                slots: Tale::from_content(DiskPail::Atom { data: vec![7] })
            };
            let sys_doc = Note {
                path:  "/sys".to_string(),
                mode: Mode::Add,
                slots: Tale::from_docs("System".to_string(), "".to_string())
            };

            let reef = Note {
                path: "/sys/reef".to_string(),
                mode: Mode::Add,
                slots: Tale::from_pot(DiskPail::Jam { typ: "noun".to_string(), data: germ.reef.to_vec() })
            };

            let boot = Note {
                path: "/sys/boot".to_string(),
                mode: Mode::Add,
                slots: Tale::from_content(DiskPail::noun(Noun::null())),
            };


            let ovum = Ovum::Poke(Card::quad(root, sys_doc, reef, boot));
            self.queue.push_back(ovum);

            // info!("added init to queu");
        }
        for driver in pending {
            self.spawn_actor(driver, None, None);
        }

        let mut is_quiet = false;

        // XX: need scheduling refactor
        loop {
            if self.stop.load(std::sync::atomic::Ordering::Acquire) {
                return Ok(());
            }
            if let Some(ovo) = self.queue.pop_front() {
                is_quiet = false;
                self.step(ovo).await?;
                continue;
            }
            match self.supervisor_rx.try_recv() {
                Ok(cmd) => {
                    self.handle_supervisor_cmd(cmd);
                    continue;
                }
                Err(TryRecvError::Empty) => {}
                Err(e) => {
                    return Err(ShrineError::Mpsc(e));
                }
            }
            match self.inbox.try_recv() {
                Ok(commands) => {
                    self.enqueue(commands.into_iter());
                }
                Err(mpsc::error::TryRecvError::Empty) => {
                    if !is_quiet {
                        self.booted = true;
                        is_quiet = true;
                        for (_id,drv) in &self.drivers {
                            let _ = drv.tx.send(DriverSign::Quiet).await;
                        }
                    }
                    tokio::task::yield_now().await;
                    continue;
                }
                Err(e) => {
                    return Err(ShrineError::Mpsc(e));
                }
            }
        }
    }

    fn enqueue(&mut self, iter: impl DoubleEndedIterator<Item = DriverCard>) {
        let mut r = iter.rev();
        let Some(card) = r.next() else {
            return;
        };
        let mut res = card.note.into_noun_card(self.namespace());
        for item in r {
            let note = item.note.into_noun_card(self.namespace());
            res = Card::Pair(Box::new(note), Box::new(res));
        }
        self.queue.push_back(Ovum::Poke(res));
    }

    pub fn halt(&self) -> Arc<AtomicBool> {
        self.stop.clone()
    }
}
