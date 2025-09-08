/*!
Combinations of if-ladder, if-tree (bisection) and jump table.

On rustc 1.91.0-nightly (c8ca44c98 2025-08-10):
```c
if ( a1 <= 0x1C7F )
{
  switch ( a1 )
  {
    case 65:
      result = 97LL;
      break;
    case 66:
      result = 98LL;
      break;
    ...
  }
} else if ( a1 <= 0x103FF ) {
  switch ( a1 )
  {
    case 42560:
      result = 42561LL;
      break;
    case 42561:
    ...
    case 43887:
      return result;
    ...
  }
} else if ( a1 <= 0x10C7F ) {
  ...
} else if ( a1 <= 0x1189F )
{
  ...
}
else {
  switch ( a1 )
  {
    ...
    default:
      switch ( a1 )
      {
        case 71840:
          result = 71872LL;
          break;
        ...
        default:
          switch ( a1 )
          {
            case 0x16E40:
              result = 0x16E60LL;
              break;
            ...
            default:
              return result;
          }
      }
  }
}
```
*/

pub fn fold(c: char) -> char {
    include!("map.in.rs")
}

/// ucd-generate case-folding-simple ucd-16.0.0 --chars > case-folding-simple-chars.rs
#[cfg(all(not(feature = "doc"), feature = "_test_data"))]
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
