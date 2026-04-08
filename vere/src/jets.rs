use super::noun::Noun;
  use crate::traits::IntoNoun;

pub mod set {
  use vere_sys::u3qdi_put;
  use super::*;

  pub fn from_iter<'a, T: 'a + IntoNoun>(iter: impl Iterator<Item = &'a T>) -> Noun {
    let mut set = Noun::null();
    for item in iter {
      set = unsafe { 
        let raw = u3qdi_put(set.retain(), item.into_noun().unwrap().retain());
      Noun::from_raw(raw)
    }
    }
    set
  }

  pub fn from_iter_noun(iter: impl Iterator<Item = Noun>) -> Noun {
    let mut set = Noun::null();
    for item in iter {
      set = unsafe { 
        let raw = u3qdi_put(set.retain(), item.retain());
      Noun::from_raw(raw)
    }
    }
    set

  }
}
pub mod map {
  use std::{collections::HashMap, hash::Hash};

use vere_sys::{u3qdb_put, u3qdi_tap};

  use crate::traits::IntoNoun;

use super::*;
pub fn merge(a: Noun, b: Noun) -> Noun {
  unsafe {
    Noun::from_raw(vere_sys::u3qdb_uni(a.retain(), b.retain()))
  }
}

  pub fn to_vec<K,V>(treap: Noun, f: impl Fn(Noun, Noun) -> (K, V)) -> Vec<(K, V)> {
    let list = unsafe { Noun::from_raw(u3qdi_tap(treap.retain())) };
    let list = super::list::to_vec(list, |n| {
      let cell = n.as_cell().unwrap();
      f(cell.head().to_owned(), cell.tail().to_owned())
    });
    list
  }

  pub fn from_iter<'a, K: 'a + IntoNoun,V: 'a + IntoNoun>(iter: impl Iterator<Item = (&'a K, &'a V)>) -> Noun {
    let mut treap = Noun::null();
    for (k, v) in iter {
      treap = unsafe { 
        let raw = u3qdb_put(treap.retain(), k.into_noun().unwrap().retain(), v.into_noun().unwrap().retain());
      Noun::from_raw(raw)
     }
    }
    treap
  }

  pub fn to_hashmap<K: Eq + Hash,V>(treap: Noun, f: impl Fn(Noun, Noun) -> (K, V)) -> HashMap<K, V> {
    let list = to_vec(treap, f);
    let mut map = HashMap::new();
    for (k, v) in list {
      map.insert(k, v);
    }
    map
  }

  pub fn insert(treap: Noun, key: Noun, value: Noun) -> Noun {
    unsafe {
      Noun::from_raw(vere_sys::u3qdb_put(treap.retain(), key.retain(), value.retain()))
    }
  }
    

}

pub mod list {
  use crate::cell;

use super::*;

  pub fn to_vec<T>(list: Noun, f: impl Fn(Noun) -> T) -> Vec<T> {
    let mut vec = Vec::new();
    if list.is_atom() {
      assert!(list.as_raw() == 0);
      return vec![];
    }

    let mut current = list;

    loop {
      let (head, tail) = {
        match current.borrow().as_cell() {
          Some(cell) => (cell.head().to_owned(), cell.tail().to_owned()),
          None => break,
        }
      };
      vec.push(f(head));
      current = tail;
    }
    vec
  }

  pub fn from_iter<'a, T: 'a>(iter: impl Iterator<Item = &'a T> + DoubleEndedIterator, f: impl Fn(&'a T) -> Noun) -> Noun {
    let mut list = Noun::null();
    for item in iter.rev() {
      list = cell(f(item), list).into_noun();
    }
    list
  }
}

pub mod axal {
use crate::cell;

use super::*;

  pub fn empty() -> Noun {
    cell(Noun::null(), Noun::null()).into_noun()
  }

  pub fn insert(noun: Noun, pax: Noun, val: Noun) -> Noun {
    unsafe {
      Noun::from_raw(vere_sys::u3qfo_put(noun.retain(), pax.retain(), val.retain()))
    }
  }

  pub fn get(noun: Noun, pax: Noun) -> Option<Noun> {
    unsafe {
      let raw = vere_sys::u3qfa_get(noun.transfer(), pax.transfer());
      Noun::from_weak(raw)
    }
  }
}

pub mod serial {


}