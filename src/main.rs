use clap::Parser;
use std::num::ParseIntError;
use std::str::FromStr;

mod dump;
mod incremental;
use dump::Dumper;

pub const MAX_CHUNK_LEN: usize = 64 * 1024;

/// Newtype so we can easily parse hex digits
#[derive(Debug, Clone, Copy)]
struct Size(usize);

impl FromStr for Size {
    type Err = ParseIntError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let (stripped, base) = match s.strip_prefix("0x") {
            Some(s) => (s, 16),
            None => (s, 10),
        };
        usize::from_str_radix(stripped, base).map(Size)
    }
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Address to start dumping memory at
    start_addr: Size,
    /// How much to dump (in bytes)
    count_bytes: Size,
    /// Output file
    outfile: String,
    /// Compress output file with LZ4 (level 1)
    #[arg(short, long)]
    compress: bool,
    /// Incremental mode.
    /// Saves an initial snapshot and delta every time SIGUSR1 is received by the process.
    /// The snapshots are not simply flat files, and require rmemdump to unpack.
    #[arg(short, long)]
    incremental_mode: bool,
}

fn main() {
    let cli = Cli::parse();
    let start_addr = cli.start_addr.0;
    let count_bytes = cli.count_bytes.0;
    let out_file = cli.outfile;
    let compress = cli.compress;
    let incremental = cli.incremental_mode;
    let mut dumper = Dumper::new(start_addr, count_bytes, compress);
    if incremental {
        dumper.dump_incremental(&out_file);
    } else {
        dumper.dump_full(&out_file);
    }
}
