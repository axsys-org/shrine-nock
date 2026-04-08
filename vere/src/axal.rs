use crate::cell;

use super::*;

pub struct Axal(Noun);

impl Axal {
    pub fn new() -> Self {
        Self(raw_empty())
    }

    pub fn insert(&mut self, key: Noun, val: Noun) {
        let next = raw_insert(self.0.clone(), key, val);
        self.0 = next;
    }

    pub fn get(&mut self, key: Noun) -> Option<Noun> {
        let res = raw_get(self.0.clone(), key);
        if let Ok((_hed, tel)) = res.into_pair() {
            Some(tel)
        } else {
            None
        }
    }

    pub fn delete(&mut self, _key: Noun) {}
}
pub fn raw_del(ax: Noun, key: Noun) -> Noun {
    unsafe {
        let res = vere_sys::u3qfo_del(ax.into_raw(), key.into_raw());
        Noun::from_raw(res)
    }
}

pub fn raw_empty() -> Noun {
    cell(Noun::null(), Noun::null()).into_noun()
}

pub fn raw_insert(noun: Noun, pax: Noun, val: Noun) -> Noun {
    unsafe {
        Noun::from_raw(vere_sys::u3qfo_put(
            noun.retain(),
            pax.retain(),
            val.retain(),
        ))
    }
}

pub fn raw_get(noun: Noun, pax: Noun) -> Noun {
    unsafe {
        let raw = vere_sys::u3qfo_get(noun.transfer(), pax.transfer());
        Noun::from_raw(raw)
    }
}
