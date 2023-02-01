use clap::Parser;
use std::fs;
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

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let dir = Path::new(&args.dir);
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("misc").to_lowercase();
        let target = dir.join(&ext);
        if args.dry_run {
            println!("would move {} -> {}/", path.display(), ext);
        } else {
            fs::create_dir_all(&target)?;
            fs::rename(&path, target.join(path.file_name().unwrap()))?;
            println!("moved {} -> {}/", path.display(), ext);
        }
    }
    Ok(())
}
