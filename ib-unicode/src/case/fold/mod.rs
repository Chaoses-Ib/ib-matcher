#[cfg(feature = "perf-case-fold")]
pub mod map;
// pub mod ptr_hash;
#[cfg(any(not(feature = "perf-case-fold"), feature = "bench"))]
pub mod unicase;
