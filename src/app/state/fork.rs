//! herdsman's own state on `AppState`, kept in one field so herdr's struct
//! gains a single line. `state.rs` and `app/mod.rs` reach it through hooks
//! marked `// fork: state`.

use crate::config::Config;
use crate::ui::StripPlacement;

/// Everything herdsman keeps on `AppState`.
#[derive(Debug, Default)]
pub(crate) struct ForkState {
    /// Where stacked slots draw their strip, and whether a line separates it.
    pub stack_strip: StripPlacement,
    /// Set by `pane.stack` while it reuses `pane.split`, which then folds the
    /// new pane into the target's slot instead of leaving it beside it.
    pub(crate) split_into_stack: bool,
}

impl ForkState {
    pub fn from_config(config: &Config) -> Self {
        let mut state = Self::default();
        state.apply_config(config);
        state
    }

    /// Take the settings herdsman reads from a (re)loaded config.
    pub fn apply_config(&mut self, config: &Config) {
        self.stack_strip = StripPlacement {
            position: config.ui.stack_strip_position,
            separator: config.ui.stack_strip_separator,
        };
    }
}
