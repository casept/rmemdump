use clap::Parser;
use std::num::ParseIntError;
use std::str::FromStr;

mod dump;
use dump::Dumper;

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
}
fn main() {
    let cli = Cli::parse();
    let start_addr = cli.start_addr.0;
    let count_bytes = cli.count_bytes.0;
    let out_file = cli.outfile;
    let compress = cli.compress;
    let mut dumper = Dumper::new(start_addr, count_bytes, compress);
    dumper.dump(&out_file);
}
