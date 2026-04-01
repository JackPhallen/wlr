use std::io::{self, Read, Write};

use clap::Parser as ClapParser;
use wlr::ansi::ColorLineFilter;
use wlr::colors::{ColorArg, ColorSelection};
use wlr::emitter::FilterConfig;
use wlr::util::unescape_separator;

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    let selection = ColorSelection::from(cli.color);
    let separator = unescape_separator(&cli.separator);
    let config = FilterConfig {
        separator: separator.into_bytes(),
        before: cli.before,
        after: cli.after,
    };

    let mut stdin = io::stdin().lock();
    let stdout = io::stdout();
    let mut stdout = io::BufWriter::new(stdout.lock());
    let mut filter = ColorLineFilter::new(selection, config);
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
    #[arg(short = 'B', long, default_value_t = 0)]
    before: usize,
    #[arg(short = 'A', long, default_value_t = 0)]
    after: usize,
}
