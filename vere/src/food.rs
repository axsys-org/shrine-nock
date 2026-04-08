use core::slice;
use std::{mem::MaybeUninit, ptr::null_mut, sync::atomic::AtomicU32};

use bytemuck::{AnyBitPattern, cast_slice};
use num::{PrimInt, Unsigned};
use vere_sys::{u3f_chow, u3f_chow_create, u3f_chow_fill, u3f_chow_free, u3f_chow_put, u3f_jim, u3f_mew, u3i_slab};

use crate::{NONE, Noun, slab::Slab};




pub struct ChowInner(*mut u3f_chow, AtomicU32);
pub struct Chow(*mut ChowInner);

unsafe impl Send for Chow {}
unsafe impl Sync for Chow {}

impl ChowInner {
    pub fn count(&self) -> u32 {
        let val: u32 = self.1.load(std::sync::atomic::Ordering::SeqCst);
        return val;
    }

    pub fn gain(&mut self) {
        let old = self.1.get_mut();
        *old += 1;
    }

    pub fn lose(&mut self) {
        let rc = self.1.get_mut();
        assert!(*rc > 0);
        *rc -= 1;
    }

    pub fn new() -> Self {
        unsafe {
            Self(u3f_chow_create(), AtomicU32::new(1))
        }
    }

    pub fn dispose(&self) {
        unsafe {
            u3f_chow_free(self.0);
        }
    }

    pub fn inner(&self) -> *mut u3f_chow {
        self.0
    }
}

impl Clone for Chow {
    fn clone(&self) -> Self {
        unsafe {
            (*self.0).gain();
            return Self(self.0)
        }
    }
}

impl Drop for Chow {
    fn drop(&mut self) {
        unsafe {
            (*self.0).lose();
            if (*self.0).count() == 0 {
                (*self.0).dispose();
            }
        }
    }
}

impl Chow {
    pub fn new() -> Chow {
        unsafe {
            let bxed = Box::new(ChowInner::new());
            return Self(Box::leak(bxed));
        }
    }

    fn handle(&self) -> *mut u3f_chow {
        unsafe {
            (*self.0).inner()
        }
    }

    pub fn put(&self, val: Noun) {
        unsafe {
            u3f_chow_put(self.handle(), val.into_raw());
        }
    }

    pub fn fill(&self, val: Noun) {
        unsafe {
            u3f_chow_fill(self.handle(), val.retain());
        }

    }

    unsafe fn as_ptr(&self) -> *mut u3f_chow {
        return self.handle();
    }


}



pub fn jim<N: Unsigned + PrimInt + AnyBitPattern>(chow: &Chow, noun: Noun) -> Vec<N> {
    unsafe {
        let mut slab_ptr = MaybeUninit::<u3i_slab>::uninit();
        let mut direct_bytes = [0u8; 4];

        let _ = u3f_jim(chow.as_ptr(), slab_ptr.as_mut_ptr(), noun.retain());
        let slab = Slab::from_raw(slab_ptr.as_mut_ptr());
        let slice = slab.get_bytes().unwrap_or_else(|| {
            let bytes = slab.get_direct().to_le_bytes();
            direct_bytes.copy_from_slice(&bytes);
            &direct_bytes
        });
        let vals: &[N] = cast_slice(slice);

        return vals.to_vec();
    }
}

pub fn mew(chow: &Chow, noun: Noun) -> Option<Noun> {
    unsafe {
        let res = u3f_mew(chow.as_ptr(), noun.retain());
        if res == NONE {
            return None;
        }
        return Some(Noun::from_raw(res));
    }
}






