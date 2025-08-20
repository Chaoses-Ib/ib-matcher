pub fn fold(c: char) -> char {
    include!("map.in.rs")
}

/// ucd-generate case-folding-simple ucd-16.0.0 --chars > case-folding-simple-chars.rs
#[cfg(feature = "_test_data")]
mod codegen {
    use std::{fmt::Write, fs};

    include!("../../../data/case-folding-simple-chars.rs");

    #[test]
    fn codegen() {
        let mut s = String::new();
        write!(s, "match c {{\n").unwrap();
        let mut range = 0;
        for (a, b) in CASE_FOLDING_SIMPLE {
            write!(s, "{a:?}=>{b:?},").unwrap();

            // Natural align
            if *a as u32 / 10 != range {
                range = *a as u32 / 10;
                s.push('\n');
            }
        }
        write!(s, "\n_ => c\n}}").unwrap();
        fs::write("src/case/fold/map.in.rs", s).unwrap();
    }
}
