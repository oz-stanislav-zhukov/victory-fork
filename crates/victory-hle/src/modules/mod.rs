//! Implementations, one module per area of the console API.
//!
//! Adding an API: write an `extern "system"` function named exactly like
//! the import, then list it with `hle_export!` in the module's `module()`.
//! Run `victory census` over your titles to see what is still missing.
//!
//! Planned areas (driven by census results, not by completeness):
//! `file` (package paths), `thread` (TLS callbacks for title threads),
//! `xaudio2`, `xstore` stubs.

#[cfg(windows)]
pub mod apu;
/// Portable sample-rate conversion used by the Windows-gated APU.
pub(crate) mod apu_resample;
pub mod audio;
pub mod d3d12x;
pub mod display;
pub mod dsp;
#[cfg(windows)]
pub mod dstorage;
#[cfg(windows)]
pub mod files;
pub mod gameinput;
pub mod kernel;
#[cfg(windows)]
pub mod lifecycle;
#[cfg(windows)]
pub mod media;
#[cfg(windows)]
pub mod net;
pub mod speech;
pub mod storage;
pub mod xasync;
pub mod xg;
pub mod xgame;
pub mod xgameruntime;
pub mod xgamesave;
pub mod xgameui;
#[cfg(windows)]
pub mod xmem;
pub mod xnetworking;
pub mod xpackage;
pub mod xstore;
pub mod xsystem;
pub mod xuser;

use std::sync::atomic::{AtomicU32, Ordering};

use crate::HleModule;

/// Takes one reference off a COM count: the count left, or `None` when
/// none was left (the title released once too often). Then the count
/// stays 0 and the caller frees nothing a second time.
pub(crate) fn release_ref(refs: &AtomicU32, object: &'static str) -> Option<u32> {
    match refs.fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| n.checked_sub(1)) {
        Ok(n) => Some(n - 1),
        Err(_) => {
            tracing::warn!(object, "released with no references left");
            None
        }
    }
}

pub(crate) fn all() -> Vec<HleModule> {
    let mut all = vec![
        kernel::module(),
        xgameruntime::module(),
        gameinput::module(),
        d3d12x::module(),
        audio::module(),
        dsp::module(),
        xg::module(),
    ];
    #[cfg(windows)]
    all.push(apu::module());
    #[cfg(windows)]
    all.push(media::module());
    #[cfg(windows)]
    all.push(dstorage::module());
    #[cfg(windows)]
    all.push(files::module());
    #[cfg(windows)]
    all.push(lifecycle::module());
    #[cfg(windows)]
    all.push(net::module());
    #[cfg(windows)]
    all.push(xmem::module());
    #[cfg(windows)]
    all.push(xsystem::module());
    all
}
