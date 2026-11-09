//! Shared firmware code used by the application and the bring-up test
//! binaries in `src/bin/`.

#![no_std]

pub mod clocks;
pub mod flash;
pub mod logger;
pub mod output;
pub mod rand;
pub mod state;
pub mod usb;
pub mod web;
