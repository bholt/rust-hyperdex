#![feature(concat_idents)]
#![feature(box_syntax)]
#![feature(unique)]
#![feature(ip_addr)]
#![feature(convert)]
#![feature(mpsc_select)]

extern crate libc;
extern crate rustc_serialize;
extern crate eventual;

pub use common::HyperError;
pub use client::{Client};
pub use client_types::{F64, HyperMapAttribute, HyperObject, HyperPredicate, HyperObjectKeyError, HyperPredicateType, HyperValue};
pub use admin::Admin;

mod helpers;
mod client;
mod admin;

mod hyperdex;
mod hyperdex_client;
mod hyperdex_admin;
mod hyperdex_datastructures;
mod hyperdex_hyperspace_builder;
mod common;
mod test;
mod client_types;
