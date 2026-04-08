use std::sync::Arc;

use milton::{Noun, cell, jets::{self, list}, traits::{FromNoun, NounConversionError}, trel};
use shrine_storage::{Note, store::Namespace};



#[derive(Debug, Clone)]
pub struct Fate {
  pub notes: Vec<Note>,
  pub effects: Noun,
  pub todo: Option<Noun>,
  pub jam: Option<Vec<u8>>
}

impl Fate {
  pub fn new(notes: Vec<Note>, effects: Noun, todo: Option<Noun>) -> Self {
    Self { notes, effects, todo, jam: None }
  }

  pub fn into_noun(&self, ns: Arc<Namespace>) -> Result<Noun, NounConversionError> {
    let notes = list::from_iter(self.notes.iter(), |note| note.to_noun(&ns).unwrap());
    let todo = self.todo.clone().map(|todo| cell(Noun::null(), todo).into_noun()).unwrap_or(Noun::null());
    Ok(trel(notes, self.effects.clone(), todo).into_noun())
  }

  pub fn from_noun(noun: Noun) -> Result<Self, NounConversionError> {
    let jam = noun.jam();
    let cell = noun.into_cell().map_err(|_e| NounConversionError::InvalidNoun)?;
    let (notes, rest) = cell.into_parts();
    let rest = rest.into_cell().map_err(|_e| {
      NounConversionError::InvalidNoun
    })?;
    let (_efs, todo) = rest.into_parts();
    if !todo.equals(&Noun::null()) {
      panic!("deferral not implemented");
    }
    let todo = None;
    let notes = jets::list::to_vec(notes, |n| {
      Note::from_noun(n).unwrap()
    });

    let effects = Noun::null();


    Ok(Self {
      notes,
      effects,
      todo,
      jam: Some(jam)
    })
  }
}
