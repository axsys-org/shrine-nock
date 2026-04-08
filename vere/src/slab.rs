use std::borrow::Cow;

use vere_sys::{u3i_slab, u3i_slab_free, u3i_slab_mint};

use crate::Noun;



pub struct Slab(*mut u3i_slab);



impl Slab {
    pub fn from_raw(ptr: *mut u3i_slab) -> Self {
        unsafe {
            Self(ptr)
        }
    }

    pub fn is_direct(&self) -> bool {
        unsafe {
            return (*self.0).len_w <= 1; }
    }

    pub fn get_direct(&self) -> u32 {
        unsafe {
            return (*self.0).__._sat_w;
        }
    }

    pub fn get_bytes(&self) -> Option<&[u8]> {
        unsafe {
            let ptr = (*self.0).__bindgen_anon_1.buf_y;
            if self.is_direct() {
                return None;
            }
            let len_w = (*self.0).len_w as usize;
            return Some(std::slice::from_raw_parts(ptr, len_w * 4));

        }
    }

    pub fn get_words(&self) -> Option<&[u32]> {
        unsafe {
            let ptr = (*self.0).__bindgen_anon_1.buf_w;
            if self.is_direct() {
                return None;
            };
            let len_w = (*self.0).len_w as usize;
            return Some(std::slice::from_raw_parts(ptr, len_w ));
        }
    }

    pub fn to_atom(self) -> Noun {
        unsafe {
            let noun = u3i_slab_mint(self.0);
            self.leak();
            return Noun::from_raw(noun);
        }
    }


    pub fn leak(self) {
        std::mem::forget(self);
    }
}

impl Drop for Slab {
    fn drop(&mut self) {
        unsafe {
            u3i_slab_free(self.0);
        }

    }

}
