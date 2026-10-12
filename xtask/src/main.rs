//! Maintainer tasks for dxl_rs_v2. Run through the `cargo xtask` alias.

mod codegen;

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const USAGE: &str = "usage: cargo xtask codegen";

fn main() -> ExitCode {
    let task = std::env::args().nth(1);
    let result = match task.as_deref() {
        Some("codegen") => run_codegen(&workspace_root()),
        _ => Err(USAGE.to_string()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
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

/// Regenerates `src/models/` from `control_tables/`, then formats it.
///
/// The output directory is wiped first so tables removed from the map don't
/// leave stale files behind.
fn run_codegen(root: &Path) -> Result<(), String> {
    let output = codegen::generate(&root.join("control_tables"))?;
    for file in &output.unmapped {
        eprintln!("warning: control_tables/{file}.ron is not in dynamixel.ron, skipped");
    }

    let out_dir = root.join("src/models");
    if out_dir.exists() {
        std::fs::remove_dir_all(&out_dir).map_err(|e| format!("{}: {e}", out_dir.display()))?;
    }
    std::fs::create_dir_all(&out_dir).map_err(|e| format!("{}: {e}", out_dir.display()))?;

    let mut paths = Vec::with_capacity(output.files.len());
    for (name, contents) in &output.files {
        let path = out_dir.join(name);
        std::fs::write(&path, contents).map_err(|e| format!("{}: {e}", path.display()))?;
        paths.push(path);
    }

    let status = Command::new("rustfmt")
        .args(["--edition", "2024"])
        .args(&paths)
        .status()
        .map_err(|e| format!("rustfmt: {e}"))?;
    if !status.success() {
        return Err(format!("rustfmt failed: {status}"));
    }

    println!("generated {} files in src/models/", paths.len());
    Ok(())
}
