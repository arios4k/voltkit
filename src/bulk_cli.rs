//! `voltkit rename` and `voltkit resize`: the Batch Rename and Image Resizer
//! tools from a terminal or a script.

use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::{Args, ValueEnum};

use crate::i18n::tr;
use crate::modules::{rename, resize};

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum ScopeArg {
    /// The name without its extension
    Name,
    /// Only the extension
    Extension,
    /// The whole name, extension included
    Full,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum CaseArg {
    Keep,
    Upper,
    Lower,
    Title,
}

#[derive(Args, Debug)]
#[command(
    about = "Rename many files at once: search and replace, regular expressions, numbering and letter case",
    long_about = "Shows what would be renamed and changes nothing, until you add --apply. Every conflict (two files getting one name, a name that is taken) is reported first, and a batch is renamed all or nothing. The last batch can be undone with --undo.\n\nIn the replacement, {n} is the file's number and {n:3} is the number padded to 3 digits; with --regex, $1 and ${name} are the groups of the match. An empty --find matches the whole name, so --replace 'photo-{n:3}' numbers every file.",
    after_help = "Examples:\n  voltkit rename *.jpg --find IMG --replace Holiday\n  voltkit rename *.jpg --find IMG --replace Holiday --apply\n  voltkit rename *.txt --regex --find '(\\d{4})-(\\d{2})' --replace '$2.$1'\n  voltkit rename *.png --replace 'shot-{n:3}' --apply\n  voltkit rename --undo"
)]
pub struct RenameArgs {
    /// Files and folders to rename, in the order they are numbered
    #[arg(value_name = "FILE", required_unless_present = "undo")]
    pub files: Vec<PathBuf>,
    /// The text to look for (empty matches the whole name)
    #[arg(long, default_value = "")]
    pub find: String,
    /// The text to put instead
    #[arg(long, default_value = "")]
    pub replace: String,
    /// Treat --find as a regular expression
    #[arg(long)]
    pub regex: bool,
    /// Ignore the difference between upper and lower case when searching
    #[arg(long)]
    pub ignore_case: bool,
    /// Replace only the first match in each name
    #[arg(long)]
    pub first: bool,
    /// Which part of the name to change
    #[arg(long, value_enum, default_value = "name")]
    pub scope: ScopeArg,
    /// Change the letter case afterwards
    #[arg(long = "case", value_enum, default_value = "keep")]
    pub letter_case: CaseArg,
    /// The first number for {n}
    #[arg(long, default_value_t = 1)]
    pub start: i64,
    /// How much each next file adds to the number
    #[arg(long, default_value_t = 1)]
    pub step: i64,
    /// Really rename the files; without it only the preview is shown
    #[arg(long)]
    pub apply: bool,
    /// Undo the last batch renamed with --apply (or in the app)
    #[arg(long, conflicts_with_all = ["apply", "find", "replace"])]
    pub undo: bool,
}

pub fn rename(args: RenameArgs) -> Result<()> {
    if args.undo {
        let n = rename::undo()?;
        println!("{}", trnf!("Undone: {} item has its old name again", "Undone: {} items have their old names again", n));
        return Ok(());
    }
    let options = rename::Options {
        search: args.find,
        replace: args.replace,
        regex: args.regex,
        case_sensitive: !args.ignore_case,
        all_matches: !args.first,
        scope: match args.scope {
            ScopeArg::Name => rename::Scope::Name,
            ScopeArg::Extension => rename::Scope::Extension,
            ScopeArg::Full => rename::Scope::Full,
        },
        case: match args.letter_case {
            CaseArg::Keep => rename::Case::Keep,
            CaseArg::Upper => rename::Case::Upper,
            CaseArg::Lower => rename::Case::Lower,
            CaseArg::Title => rename::Case::Title,
        },
        start: args.start,
        step: args.step,
    };
    let items = rename::plan(&args.files, &options)?;
    let (mut ready, mut unchanged, mut problems) = (0, 0, 0);
    for item in &items {
        let from = item.from.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let to = item.to.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        match &item.status {
            rename::Status::Ready => {
                ready += 1;
                println!("  {from}  →  {to}");
            }
            rename::Status::Unchanged => {
                unchanged += 1;
                println!("  {from}  =  {}", tr("unchanged"));
            }
            rename::Status::Problem(why) => {
                problems += 1;
                println!("  {from}  ✗  {why}");
            }
        }
    }
    println!("{}", crate::i18n::fill(tr("{} to rename, {} unchanged, {} with problems"), &[&ready, &unchanged, &problems]));
    if problems > 0 {
        bail!("{}", tr("Nothing was renamed: fix the problems above first"));
    }
    if !args.apply {
        if ready > 0 {
            println!("{}", tr("This was only a preview. Add --apply to rename."));
        }
        return Ok(());
    }
    if ready == 0 {
        return Ok(());
    }
    let done = rename::apply(&items)?;
    if let Err(e) = rename::remember(&done) {
        eprintln!("{}: {e:#}", tr("The rename worked, but it can't be undone"));
    }
    println!("{} {}", trnf!("Renamed {} item", "Renamed {} items", done.len()), tr("Undo with `voltkit rename --undo`."));
    Ok(())
}

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum ModeArg {
    /// Keep the shape and fit inside the size
    Fit,
    /// Keep the shape, fill the size and trim the overflow
    Fill,
    /// Exactly the size, even if it distorts the picture
    Stretch,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum FormatArg {
    /// The format of the original
    Same,
    Png,
    Jpeg,
    Webp,
}

#[derive(Args, Debug)]
#[command(
    about = "Resize pictures in bulk",
    long_about = "Each FILE is written to NAME (WxH).EXT beside it; originals are never modified. A picture's own rotation is applied; other metadata is not copied. Choose the size with --longest-side, --percent, or --width and/or --height (default: --longest-side 1920). Pictures are not enlarged unless you add --enlarge.",
    after_help = "Examples:\n  voltkit resize photos/*.jpg --longest-side 1280\n  voltkit resize *.png --percent 50\n  voltkit resize banner.jpg --width 1200 --height 400 --mode fill\n  voltkit resize *.png --longest-side 800 --format webp --quality 85"
)]
pub struct ResizeArgs {
    /// Pictures to resize (.png, .jpg/.jpeg, .webp, .gif, .bmp, .tif/.tiff)
    #[arg(required = true, value_name = "FILE")]
    pub files: Vec<PathBuf>,
    /// The longest side becomes this many pixels
    #[arg(long, value_name = "PIXELS", conflicts_with_all = ["percent", "width", "height"])]
    pub longest_side: Option<u32>,
    /// A share of the original size, in percent
    #[arg(long, value_name = "PERCENT", conflicts_with_all = ["width", "height"])]
    pub percent: Option<u32>,
    /// At most this wide, in pixels
    #[arg(long, value_name = "PIXELS")]
    pub width: Option<u32>,
    /// At most this high, in pixels
    #[arg(long, value_name = "PIXELS")]
    pub height: Option<u32>,
    /// How the picture meets --width and --height
    #[arg(long, value_enum, default_value = "fit")]
    pub mode: ModeArg,
    /// Allow pictures to be made bigger than they are
    #[arg(long)]
    pub enlarge: bool,
    /// The format to write
    #[arg(long, value_enum, default_value = "same")]
    pub format: FormatArg,
    /// JPEG and WebP quality from 1 to 100
    #[arg(long, default_value_t = 90)]
    pub quality: u8,
}

pub fn resize(args: ResizeArgs) -> Result<()> {
    let size = if let Some(n) = args.longest_side {
        resize::Size::LongestSide(n)
    } else if let Some(p) = args.percent {
        resize::Size::Percent(p)
    } else if args.width.is_some() || args.height.is_some() {
        resize::Size::Box { width: args.width, height: args.height }
    } else {
        resize::Size::LongestSide(1920)
    };
    let options = resize::Options {
        size,
        mode: match args.mode {
            ModeArg::Fit => resize::Mode::Fit,
            ModeArg::Fill => resize::Mode::Fill,
            ModeArg::Stretch => resize::Mode::Stretch,
        },
        no_enlarge: !args.enlarge,
        format: match args.format {
            FormatArg::Same => resize::Format::Same,
            FormatArg::Png => resize::Format::Png,
            FormatArg::Jpeg => resize::Format::Jpeg,
            FormatArg::Webp => resize::Format::WebP,
        },
        quality: args.quality,
    };
    let mut failed = 0;
    for file in &args.files {
        match resize::resize(file, &options) {
            Ok(r) => println!(
                "{} → {}  ({}×{} → {}×{}, {} → {})",
                file.display(),
                r.output.display(),
                r.from.0,
                r.from.1,
                r.to.0,
                r.to.1,
                size_text(r.bytes_before),
                size_text(r.bytes_after)
            ),
            Err(e) => {
                failed += 1;
                eprintln!("{}: {e:#}", file.display());
            }
        }
    }
    if failed > 0 {
        bail!("{}", trnf!("{} picture could not be resized", "{} pictures could not be resized", failed));
    }
    Ok(())
}

/// "1.4 MB", "820 KB".
pub fn size_text(bytes: u64) -> String {
    let b = bytes as f64;
    if b >= 1_048_576.0 { format!("{:.1} MB", b / 1_048_576.0) } else { format!("{:.0} KB", (b / 1024.0).max(1.0)) }
}
