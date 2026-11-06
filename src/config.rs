//! `~/.config/tack/config.toml`, merged under the command-line flags into
//! the `Settings` the editor uses. Every key is optional:
//!
//! ```toml
//! [general]
//! output-filename = "~/Pictures/Screenshots/tack-%Y-%m-%d_%H-%M-%S.png"
//! copy-command = "wl-copy"
//! actions-on-enter = ["save-to-clipboard"]
//! actions-on-escape = ["exit"]
//! early-exit = ["copy"]
//! save-after-copy = false
//! initial-tool = "arrow"
//! annotation-size-factor = 1.0
//! corner-roundness = 12
//! font-family = "Inter"
//! default-fill-shapes = false
//! disable-notifications = false
//!
//! [colors]
//! palette = ["#ef4444", "#f59e0b", "#22c55e", "#3b82f6", "#a855f7", "#ffffff", "#000000"]
//! ```

use crate::cli::{Action, Cli, EarlyExit};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Deserialize, Default, Debug)]
#[serde(rename_all = "kebab-case", default)]
struct General {
    output_filename: Option<String>,
    copy_command: Option<String>,
    actions_on_enter: Option<Vec<Action>>,
    actions_on_escape: Option<Vec<Action>>,
    actions_on_right_click: Option<Vec<Action>>,
    early_exit: Option<Vec<EarlyExit>>,
    save_after_copy: Option<bool>,
    auto_copy: Option<bool>,
    initial_tool: Option<String>,
    annotation_size_factor: Option<f64>,
    corner_roundness: Option<f64>,
    font_family: Option<String>,
    font_style: Option<String>,
    default_fill_shapes: Option<bool>,
    default_hide_toolbars: Option<bool>,
    disable_notifications: Option<bool>,
    zoom_factor: Option<f64>,
    pin_save_dir: Option<String>,
}

#[derive(Deserialize, Default, Debug)]
#[serde(default)]
struct Colors {
    palette: Option<Vec<String>>,
}

#[derive(Deserialize, Default, Debug)]
#[serde(default)]
struct File {
    general: General,
    colors: Colors,
}

/// Everything the editor needs, flags over config over defaults.
#[derive(Clone, Debug)]
pub struct Settings {
    pub output_filename: Option<String>,
    pub copy_command: Option<String>,
    pub actions_on_enter: Vec<Action>,
    pub actions_on_escape: Vec<Action>,
    pub actions_on_right_click: Vec<Action>,
    pub early_exit: Vec<EarlyExit>,
    pub save_after_copy: bool,
    pub auto_copy: bool,
    pub initial_tool: Option<String>,
    pub size_factor: f64,
    pub corner_roundness: f64,
    pub font_family: String,
    pub font_style: String,
    pub fill_shapes: bool,
    pub hide_toolbars: bool,
    pub notifications: bool,
    pub zoom_factor: f64,
    pub palette: Vec<String>,
    pub fullscreen: bool,
    pub resize: Option<(i32, i32)>,
    pub title: String,
    pub decorated: bool,
}

pub const DEFAULT_PALETTE: [&str; 8] = [
    "#ef4444", "#f97316", "#facc15", "#22c55e", "#3b82f6", "#a855f7", "#ffffff", "#000000",
];

pub fn config_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| home().join(".config"))
        .join("tack")
}

pub fn home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

/// `~/x` → `$HOME/x`. Other paths are returned as they are.
pub fn expand_tilde(path: &str) -> PathBuf {
    if path == "~" {
        home()
    } else if let Some(rest) = path.strip_prefix("~/") {
        home().join(rest)
    } else {
        PathBuf::from(path)
    }
}

fn read_file(path: &Path) -> Result<File, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(File::default()),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

fn parse_size(text: &str) -> Option<(i32, i32)> {
    let (w, h) = text.split_once(['x', 'X'])?;
    let w = w.trim().parse::<i32>().ok()?;
    let h = h.trim().parse::<i32>().ok()?;
    (w > 0 && h > 0).then_some((w, h))
}

fn color_ok(text: &str) -> bool {
    let hex = text.strip_prefix('#').unwrap_or("");
    matches!(hex.len(), 6 | 8) && hex.chars().all(|c| c.is_ascii_hexdigit())
}

impl Settings {
    /// Flags over the config file over defaults. A broken config file is
    /// reported and ignored, so the editor still opens.
    pub fn load(cli: &Cli) -> Settings {
        let path = cli
            .config
            .clone()
            .unwrap_or_else(|| config_dir().join("config.toml"));
        let file = read_file(&path).unwrap_or_else(|e| {
            eprintln!("tack: ignoring config: {e}");
            File::default()
        });
        let g = file.general;

        let enter = cli
            .actions_on_enter
            .clone()
            .or_else(|| cli.action_on_enter.map(|a| vec![a]))
            .or(g.actions_on_enter)
            .unwrap_or_else(|| vec![Action::SaveToClipboard]);

        let palette: Vec<String> = file
            .colors
            .palette
            .unwrap_or_default()
            .into_iter()
            .filter(|c| color_ok(c))
            .collect();

        Settings {
            output_filename: cli.output_filename.clone().or(g.output_filename),
            copy_command: cli.copy_command.clone().or(g.copy_command),
            actions_on_enter: enter,
            actions_on_escape: cli
                .actions_on_escape
                .clone()
                .or(g.actions_on_escape)
                .unwrap_or_else(|| vec![Action::Exit]),
            actions_on_right_click: cli
                .actions_on_right_click
                .clone()
                .or(g.actions_on_right_click)
                .unwrap_or_default(),
            early_exit: cli.early_exit.clone().or(g.early_exit).unwrap_or_default(),
            save_after_copy: cli.save_after_copy || g.save_after_copy.unwrap_or(false),
            auto_copy: cli.auto_copy || g.auto_copy.unwrap_or(false),
            initial_tool: cli.initial_tool.clone().or(g.initial_tool),
            size_factor: cli
                .annotation_size_factor
                .or(g.annotation_size_factor)
                .filter(|f| *f > 0.0 && *f <= 10.0)
                .unwrap_or(1.0),
            corner_roundness: cli
                .corner_roundness
                .or(g.corner_roundness)
                .unwrap_or(12.0)
                .clamp(0.0, 200.0),
            font_family: cli
                .font_family
                .clone()
                .or(g.font_family)
                .unwrap_or_else(|| "Sans".into()),
            font_style: cli.font_style.clone().or(g.font_style).unwrap_or_default(),
            fill_shapes: cli.default_fill_shapes || g.default_fill_shapes.unwrap_or(false),
            hide_toolbars: cli.default_hide_toolbars || g.default_hide_toolbars.unwrap_or(false),
            notifications: !(cli.disable_notifications || g.disable_notifications.unwrap_or(false)),
            zoom_factor: cli
                .zoom_factor
                .or(g.zoom_factor)
                .filter(|f| *f > 1.0 && *f < 4.0)
                .unwrap_or(1.1),
            palette: if palette.is_empty() {
                DEFAULT_PALETTE.iter().map(|s| s.to_string()).collect()
            } else {
                palette
            },
            fullscreen: cli.fullscreen.is_some(),
            resize: cli.resize.as_deref().and_then(parse_size),
            title: cli.title.clone().unwrap_or_else(|| "Tack".into()),
            decorated: !cli.no_window_decoration,
        }
    }
}

/// Folder the pin menu's Save writes to.
pub fn pin_save_dir() -> PathBuf {
    let file = read_file(&config_dir().join("config.toml")).unwrap_or_default();
    file.general
        .pin_save_dir
        .map(|p| expand_tilde(&p))
        .unwrap_or_else(|| home().join("Pictures"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_and_colors() {
        assert_eq!(parse_size("800x600"), Some((800, 600)));
        assert_eq!(parse_size("smart"), None);
        assert!(color_ok("#a855f7"));
        assert!(!color_ok("purple"));
    }

    #[test]
    fn config_keys_parse() {
        let f: File = toml::from_str(
            "[general]\nactions-on-enter = [\"save-to-clipboard\", \"exit\"]\nsave-after-copy = true\n[colors]\npalette = [\"#ff0000\"]\n",
        )
        .unwrap();
        assert_eq!(f.general.actions_on_enter.unwrap().len(), 2);
        assert_eq!(f.colors.palette.unwrap(), vec!["#ff0000".to_string()]);
    }
}
