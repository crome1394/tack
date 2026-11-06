//! Command-line flags. The editor flags match satty's names so a capture
//! script can switch by changing the program name. Flags tack does not act
//! on are still accepted, so an existing satty command line never fails.

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug, Default)]
#[command(name = "tack", version, about = "Screenshot editor and screen pins")]
#[command(args_conflicts_with_subcommands = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Config file (default: ~/.config/tack/config.toml)
    #[arg(short = 'c', long)]
    pub config: Option<PathBuf>,

    /// Image to edit, or '-' to read it from stdin
    #[arg(short = 'f', long)]
    pub filename: Option<String>,

    /// Image to edit (same as -f)
    #[arg(value_name = "FILE", conflicts_with = "filename")]
    pub file: Option<String>,

    /// Where Save writes. strftime codes and ~ work. '-' prints to stdout
    #[arg(short = 'o', long)]
    pub output_filename: Option<String>,

    /// Program that receives the PNG on stdin for Copy, e.g. wl-copy
    #[arg(long)]
    pub copy_command: Option<String>,

    /// Actions for Enter, comma separated
    #[arg(long, value_delimiter = ',')]
    pub actions_on_enter: Option<Vec<Action>>,

    /// Actions for Escape, comma separated (default: exit)
    #[arg(long, value_delimiter = ',')]
    pub actions_on_escape: Option<Vec<Action>>,

    /// Actions for a right click on the canvas, comma separated
    #[arg(long, value_delimiter = ',')]
    pub actions_on_right_click: Option<Vec<Action>>,

    /// Old single-action form of --actions-on-enter
    #[arg(long, hide = true)]
    pub action_on_enter: Option<Action>,

    /// Exit after these actions (no value: after any of them)
    #[arg(long, value_delimiter = ',', num_args = 0.., default_missing_value = "all")]
    pub early_exit: Option<Vec<EarlyExit>>,

    /// Save to the output file after copying
    #[arg(long)]
    pub save_after_copy: bool,

    /// Copy after every change
    #[arg(long)]
    pub auto_copy: bool,

    /// Tool selected at start
    #[arg(long, alias = "init-tool")]
    pub initial_tool: Option<String>,

    /// Scale annotation sizes, e.g. 1.5
    #[arg(long)]
    pub annotation_size_factor: Option<f64>,

    /// Corner radius for rectangles (0 for square corners)
    #[arg(long)]
    pub corner_roundness: Option<f64>,

    /// Font family for text
    #[arg(long)]
    pub font_family: Option<String>,

    /// Font style for text, e.g. Bold
    #[arg(long)]
    pub font_style: Option<String>,

    /// Fill shapes by default
    #[arg(long)]
    pub default_fill_shapes: bool,

    /// Start with the toolbars hidden (Tab shows them)
    #[arg(short = 'd', long)]
    pub default_hide_toolbars: bool,

    /// Start fullscreen
    #[arg(long, num_args = 0..=1, default_missing_value = "current-screen")]
    pub fullscreen: Option<String>,

    /// Window size: smart, or WIDTHxHEIGHT
    #[arg(long, num_args = 0..=1, default_missing_value = "smart")]
    pub resize: Option<String>,

    /// Window title
    #[arg(long)]
    pub title: Option<String>,

    /// Ask for no title bar or borders
    #[arg(long)]
    pub no_window_decoration: bool,

    /// No notifications
    #[arg(long)]
    pub disable_notifications: bool,

    /// Zoom step for Ctrl+scroll (default 1.1)
    #[arg(long)]
    pub zoom_factor: Option<f64>,

    // Satty flags accepted for compatibility; tack does not act on them.
    #[arg(long, hide = true)]
    pub floating_hack: bool,
    #[arg(long, hide = true)]
    pub focus_toggles_toolbars: bool,
    #[arg(long, hide = true)]
    pub right_click_copy: bool,
    #[arg(long, hide = true)]
    pub profile_startup: bool,
    #[arg(long, hide = true)]
    pub primary_highlighter: Option<String>,
    #[arg(long, hide = true)]
    pub brush_smooth_history_size: Option<u32>,
    #[arg(long, hide = true)]
    pub pan_step_size: Option<f64>,
    #[arg(long, hide = true)]
    pub text_move_length: Option<f64>,
    #[arg(long, hide = true)]
    pub input_scale: Option<f64>,
    #[arg(long, hide = true)]
    pub app_id: Option<String>,
    #[arg(long, hide = true)]
    pub notification_thumbnail: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Pin images to the screen: 1:1, no border, above every window
    Pin {
        /// Images to pin
        #[arg(required = true)]
        files: Vec<PathBuf>,
        /// Top-left corner in screen pixels, e.g. 100,80 (default: centered)
        #[arg(long, value_parser = parse_point)]
        at: Option<(i32, i32)>,
    },
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Action {
    SaveToClipboard,
    SaveToFile,
    SaveToFileAs,
    CopyFilepathToClipboard,
    Pin,
    Exit,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EarlyExit {
    All,
    Copy,
    Save,
    SaveAs,
    Pin,
}

fn parse_point(text: &str) -> Result<(i32, i32), String> {
    let (x, y) = text
        .split_once(',')
        .ok_or_else(|| "expected X,Y, e.g. 100,80".to_string())?;
    let x = x.trim().parse::<i32>().map_err(|e| e.to_string())?;
    let y = y.trim().parse::<i32>().map_err(|e| e.to_string())?;
    Ok((x, y))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn satty_command_line_parses() {
        let cli = Cli::try_parse_from([
            "tack",
            "--filename",
            "/tmp/a.png",
            "--output-filename",
            "/tmp/a.png",
            "--actions-on-enter",
            "save-to-clipboard",
            "--save-after-copy",
            "--copy-command",
            "wl-copy",
        ])
        .unwrap();
        assert_eq!(cli.filename.as_deref(), Some("/tmp/a.png"));
        assert_eq!(cli.actions_on_enter, Some(vec![Action::SaveToClipboard]));
        assert!(cli.save_after_copy);
    }

    #[test]
    fn a_bare_file_name_opens_the_editor() {
        let cli = Cli::try_parse_from(["tack", "shot.png"]).unwrap();
        assert_eq!(cli.file.as_deref(), Some("shot.png"));
        assert!(cli.command.is_none());
    }

    #[test]
    fn early_exit_without_value_means_all() {
        let cli = Cli::try_parse_from(["tack", "-f", "a.png", "--early-exit"]).unwrap();
        assert_eq!(cli.early_exit, Some(vec![EarlyExit::All]));
    }

    #[test]
    fn pin_takes_files_and_position() {
        let cli = Cli::try_parse_from(["tack", "pin", "--at", "10,20", "a.png", "b.png"]).unwrap();
        match cli.command {
            Some(Command::Pin { files, at }) => {
                assert_eq!(files.len(), 2);
                assert_eq!(at, Some((10, 20)));
            }
            _ => panic!("expected pin"),
        }
    }
}
