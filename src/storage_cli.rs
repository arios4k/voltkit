//! Terminal entry points for Folder Compare and Storage Map.
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::Args;
use gtk::glib;

use crate::i18n::tr;
use crate::modules::{folder_compare, storage_map};

#[derive(Args, Debug)]
#[command(about = "Compare two folders or verify a backup")]
pub struct CompareArgs {
    /// Original folder
    original: PathBuf,
    /// Backup or comparison folder
    backup: PathBuf,
    /// Compare file contents byte for byte instead of using size and modification time
    #[arg(long)]
    verify: bool,
    /// Include matching entries in the report
    #[arg(long)]
    all: bool,
    /// Verify file contents and exit with status 1 if anything differs or cannot be verified
    #[arg(long)]
    check: bool,
}

pub fn compare(args: CompareArgs) -> Result<()> {
    let roots = [args.original, args.backup];
    let verified = args.verify || args.check;
    let entries = folder_compare::compare(
        roots.each_ref().map(PathBuf::as_path),
        verified,
        &folder_compare::Progress::default(),
    )
    .context(tr("Could not compare folders"))?;
    let result = folder_compare::ResultSet {
        roots,
        verified,
        entries,
    };
    print!("{}", result.report(args.all));
    if args.check && result.entries.iter().any(|entry| !entry.status.matches()) {
        bail!("{}", tr("The backup differs or could not be verified."));
    }
    Ok(())
}

#[derive(Args, Debug)]
#[command(about = "Show which folders and files use the most disk space")]
pub struct MapArgs {
    /// Folder to scan
    folder: PathBuf,
    /// List the largest files throughout the folder and its subfolders
    #[arg(long)]
    largest_files: bool,
    /// Maximum number of entries to show
    #[arg(long, default_value_t = 20, value_parser = clap::value_parser!(u32).range(1..))]
    limit: u32,
}

pub fn map(args: MapArgs) -> Result<()> {
    let scan = storage_map::scan(&args.folder, &storage_map::Progress::default())
        .context(tr("Could not scan the folder"))?;
    let root = &scan.nodes[0];
    let total = trf!("{} on disk", glib::format_size(root.bytes));
    println!("{total} · {:?}", root.path);
    if root.skipped > 0 {
        println!("{}", trf!("{} entries could not be scanned.", root.skipped));
    }
    let items = if args.largest_files {
        &scan.files
    } else {
        &root.children
    };
    for &id in items.iter().take(args.limit as usize) {
        let node = &scan.nodes[id];
        let relative = node.path.strip_prefix(&root.path).unwrap_or(&node.path);
        let suffix = if node.directory { "/" } else { "" };
        println!(
            "{:>10}  {relative:?}{suffix}",
            glib::format_size(node.bytes)
        );
    }
    Ok(())
}
