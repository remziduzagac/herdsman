//! The user's text editor, for opening a file to edit in place: `$VISUAL`,
//! then `$EDITOR`, else `vi`, or Notepad on Windows. `herdsman preset edit`
//! runs it in the caller's terminal and the server's `preset.edit` in a popup
//! pane, each with its own process's environment.
//!
//! herdsman's own module. `platform/mod.rs` reaches it through a hook marked
//! `// fork: presets`.

use std::path::Path;

/// The editor's argv for `path`, from this process's environment.
pub(crate) fn editor_argv(path: &Path) -> Vec<String> {
    let mut argv = editor_words(
        std::env::var("VISUAL").ok().as_deref(),
        std::env::var("EDITOR").ok().as_deref(),
    );
    argv.push(path.display().to_string());
    argv
}

/// The first non-empty of `visual` and `editor`, split on whitespace so
/// values such as `code --wait` work; else the platform's fallback.
fn editor_words(visual: Option<&str>, editor: Option<&str>) -> Vec<String> {
    let fallback = if cfg!(windows) { "notepad" } else { "vi" };
    [visual, editor]
        .into_iter()
        .flatten()
        .map(|value| {
            value
                .split_whitespace()
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .find(|words| !words.is_empty())
        .unwrap_or_else(|| vec![fallback.to_string()])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_editor_comes_from_visual_then_editor() {
        assert_eq!(
            editor_words(Some("code --wait"), Some("vim")),
            ["code", "--wait"]
        );
        assert_eq!(editor_words(None, Some("nvim")), ["nvim"]);
        assert_eq!(editor_words(Some("  "), Some("nvim")), ["nvim"]);
        let fallback = if cfg!(windows) { "notepad" } else { "vi" };
        assert_eq!(editor_words(None, None), [fallback]);
    }
}
