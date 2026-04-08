use std::collections::HashMap;

use milton::{Noun, cell, jets, traits::{IntoNoun, NounConversionError}};
use roaring::RoaringBitmap;
use shrine_storage::{core::{path::{HuntIdx, PathIdx, path_noun_from_string}, types::{Care, Gift}}, store::Namespace};
use crate::driver::DriverID;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Reaction {
  Driver(DriverID),
  Path(PathIdx),
}

impl Default for Reaction {
  fn default() -> Self {
    Self::Driver(0)
  }
}


pub struct React {
  hunts: HashMap<HuntIdx, Vec<Reaction>>,
  /// Reverse index: watcher path → list of (care, target) pairs it's watching
  watches: HashMap<PathIdx, Vec<(Care, PathIdx)>>,
}




impl React {
  pub fn new() -> Self {
    Self { hunts: HashMap::new(), watches: HashMap::new() }
  }

  pub fn add_reaction(&mut self, care: Care, path: PathIdx, reaction: Reaction) {
self.hunts.entry(HuntIdx::new(care, path)).or_default().push(reaction);
  }

  pub fn remove_reaction(&mut self, care: Care, path: PathIdx, reaction: &Reaction) {
    let hunt = HuntIdx::new(care, path);
    if let Some(reactions) = self.hunts.get_mut(&hunt) {
      reactions.retain(|r| r != reaction);
    }
  }

  /// Remove all old watches for `watcher`, then register new ones as `Reaction::Path(watcher)`.
  pub fn set_watches(&mut self, watcher: PathIdx, targets: Vec<(Care, PathIdx)>) {
    self.clear_watches(watcher);
    for &(care, target) in &targets {
      self.add_reaction(care, target, Reaction::Path(watcher));
    }
    self.watches.insert(watcher, targets);
  }

  /// Remove all reactions for `watcher` (called on delete or crew slot removal).
  pub fn clear_watches(&mut self, watcher: PathIdx) {
    if let Some(old) = self.watches.remove(&watcher) {
      for (care, target) in old {
        self.remove_reaction(care, target, &Reaction::Path(watcher));
      }
    }
  }

  pub fn run(&self, care: Care, path: PathIdx) -> Vec<Reaction> {
    let hunt = HuntIdx::new(care, path);
    self.hunts.get(&hunt).cloned().unwrap_or_default()
  }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Word {
  pub fore: RoaringBitmap,
  pub gift: Gift
}

impl Word {
  pub fn new(gift: Gift) -> Self {
    Self { fore: RoaringBitmap::new(), gift: gift }
  }
}

#[derive(Debug, Clone, PartialEq)]
pub struct News(HashMap<PathIdx, Word>);

impl News {
  pub fn is_empty(&self) -> bool {
    self.0.is_empty()
  }
  pub fn new() -> Self {
    Self(HashMap::new())
  }

  pub fn add_word(&mut self, path: PathIdx, word: Word) {
    self.0.insert(path, word);
  }

  pub fn add_gift(&mut self, fore: PathIdx, gift: Gift) {
    let word = self.0.entry(gift.path).or_insert_with(|| Word::new(gift.clone()));
    word.fore.insert(fore.raw());
    word.gift = gift;
  }

  pub fn get_word(&self, path: PathIdx) -> Option<&Word> {
    self.0.get(&path)
  }

  pub fn words(&self) -> impl Iterator<Item = &Word> {
    self.0.values().into_iter()
  }

  pub fn into_noun(&self, store: &Namespace) -> Result<Noun, NounConversionError> {
    let mut result = cell(Noun::null(), Noun::null()).into_noun();
    for (path, word) in self.0.iter() {
      let path = path_noun_from_string(&store.path_idx_to_str(&path).unwrap().expect("path not found").as_str());
      let fore = jets::set::from_iter_noun(word.fore.iter().map(|fore| {
        let fore_str = store.path_idx_to_str(&PathIdx::new(fore)).unwrap().expect("fore not found");
        let non = path_noun_from_string(&fore_str.as_str());
        return non;
      }));
      let gift = word.gift.into_noun().unwrap();
      let val = cell(fore, gift).into_noun();
      result = jets::axal::insert(result, path, val);
    }
    Ok(result)
  }
}

