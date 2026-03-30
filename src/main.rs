//! Streaming "keep whole line if it contains any text in a target color" filter.
//!
//! Default: RED
//! Optional: choose one or more colors via `--color`: `wlr --color green`
//! Optional: choose a block separator via `--separator`
//!
//! How it works:
//! - We buffer raw bytes per line so matching lines can be written back verbatim.
//! - We run the stream through a small ANSI parser that only tracks foreground
//!   color SGR sequences and printable text.
//! - If any printable character on a line appears while the foreground color
//!   matches the selected target, we emit the whole line.
//!
//! Notes:
//! - This is line-oriented and treats raw '\n' bytes as line boundaries.
//! - Supports ANSI 16-color, 256-color (`38;5;n`), and truecolor (`38;2;r;g;b`)
//!   foreground colors.

use std::io::{self, Read, Write};

use clap::Parser as ClapParser;
use wlr::ansi::ColorLineFilter;
use wlr::colors::ColorArg;
use wlr::util::unescape_separator;

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    let selection = ColorArg::into_selection(cli.color);
    let separator = unescape_separator(&cli.separator);

    let mut stdin = io::stdin().lock();
    let stdout = io::stdout();
    let mut stdout = io::BufWriter::new(stdout.lock());
    let mut filter = ColorLineFilter::new(selection, separator.into_bytes());
    let mut buf = [0u8; 64 * 1024];

    loop {
        let n = stdin.read(&mut buf)?;
        if n == 0 {
            break;
        }

        filter.process_bytes(&buf[..n], &mut stdout)?;
    }

    filter.finish(&mut stdout)?;
    stdout.flush()?;
    Ok(())
}

#[derive(Debug, ClapParser)]
#[command(about = "Keep whole lines if they contain text in the target color")]
struct Cli {
    #[arg(long, value_enum)]
    color: Vec<ColorArg>,
    #[arg(long, default_value = "\\n")]
    separator: String,
}
