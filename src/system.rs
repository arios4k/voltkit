//! Thin wrappers around desktop tools so every module works on both
//! Wayland and X11 without desktop-specific extensions.

use std::io::{Read, Write};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

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
        .with_context(|| format!("`{program}` is not installed; `voltkit doctor` shows how to install it"))?;
    if !out.status.success() {
        bail!("`{program}` failed: {}", String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(out.stdout)
}

/// Clipboard tools can wait forever, e.g. when the compositor won't hand
/// `wl-copy` the selection. Nothing should hang the daemon on that.
const CLIPBOARD_TIMEOUT: Duration = Duration::from_secs(3);

/// Waits for `child`, killing it if it runs longer than `timeout`.
fn wait_with_timeout(child: &mut Child, timeout: Duration) -> Result<ExitStatus> {
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(status);
        }
        if start.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            bail!("timed out after {}s", timeout.as_secs());
        }
        std::thread::sleep(Duration::from_millis(15));
    }
}

pub fn copy_to_clipboard(text: &str) -> Result<()> {
    let mut cmd = if is_wayland() {
        Command::new("wl-copy")
    } else {
        let mut c = Command::new("xclip");
        c.args(["-selection", "clipboard"]);
        c
    };
    // wl-copy and xclip fork a process that keeps serving the clipboard;
    // it must not hold on to our terminal.
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("no clipboard tool found; `voltkit doctor` shows how to install one")?;
    child.stdin.take().unwrap().write_all(text.as_bytes())?;
    let status = wait_with_timeout(&mut child, CLIPBOARD_TIMEOUT).context("could not set the clipboard")?;
    if !status.success() {
        bail!("could not set the clipboard ({status})");
    }
    Ok(())
}

pub fn read_clipboard() -> Result<String> {
    let mut cmd = if is_wayland() {
        let mut c = Command::new("wl-paste");
        c.arg("--no-newline");
        c
    } else {
        let mut c = Command::new("xclip");
        c.args(["-selection", "clipboard", "-o"]);
        c
    };
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .context("no clipboard tool found; `voltkit doctor` shows how to install one")?;
    // Read on another thread so a large clipboard can't fill the pipe and stall the child.
    let mut stdout = child.stdout.take().unwrap();
    let reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        stdout.read_to_end(&mut buf).map(|_| buf)
    });
    let status = wait_with_timeout(&mut child, CLIPBOARD_TIMEOUT).context("could not read the clipboard")?;
    let bytes = reader.join().map_err(|_| anyhow::anyhow!("clipboard reader crashed"))??;
    if !status.success() {
        // wl-paste fails when the clipboard is empty or holds no text.
        bail!("clipboard has no text");
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waits_for_quick_commands() {
        let mut child = Command::new("true").spawn().unwrap();
        assert!(wait_with_timeout(&mut child, Duration::from_secs(2)).unwrap().success());
    }

    #[test]
    fn kills_commands_that_hang() {
        let mut child = Command::new("sleep").arg("10").spawn().unwrap();
        let start = Instant::now();
        assert!(wait_with_timeout(&mut child, Duration::from_millis(200)).is_err());
        assert!(start.elapsed() < Duration::from_secs(2));
        assert!(child.try_wait().unwrap().is_some(), "child should be dead");
    }
}
