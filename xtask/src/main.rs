//! Maintainer tasks for dxl_rs_v2. Run through the `cargo xtask` alias.

#![warn(clippy::mod_module_files)]

mod codegen;

use std::fmt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "usage: cargo xtask codegen";

/// Arguments other than the one `codegen`.
#[derive(Debug, PartialEq, Eq)]
struct Usage {
    /// The arguments received, program name excluded.
    received: Vec<String>,
}

impl Usage {
    /// The usage line.
    fn hint(&self) -> Option<&'static str> {
        Some(USAGE)
    }
}

impl fmt::Display for Usage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "expected the one argument \"codegen\", received {:?}",
            self.received
        )
    }
}

/// `Ok` when `args` is exactly `codegen`.
fn check_args(args: &[String]) -> Result<(), Usage> {
    if args == ["codegen"] {
        Ok(())
    } else {
        Err(Usage {
            received: args.to_vec(),
        })
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Err(usage) = check_args(&args) {
        eprintln!("error: {usage}");
        if let Some(hint) = usage.hint() {
            eprintln!("hint: {hint}");
        }
        return ExitCode::FAILURE;
    }
    match codegen::run(&workspace_root()) {
        Ok(report) => {
            println!("{}", report.summary());
            for line in report.warning_lines() {
                eprintln!("{line}");
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

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn only_codegen_is_accepted() {
        assert_eq!(check_args(&args(&["codegen"])), Ok(()));
    }

    #[test]
    fn usage_echoes_what_was_received() {
        for received in [&[][..], &["codegen", "--check"], &["gen"]] {
            let usage = check_args(&args(received)).unwrap_err();
            assert_eq!(
                usage,
                Usage {
                    received: args(received)
                }
            );
            assert_eq!(
                usage.to_string(),
                format!("expected the one argument \"codegen\", received {received:?}")
            );
            assert_eq!(usage.hint(), Some("usage: cargo xtask codegen"));
        }
    }
}
