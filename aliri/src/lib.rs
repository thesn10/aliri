//! # aliri
//!
//! Token-based authorization with authorities that verify access grants.

#![warn(
    missing_docs,
    unused_import_braces,
    unused_imports,
    unused_qualifications
)]
#![deny(
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_must_use
)]
#![forbid(unsafe_code)]

mod authority;

pub use authority::Authority;