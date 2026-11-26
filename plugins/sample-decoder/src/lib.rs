//! A sample decoder plugin (#173).
//!
//! Frame: four bytes, big-endian. A `u16` battery voltage in 0.01 V and an
//! `i16` battery current in 0.1 A. Anything else is declined as malformed.
//! Quantity 0 is the voltage and quantity 1 the current, as the host's
//! manifest says.
//!
//! It imports nothing and uses no allocator crate: a fixed arena serves the
//! host's `cabi_realloc` calls. The canonical ABI is written by hand here so
//! the build needs no binding generator; the layout is described in
//! `crates/sawbones-plugin-host/tests/support/mod.rs`.

#![no_std]

use core::cell::UnsafeCell;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    core::arch::wasm32::unreachable()
}

const ARENA_BYTES: usize = 8192;

#[repr(C, align(8))]
struct Arena {
    bytes: UnsafeCell<[u8; ARENA_BYTES]>,
    used: UnsafeCell<usize>,
}

// One thread: wasm32 without threads.
unsafe impl Sync for Arena {}

static ARENA: Arena = Arena {
    bytes: UnsafeCell::new([0; ARENA_BYTES]),
    used: UnsafeCell::new(0),
};

#[repr(C, align(8))]
struct Obs {
    quantity: u16,
    _pad: [u8; 6],
    mantissa: i64,
    exponent: i8,
    _pad2: [u8; 7],
}

/// The canonical ABI return area for `result<list<observation>, decode-fault>`.
#[repr(C, align(4))]
pub struct RetArea {
    tag: u8,
    _pad: [u8; 3],
    a: u32,
    b: u32,
}

struct Out {
    ret: UnsafeCell<RetArea>,
    obs: UnsafeCell<[Obs; 2]>,
}

unsafe impl Sync for Out {}

const ZERO_OBS: Obs = Obs {
    quantity: 0,
    _pad: [0; 6],
    mantissa: 0,
    exponent: 0,
    _pad2: [0; 7],
};

static OUT: Out = Out {
    ret: UnsafeCell::new(RetArea {
        tag: 0,
        _pad: [0; 3],
        a: 0,
        b: 0,
    }),
    obs: UnsafeCell::new([ZERO_OBS, ZERO_OBS]),
};

/// Bump allocation from the arena; traps when it is full.
#[no_mangle]
pub unsafe extern "C" fn cabi_realloc(_old: *mut u8, _old_size: usize, align: usize, size: usize) -> *mut u8 {
    let used = &mut *ARENA.used.get();
    let start = (*used + align - 1) & !(align - 1);
    if start + size > ARENA_BYTES {
        core::arch::wasm32::unreachable()
    }
    *used = start + size;
    (ARENA.bytes.get() as *mut u8).add(start)
}

#[no_mangle]
pub unsafe extern "C" fn decode(ptr: *const u8, len: usize) -> *const RetArea {
    let frame = core::slice::from_raw_parts(ptr, len);
    let ret = &mut *OUT.ret.get();
    if frame.len() != 4 {
        ret.tag = 1;
        ret.a = 0; // malformed
        return ret;
    }
    let volts = u16::from_be_bytes([frame[0], frame[1]]);
    let amps = i16::from_be_bytes([frame[2], frame[3]]);
    let obs = &mut *OUT.obs.get();
    obs[0].quantity = 0;
    obs[0].mantissa = i64::from(volts);
    obs[0].exponent = -2;
    obs[1].quantity = 1;
    obs[1].mantissa = i64::from(amps);
    obs[1].exponent = -1;
    ret.tag = 0;
    ret.a = obs.as_ptr() as u32;
    ret.b = 2;
    ret
}
