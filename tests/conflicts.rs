use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "fileforge-test-{}-{}-{}",
            std::process::id(),
            label,
            nanos
        ));
        fs::create_dir_all(&path).unwrap();
        TempDir(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn fileforge() -> Command {
    Command::new(env!("CARGO_BIN_EXE_fileforge"))
}

fn run(command: &mut Command) -> (bool, String) {
    let output = command.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(stderr.is_empty(), "fileforge wrote to stderr: {stderr}");
    (output.status.success(), stdout)
}

fn has_line(stdout: &str, expected: &str) -> bool {
    stdout.lines().any(|line| line == expected)
}

#[test]
fn a_live_run_keeps_the_existing_file() {
    let dir = TempDir::new("live");
    let source = dir.path().join("a.txt");
    let destination = dir.path().join("txt/a.txt");
    fs::create_dir(dir.path().join("txt")).unwrap();
    fs::write(&destination, "ORIGINAL-PRECIOUS").unwrap();
    fs::write(&source, "incoming").unwrap();

    let (ok, stdout) = run(fileforge().arg(dir.path()));
    assert!(ok, "a skipped file is not a failure");
    assert!(has_line(
        &stdout,
        &format!(
            "skipped {} -> txt/ ({} already exists)",
            source.display(),
            destination.display()
        )
    ));
    assert_eq!(
        fs::read_to_string(&destination).unwrap(),
        "ORIGINAL-PRECIOUS"
    );
    assert_eq!(
        fs::read_to_string(&source).unwrap(),
        "incoming",
        "the source stays put when it cannot be moved"
    );
}

#[test]
fn a_dry_run_reports_the_conflict_it_would_hit() {
    let dir = TempDir::new("dry");
    let source = dir.path().join("a.txt");
    let destination = dir.path().join("txt/a.txt");
    fs::create_dir(dir.path().join("txt")).unwrap();
    fs::write(&destination, "ORIGINAL-PRECIOUS").unwrap();
    fs::write(&source, "incoming").unwrap();

    let (ok, stdout) = run(fileforge().arg(dir.path()).arg("--dry-run"));
    assert!(ok);
    assert!(has_line(
        &stdout,
        &format!(
            "would skip {} -> txt/ ({} already exists)",
            source.display(),
            destination.display()
        )
    ));
    assert_eq!(
        fs::read_to_string(&destination).unwrap(),
        "ORIGINAL-PRECIOUS"
    );
    assert_eq!(fs::read_to_string(&source).unwrap(), "incoming");
}

#[test]
fn moves_a_file_into_its_extension_folder() {
    let dir = TempDir::new("move");
    let source = dir.path().join("a.pdf");
    fs::write(&source, "page").unwrap();

    let (ok, stdout) = run(fileforge().arg(dir.path()));
    assert!(ok);
    assert!(has_line(
        &stdout,
        &format!("moved {} -> pdf/", source.display())
    ));
    assert_eq!(
        fs::read_to_string(dir.path().join("pdf/a.pdf")).unwrap(),
        "page"
    );
    assert!(!source.exists());
}

#[cfg(unix)]
#[test]
fn a_dangling_symlink_at_the_destination_is_not_replaced() {
    use std::os::unix::fs::symlink;

    let dir = TempDir::new("symlink");
    let source = dir.path().join("a.txt");
    let destination = dir.path().join("txt/a.txt");
    fs::create_dir(dir.path().join("txt")).unwrap();
    symlink(dir.path().join("missing"), &destination).unwrap();
    fs::write(&source, "incoming").unwrap();

    let (ok, stdout) = run(fileforge().arg(dir.path()));
    assert!(ok);
    assert!(has_line(
        &stdout,
        &format!(
            "skipped {} -> txt/ ({} already exists)",
            source.display(),
            destination.display()
        )
    ));
    assert!(
        fs::symlink_metadata(&destination).is_ok(),
        "the existing symlink must survive"
    );
    assert_eq!(fs::read_to_string(&source).unwrap(), "incoming");
}
