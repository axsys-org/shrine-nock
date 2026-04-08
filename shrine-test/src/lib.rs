//! # Shrine Test
//! This crate contains testing infrastructure for the related shrine-* crates, specifically Shrine Core
#![feature(async_fn_traits)]
use async_trait::async_trait;
use shrine_storage::{Gift, store::Namespace};
use std::{collections::VecDeque, path::Path, sync::Arc};
use tokio::{sync::mpsc, time::timeout};
use uuid::Uuid;

use shrine_core::{
    Care,
    driver::{DriverAck, DriverCard, DriverID, DriverNote, ShrineDriver, Subscription},
    shrine::Shrine,
    test_utils::TEST_MUTEX,
};
use tracing_appender::{
    non_blocking::{self, NonBlockingBuilder},
    rolling,
};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, prelude::*};
type StdError = Box<dyn std::error::Error + Send>;
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

pub async fn setup_test(millis: u64, f: impl AsyncFnOnce(&mut Shrine) -> ()) {
    let id = Uuid::new_v4();
    let location = format!("/tmp/{}", id);
    init_tracing(true);
    // println!("Test location: {location}");
    let path = Path::new(location.as_str());
    let _hold = TEST_MUTEX.lock().await;
    let mut shrine = Shrine::new(&path).expect("Failed to create new shrine");
    timeout(tokio::time::Duration::from_millis(millis), f(&mut shrine))
        .await
        .expect("Test timed out");
}

pub enum AssertStep {
    Result(AssertResult),
    Done
}

pub struct TestingDriver {
    id: DriverID,
    namespace: Arc<Namespace>,
    tx: Option<mpsc::Sender<Vec<DriverCard>>>,
    steps: VecDeque<TestStep>,
    snap_id: u32,
    card_id: u32,
    done: bool,
    asserts: mpsc::Sender<AssertStep>
}

impl TestingDriver {
    pub fn new(id: DriverID, namespace: Arc<Namespace>, asserts: mpsc::Sender<AssertStep>) -> Self {
        Self {
            id,
            namespace,
            tx: None,
            steps: VecDeque::new(),
            snap_id: 0,
            card_id: 0,
            done: false,
            asserts
        }
    }

    pub async fn run_poke(&mut self, poke: DriverCard) {
        let tx = self.tx.as_ref().unwrap();
        let _ = tx.send(vec![poke]).await;
    }

    async fn run_snap(&mut self, snap: Snapshot) {
        self.asserts.send(AssertStep::Result((snap.check)(self.namespace.clone()))).await.unwrap();
    }


    pub fn snap(&mut self, name: impl Into<String>, f: AssertFunc) {
        let id = self.snap_id;
        self.snap_id += 1;
        let snap = Snapshot { id, name: name.into(), check: f };
        self.steps.push_back(TestStep::Check(snap));
    }

    pub fn quiet(&mut self) {
        self.steps.push_back(TestStep::Quiet);
    }

    pub fn poke(&mut self, note: DriverNote) {
        let id = self.card_id;
        self.card_id += 1;
        let card = DriverCard { id, note };
        self.steps.push_back(TestStep::Poke(card));
    }

    pub async fn check_done(&mut self) {

        while let Some(next) = self.steps.pop_front_if(|s| !matches!(s, TestStep::Quiet)) {
            eprintln!("next {next:?}");
            match next {
                TestStep::Quiet => { unreachable!() },
                TestStep::Poke(poke) => self.run_poke(poke).await,
                TestStep::Check(snap) => self.run_snap(snap).await
            }
        }
        if self.steps.is_empty() && !self.done {
            eprintln!("sending done");
            let _ =  self.asserts.send(AssertStep::Done).await;
            self.done = true;
        }
    }

}

#[async_trait]
impl ShrineDriver for TestingDriver {
    fn id(&self) -> DriverID {
        self.id
    }

    fn subscribe(&self) -> shrine_core::driver::Subscription {
        let path_idx = self
            .namespace
            .get_path_id("/")
            .expect("Failed to get root path");

        Subscription {
            path: path_idx.raw(),
            care: Care::Z,
        }
    }

    fn set_tx(&mut self, tx: mpsc::Sender<Vec<DriverCard>>) {
        self.tx = Some(tx);
    }

    async fn on_start(&mut self) -> Result<(), StdError> {
        eprintln!("on_start testing driver");
        Ok(())
    }

    async fn on_dirty(&mut self, dirty: &[Gift]) -> Result<(), StdError> {
        eprintln!("on_dirty testing driver {dirty:?}");
        Ok(())
    }

    ///  Assumes that the driver cards are always processed in order, which is currently true
    ///
    async fn on_done(&mut self, ack: &DriverAck) -> Result<(), StdError> {
        eprintln!("on_dirty testing driver {ack:?}");
        Ok(())
    }

    async fn on_quiet(&mut self) -> Result<(), StdError> {
        eprintln!("onquiet before {:?}", self.steps);
        while let Some(_q) = self.steps.pop_front_if(|step| matches!(step, TestStep::Quiet)) {}
        self.check_done().await;
        eprintln!("onquiet after {:?}", self.steps);

        Ok(())
    }
}

pub type AssertFunc = Box<dyn FnOnce(Arc<Namespace>) -> AssertResult + Send>;
pub type AssertResult = Result<(), Box<dyn std::error::Error + Send>>;

pub struct Snapshot {
    id: u32,
    name: String,
    check: AssertFunc
}

impl std::fmt::Debug for Snapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Snapshot({})", self.name)?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum TestStep {
    Poke(DriverCard),
    Check(Snapshot),
    Quiet
}


#[cfg(test)]
mod tests {

    use std::{collections::HashMap, sync::atomic::{AtomicBool, Ordering}};

    use shrine_core::{DiskPail, Tale, driver::IntoTale, shrine::Germ};
    use shrine_storage::{Mode, slot};

    use super::*;

    async fn wakeup(flag: Arc<AtomicBool>, duration: tokio::time::Duration) {
        eprintln!("Going to bed!!");
        tokio::time::sleep(duration).await;
        eprintln!("Wakeup!!");
        flag.store(true, Ordering::Release);
    }

    #[derive(Debug, Clone)]
    struct DocsTale {
        lede: String,
        help: String
    }

    #[derive(Debug, Clone)]
    struct CounterTale {
        val: u32
    }

    impl CounterTale {
        pub fn new(val: u32) -> Self {
            Self {
                val
            }
        }
    }

    impl IntoTale for CounterTale {
        fn into_tale(&self, namespace: Arc<Namespace>) -> Tale {
            let mut ret = Tale::empty();
            ret.push(slot::CONTENT, DiskPail::word(self.val));
            return ret;
        }
    }

    #[derive(Debug, Clone)]
    pub struct SumTale {
        lhs: String,
        rhs: String
    }

    impl SumTale {
        pub fn new(lhs: String, rhs: String) -> Self {
            Self {
                lhs,
                rhs
            }
        }
    }

    impl IntoTale for SumTale {
        fn into_tale(&self, namespace: Arc<Namespace>) -> Tale {
            let mut ret = Tale::empty();
            let mut crew = HashMap::new();
            let lhs = namespace.get_path_id(self.lhs.as_ref()).unwrap();
            let rhs = namespace.get_path_id(self.rhs.as_ref()).unwrap();
            let add = namespace.get_path_id("/add").unwrap();
            crew.insert("lhs".to_string(), lhs);
            crew.insert("rhs".to_string(), rhs);

            ret.insert(slot::CREW, DiskPail::Mesh { data: crew });
            ret.insert(slot::KOOK, DiskPail::duct(&[add]));

            return ret;
        }
    }



    #[derive(Debug, Clone)]
    struct KookDef {
        code: String
    }

    const ADD_KOOK: &'static str = include_str!("./add.hoon");

    impl KookDef {
        pub fn add() -> Self {
            Self {
                code: ADD_KOOK.to_string()
            }
        }
    }

    impl IntoTale for KookDef {
        fn into_tale(&self, namespace: Arc<Namespace>) -> Tale {
            let mut ret = Tale::empty();
            // use bootstrap, no build system in test
            let boot = namespace.get_path_id("/sys/boot").unwrap();
            let duct = DiskPail::duct(&[boot]);
            ret.push(slot::KOOK, duct);
            ret.push(slot::SRC, DiskPail::hoon(self.code.clone()));
            return ret;
        }
    }

    impl DocsTale {
        pub fn new(lede: String, help: String) -> Self {
            Self {
                lede,
                help
            }

        }
    }

    impl IntoTale for DocsTale {
        fn into_tale(&self, _namespace: Arc<Namespace>) -> Tale {
            let mut res = Tale::empty();
            res.push(slot::LEDE, DiskPail::text(self.lede.clone()));
            res.push(slot::HELP, DiskPail::text(self.help.clone()));
            return res;
        }
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn react_test() {
        setup_test(40000, async |shrine| {
            eprintln!("running test");
            let harness_id = shrine.next_driver_id();
            let (tx, mut rx) = mpsc::channel::<AssertStep>(1024);
            let mut harness = TestingDriver::new(harness_id, shrine.namespace(), tx);
            harness.quiet();
            let add_kook = DriverNote { path: "/add".to_string(), mode: Mode::Add, slots: Arc::new(KookDef::add()) };
            harness.poke(add_kook);
            harness.quiet();
            let lhs = CounterTale::new(3);
            let rhs = CounterTale::new(5);

            harness.poke(DriverNote { path: "/lhs".to_string(), mode: Mode::Add, slots: Arc::new(lhs) });
            harness.poke(DriverNote { path: "/rhs".to_string(), mode: Mode::Add, slots: Arc::new(rhs) });
            harness.quiet();
            let sum_tale = SumTale::new("/lhs".to_string(), "/rhs".to_string());
            let sum = DriverNote { path: "/sum".to_string(), mode: Mode::Add, slots:  Arc::new(sum_tale) };
            harness.poke(sum);
            harness.quiet();
            harness.snap("check /sum", Box::new(|ns| {
                let res = ns.grab("/sum").unwrap();
                eprintln!("res {res:?}");
                Ok(())
            }));

            shrine.add_driver(Box::new(harness));
            let mut steps = vec![];
            let mut is_done = false;

            let duration = tokio::time::Duration::from_millis(35000);
            let germ = Some(Germ::default());
            let flag = shrine.halt();
            tokio::select! {
                _ = wakeup(flag, duration) => {
                    println!("Timeout");
                },
                _ = shrine.run(germ) => {},
            };
            loop {
                match rx.try_recv() {
                    Ok(AssertStep::Done) => { is_done = true; },
                    Ok(AssertStep::Result(r)) => { steps.push(r); },
                    Err(mpsc::error::TryRecvError::Empty) => {
                        break;
                    },
                    _ => {
                        panic!("Weird error")
                    }
                }
            }


            if !is_done {
                panic!("test never finished");
            }

            let errs = steps.into_iter().filter_map(|s| s.err()).collect::<Vec<_>>();
            if !errs.is_empty() {
                panic!("Assertions failed\n{}", errs.iter().map(|e| format!("- {}", e)).collect::<Vec<_>>().join("\n"));
            }

        })
        .await;
    }


    #[tokio::test(flavor = "multi_thread")]
    async fn write_test() {
        setup_test(20000, async |shrine| {
            eprintln!("running test");
            let harness_id = shrine.next_driver_id();
            let (tx, mut rx) = mpsc::channel::<AssertStep>(1024);
            let mut harness = TestingDriver::new(harness_id, shrine.namespace(), tx);
            harness.quiet();
            let tale = DocsTale::new("test".to_string(), "hello".to_string());
            let card = DriverNote { path: "/test".to_string(), mode: Mode::Add, slots:  Arc::new(tale) };
            harness.poke(card);
            harness.quiet();
            harness.snap("check /test", Box::new(|ns| {
                let res = ns.grab("/test").unwrap();
                eprintln!("res {res:?}");
                Ok(())
            }));

            shrine.add_driver(Box::new(harness));
            let mut steps = vec![];
            let mut is_done = false;

            let duration = tokio::time::Duration::from_millis(10000);
            let germ = Some(Germ::default());
            let flag = shrine.halt();
            tokio::select! {
                _ = wakeup(flag, duration) => {
                    println!("Timeout");
                },
                _ = shrine.run(germ) => {},
            };
            loop {
                match rx.try_recv() {
                    Ok(AssertStep::Done) => { is_done = true; },
                    Ok(AssertStep::Result(r)) => { steps.push(r); },
                    Err(mpsc::error::TryRecvError::Empty) => {
                        break;
                    },
                    _ => {
                        panic!("Weird error")
                    }
                }
            }


            if !is_done {
                panic!("test never finished");
            }

            let errs = steps.into_iter().filter_map(|s| s.err()).collect::<Vec<_>>();
            if !errs.is_empty() {
                panic!("Assertions failed\n{}", errs.iter().map(|e| format!("- {}", e)).collect::<Vec<_>>().join("\n"));
            }
        })
        .await;
    }

    #[test]
    fn new_test() {
        let want = 2;
        let have = 2;
        assert_eq!(want, have);
    }
}
