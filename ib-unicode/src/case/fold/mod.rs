#[cfg(feature = "perf-case-fold")]
pub mod map;
#[cfg(any(not(feature = "perf-case-fold"), feature = "bench"))]
pub mod unicase;
