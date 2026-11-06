//! Desktop notifications through notify-send, when it is installed.

use std::path::Path;
use std::process::{Command, Stdio};

pub fn send(summary: &str, body: &str, image: Option<&Path>) {
    if crate::clipboard::find_program("notify-send").is_none() {
        return;
    }
    let mut cmd = Command::new("notify-send");
    cmd.args(["-a", "Tack", "-t", "4000"]);
    if let Some(img) = image {
        cmd.arg("-i").arg(img);
    }
    cmd.arg("--").arg(summary).arg(body);
    let _ = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}
