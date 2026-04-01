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

impl From<Vec<ColorArg>> for ColorSelection {
    fn from(args: Vec<ColorArg>) -> Self {
        if args.is_empty() {
            return Self::Named(vec![TargetColor::Red]);
        }

        if args.iter().any(|arg| matches!(arg, ColorArg::All)) {
            return Self::All;
        }

        Self::Named(
            args.into_iter()
                .map(|arg| match arg {
                    ColorArg::All => unreachable!("handled above"),
                    ColorArg::Red => TargetColor::Red,
                    ColorArg::Orange => TargetColor::Orange,
                    ColorArg::Yellow => TargetColor::Yellow,
                    ColorArg::Green => TargetColor::Green,
                    ColorArg::Blue => TargetColor::Blue,
                    ColorArg::Violet => TargetColor::Violet,
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
