#![feature(fnbox)]
#![feature(try_from)]
#![feature(vec_remove_item)]
#![warn(unused_extern_crates)]

//! holochain_net is a library that defines an abstract networking layer for
//! different network transports, providing a configurable interface
//! for swapping different backends connection modules at load time

#[macro_use]
extern crate failure;
#[macro_use]
pub extern crate holochain_core_types_derive;
#[macro_use]
extern crate lazy_static;
// macros used in tests
#[allow(unused_imports)]
#[macro_use]
extern crate serde_json;
#[allow(unused_imports)]
#[macro_use]
extern crate serde_derive;

pub mod connection;
pub mod error;
pub mod in_memory;
pub mod ipc;
pub mod ipc_net_worker;
pub mod p2p_config;
pub mod p2p_network;
pub mod tweetlog;
