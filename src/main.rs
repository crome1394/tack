//! Tack: a screenshot editor and screen pins for Wayland.
//!
//!   tack -f shot.png [satty-style flags]   edit an image
//!   tack pin shot.png [more.png…]          pin images to the screen

mod cli;
mod clipboard;
mod config;
mod editor;
mod image_io;
mod notify;
mod pin;
mod runtime;

use clap::Parser;
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = cli::Cli::parse();
    match cli.command {
        Some(cli::Command::Pin { files, at }) => pin::run(files, at),
        None => editor::run(&cli),
    }
}
