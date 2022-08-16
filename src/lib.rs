extern crate approx;
#[cfg(feature = "gui")]
extern crate eframe;
pub extern crate hibitset;
extern crate uom;

pub mod core;
pub mod env;
pub mod exception;
pub mod facility;
pub mod file_access;
pub mod float;
pub mod loader;
pub mod memory;
mod mock_vm;
pub mod output;
pub(crate) mod parser;
pub mod tools;
pub mod units;

#[cfg(feature = "gui")]
pub mod gui;

use core::Core;
use exception::Exception;
use memory::Memory;
use std::result;

pub const TRUE: isize = -1;
pub const FALSE: isize = 0;
pub const NUM_TASKS: usize = 5;

pub type Result = result::Result<(), Exception>;
