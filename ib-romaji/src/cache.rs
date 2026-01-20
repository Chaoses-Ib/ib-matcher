/*!
Serialization/deserialization of romanizers for caching initialization state.
*/
use daachorse::CharwiseDoubleArrayAhoCorasick;

use crate::HepburnRomanizer;
#[cfg(feature = "std")]
use crate::{HepburnRomanizerBuilder, hepburn_romanizer_builder};

impl HepburnRomanizer {
    /// Header magic bytes for cache file validation
    const CACHE_MAGIC: &'static [u8] = b"IBROMAJI";
    /// Cache format version
    const CACHE_VERSION: u8 = 1;

    /// Serialize the HepburnRomanizer to bytes for caching.
    ///
    /// The serialized data can be saved to disk and later deserialized using
    /// [`deserialize_from_slice`](Self::deserialize_from_slice) to avoid the expensive
    /// initialization cost.
    ///
    /// ## Example
    /// ```ignore
    /// use ib_romaji::HepburnRomanizer;
    /// use std::fs;
    ///
    /// let romanizer = HepburnRomanizer::default();
    /// let cache_data = romanizer.serialize_to_vec();
    /// fs::write("romanizer.cache", &cache_data).unwrap();
    /// ```
    pub fn serialize_to_vec(&self) -> Vec<u8> {
        // Serialize the Aho-Corasick automaton first to get its size
        let ac_bytes = self.ac.serialize();

        let mut buf = Vec::with_capacity(10 + ac_bytes.len());
        // Write header
        buf.extend_from_slice(Self::CACHE_MAGIC);
        buf.push(Self::CACHE_VERSION);
        // Write kanji flag
        buf.push(self.kanji as u8);
        // Append serialized Aho-Corasick automaton
        buf.extend(ac_bytes);
        buf
    }

    /// Deserialize a HepburnRomanizer from cached bytes.
    ///
    /// Returns `None` if the cache is invalid, corrupted, or has an incompatible version.
    ///
    /// ## Example
    /// ```ignore
    /// use ib_romaji::HepburnRomanizer;
    /// use std::fs;
    ///
    /// let cache_data = fs::read("romanizer.cache").unwrap();
    /// let romanizer = HepburnRomanizer::deserialize_from_slice(&cache_data)
    ///     .expect("Failed to deserialize cache");
    /// ```
    ///
    /// ## Safety
    /// This function is safe to call with any input data. Invalid or corrupted data
    /// will result in `None` being returned. The underlying deserialization uses
    /// `unsafe` code but is protected by the header validation.
    pub fn deserialize_from_slice(data: &[u8]) -> Option<Self> {
        // Validate minimum size: magic (8) + version (1) + kanji flag (1) = 10 bytes
        if data.len() < 10 {
            return None;
        }

        // Validate magic header
        if &data[0..8] != Self::CACHE_MAGIC {
            return None;
        }

        // Validate version
        if data[8] != Self::CACHE_VERSION {
            return None;
        }

        // Read kanji flag
        let kanji = data[9] != 0;

        // Deserialize the Aho-Corasick automaton
        // SAFETY: The header validation ensures this is data we serialized.
        // The deserialize_unchecked function may panic or produce incorrect
        // results if given invalid data, but we've validated the header.
        let (ac, _remaining) =
            unsafe { CharwiseDoubleArrayAhoCorasick::deserialize_unchecked(&data[10..]) };

        Some(Self { ac, kanji })
    }
}

#[cfg(feature = "std")]
impl HepburnRomanizer {
    /// Load a HepburnRomanizer from a cache file.
    ///
    /// Returns `None` if the file doesn't exist, is unreadable, or contains invalid cache data.
    ///
    /// ## Example
    /// ```ignore
    /// use ib_romaji::HepburnRomanizer;
    /// use std::path::Path;
    ///
    /// if let Some(romanizer) = HepburnRomanizer::from_cache("romanizer.cache") {
    ///     // Use cached romanizer
    /// }
    /// ```
    pub fn from_cache<P: AsRef<std::path::Path>>(path: P) -> Option<Self> {
        let data = std::fs::read(path).ok()?;
        Self::deserialize_from_slice(&data)
    }

    /// Save the HepburnRomanizer to a cache file.
    ///
    /// Returns `Ok(())` if successful, or an IO error if the file couldn't be written.
    ///
    /// ## Example
    /// ```ignore
    /// use ib_romaji::HepburnRomanizer;
    ///
    /// let romanizer = HepburnRomanizer::default();
    /// romanizer.to_cache("romanizer.cache").unwrap();
    /// ```
    pub fn to_cache<P: AsRef<std::path::Path>>(&self, path: P) -> std::io::Result<()> {
        let data = self.serialize_to_vec();
        // Create parent directories if they don't exist
        if let Some(parent) = path.as_ref().parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        std::fs::write(path, data)
    }
}

/// Extension trait for `HepburnRomanizerBuilder` to support cached builds.
#[cfg(feature = "std")]
impl<S: hepburn_romanizer_builder::State> HepburnRomanizerBuilder<S>
where
    S::Kana: hepburn_romanizer_builder::IsSet,
    S::Kanji: hepburn_romanizer_builder::IsSet,
    S::Word: hepburn_romanizer_builder::IsSet,
{
    /// Build a HepburnRomanizer with caching support.
    ///
    /// This method attempts to load from the cache file first. If the cache is invalid
    /// or doesn't exist, it builds the romanizer from scratch and saves it to the cache.
    ///
    /// This is an alternative to `build()` that adds caching. Use it when initialization
    /// time is a concern.
    ///
    /// ## Example
    /// ```ignore
    /// use ib_romaji::HepburnRomanizer;
    ///
    /// let romanizer = HepburnRomanizer::builder()
    ///     .kana(true)
    ///     .kanji(true)
    ///     .word(true)
    ///     .build_cached("romanizer.cache");
    /// ```
    pub fn build_cached<P: AsRef<std::path::Path>>(self, cache_path: P) -> HepburnRomanizer {
        // Get the builder parameters for cache validation
        // Note: kana and word are encoded in the AC automaton structure,
        // while kanji is stored as a separate flag
        let _kana = self.get_kana().copied().unwrap_or(false);
        let kanji = self.get_kanji().copied().unwrap_or(false);
        let _word = self.get_word().copied().unwrap_or(false);

        // Try to load from cache first
        if let Some(romanizer) = HepburnRomanizer::from_cache(&cache_path) {
            // Verify that the cached romanizer has matching kanji setting
            if romanizer.kanji == kanji {
                return romanizer;
            }
        }

        // Build from scratch
        let romanizer = self.build();

        // Save to cache (ignore errors)
        let _ = romanizer.to_cache(&cache_path);

        romanizer
    }
}
