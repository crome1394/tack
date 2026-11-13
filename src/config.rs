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
//! default-color = "#ef4444"     # else the first palette color
//! default-size = 4              # 1 to 40
//! default-view = "actual"       # "actual" (1:1) or "fit"
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
    default_color: Option<String>,
    default_size: Option<f64>,
    default_view: Option<StartView>,
}

/// How the editor first shows the image.
#[derive(Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum StartView {
    /// 1:1, centered.
    #[default]
    Actual,
    /// Scaled down to fit the window.
    Fit,
}

impl StartView {
    pub fn key(self) -> &'static str {
        match self {
            StartView::Actual => "actual",
            StartView::Fit => "fit",
        }
    }
}

pub const SIZE_MIN: f64 = 1.0;
pub const SIZE_MAX: f64 = 40.0;
pub const SIZE_DEFAULT: f64 = 4.0;

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
    /// Starting color: `default-color`, else the first palette color.
    pub color: String,
    pub size: f64,
    pub view: StartView,
    /// The config file these came from; the settings window writes here.
    pub config_path: PathBuf,
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

pub fn color_ok(text: &str) -> bool {
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
        let palette = if palette.is_empty() {
            DEFAULT_PALETTE.iter().map(|s| s.to_string()).collect()
        } else {
            palette
        };
        let color = g
            .default_color
            .filter(|c| color_ok(c))
            .unwrap_or_else(|| palette[0].clone());

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
            palette,
            color,
            size: g
                .default_size
                .filter(|s| s.is_finite())
                .map(|s| s.clamp(SIZE_MIN, SIZE_MAX).round())
                .unwrap_or(SIZE_DEFAULT),
            view: g.default_view.unwrap_or_default(),
            config_path: path,
            fullscreen: cli.fullscreen.is_some(),
            resize: cli.resize.as_deref().and_then(parse_size),
            title: cli.title.clone().unwrap_or_else(|| "Tack".into()),
            decorated: !cli.no_window_decoration,
        }
    }
}

/// The defaults the settings window changes.
pub struct Defaults {
    pub color: String,
    pub size: f64,
    pub view: StartView,
}

/// Writes the defaults into `[general]` of the config file, keeping every
/// other line (comments too) as it is. The write is atomic.
pub fn save_defaults(path: &Path, d: &Defaults) -> Result<(), String> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    let text = set_general_keys(
        &text,
        &[
            ("default-color", format!("\"{}\"", d.color)),
            ("default-size", format!("{}", d.size.round() as i64)),
            ("default-view", format!("\"{}\"", d.view.key())),
        ],
    );
    // Never write a file we can't read back.
    toml::from_str::<File>(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    crate::image_io::write_atomic(path, text.as_bytes())
}

/// Sets `key = value` lines in `[general]`: replaces a key that is there,
/// adds one that isn't, and adds the table if the file has none.
fn set_general_keys(text: &str, pairs: &[(&str, String)]) -> String {
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let header = |l: &str| l.trim_start().starts_with('[');
    let start = match lines.iter().position(|l| l.trim() == "[general]") {
        Some(i) => i,
        None => {
            // A new table goes first, so keys above any other table stay put.
            let at = lines.iter().position(|l| header(l)).unwrap_or(lines.len());
            lines.insert(at, "[general]".into());
            if at < lines.len() - 1 {
                lines.insert(at + 1, String::new());
            }
            at
        }
    };
    for (key, value) in pairs {
        let end = lines[start + 1..]
            .iter()
            .position(|l| header(l))
            .map_or(lines.len(), |i| start + 1 + i);
        let line = format!("{key} = {value}");
        let found = (start + 1..end).find(|&i| {
            lines[i]
                .split_once('=')
                .is_some_and(|(k, _)| k.trim() == *key)
        });
        match found {
            Some(i) => lines[i] = line,
            None => {
                // After the table's last key, before trailing blank lines.
                let mut at = end;
                while at > start + 1 && lines[at - 1].trim().is_empty() {
                    at -= 1;
                }
                lines.insert(at, line);
            }
        }
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
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

    #[test]
    fn default_keys_parse() {
        let f: File = toml::from_str(
            "[general]\ndefault-color = \"#00ff00\"\ndefault-size = 9\ndefault-view = \"fit\"\n",
        )
        .unwrap();
        assert_eq!(f.general.default_view, Some(StartView::Fit));
        assert_eq!(f.general.default_size, Some(9.0));
    }

    #[test]
    fn general_keys_are_set_in_place() {
        let pairs = [("default-size", "7".to_string())];
        // Empty file: the table is made.
        assert_eq!(
            set_general_keys("", &pairs),
            "[general]\ndefault-size = 7\n"
        );
        // A key that is there is replaced; comments and other tables stay.
        let text = "# mine\n[general]\ndefault-size = 2 # old\ncopy-command = \"wl-copy\"\n\n[colors]\npalette = []\n";
        assert_eq!(
            set_general_keys(text, &pairs),
            "# mine\n[general]\ndefault-size = 7\ncopy-command = \"wl-copy\"\n\n[colors]\npalette = []\n"
        );
        // A missing key goes at the end of [general], not into [colors].
        let text = "[general]\ncopy-command = \"wl-copy\"\n\n[colors]\npalette = []\n";
        assert_eq!(
            set_general_keys(text, &pairs),
            "[general]\ncopy-command = \"wl-copy\"\ndefault-size = 7\n\n[colors]\npalette = []\n"
        );
        // No [general] yet: it goes before the first table.
        let text = "[colors]\npalette = []\n";
        assert_eq!(
            set_general_keys(text, &pairs),
            "[general]\ndefault-size = 7\n\n[colors]\npalette = []\n"
        );
    }
}
