//! Thin wrappers around desktop tools so every module works on both
//! Wayland and X11 without desktop-specific extensions.

use std::io::Write;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

pub fn is_wayland() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some()
}

/// Returns true if `program` is found on `$PATH`.
pub fn has(program: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|dir| dir.join(program).is_file()))
        .unwrap_or(false)
}

/// Runs a command and fails with its stderr if it exits unsuccessfully.
pub fn run(cmd: &mut Command) -> Result<Vec<u8>> {
    let program = cmd.get_program().to_string_lossy().into_owned();
    let out = cmd
        .output()
        .with_context(|| format!("failed to start `{program}` (is it installed?)"))?;
    if !out.status.success() {
        bail!("`{program}` failed: {}", String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(out.stdout)
}

pub fn copy_to_clipboard(text: &str) -> Result<()> {
    let mut cmd = if is_wayland() {
        Command::new("wl-copy")
    } else {
        let mut c = Command::new("xclip");
        c.args(["-selection", "clipboard"]);
        c
    };
    let mut child = cmd
        .stdin(Stdio::piped())
        .spawn()
        .context("no clipboard tool found (install wl-clipboard or xclip)")?;
    child.stdin.take().unwrap().write_all(text.as_bytes())?;
    child.wait()?;
    Ok(())
}

pub fn read_clipboard() -> Result<String> {
    let out = if is_wayland() {
        run(Command::new("wl-paste").arg("--no-newline"))?
    } else {
        run(Command::new("xclip").args(["-selection", "clipboard", "-o"]))?
    };
    Ok(String::from_utf8_lossy(&out).into_owned())
}
