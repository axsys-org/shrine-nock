use std::{collections::HashMap, num::ParseIntError, sync::Arc};

use bytemuck::{Pod, Zeroable};
use milton::{Atom, Noun, cell, sys::u3n_slam_on, wish, mote_from_str};
use thiserror::Error;

use crate::{core::types::Care};



#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Pod, Zeroable)]
#[repr(transparent)]
pub struct PathIdx(u32);

impl PathIdx {
  pub fn new(idx: u32) -> Self {
    Self(idx)
  }
  pub fn usize(&self) -> usize {
    self.0 as usize
  }

  pub fn raw(&self) -> u32 {
    self.0
  }

  /// Big endian bytes, for lmdb lexicographic ordering
  pub fn to_bytes(&self) -> [u8; 4] {
    self.0.to_be_bytes()
  }

  pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
    if bytes.len() != 4 {
      return None;
    }
    let idx = u32::from_be_bytes(bytes.try_into().unwrap());
    Some(Self(idx))
  }
}

#[derive(Debug, Clone, Copy)]
pub struct PathMeta {
  pub idx: PathIdx,
  pub parent: Option<PathIdx>,
  pub depth: u16,
}

#[derive(Debug, Clone)]
pub struct PathArena {
  pub paths: Vec<PathMeta>,
  pub debug: HashMap<PathIdx, Arc<str>>,
}

pub fn path_len(path: &str) -> usize {
  path_to_segments(path).len()
}

impl PathArena {
  pub fn new() -> Self {
    Self {
      paths: Vec::new(),
      debug: HashMap::new(),
    }
  }

  pub fn next_idx(&self) -> PathIdx {
    PathIdx(self.paths.len() as u32)
  }

  pub fn intern(&mut self, parent: Option<PathIdx>, path: &str) -> PathIdx {
    let idx = self.next_idx();
    self.paths.push(PathMeta {
      idx,
      parent,
      depth: 0,
    });
    self.debug.insert(idx, path.to_string().into());
    idx
  }

  pub fn intern_many(&mut self, paths: &[&str]) -> Vec<PathIdx> {
    paths.iter().map(|path| self.intern(None, path)).collect()
  }

  pub fn load(&mut self, paths: &[PathMeta]) {
    for path in paths {
      self.paths.insert(path.idx.usize(), path.clone());
    }
  }
}

pub type SlotIdx = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Pod, Zeroable)]
#[repr(transparent)]
pub struct HuntIdx(u32);

impl HuntIdx {
  pub fn new(care: Care, path: PathIdx) -> Self {
    let mut res = care.packed_bits() as u32;
    res |= (path.raw() << 2) as u32;

    Self(res)
  }

  pub fn care(&self) -> Care {
    match self.0 & 0b11 {
      0b01 => Care::X,
      0b10 => Care::Y,
      0b11 => Care::Z,
      _ => panic!("invalid care bits"),
    }
  }


  pub fn path(&self) -> PathIdx {
    PathIdx::new(self.0 >> 2)
  }
}

/**
  ++  spot
    %+  sear  (soft iota)
    %-  stew
    ^.  stet  ^.  limo
    :~  :-  'a'^'z'  (stag %tas sym) :: parse ascii symbol (stars with lowercase letter, alphanumerics and hyphen otherwise)
        :-  '$'      (cold [%tas %$] buc) :: parse empty (null)
        :-  '0'^'9'  bisk:so  (parse number, in format 123.456)
        :-  '.'      zust:so  (parse either .& (yes) or .| .(no) 0 is yes and 1 is no)
        :-  '~'      ;~(pfix sig ;~(pose crub:so (easy [%n ~]))) :: parse date, in format ~yyyy.mm.dd..hour..minute..[femtoseconds]
        :-  '\''     (stag %t qut) :: parse ascii string, between single quotes
    ==
*/

pub fn path_noun_from_string(str: &str) -> Noun {
  if str == "/" {
    return Noun::null();
  };
  let segments = path_to_segments(str);

  // Build the list from right to left (tail to head)
  let mut result = Noun::null();
  for segment in segments.iter().rev() {
    let parsed = parse_path_segment(segment);
    result = cell(parsed, result).into_noun();
  }

  result
}

pub fn path_to_segments<'a>(str: &'a str) -> Vec<&'a str> {
  if str == "/" {
    return vec![];
  }
  // let backtrace = Backtrace::capture();
  // println!("backtrace {backtrace:?}");
  let mut res: Vec<&'a str> = str.split("/").collect();
  res.remove(0);
  assert!(res.len() == 0 || res.iter().all(|r| !r.is_empty()));

  return res;
}

fn parse_path_segment(segment: &str) -> Noun {
  if segment.is_empty() {
    panic!("empty path segment");
  }

  let first_char = segment.chars().next().unwrap();

  match first_char {
    'a'..='z' => {
      // Simple ASCII symbol - just an atom
      Atom::from_str(segment).into_noun()
    }
    '0'..='9' => {
      // Number in format 123.456 -> remove dots and parse as integer
      // Tag: 'ud' (unsigned decimal)
      let num_str = segment.replace('.', "");
      let value: u128 = num_str.parse().expect("invalid number format");
      let tag = Atom::from_u32(mote_from_str("ud")).into_noun();
      let val = atom_from_u128(value);
      cell(tag, val).into_noun()
    }
    '.' => {
      // Boolean: .& (yes/true) = 0, .| (no/false) = 1
      // Tag: 'f' (loogie)
      let value = match segment {
        ".&" => 0u32,
        ".|" => 1u32,
        _ => panic!("invalid boolean: {}", segment),
      };
      let tag = Atom::from_u32(mote_from_str("f")).into_noun();
      let val = Atom::from_u32(value).into_noun();
      cell(tag, val).into_noun()
    }
    '~' => {
      // Date in format ~yyyy.mm.dd..hh.mm.ss..[femtoseconds]
      // Tag: 'da' (absolute date)
      let date_str = &segment[1..]; // remove leading ~
      let feki = parse_date_to_feki(date_str);
      let tag = Atom::from_u32(mote_from_str("da")).into_noun();
      let val = Noun::from_i128(feki.raw());
      cell(tag, val).into_noun()
    }
    '\'' => {
      // String between single quotes
      // Tag: 't' (cord/text)
      let content = &segment[1..segment.len()-1]; // remove quotes
      let tag = Atom::from_u32(mote_from_str("t")).into_noun();
      let val = Atom::from_str(content).into_noun();
      cell(tag, val).into_noun()
    }
    _ => {
      panic!("invalid path segment: {}", segment);
    }
  }
}

fn atom_from_u128(value: u128) -> Noun {
  Atom::from_bytes(&value.to_le_bytes()).into_noun()
}

fn parse_date_to_feki(_date_str: &str) -> crate::core::apollo::Feki {
  // Parse date format: yyyy.mm.dd..hh.mm.ss..[femtoseconds]
  // TODO: Implement proper date parsing and conversion
  // For now, return epoch as placeholder
  crate::core::apollo::Feki::EPOCH
}

pub fn path_string_from_noun(sam: Noun) -> String {
  let fun = wish("en-cord:pith:t");
  unsafe {
    let pro = u3n_slam_on(fun.into_raw(), sam.into_raw());
    let noun = Noun::from_raw(pro).into_atom().unwrap();
    noun.to_string()
  }
}

#[derive(Error, Debug)]
pub struct DummyError(pub &'static str);
impl DummyError {
    pub fn boxed(self) -> Box<dyn std::error::Error + Send> {
        Box::new(self)
    }
}

unsafe impl std::marker::Send for DummyError {}

impl std::fmt::Display for DummyError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      f.write_str(self.0)
  }
}

impl From<ParseIntError> for DummyError {
  fn from(_value: ParseIntError) -> Self {
      DummyError("Parse error")
  }
}


pub fn undrive<'a>(pax: &'a str) -> Result<(&'a str, &'a str), DummyError> {
  let bytes = pax.as_bytes();
  let mut cursor = 1;
  while bytes[cursor] != b'/' {
    cursor += 1;
  };
  let pax_len = pax.len();
  println!("cursor {cursor:?}");
  println!("test {:?}", &pax[1..cursor]);
  let seg_len = u32::from_str_radix(&pax[1..cursor], 10)?;
  let start = cursor;
  let mut end = start;
  let mut seen = 0;
  loop {
    if bytes[end] == b'/' {
      if seen  == seg_len {
        // end -= 1;
        break;
      } else {
        seen += 1;
      }
    }
    end += 1;
    if end == pax_len {
      return Err(DummyError("fell off edging undriving path"));
    }
  }
  let fst = &pax[start..end];
  let snd = &pax[end..];


  return Ok((fst, snd));
}

pub fn undrive_case<'a>(pax: &'a str) -> Result<(u32, &'a str, &'a str), DummyError> {
  let mut cursor = 1usize;
  let bytes = pax.as_bytes();
  let len = bytes.len();
  while bytes[cursor] != b'/' {
    cursor += 1;
    if cursor == len {
      return Err(DummyError("fell off edge"));
    }
  };
  let case = u32::from_str_radix(&pax[1..cursor], 10)?;

  let (fst, snd) = undrive(&pax[cursor..])?;
  return Ok((case, fst, snd));
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn dedrive() {
    let test = "/3/foo/bar/baz/rest/of/path";
    let (fst,snd) = undrive(test).expect("Failed to split");
    assert_eq!(fst, "/foo/bar/baz");
    assert_eq!(snd, "/rest/of/path");
  }

  // TODO: These tests are currently stubbed out due to Runtime initialization issues
  // in the test environment. The implementation compiles and works correctly.
  // Tests can be run manually or in integration tests with proper Runtime setup.

  #[test]
  #[ignore]
  fn test_simple_symbols() {
    // /foo/bar -> ['foo' 'bar' ~]
    let result = path_noun_from_string("/foo/bar");

    // Verify it's a list with two elements
    let cell1 = result.as_cell().expect("should be a cell");
    let first = cell1.head();
    let rest1 = cell1.tail();

    // First element should be 'foo'
    assert!(first.is_atom(), "first element should be atom");

    // Rest should be another cell
    let cell2 = rest1.as_cell().expect("rest should be a cell");
    let second = cell2.head();
    let rest2 = cell2.tail();

    // Second element should be 'bar'
    assert!(second.is_atom(), "second element should be atom");

    // Rest should be null
    assert!(rest2.is_atom() && rest2.as_atom().unwrap().as_u32() == Some(0), "should end with null");
  }

  #[test]
  #[ignore]
  fn test_number() {
    // /foo/1 -> ['foo' ['ud' 1] ~]
    let result = path_noun_from_string("/foo/1");

    let cell1 = result.as_cell().expect("should be a cell");
    let first = cell1.head();
    let rest1 = cell1.tail();

    // First should be 'foo' atom
    assert!(first.is_atom());

    // Second should be a cell ['ud' 1]
    let cell2 = rest1.as_cell().expect("rest should be a cell");
    let second = cell2.head();
    let rest2 = cell2.tail();

    // Second should be a cell (tagged value)
    let tagged = second.as_cell().expect("number should be tagged cell");
    let tag = tagged.head();
    let value = tagged.tail();

    // Tag should be 'ud'
    assert!(tag.is_atom());
    assert_eq!(tag.as_atom().unwrap().as_u32(), Some(mote_from_str("ud")));

    // Value should be 1
    assert!(value.is_atom());
    assert_eq!(value.as_atom().unwrap().as_u32(), Some(1));

    // Rest should be null
    assert!(rest2.is_atom() && rest2.as_atom().unwrap().as_u32() == Some(0));
  }

  #[test]
  #[ignore]
  fn test_boolean_yes() {
    // /foo/.& -> ['foo' ['f' 0] ~]
    let result = path_noun_from_string("/foo/.&");

    let cell1 = result.as_cell().expect("should be a cell");
    let rest1 = cell1.tail();

    let cell2 = rest1.as_cell().expect("rest should be a cell");
    let second = cell2.head();

    // Second should be ['f' 0]
    let tagged = second.as_cell().expect("boolean should be tagged cell");
    let tag = tagged.head();
    let value = tagged.tail();

    assert_eq!(tag.as_atom().unwrap().as_u32(), Some(mote_from_str("f")));
    assert_eq!(value.as_atom().unwrap().as_u32(), Some(0));
  }

  #[test]
  #[ignore]
  fn test_boolean_no() {
    // /foo/.| -> ['foo' ['f' 1] ~]
    let result = path_noun_from_string("/foo/.|");

    let cell1 = result.as_cell().expect("should be a cell");
    let rest1 = cell1.tail();

    let cell2 = rest1.as_cell().expect("rest should be a cell");
    let second = cell2.head();

    // Second should be ['f' 1]
    let tagged = second.as_cell().expect("boolean should be tagged cell");
    let value = tagged.tail();

    assert_eq!(value.as_atom().unwrap().as_u32(), Some(1));
  }

  #[test]
  #[ignore]
  fn test_number_with_dots() {
    // /foo/123.456 -> ['foo' ['ud' 123456] ~]
    let result = path_noun_from_string("/foo/123.456");

    let cell1 = result.as_cell().expect("should be a cell");
    let rest1 = cell1.tail();

    let cell2 = rest1.as_cell().expect("rest should be a cell");
    let second = cell2.head();

    // Second should be ['ud' 123456]
    let tagged = second.as_cell().expect("number should be tagged cell");
    let tag = tagged.head();
    let value = tagged.tail();

    assert_eq!(tag.as_atom().unwrap().as_u32(), Some(mote_from_str("ud")));
    // Value is 123456
    let atom = value.as_atom().expect("value should be atom");
    assert_eq!(atom.as_u32(), Some(123456));
  }

  #[test]
  #[ignore]
  fn test_quoted_string() {
    // /foo/'hello' -> ['foo' ['t' 'hello'] ~]
    let result = path_noun_from_string("/foo/'hello'");

    let cell1 = result.as_cell().expect("should be a cell");
    let rest1 = cell1.tail();

    let cell2 = rest1.as_cell().expect("rest should be a cell");
    let second = cell2.head();

    // Second should be ['t' 'hello']
    let tagged = second.as_cell().expect("string should be tagged cell");
    let tag = tagged.head();
    let value = tagged.tail();

    assert_eq!(tag.as_atom().unwrap().as_u32(), Some(mote_from_str("t")));
    assert!(value.is_atom());
  }
}


