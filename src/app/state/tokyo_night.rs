//! The four Tokyo Night styles: night, storm, moon and day. The colours are the
//! official palettes from folke/tokyonight.nvim (`extras/lua/tokyonight_*.lua`),
//! under the names that project gives them, and every style maps them onto
//! herdsman's palette the same way.

use ratatui::style::Color;

use super::Palette;

/// The colours of one Tokyo Night style that herdsman's palette uses.
struct Colors {
    bg: u32,
    bg_highlight: u32,
    bg_visual: u32,
    border: u32,
    terminal_black: u32,
    comment: u32,
    dark5: u32,
    fg: u32,
    fg_dark: u32,
    blue: u32,
    cyan: u32,
    green: u32,
    magenta: u32,
    orange: u32,
    red: u32,
    yellow: u32,
}

const NIGHT: Colors = Colors {
    bg: 0x1a1b26,
    bg_highlight: 0x292e42,
    bg_visual: 0x283457,
    border: 0x15161e,
    terminal_black: 0x414868,
    comment: 0x565f89,
    dark5: 0x737aa2,
    fg: 0xc0caf5,
    fg_dark: 0xa9b1d6,
    blue: 0x7aa2f7,
    cyan: 0x7dcfff,
    green: 0x9ece6a,
    magenta: 0xbb9af7,
    orange: 0xff9e64,
    red: 0xf7768e,
    yellow: 0xe0af68,
};

const STORM: Colors = Colors {
    bg: 0x24283b,
    bg_highlight: 0x292e42,
    bg_visual: 0x2e3c64,
    border: 0x1d202f,
    terminal_black: 0x414868,
    comment: 0x565f89,
    dark5: 0x737aa2,
    fg: 0xc0caf5,
    fg_dark: 0xa9b1d6,
    blue: 0x7aa2f7,
    cyan: 0x7dcfff,
    green: 0x9ece6a,
    magenta: 0xbb9af7,
    orange: 0xff9e64,
    red: 0xf7768e,
    yellow: 0xe0af68,
};

const MOON: Colors = Colors {
    bg: 0x222436,
    bg_highlight: 0x2f334d,
    bg_visual: 0x2d3f76,
    border: 0x1b1d2b,
    terminal_black: 0x444a73,
    comment: 0x636da6,
    dark5: 0x737aa2,
    fg: 0xc8d3f5,
    fg_dark: 0x828bb8,
    blue: 0x82aaff,
    cyan: 0x86e1fc,
    green: 0xc3e88d,
    magenta: 0xc099ff,
    orange: 0xff966c,
    red: 0xff757f,
    yellow: 0xffc777,
};

const DAY: Colors = Colors {
    bg: 0xe1e2e7,
    bg_highlight: 0xc4c8da,
    bg_visual: 0xb7c1e3,
    border: 0xb4b5b9,
    terminal_black: 0xa1a6c5,
    comment: 0x848cb5,
    dark5: 0x68709a,
    fg: 0x3760bf,
    fg_dark: 0x6172b0,
    blue: 0x2e7de9,
    cyan: 0x007197,
    green: 0x587539,
    magenta: 0x9854f1,
    orange: 0xb15c00,
    red: 0xf52a65,
    yellow: 0x8c6c3e,
};

pub(super) fn night() -> Palette {
    palette(&NIGHT)
}

pub(super) fn day() -> Palette {
    palette(&DAY)
}

impl Palette {
    /// Tokyo Night Storm: the night style on a lighter, bluer background.
    pub fn tokyo_night_storm() -> Self {
        palette(&STORM)
    }

    /// Tokyo Night Moon: the softer, warmer dark style.
    pub fn tokyo_night_moon() -> Self {
        palette(&MOON)
    }
}

fn palette(c: &Colors) -> Palette {
    Palette {
        accent: rgb(c.blue),
        panel_bg: rgb(c.bg),
        sidebar_bg: Color::Reset,
        active_row_bg: rgb(c.bg_highlight),
        selection_bg: rgb(c.bg_visual),
        surface0: rgb(c.bg_highlight),
        surface1: rgb(c.terminal_black),
        surface_dim: rgb(c.border),
        overlay0: rgb(c.comment),
        overlay1: rgb(c.dark5),
        text: rgb(c.fg),
        subtext0: rgb(c.fg_dark),
        mauve: rgb(c.magenta),
        green: rgb(c.green),
        yellow: rgb(c.yellow),
        red: rgb(c.red),
        blue: rgb(c.blue),
        teal: rgb(c.cyan),
        peach: rgb(c.orange),
    }
}

const fn rgb(hex: u32) -> Color {
    Color::Rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_style_resolves_by_its_names() {
        let styles = [
            ("tokyo-night", Palette::tokyo_night()),
            ("tokyo-night-storm", Palette::tokyo_night_storm()),
            ("tokyonight-storm", Palette::tokyo_night_storm()),
            ("tokyo-storm", Palette::tokyo_night_storm()),
            ("tokyo-night-moon", Palette::tokyo_night_moon()),
            ("tokyonight-moon", Palette::tokyo_night_moon()),
            ("tokyo-moon", Palette::tokyo_night_moon()),
            ("tokyo-night-day", Palette::tokyo_night_day()),
        ];
        for (name, palette) in styles {
            assert_eq!(Palette::from_name(name), Some(palette), "{name}");
        }
    }

    #[test]
    fn the_dark_styles_switch_to_day_in_light_mode() {
        for name in [
            "tokyo-night-storm",
            "tokyo-storm",
            "tokyo-night-moon",
            "tokyonight-moon",
        ] {
            let (dark, light) = crate::app::sibling_theme_names(name);
            assert_eq!(
                Some(dark.as_str()),
                crate::config::canonical_theme_name(name),
                "{name}"
            );
            assert_eq!(light, "tokyo-night-day", "{name}");
        }
    }

    #[test]
    fn styles_take_the_official_colors() {
        let storm = Palette::tokyo_night_storm();
        assert_eq!(storm.panel_bg, Color::Rgb(0x24, 0x28, 0x3b));
        assert_eq!(storm.selection_bg, Color::Rgb(0x2e, 0x3c, 0x64));
        let day = Palette::tokyo_night_day();
        assert_eq!(day.text, Color::Rgb(0x37, 0x60, 0xbf));
        assert_eq!(day.mauve, Color::Rgb(0x98, 0x54, 0xf1));
    }
}
