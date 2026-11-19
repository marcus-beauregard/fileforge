use clap::Parser;
use std::fs;
use std::io;
use std::path::Path;
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "fileforge", about = "Organize files by extension")]
struct Args {
    /// Directory to organize
    dir: String,
    /// Preview changes without moving anything
    #[arg(long)]
    dry_run: bool,
}

#[derive(Default)]
struct Tally {
    /// Files moved, or that a dry run would move.
    moved: usize,
    /// Files left where they were, for any reason.
    skipped: usize,
    /// Skipped because the move failed, as opposed to a name collision.
    failed: usize,
}

fn main() -> ExitCode {
    let args = Args::parse();
    let dir = Path::new(&args.dir);
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) => {
            eprintln!("error: cannot read {}: {err}", dir.display());
            return ExitCode::FAILURE;
        }
    };

    let mut tally = Tally::default();
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            // One unreadable entry is not a reason to abandon the rest.
            Err(err) => {
                eprintln!("skipped entry in {}: {err}", dir.display());
                tally.skipped += 1;
                tally.failed += 1;
                continue;
            }
        };
        let path = entry.path();
        // fs::metadata follows symlinks, which is the view rename(2) sees, but
        // unlike is_file() it reports an entry that cannot be inspected instead
        // of dropping it from the tally without a word.
        match fs::metadata(&path) {
            Ok(meta) if meta.is_file() => {}
            Ok(_) => continue,
            Err(err) => {
                eprintln!(
                    "skipped {}: cannot read its metadata: {err}",
                    path.display()
                );
                tally.skipped += 1;
                tally.failed += 1;
                continue;
            }
        }
        let ext = extension_for(&path);
        match organize(&path, dir, &ext, args.dry_run) {
            Ok(Outcome::Moved) => tally.moved += 1,
            Ok(Outcome::Skipped) => tally.skipped += 1,
            // Name the file and what was being attempted: a bare Os error names
            // neither, which is what made a half-organized folder unexplainable.
            Err(err) => {
                eprintln!("skipped {} -> {ext}/: {err}", path.display());
                tally.skipped += 1;
                tally.failed += 1;
            }
        }
    }

    let verb = if args.dry_run {
        "would organize"
    } else {
        "organized"
    };
    println!(
        "{verb} {}, skipped {}",
        plural(tally.moved, "file"),
        tally.skipped
    );

    if tally.failed > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

enum Outcome {
    Moved,
    Skipped,
}

/// Move one file into its extension folder, or say why it stayed put. Errors
/// come back already carrying the path that failed; the caller only decides
/// whether to keep going (it does).
fn organize(path: &Path, dir: &Path, ext: &str, dry_run: bool) -> io::Result<Outcome> {
    let Some(file_name) = path.file_name() else {
        return Ok(Outcome::Skipped);
    };
    let target = dir.join(ext);
    let destination = target.join(file_name);

    // rename(2) replaces the destination on Unix, so a name collision has to
    // be caught before the call. Checking here means --dry-run reports the
    // same outcome the real run would have.
    let taken = destination_taken(&destination).map_err(|err| {
        io::Error::new(
            err.kind(),
            format!("cannot check {}: {err}", destination.display()),
        )
    })?;
    if taken {
        if dry_run {
            println!(
                "would skip {} -> {ext}/ ({} already exists)",
                path.display(),
                destination.display()
            );
        } else {
            println!(
                "skipped {} -> {ext}/ ({} already exists)",
                path.display(),
                destination.display()
            );
        }
        return Ok(Outcome::Skipped);
    }

    if dry_run {
        println!("would move {} -> {ext}/", path.display());
        return Ok(Outcome::Moved);
    }

    fs::create_dir_all(&target).map_err(|err| {
        io::Error::new(
            err.kind(),
            format!("cannot create {}: {err}", target.display()),
        )
    })?;
    fs::rename(path, &destination).map_err(|err| {
        io::Error::new(
            err.kind(),
            format!("cannot move to {}: {err}", destination.display()),
        )
    })?;
    println!("moved {} -> {ext}/", path.display());
    Ok(Outcome::Moved)
}

fn extension_for(path: &Path) -> String {
    path.extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("misc")
        .to_lowercase()
}

fn plural(count: usize, word: &str) -> String {
    if count == 1 {
        format!("{count} {word}")
    } else {
        format!("{count} {word}s")
    }
}

/// Path::exists follows symlinks, so a dangling link would look absent and the
/// rename would replace it. Metadata on the link itself sees every entry. Only
/// NotFound means the name is free; any other error is reported, not read as a
/// collision the caller would silently swallow.
fn destination_taken(path: &Path) -> io::Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(err),
    }
}
