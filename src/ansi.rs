use std::io::{self, Write};

use crate::colors::{ColorSelection, FgColor};
use crate::emitter::{FilterConfig, LineEmitter};

// Raw byte constants used by the ANSI parser.
const ESC: u8 = 0x1b;
const BEL: u8 = 0x07;

// SGR codes we care about for foreground color tracking.
const SGR_RESET: i64 = 0;
const SGR_DEFAULT_FOREGROUND: i64 = 39;
const SGR_FOREGROUND_BASE_START: i64 = 30;
const SGR_FOREGROUND_BASE_END: i64 = 37;
const SGR_FOREGROUND_BRIGHT_START: i64 = 90;
const SGR_FOREGROUND_BRIGHT_END: i64 = 97;
const SGR_EXTENDED_FOREGROUND: i64 = 38;
const SGR_MODE_INDEXED: i64 = 5;
const SGR_MODE_RGB: i64 = 2;

// CSI final bytes live in 0x40..=0x7e. For this tool, only `m` matters because
// that is SGR ("set graphics rendition"), which carries the color changes.
const CSI_FINAL_BYTE_START: u8 = 0x40;
const CSI_FINAL_BYTE_END: u8 = 0x7e;
const SGR_FINAL_BYTE: u8 = b'm';

pub struct ColorLineFilter {
    selection: ColorSelection,
    emitter: LineEmitter,
    line_raw: Vec<u8>,
    line_matches: bool,
    fg: FgColor,
    fg_matches_target: bool,
    parser: AnsiParser, // State
}

impl ColorLineFilter {
    pub fn new(selection: ColorSelection, config: FilterConfig) -> Self {
        Self {
            selection,
            emitter: LineEmitter::new(config),
            line_raw: Vec::new(),
            line_matches: false,
            fg: FgColor::Default,
            fg_matches_target: false,
            parser: AnsiParser::new(),
        }
    }

    pub fn process_bytes<W: Write>(&mut self, bytes: &[u8], out: &mut W) -> io::Result<()> {
        // Process one byte at a time so partial escape sequences can continue
        // cleanly across arbitrary read() chunk boundaries.
        for &byte in bytes {
            self.line_raw.push(byte);

            match self.parser.advance(byte) {
                ParserEvent::Printable => {
                    if self.fg_matches_target {
                        self.line_matches = true;
                    }
                }
                ParserEvent::SetForeground(fg) => {
                    self.fg = fg;
                    self.fg_matches_target = self.selection.matches(self.fg);
                }
                ParserEvent::FlushLine => {
                    self.emitter.finish_line(
                        std::mem::take(&mut self.line_raw),
                        self.line_matches,
                        out,
                    )?;
                    self.line_matches = false;
                }
                ParserEvent::None => {}
            }
        }

        Ok(())
    }

    /// Final flush
    pub fn finish<W: Write>(&mut self, out: &mut W) -> io::Result<()> {
        if !self.line_raw.is_empty() {
            self.emitter.finish_line(
                std::mem::take(&mut self.line_raw),
                self.line_matches,
                out,
            )?;
        }

        self.line_matches = false;
        Ok(())
    }
}

enum ParserEvent {
    None,
    // A byte that would represent visible output in a normal terminal.
    Printable,
    // Raw newline seen; end the current line.
    FlushLine,
    // The parser decoded a new foreground color from SGR.
    SetForeground(FgColor),
}

// Parser flow, at a high level:
//
// 1. Start in Ground.
//    Most bytes are either printable text, raw newlines, or the ESC byte that
//    begins an ANSI escape sequence.
//
// 2. If we see ESC, move to Escape.
//    The next byte tells us which family of escape sequence we are entering.
//    For this tool we only distinguish:
//    - CSI: ESC [ ...   used for SGR color changes such as ESC[31m
//    - OSC: ESC ] ...   used for things like terminal-title updates
//
// 3. Inside CSI, collect numeric parameters until we hit the final byte.
//    Example: ESC[38;2;255;0;0m accumulates the params
//    [38, 2, 255, 0, 0] and then the final `m` tells us to interpret them as
//    SGR ("set graphics rendition") data.
//
// 4. Inside OSC, ignore everything until the terminator.
//    OSC is not visible text and does not affect our foreground-color filter,
//    but we still need to skip over it so those bytes are not mistaken for
//    printable output.
//
// 5. Every processed byte produces a small semantic event for the caller:
//    - Printable: a visible character was seen
//    - FlushLine: a raw newline ended the line
//    - SetForeground: SGR changed the current foreground color
//    - None: nothing relevant happened for filtering
struct AnsiParser {
    state: ParserState,
}

enum ParserState {
    Ground,
    // We just saw ESC and are waiting to learn which escape family this is.
    Escape,
    // Control Sequence Introducer, e.g. ESC[31m or ESC[38;2;255;0;0m.
    Csi(CsiState),
    // Operating System Command, e.g. terminal-title sequences. We do not use
    // them for matching, but we should skip over them cleanly.
    Osc,
    // Inside OSC, we saw ESC and are checking for the string terminator ESC \.
    OscEscape,
}

struct CsiState {
    // Fully completed numeric parameters, e.g. [38, 2, 255, 0, 0].
    params: Vec<i64>,
    // The parameter currently being accumulated from digit bytes.
    current: Option<i64>,
}

impl AnsiParser {
    fn new() -> Self {
        Self {
            state: ParserState::Ground,
        }
    }

    fn advance(&mut self, byte: u8) -> ParserEvent {
        // We temporarily take ownership of the current state so each match arm
        // can decide both:
        // - which event this byte produces
        // - which parser state should be active for the next byte
        //
        // That means `advance()` is the entire state machine transition table:
        // current state + current byte -> next state + parser event.
        let state = std::mem::replace(&mut self.state, ParserState::Ground);
        let (next_state, event) = match state {
            ParserState::Ground => match byte {
                ESC => (ParserState::Escape, ParserEvent::None),
                b'\n' => (ParserState::Ground, ParserEvent::FlushLine),
                // Printable ASCII and tabs count as visible output. This is the
                // signal the filter uses to decide whether the current line
                // contains any text in the target color.
                0x20..=0x7e | b'\t' => (ParserState::Ground, ParserEvent::Printable),
                // Other control bytes are ignored.
                _ => (ParserState::Ground, ParserEvent::None),
            },
            ParserState::Escape => match byte {
                // ESC [ introduces a CSI sequence such as ESC[31m.
                b'[' => (ParserState::Csi(CsiState::new()), ParserEvent::None),
                // ESC ] introduces an OSC sequence such as a terminal-title
                // update. We do not interpret its contents; we just skip it.
                b']' => (ParserState::Osc, ParserEvent::None),
                // Any other ESC sequence is not relevant to this tool, so we
                // drop back to Ground immediately.
                _ => (ParserState::Ground, ParserEvent::None),
            },
            ParserState::Csi(mut csi) => match byte {
                // CSI parameters are ASCII digits separated by semicolons.
                b'0'..=b'9' => {
                    let digit = i64::from(byte - b'0');
                    let current = csi.current.get_or_insert(0);
                    *current = (*current * 10) + digit;
                    (ParserState::Csi(csi), ParserEvent::None)
                }
                b';' => {
                    // Semicolon ends one parameter and starts the next.
                    csi.finish_param();
                    (ParserState::Csi(csi), ParserEvent::None)
                }
                CSI_FINAL_BYTE_START..=CSI_FINAL_BYTE_END => {
                    // The final byte closes the CSI sequence. At this point the
                    // parameter list is complete and can be interpreted.
                    //
                    // For this tool, only `m` matters because it is the SGR
                    // command that changes colors and text attributes.
                    csi.finish_param();
                    let params = csi.params;
                    let event = if byte == SGR_FINAL_BYTE {
                        ParserEvent::SetForeground(parse_sgr_foreground(&params))
                    } else {
                        ParserEvent::None
                    };
                    (ParserState::Ground, event)
                }
                _ => (ParserState::Csi(csi), ParserEvent::None),
            },
            ParserState::Osc => match byte {
                // OSC can end either with BEL or with the two-byte ST sequence
                // ESC \. BEL ends it immediately.
                BEL => (ParserState::Ground, ParserEvent::None),
                // ESC may be the start of ST, so move to the intermediate state
                // that checks the next byte.
                ESC => (ParserState::OscEscape, ParserEvent::None),
                // Everything else inside OSC is ignored.
                _ => (ParserState::Osc, ParserEvent::None),
            },
            ParserState::OscEscape => {
                // If the byte after ESC is `\`, then we saw the full ST
                // terminator and can return to Ground. Otherwise the ESC was
                // just part of the OSC payload, so resume skipping OSC bytes.
                let next_state = if byte == b'\\' {
                    ParserState::Ground
                } else {
                    ParserState::Osc
                };
                (next_state, ParserEvent::None)
            }
        };

        self.state = next_state;
        event
    }
}

impl CsiState {
    fn new() -> Self {
        Self {
            params: Vec::new(),
            current: None,
        }
    }

    fn finish_param(&mut self) {
        // Empty parameters count as zero in SGR. That means ESC[m behaves the
        // same as ESC[0m.
        self.params.push(self.current.take().unwrap_or(0));
    }
}

/// Parse a complete SGR parameter list and return the resulting foreground
/// color state.
///
/// Examples:
/// - `31` -> ANSI red
/// - `91` -> bright ANSI red
/// - `38;5;196` -> indexed 256-color red
/// - `38;2;255;0;0` -> truecolor red
///
/// We intentionally ignore non-foreground SGR attributes such as bold,
/// underline, or background colors.
fn parse_sgr_foreground(params: &[i64]) -> FgColor {
    let mut flat = params;
    if flat.is_empty() {
        flat = &[SGR_RESET];
    }

    let mut fg = FgColor::Default;
    let mut i = 0usize;

    while i < flat.len() {
        match flat[i] {
            SGR_RESET | SGR_DEFAULT_FOREGROUND => {
                fg = FgColor::Default;
                i += 1;
            }
            SGR_FOREGROUND_BASE_START..=SGR_FOREGROUND_BASE_END => {
                // ANSI base colors 30..37 map to indexes 0..7.
                fg = FgColor::Indexed((flat[i] as u8) - (SGR_FOREGROUND_BASE_START as u8));
                i += 1;
            }
            SGR_FOREGROUND_BRIGHT_START..=SGR_FOREGROUND_BRIGHT_END => {
                // Bright ANSI colors 90..97 map to indexes 8..15.
                fg = FgColor::Indexed(
                    ((flat[i] as u8) - (SGR_FOREGROUND_BRIGHT_START as u8)) + 8,
                );
                i += 1;
            }
            SGR_EXTENDED_FOREGROUND => {
                // Extended foreground colors use:
                // - 38;5;n       for 256-color palette entries
                // - 38;2;r;g;b   for truecolor RGB
                if i + 1 >= flat.len() {
                    break;
                }

                match flat[i + 1] {
                    SGR_MODE_INDEXED if i + 2 < flat.len() => {
                        fg = FgColor::Indexed(flat[i + 2].clamp(0, 255) as u8);
                        i += 3;
                    }
                    SGR_MODE_RGB if i + 4 < flat.len() => {
                        let r = flat[i + 2].clamp(0, 255) as u8;
                        let g = flat[i + 3].clamp(0, 255) as u8;
                        let b = flat[i + 4].clamp(0, 255) as u8;
                        fg = FgColor::Rgb(r, g, b);
                        i += 5;
                    }
                    _ => i += 2,
                }
            }
            _ => i += 1,
        }
    }

    fg
}
