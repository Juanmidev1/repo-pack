use std::error::Error;

use arboard::Clipboard;

#[cfg(target_os = "linux")]
pub const DAEMON_ENV: &str = "REPO_PACK_CLIPBOARD_DAEMON";

// On X11 and Wayland the copied text lives in the process that set it and vanishes when it
// exits, so a detached copy of this binary keeps serving it until another app takes over
#[cfg(target_os = "linux")]
pub fn copy(text: &str) -> Result<(), Box<dyn Error>> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    // Fail here with a visible error, since the detached child cannot report one
    Clipboard::new()?;
    let mut child = Command::new(std::env::current_exe()?)
        .env(DAEMON_ENV, "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let mut child_stdin = child.stdin.take().ok_or("clipboard process has no stdin")?;
    child_stdin.write_all(text.as_bytes())?;
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn serve_from_stdin() -> Result<(), Box<dyn Error>> {
    use std::io::Read;

    use arboard::SetExtLinux;

    let mut text = String::new();
    std::io::stdin().read_to_string(&mut text)?;
    Clipboard::new()?.set().wait().text(text)?;
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub fn copy(text: &str) -> Result<(), Box<dyn Error>> {
    Clipboard::new()?.set_text(text)?;
    Ok(())
}
