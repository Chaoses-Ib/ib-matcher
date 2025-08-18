//! Unicode utils.
//!
//! ## Features
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
