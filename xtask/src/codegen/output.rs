//! Formatting through `rustfmt` on stdin, and writing the result to disk:
//! only changed files are written, stale model files are removed, and
//! nothing outside `src/models.rs` and `src/models/` is touched.

use std::io::{ErrorKind, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::{Failure, IoFailure, MODULE_DIR, Report, entries};

/// Formats `text`, the generated file `src/<rel>`, with `rustfmt`.
pub(crate) fn format(rel: &str, text: &str) -> Result<String, Failure> {
    format_with("rustfmt", rel, text)
}

/// Formats `text` with `program` (a rustfmt binary) fed on stdin.
pub(crate) fn format_with(program: &str, rel: &str, text: &str) -> Result<String, Failure> {
    let failed = |detail: &dyn std::fmt::Display| {
        Failure::Rustfmt(format!("rustfmt failed on src/{rel}: {detail}"))
    };
    let mut child = Command::new(program)
        .args(["--edition", "2024", "--emit", "stdout"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| match e.kind() {
            ErrorKind::NotFound => Failure::RustfmtNotFound {
                file: rel.to_string(),
            },
            _ => failed(&e),
        })?;
    let mut stdin = child.stdin.take().ok_or_else(|| failed(&"no stdin"))?;
    let input = text.to_string();
    let writer = std::thread::spawn(move || stdin.write_all(input.as_bytes()));
    let output = child.wait_with_output().map_err(|e| failed(&e))?;
    match writer.join() {
        Ok(Ok(())) => {}
        Ok(Err(e)) => return Err(failed(&e)),
        Err(_) => return Err(failed(&"stdin writer panicked")),
    }
    if !output.status.success() {
        return Err(failed(&String::from_utf8_lossy(&output.stderr).trim_end()));
    }
    String::from_utf8(output.stdout).map_err(|e| failed(&e))
}

/// Writes `files` (path relative to `src`, contents) under `src`.
///
/// A file whose bytes are unchanged is not rewritten. Every `.rs` file in
/// `src/models/` that is not in `files` is removed. Paths in the report are
/// relative to `src`. `src/models/` is listed before anything is written,
/// so a listing failure writes nothing.
pub(crate) fn sync(src: &Path, files: &[(String, String)]) -> Result<Report, Failure> {
    let dir = src.join(MODULE_DIR);
    std::fs::create_dir_all(&dir).map_err(|source| IoFailure::CreateDir {
        path: dir.clone(),
        source,
    })?;
    let stale = stale(&dir, files)?;

    let mut report = Report::default();
    for (rel, contents) in files {
        let path = src.join(rel);
        match std::fs::read(&path) {
            Ok(old) if old == contents.as_bytes() => {
                report.unchanged += 1;
                continue;
            }
            Ok(_) => {}
            Err(e) if e.kind() == ErrorKind::NotFound => {}
            Err(source) => return Err(IoFailure::Read { path, source }.into()),
        }
        std::fs::write(&path, contents).map_err(|source| IoFailure::Write {
            path: path.clone(),
            source,
        })?;
        report.written.push(PathBuf::from(rel));
    }

    for (path, rel) in stale {
        std::fs::remove_file(&path).map_err(|source| IoFailure::Remove {
            path: path.clone(),
            source,
        })?;
        report.removed.push(PathBuf::from(rel));
    }
    Ok(report)
}

/// The `.rs` files in `dir` (`src/models/`) that are not in `files`, sorted,
/// each with its path relative to `src`.
fn stale(dir: &Path, files: &[(String, String)]) -> Result<Vec<(PathBuf, String)>, IoFailure> {
    let mut stale = Vec::new();
    for entry in entries(dir)? {
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }
        let metadata = std::fs::metadata(&path).map_err(|source| IoFailure::Metadata {
            path: path.clone(),
            source,
        })?;
        if !metadata.is_file() {
            continue;
        }
        // DECISION: refuse a non-UTF-8 `.rs` name instead of removing it as
        // stale: codegen never writes one, so the maintainer decides its fate.
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            return Err(IoFailure::NonUtf8Name { path });
        };
        let rel = format!("{MODULE_DIR}/{name}");
        if !files.iter().any(|(f, _)| *f == rel) {
            stale.push((path, rel));
        }
    }
    stale.sort();
    Ok(stale)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::fixtures::Dir;

    fn files() -> Vec<(String, String)> {
        vec![
            ("models.rs".to_string(), "mod a;\n".to_string()),
            ("models/a.rs".to_string(), "pub struct A;\n".to_string()),
        ]
    }

    fn mtime(path: &Path) -> std::time::SystemTime {
        std::fs::metadata(path).unwrap().modified().unwrap()
    }

    #[test]
    fn formats_on_stdin() {
        assert_eq!(format("models.rs", "fn  a(){}").unwrap(), "fn a() {}\n");
    }

    #[test]
    fn rustfmt_error_names_file() {
        let Err(Failure::Rustfmt(message)) = format("models/x.rs", "fn {") else {
            panic!("expected a rustfmt failure");
        };
        assert!(message.contains("src/models/x.rs"), "{message}");
    }

    #[test]
    fn missing_rustfmt() {
        let failure = format_with("rustfmt-does-not-exist", "models.rs", "").unwrap_err();
        assert!(
            matches!(&failure, Failure::RustfmtNotFound { file } if file == "models.rs"),
            "{failure:?}"
        );
        assert_eq!(
            failure.lines(),
            [
                "error: rustfmt not found to format src/models.rs",
                "hint: run `rustup component add rustfmt`",
            ]
        );
    }

    #[test]
    fn sync_writes_only_changed() {
        let dir = Dir::new();
        let first = sync(dir.path(), &files()).unwrap();
        assert_eq!(first.written.len(), 2);
        let before = mtime(&dir.path().join("models/a.rs"));
        std::thread::sleep(std::time::Duration::from_millis(20));
        let second = sync(dir.path(), &files()).unwrap();
        assert!(second.written.is_empty());
        assert!(second.removed.is_empty());
        assert_eq!(second.unchanged, 2);
        assert_eq!(mtime(&dir.path().join("models/a.rs")), before);
    }

    #[test]
    fn sync_removes_stale_rs_only() {
        let dir = Dir::new();
        std::fs::create_dir_all(dir.path().join("models")).unwrap();
        std::fs::write(dir.path().join("models/gone.rs"), "old").unwrap();
        std::fs::write(dir.path().join("models/notes.txt"), "keep").unwrap();
        std::fs::write(dir.path().join("lib.rs"), "keep").unwrap();
        let report = sync(dir.path(), &files()).unwrap();
        assert_eq!(report.removed, [PathBuf::from("models/gone.rs")]);
        assert!(!dir.path().join("models/gone.rs").exists());
        assert!(dir.path().join("models/notes.txt").exists());
        assert_eq!(
            std::fs::read_to_string(dir.path().join("lib.rs")).unwrap(),
            "keep"
        );
    }

    #[test]
    fn sync_creates_models_dir() {
        let dir = Dir::new();
        sync(dir.path(), &files()).unwrap();
        assert!(dir.path().join("models/a.rs").is_file());
        assert_eq!(
            std::fs::read_to_string(dir.path().join("models.rs")).unwrap(),
            "mod a;\n"
        );
    }

    #[test]
    fn sync_reports_write_errors() {
        let dir = Dir::new();
        let files = [("gone/x.rs".to_string(), String::new())];
        let failure = sync(dir.path(), &files).unwrap_err();
        assert!(
            matches!(&failure, Failure::Io(IoFailure::Write { path, source })
                if *path == dir.path().join("gone/x.rs") && source.kind() == ErrorKind::NotFound),
            "{failure:?}"
        );
    }

    #[test]
    fn sync_reports_read_errors() {
        let dir = Dir::new();
        std::fs::create_dir_all(dir.path().join("models.rs")).unwrap();
        let failure = sync(dir.path(), &files()).unwrap_err();
        assert!(
            matches!(&failure, Failure::Io(IoFailure::Read { path, source })
                if *path == dir.path().join("models.rs") && source.kind() == ErrorKind::IsADirectory),
            "{failure:?}"
        );
    }

    #[test]
    fn sync_reports_create_dir_errors() {
        let dir = Dir::new();
        std::fs::write(dir.path().join("models"), "not a directory").unwrap();
        let failure = sync(dir.path(), &files()).unwrap_err();
        assert!(
            matches!(&failure, Failure::Io(IoFailure::CreateDir { path, .. })
                if *path == dir.path().join("models")),
            "{failure:?}"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn sync_reports_dangling_rs_links() {
        let dir = Dir::new();
        std::fs::create_dir_all(dir.path().join("models")).unwrap();
        let link = dir.path().join("models/dead.rs");
        std::os::unix::fs::symlink(dir.path().join("nowhere.rs"), &link).unwrap();
        let failure = sync(dir.path(), &files()).unwrap_err();
        assert!(
            matches!(&failure, Failure::Io(IoFailure::Metadata { path, source })
                if *path == link && source.kind() == ErrorKind::NotFound),
            "{failure:?}"
        );
        assert!(!dir.path().join("models.rs").exists());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn sync_refuses_non_utf8_rs_names_before_writing() {
        use std::os::unix::ffi::OsStrExt as _;
        let dir = Dir::new();
        let models = dir.path().join("models");
        std::fs::create_dir_all(&models).unwrap();
        std::fs::write(
            models.join(std::ffi::OsStr::from_bytes(b"\xff.txt")),
            "keep",
        )
        .unwrap();
        assert!(sync(dir.path(), &files()).is_ok());

        let dir = Dir::new();
        let models = dir.path().join("models");
        std::fs::create_dir_all(&models).unwrap();
        let odd = models.join(std::ffi::OsStr::from_bytes(b"\xff.rs"));
        std::fs::write(&odd, "old").unwrap();
        let failure = sync(dir.path(), &files()).unwrap_err();
        assert!(
            matches!(&failure, Failure::Io(IoFailure::NonUtf8Name { path }) if *path == odd),
            "{failure:?}"
        );
        assert_eq!(
            failure.lines(),
            [
                format!("error: {}: file name is not UTF-8", odd.display()),
                "hint: codegen writes only UTF-8 names, so it did not write this file: move or remove it".to_string(),
            ]
        );
        assert!(!dir.path().join("models.rs").exists());
        assert!(odd.exists());
    }
}
