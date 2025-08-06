#[cfg(feature = "perf-case-map")]
mod map;

pub trait CharToMonoLowercase {
    /// The only multi-char lowercase mapping is 'İ' -> "i\u{307}", we just ignore the '\u{307}'.
    fn to_mono_lowercase(self) -> char;
}

impl CharToMonoLowercase for char {
    fn to_mono_lowercase(self) -> char {
        #[cfg(not(feature = "perf-case-map"))]
        return self.to_lowercase().next().unwrap();

        // Optimize away the binary search
        // Reduce total match time by ~37%
        #[cfg(feature = "perf-case-map")]
        map::to_mono_lowercase(self)
    }
}

pub trait StrToMonoLowercase {
    fn to_mono_lowercase(&self) -> String;
}

impl StrToMonoLowercase for str {
    fn to_mono_lowercase(&self) -> String {
        self.chars().map(|c| c.to_mono_lowercase()).collect()
    }
}
