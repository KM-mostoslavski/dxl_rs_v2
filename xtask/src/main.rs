//! Maintainer tasks for dxl_rs_v2. Run through the `cargo xtask` alias.

#![warn(clippy::mod_module_files)]

mod codegen;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "usage: cargo xtask codegen";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args != ["codegen"] {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    }
    match codegen::run(&workspace_root()) {
        Ok(report) => {
            println!("{}", report.summary());
            for warning in &report.warnings {
                eprintln!("warning: {warning}");
            }
            ExitCode::SUCCESS
        }
        Err(failure) => {
            for line in failure.lines() {
                eprintln!("{line}");
            }
            ExitCode::FAILURE
        }
    }
}

/// Root of the workspace, one level above this crate's manifest.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives inside the workspace")
        .to_path_buf()
}
