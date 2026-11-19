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

fn run_any(command: &mut Command) -> (bool, String, String) {
    let output = command.output().unwrap();
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[cfg(unix)]
#[test]
fn an_entry_that_cannot_be_inspected_is_reported() {
    use std::os::unix::fs::symlink;

    let dir = TempDir::new("dangling");
    let ghost = dir.path().join("ghost");
    symlink(dir.path().join("missing"), &ghost).unwrap();
    let movable = dir.path().join("b.pdf");
    fs::write(&movable, "moves").unwrap();

    let (ok, stdout, stderr) = run_any(fileforge().arg(dir.path()));
    assert!(
        !ok,
        "an entry that could not be inspected is reported in the exit code"
    );
    assert!(
        stderr.contains(&format!(
            "skipped {}: cannot read its metadata",
            ghost.display()
        )),
        "got: {stderr}"
    );
    assert!(has_line(
        &stdout,
        &format!("moved {} -> pdf/", movable.display())
    ));
    assert!(
        has_line(&stdout, "organized 1 file, skipped 1"),
        "got: {stdout}"
    );
}

#[cfg(unix)]
#[test]
fn a_folder_that_cannot_be_created_is_reported_and_the_run_continues() {
    use std::os::unix::fs::symlink;

    let dir = TempDir::new("blocked");
    // A dangling symlink where the txt/ folder belongs: nothing can be created
    // underneath it. The link itself cannot be inspected either, so it is
    // reported as well -- hence two skips below.
    symlink(dir.path().join("missing"), dir.path().join("txt")).unwrap();
    let blocked = dir.path().join("a.txt");
    let movable = dir.path().join("b.pdf");
    fs::write(&blocked, "stays").unwrap();
    fs::write(&movable, "moves").unwrap();

    let (ok, stdout, stderr) = run_any(fileforge().arg(dir.path()));
    assert!(!ok, "a failed move must set a failing exit code");
    assert!(
        stderr.contains(&format!("skipped {} -> txt/:", blocked.display())),
        "the failure names the file and the folder it was headed for, got: {stderr}"
    );
    assert!(
        has_line(&stdout, &format!("moved {} -> pdf/", movable.display())),
        "the next file is still organized, got: {stdout}"
    );
    assert!(
        has_line(&stdout, "organized 1 file, skipped 2"),
        "got: {stdout}"
    );
    assert_eq!(fs::read_to_string(&blocked).unwrap(), "stays");
    assert_eq!(
        fs::read_to_string(dir.path().join("pdf/b.pdf")).unwrap(),
        "moves"
    );
}

#[test]
fn a_dry_run_counts_what_a_real_run_would_do() {
    let dir = TempDir::new("counts");
    fs::write(dir.path().join("a.pdf"), "page").unwrap();
    fs::write(dir.path().join("b.pdf"), "page").unwrap();
    fs::write(dir.path().join("c.md"), "text").unwrap();
    fs::write(dir.path().join("README"), "no extension").unwrap();

    let (ok, stdout) = run(fileforge().arg(dir.path()).arg("--dry-run"));
    assert!(ok);
    assert!(
        has_line(&stdout, "would organize 4 files, skipped 0"),
        "got: {stdout}"
    );
}

#[cfg(unix)]
#[test]
fn a_permission_failure_is_reported_and_the_run_continues() {
    use std::os::unix::fs::PermissionsExt;

    let dir = TempDir::new("perm");
    let locked = dir.path().join("txt");
    fs::create_dir(&locked).unwrap();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o555)).unwrap();
    // root ignores the mode bits; the test only means something for an
    // unprivileged user (which is what CI runs as).
    if fs::write(locked.join("probe"), "x").is_ok() {
        let _ = fs::remove_file(locked.join("probe"));
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
        return;
    }

    let blocked = dir.path().join("a.txt");
    let movable = dir.path().join("b.pdf");
    fs::write(&blocked, "stays").unwrap();
    fs::write(&movable, "moves").unwrap();

    let (ok, stdout, stderr) = run_any(fileforge().arg(dir.path()));
    assert!(!ok, "a failed move must set a failing exit code");
    assert!(
        stderr.contains(&format!("skipped {} -> txt/:", blocked.display())),
        "got: {stderr}"
    );
    assert!(
        has_line(&stdout, &format!("moved {} -> pdf/", movable.display())),
        "got: {stdout}"
    );
    assert!(
        has_line(&stdout, "organized 1 file, skipped 1"),
        "got: {stdout}"
    );
    assert_eq!(fs::read_to_string(&blocked).unwrap(), "stays");
    assert_eq!(
        fs::read_to_string(dir.path().join("pdf/b.pdf")).unwrap(),
        "moves"
    );

    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
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
