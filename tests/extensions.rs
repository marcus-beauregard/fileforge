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

fn run(dir: &Path, dry_run: bool) -> (bool, String) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_fileforge"));
    command.arg(dir);
    if dry_run {
        command.arg("--dry-run");
    }
    let output = command.output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(stderr.is_empty(), "fileforge wrote to stderr: {stderr}");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
    )
}

fn has_line(stdout: &str, expected: &str) -> bool {
    stdout.lines().any(|line| line == expected)
}

#[test]
fn archives_keep_their_compound_suffix_as_the_folder_name() {
    let dir = TempDir::new("compound");
    let cases = [
        ("backup.tar.gz", "tar.gz"),
        ("releases.tar.bz2", "tar.bz2"),
        ("source.tar.xz", "tar.xz"),
        ("vendor.tar.zst", "tar.zst"),
        ("old.tar.z", "tar.z"),
        ("notes.gz", "gz"),
    ];
    for (name, _) in cases {
        fs::write(dir.path().join(name), "bytes").unwrap();
    }

    let (ok, stdout) = run(dir.path(), false);
    assert!(ok, "nothing failed: {stdout}");
    for (name, folder) in cases {
        let source = dir.path().join(name);
        assert!(
            has_line(&stdout, &format!("moved {} -> {folder}/", source.display())),
            "{name} belongs in {folder}/, got: {stdout}"
        );
        assert!(
            dir.path().join(folder).join(name).exists(),
            "{name} was not moved into {folder}/"
        );
    }
    assert!(
        !dir.path().join("gz").join("backup.tar.gz").exists(),
        "a tarball must not land in gz/ next to single-file gzip"
    );
    let gz: Vec<_> = fs::read_dir(dir.path().join("gz"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(gz, ["notes.gz"], "gz/ holds single-file gzip only");
}

#[test]
fn compound_matching_ignores_case() {
    let dir = TempDir::new("upper");
    let tarball = dir.path().join("BACKUP.TAR.GZ");
    fs::write(&tarball, "bytes").unwrap();

    let (ok, stdout) = run(dir.path(), true);
    assert!(ok);
    assert!(
        has_line(
            &stdout,
            &format!("would move {} -> tar.gz/", tarball.display())
        ),
        "got: {stdout}"
    );
}

#[test]
fn a_dotfile_that_looks_like_a_tarball_is_not_one() {
    let dir = TempDir::new("dotfile");
    let dotfile = dir.path().join(".tar.gz");
    fs::write(&dotfile, "config").unwrap();

    let (ok, stdout) = run(dir.path(), false);
    assert!(ok);
    assert!(
        has_line(&stdout, &format!("moved {} -> gz/", dotfile.display())),
        "a file named .tar.gz has no stem before the suffix, got: {stdout}"
    );
    assert!(dir.path().join("gz/.tar.gz").exists());
}

/// `Path::extension()` reports `Some("")` for a name ending in a dot, and an
/// empty folder name resolves to the directory itself, so these used to be
/// reported as colliding with their own destination and never moved.
#[test]
fn a_name_ending_in_a_dot_is_extensionless() {
    let dir = TempDir::new("trailing-dot");
    // A compound suffix that is itself followed by a dot is still no
    // extension: "archive.tar.gz." matches neither "tar.gz" nor "gz".
    let names = ["foo.", "a.b.", "archive.tar.gz."];
    for name in names {
        fs::write(dir.path().join(name), "bytes").unwrap();
    }

    let (ok, stdout) = run(dir.path(), true);
    assert!(ok, "a dry run does not fail: {stdout}");
    for name in names {
        let source = dir.path().join(name);
        assert!(
            has_line(
                &stdout,
                &format!("would move {} -> misc/", source.display())
            ),
            "{name} has no extension and belongs in misc/, got: {stdout}"
        );
    }

    let (ok, stdout) = run(dir.path(), false);
    assert!(ok, "nothing failed: {stdout}");
    for name in names {
        let source = dir.path().join(name);
        assert!(
            has_line(&stdout, &format!("moved {} -> misc/", source.display())),
            "got: {stdout}"
        );
        assert!(
            dir.path().join("misc").join(name).exists(),
            "{name} was not moved into misc/"
        );
        assert!(!source.exists(), "{name} was left in the root");
    }
}

/// The compound check needs the whole name as UTF-8; a name that is not must
/// still be classified by its extension rather than dropped into misc/.
#[cfg(unix)]
#[test]
fn a_name_that_is_not_utf8_still_uses_its_extension() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let dir = TempDir::new("nonutf8");
    let name = OsStr::from_bytes(b"report\xFF.txt");
    let source = dir.path().join(name);
    fs::write(&source, "page").unwrap();

    let (ok, stdout) = run(dir.path(), false);
    assert!(ok, "nothing failed: {stdout}");
    assert!(
        has_line(&stdout, &format!("moved {} -> txt/", source.display())),
        "got: {stdout}"
    );
    assert!(dir.path().join("txt").join(name).exists());
}

/// The suffix is ASCII even when the stem is not, so the compound rule applies
/// to a name that would not survive as UTF-8.
#[cfg(unix)]
#[test]
fn a_compound_suffix_still_matches_when_the_stem_is_not_utf8() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let dir = TempDir::new("nonutf8-tarball");
    let name = OsStr::from_bytes(b"backup\xFF.tar.gz");
    let source = dir.path().join(name);
    fs::write(&source, "bytes").unwrap();

    let (ok, stdout) = run(dir.path(), false);
    assert!(ok, "nothing failed: {stdout}");
    assert!(
        has_line(&stdout, &format!("moved {} -> tar.gz/", source.display())),
        "got: {stdout}"
    );
    assert!(dir.path().join("tar.gz").join(name).exists());
}
