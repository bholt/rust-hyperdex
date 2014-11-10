#![feature(globs)]
#![feature(concat_idents)]
#![feature(macro_rules)]

extern crate libc;
extern crate sync;
extern crate green;

pub use common::HyperError;

pub mod client;
pub mod admin;

mod hyperdex;
mod hyperdex_client;
mod hyperdex_admin;
mod hyperdex_datastructures;
mod hyperdex_hyperspace_builder;
mod common;
mod test;

