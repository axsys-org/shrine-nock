use shrine_storage::{core::types::Note, store::Namespace};
use milton::{Noun, cell, traits::{FromNoun, NounConversionError}};

use crate::react::News;

#[derive(Debug, Clone, PartialEq)]
pub enum Card {
  Pair(Box<Card>, Box<Card>),
  Note(Note)
}

impl Into<Card> for Note {
    fn into(self) -> Card {
        Card::Note(self)
    }
}

#[allow(dead_code)]
impl Card {
  pub fn trel(p: impl Into<Card>, q: impl Into<Card>, r: impl Into<Card>) -> Self {
      Self::pair(p.into(), Self::pair(q.into(),r.into()))
  }
  pub fn quad(p: impl Into<Card>, q: impl Into<Card>, r: impl Into<Card>, s: impl Into<Card>) -> Self {
      Self::pair(p.into(), Self::trel(q,r,s))
  }

  pub fn pair(l: Card, r: Card) -> Self {
      Self::Pair(Box::new(l), Box::new(r))
  }

  fn from_noun(non: Noun) -> Result<Self, NounConversionError> {
    let Some(cell) = non.as_cell() else {
      return Err(NounConversionError::InvalidNoun);
    };
    let is_list = cell.head().is_cell();
    if is_list {
      let head = Self::from_noun(cell.head().to_owned())?;
      let tail = Self::from_noun(cell.tail().to_owned())?;
      Ok(Self::Pair(Box::new(head), Box::new(tail)))
    } else {
      let note = Note::from_noun(non)?;
      Ok(Self::Note(note))
    }
  }

  fn from_iter_inner(next: Note, mut iter: impl Iterator<Item = Note>) -> Result<Self, NounConversionError> {
    // let one = iter.next().ok_or(NounConversionError::InvalidNoun)?;
    let next = Card::Note(next);
    let Some(two) = iter.next() else {
      return Ok(next);
    };

    Ok(Card::Pair(Box::new(next), Box::new(Self::from_iter_inner(two, iter)?)))

  }

  pub fn from_iter(mut iter: impl Iterator<Item = Note>) -> Result<Self, NounConversionError> {
    let next = iter.next().ok_or(NounConversionError::InvalidNoun)?;
    Self::from_iter_inner(next, iter)
  }

  fn to_noun(&self, ns: &Namespace) -> Result<Noun, NounConversionError> {
    match self {
      Card::Note(note) => note.to_noun(ns),
      Card::Pair(hed, tal) => Ok(cell(hed.to_noun(ns)?, tal.to_noun(ns)?).into_noun())
    }
  }
}


#[derive(Debug, Clone, PartialEq)]
pub enum Ovum {
  Poke(Card),
  Hear(News)
}







impl Ovum {
  pub fn into_noun(&self, namespace: &Namespace) -> Result<Noun, NounConversionError> {
    match self {
      Ovum::Poke(note) => {
        let tag = Noun::mote("vine");

        let note_noun = note.to_noun(namespace)?;
        Ok(cell(tag, note_noun).into_noun())
      }
      Ovum::Hear(news) => {
        let tag = Noun::mote("hear");
        let news_noun = news.into_noun(namespace)?;
        Ok(cell(tag, news_noun).into_noun())
      }
    }
  }
}
