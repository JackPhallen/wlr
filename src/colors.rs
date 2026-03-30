use clap::ValueEnum;

// These thresholds are intentionally simple. The goal is not perfect color
// science; it is a practical "does this look like the target color?" filter
// for ANSI-colored program output.
const STRONG_CHANNEL_MIN: u8 = 160;
const DOMINANCE_MARGIN: u8 = 60;
const RED_GREEN_MAX: u8 = 120;
const RED_BLUE_MAX: u8 = 120;
const BLUE_RED_MAX: u8 = 120;
const BLUE_GREEN_MAX: u8 = 120;

const ORANGE_RED_MIN: u8 = 180;
const ORANGE_GREEN_MIN: u8 = 80;
const ORANGE_GREEN_MAX: u8 = 210;
const ORANGE_BLUE_MAX: u8 = 120;
const ORANGE_RED_OVER_GREEN_MARGIN: u8 = 20;

const YELLOW_RED_MIN: u8 = 170;
const YELLOW_GREEN_MIN: u8 = 170;
const YELLOW_BLUE_MAX: u8 = 120;

const VIOLET_RED_MIN: u8 = 140;
const VIOLET_BLUE_MIN: u8 = 140;
const VIOLET_GREEN_MAX: u8 = 140;

// These are representative foreground-color indices to treat as each target
// color:
// - low values like 1/2/4 are the standard ANSI colors produced by 30..37
// - values like 9/10/12 are the bright ANSI variants from 90..97
// - larger values like 196 or 226 are common 256-color palette choices
const RED_INDEXES: &[u8] = &[1, 9, 196, 160];
const ORANGE_INDEXES: &[u8] = &[208, 214, 172];
const YELLOW_INDEXES: &[u8] = &[3, 11, 226, 220];
const GREEN_INDEXES: &[u8] = &[2, 10, 46, 34];
const BLUE_INDEXES: &[u8] = &[4, 12, 21, 27];
const VIOLET_INDEXES: &[u8] = &[5, 13, 93, 177];

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum TargetColor {
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Violet,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum ColorArg {
    All,
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Violet,
}

#[derive(Clone, Copy, Debug)]
pub enum FgColor {
    // No explicit foreground color is active. In practice this means "use the
    // terminal default", which we do not try to map back to a named target.
    Default,
    // ANSI indexed colors. This covers both the basic 16-color palette and the
    // 256-color palette selected via `38;5;n`.
    Indexed(u8),
    // Truecolor selected via `38;2;r;g;b`.
    Rgb(u8, u8, u8),
}

#[derive(Clone, Copy, Debug)]
struct ColorProfile {
    indexed_matches: &'static [u8],
    matcher: Matcher,
}

#[derive(Clone, Copy, Debug)]
enum Matcher {
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Violet,
}

#[derive(Clone, Debug)]
pub enum ColorSelection {
    All,
    Named(Vec<TargetColor>),
}

impl TargetColor {
    /// Decide whether the current parsed foreground color should count as this
    /// target color.
    ///
    /// We support two matching strategies:
    /// - direct lookup for ANSI indexed colors
    /// - lightweight RGB heuristics for truecolor output
    pub fn matches(self, fg: FgColor) -> bool {
        let profile = self.profile();

        match fg {
            FgColor::Default => false,
            FgColor::Indexed(idx) => profile.indexed_matches.contains(&idx),
            FgColor::Rgb(r, g, b) => profile.matcher.matches(r, g, b),
        }
    }

    fn profile(self) -> ColorProfile {
        match self {
            Self::Red => ColorProfile {
                indexed_matches: RED_INDEXES,
                matcher: Matcher::Red,
            },
            Self::Orange => ColorProfile {
                indexed_matches: ORANGE_INDEXES,
                matcher: Matcher::Orange,
            },
            Self::Yellow => ColorProfile {
                indexed_matches: YELLOW_INDEXES,
                matcher: Matcher::Yellow,
            },
            Self::Green => ColorProfile {
                indexed_matches: GREEN_INDEXES,
                matcher: Matcher::Green,
            },
            Self::Blue => ColorProfile {
                indexed_matches: BLUE_INDEXES,
                matcher: Matcher::Blue,
            },
            Self::Violet => ColorProfile {
                indexed_matches: VIOLET_INDEXES,
                matcher: Matcher::Violet,
            },
        }
    }
}

impl ColorArg {
    pub fn into_selection(args: Vec<Self>) -> ColorSelection {
        if args.is_empty() {
            return ColorSelection::Named(vec![TargetColor::Red]);
        }

        if args.iter().any(|arg| matches!(arg, Self::All)) {
            return ColorSelection::All;
        }

        ColorSelection::Named(
            args.into_iter()
                .map(|arg| match arg {
                    Self::All => unreachable!("handled above"),
                    Self::Red => TargetColor::Red,
                    Self::Orange => TargetColor::Orange,
                    Self::Yellow => TargetColor::Yellow,
                    Self::Green => TargetColor::Green,
                    Self::Blue => TargetColor::Blue,
                    Self::Violet => TargetColor::Violet,
                })
                .collect(),
        )
    }
}

impl ColorSelection {
    pub fn matches(&self, fg: FgColor) -> bool {
        match self {
            Self::All => !matches!(fg, FgColor::Default),
            Self::Named(colors) => colors.iter().copied().any(|color| color.matches(fg)),
        }
    }
}

impl Matcher {
    /// Match RGB colors using a few deliberately coarse rules.
    ///
    /// These rules are tuned for colored logs, stack traces, and warnings rather
    /// than image processing. The thresholds are biased toward "obvious" colors
    /// so we avoid classifying muted or gray-ish text as a rainbow color.
    fn matches(self, r: u8, g: u8, b: u8) -> bool {
        match self {
            Self::Red => {
                r >= STRONG_CHANNEL_MIN
                    && g <= RED_GREEN_MAX
                    && b <= RED_BLUE_MAX
                    && r >= g.saturating_add(DOMINANCE_MARGIN)
                    && r >= b.saturating_add(DOMINANCE_MARGIN)
            }
            Self::Orange => {
                r >= ORANGE_RED_MIN
                    && g >= ORANGE_GREEN_MIN
                    && g <= ORANGE_GREEN_MAX
                    && b <= ORANGE_BLUE_MAX
                    && r >= g.saturating_add(ORANGE_RED_OVER_GREEN_MARGIN)
            }
            Self::Yellow => {
                r >= YELLOW_RED_MIN && g >= YELLOW_GREEN_MIN && b <= YELLOW_BLUE_MAX
            }
            Self::Green => {
                g >= STRONG_CHANNEL_MIN
                    && g >= r.saturating_add(DOMINANCE_MARGIN)
                    && g >= b.saturating_add(DOMINANCE_MARGIN)
            }
            Self::Blue => {
                b >= STRONG_CHANNEL_MIN
                    && r <= BLUE_RED_MAX
                    && g <= BLUE_GREEN_MAX
                    && b >= r.saturating_add(DOMINANCE_MARGIN)
                    && b >= g.saturating_add(DOMINANCE_MARGIN)
            }
            Self::Violet => {
                r >= VIOLET_RED_MIN && b >= VIOLET_BLUE_MIN && g <= VIOLET_GREEN_MAX
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ColorArg, ColorSelection, FgColor, TargetColor};

    #[test]
    fn orange_truecolor_does_not_overlap_with_red_or_yellow() {
        let orange = FgColor::Rgb(255, 140, 0);

        assert!(TargetColor::Orange.matches(orange));
        assert!(!TargetColor::Red.matches(orange));
        assert!(!TargetColor::Yellow.matches(orange));
    }

    #[test]
    fn yellow_truecolor_does_not_overlap_with_orange() {
        let yellow = FgColor::Rgb(255, 255, 0);

        assert!(TargetColor::Yellow.matches(yellow));
        assert!(!TargetColor::Orange.matches(yellow));
    }

    #[test]
    fn violet_truecolor_does_not_overlap_with_blue() {
        let violet = FgColor::Rgb(148, 0, 211);

        assert!(TargetColor::Violet.matches(violet));
        assert!(!TargetColor::Blue.matches(violet));
    }

    #[test]
    fn indexed_colors_map_to_single_expected_bucket_for_representative_values() {
        let orange = FgColor::Indexed(208);
        let violet = FgColor::Indexed(177);

        assert!(TargetColor::Orange.matches(orange));
        assert!(!TargetColor::Yellow.matches(orange));

        assert!(TargetColor::Violet.matches(violet));
        assert!(!TargetColor::Blue.matches(violet));
    }

    #[test]
    fn all_selection_matches_any_non_default_foreground() {
        let selection = ColorSelection::All;

        assert!(selection.matches(FgColor::Indexed(1)));
        assert!(selection.matches(FgColor::Rgb(255, 255, 255)));
        assert!(!selection.matches(FgColor::Default));
    }

    #[test]
    fn color_args_default_to_red_when_omitted() {
        let selection = ColorArg::into_selection(Vec::new());

        assert!(selection.matches(FgColor::Indexed(1)));
        assert!(!selection.matches(FgColor::Indexed(2)));
    }

    #[test]
    fn color_args_can_match_multiple_colors() {
        let selection = ColorArg::into_selection(vec![ColorArg::Red, ColorArg::Violet]);

        assert!(selection.matches(FgColor::Indexed(1)));
        assert!(selection.matches(FgColor::Indexed(177)));
        assert!(!selection.matches(FgColor::Indexed(2)));
    }

    #[test]
    fn all_overrides_specific_colors_when_mixed() {
        let selection = ColorArg::into_selection(vec![ColorArg::Red, ColorArg::All]);

        assert!(matches!(selection, ColorSelection::All));
    }
}
