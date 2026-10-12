//! Maintainer tasks for dxl_rs_v2. Run through the `cargo xtask` alias.

#![warn(clippy::mod_module_files)]

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

/// Regenerates `src/models.rs` and `src/models/` from `control_tables/`, then
/// formats them.
///
/// Both are deleted first so tables removed from the map don't leave stale
/// files behind.
fn run_codegen(root: &Path) -> Result<(), String> {
    let output = codegen::generate(&root.join("control_tables"))?;
    for file in &output.unmapped {
        eprintln!("warning: control_tables/{file}.ron is not in dynamixel.ron, skipped");
    }

    let src = root.join("src");
    let module_file = src.join(codegen::MODULE_FILE);
    let module_dir = src.join(codegen::MODULE_DIR);
    if module_file.exists() {
        std::fs::remove_file(&module_file)
            .map_err(|e| format!("{}: {e}", module_file.display()))?;
    }
    if module_dir.exists() {
        std::fs::remove_dir_all(&module_dir)
            .map_err(|e| format!("{}: {e}", module_dir.display()))?;
    }
    std::fs::create_dir_all(&module_dir).map_err(|e| format!("{}: {e}", module_dir.display()))?;

    let mut paths = Vec::with_capacity(output.files.len());
    for (name, contents) in &output.files {
        let path = src.join(name);
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

    println!("generated {} files under src/", paths.len());
    Ok(())
}
