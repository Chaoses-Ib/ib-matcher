//! Unicode utils.
/*!
## Features
- Fast [`to_lowercase()`](case) (simple case folding)
- Fast [ASCII](ascii) search utils
- `floor_char_boundary()` and `ceil_char_boundary()` polyfill

## Crate features
*/
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![cfg_attr(feature = "doc", doc = document_features::document_features!())]
pub mod ascii;
pub mod case;
pub mod str;

mod private {
    pub trait Sealed {}
}
use private::Sealed;

impl Sealed for str {}
