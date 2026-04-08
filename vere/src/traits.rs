use std::{collections::HashMap, hash::Hash};

use thiserror::Error;
use crate::{Atom, jets};

use super::noun::Noun;

#[derive(Error, Debug)]
pub enum NounConversionError {
  #[error("Invalid noun")]
  InvalidNoun,
}

pub trait IntoNoun {
  fn into_noun(&self) -> Result<Noun, NounConversionError>;
}


pub trait FromNoun {
  fn from_noun(noun: Noun) -> Result<Self, NounConversionError> where Self: Sized;
}

impl IntoNoun for String {
  fn into_noun(&self) -> Result<Noun, NounConversionError> {
    let atom = Atom::from_bytes(self.as_bytes());
    Ok(atom.into_noun())
  }
}

impl FromNoun for String {
  fn from_noun(noun: Noun) -> Result<Self, NounConversionError> {
    unsafe {
      if let Some(atom) = noun.as_atom() {
        if atom.is_indirect() {
          let words = atom.as_words().unwrap();
          return Ok(String::from_utf8_unchecked(bytemuck::cast_slice::<u32, u8>(words).to_vec()))
        } else {
          return Ok(String::from_utf8_unchecked(atom.as_u32().unwrap().to_le_bytes().to_vec()))
        }
      } else {
        return Err(NounConversionError::InvalidNoun)
      }
    }
  }
}

impl<K: Hash + Eq + IntoNoun, V: IntoNoun> IntoNoun for HashMap<K, V> {
  fn into_noun(&self) -> Result<Noun, NounConversionError> {
    Ok(jets::map::from_iter(self.iter()))
  }
}

impl IntoNoun for Noun {
  fn into_noun(&self) -> Result<Noun, NounConversionError> {
    Ok(self.clone())
  }

}
impl FromNoun for Noun {
  fn from_noun(noun: Noun) -> Result<Self, NounConversionError> {
    Ok(noun)
  }
}

impl IntoNoun for u32 {
  fn into_noun(&self) -> Result<Noun, NounConversionError> {
    Ok(Atom::from_u32(*self).into_noun())
  }
}

impl FromNoun for u32 {
  fn from_noun(noun: Noun) -> Result<Self, NounConversionError> {
    Ok(noun.as_atom().unwrap().as_u32().unwrap())
  }
}
