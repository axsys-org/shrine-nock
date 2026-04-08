use std::collections::HashMap;
use std::path::Path;
use milton::Runtime;
use uuid::Uuid;

use crate::core::types::{Care, Note};
use crate::store::{Namespace, Result};

fn get_namespace() -> Result<Namespace> {
  let path_str = format!("/tmp/shrine-store-test-{}", Uuid::new_v4());
  let path = Path::new(&path_str);
  let namespace = Namespace::new_at_path(&path)?;
  Ok(namespace)
}

fn make(path: &str) -> Note {
  // TODO: Fix this to use proper Tale instead of HashMap
  todo!("make() needs to be updated to use Tale type")
}

fn poke(path: &str) -> Note {
  // TODO: Fix this to use proper Tale instead of HashMap
  todo!("poke() needs to be updated to use Tale type")
}

fn cull(path: &str) -> Note {
  Note::cull(path)
}

use std::sync::Mutex;
pub static MUTEX: Mutex<()> = Mutex::new(());


// #[test]
fn test_one() -> Result<()> {
  let mutex = &MUTEX;
  let _guard = mutex.lock().unwrap();
  let _runtime = Runtime::new(1 << 31).expect("failed to initialize runtime");
  let namespace = get_namespace()?;
  let mut first = vec![
    make("/one/two/three/four"),
    make("/one/two/three"),
    make("/one/two"),
    make("/one"),
    make("/"),
  ];
  // namespace.write(&mut first)?;
  // namespace.store().env().clear_stale_readers()?;
  // let txn = namespace.store().env().read_txn().unwrap();
  // namespace.store().ancestry().debug(&txn);

  let exe = namespace.look(Care::X, "/one/two/three")?;
  let why = namespace.look(Care::Y, "/one/two")?;
  let zed = namespace.look(Care::Z, "/one")?;

  {

    let txn = namespace.store().env().read_txn().unwrap();
    println!("why");
  namespace.store().why().debug(&txn);
    println!("zed");
  namespace.store().zed().debug(&txn);

  }
  println!("exe: {:?}", exe);
  println!("why: {:?}", why);
  println!("zed: {:?}", zed);

  let mut second = vec![
    poke("/one/two/three"),
  ];
  // namespace.write(&mut second)?;

{

    let txn = namespace.store().env().read_txn().unwrap();
    println!("why");
  namespace.store().why().debug(&txn);
    println!("zed");
  namespace.store().zed().debug(&txn);

  }

  println!("exe: {:?}", namespace.look(Care::X, "/one/two/three")?);
  println!("why: {:?}", namespace.look(Care::Y, "/one/two")?);
  println!("zed: {:?}", namespace.look(Care::Z, "/one")?);
  assert!(1 == 0);


  Ok(())
}