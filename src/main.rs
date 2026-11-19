use clap::Parser;
use std::fs;
use std::io;
use std::path::Path;

#[derive(Parser)]
#[command(name = "fileforge", about = "Organize files by extension")]
struct Args {
    /// Directory to organize
    dir: String,
    /// Preview changes without moving anything
    #[arg(long)]
    dry_run: bool,
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    let dir = Path::new(&args.dir);
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(file_name) = path.file_name() else {
            continue;
        };
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("misc")
            .to_lowercase();
        let target = dir.join(&ext);
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
            if args.dry_run {
                println!(
                    "would skip {} -> {}/ ({} already exists)",
                    path.display(),
                    ext,
                    destination.display()
                );
            } else {
                println!(
                    "skipped {} -> {}/ ({} already exists)",
                    path.display(),
                    ext,
                    destination.display()
                );
            }
            continue;
        }

        if args.dry_run {
            println!("would move {} -> {}/", path.display(), ext);
        } else {
            fs::create_dir_all(&target)?;
            fs::rename(&path, &destination)?;
            println!("moved {} -> {}/", path.display(), ext);
        }
    }
    Ok(())
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
