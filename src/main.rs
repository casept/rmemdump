use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use memmap2::{Advice, Mmap, MmapOptions};
use std::cmp::min;
use std::fs::File;
use std::io::Write;
use std::num::ParseIntError;
use std::str::FromStr;

const MAX_CHUNK_LEN: usize = 512 * 1024;

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

fn save_chunk(mmap: &Mmap, start: usize, end: usize, dest: &mut dyn Write) {
    let mut src = &mmap[start..end];
    std::io::copy(&mut src, dest).unwrap();
}

fn dump(start: usize, count: usize, out_file: &str, compress: bool) {
    let mut pos: usize = start;
    let end = start + count;
    let mut out = File::create(out_file).unwrap();
    let mut compressor = None;
    if compress {
        let out = out.try_clone().unwrap();
        compressor = Some(lz4::EncoderBuilder::new().level(1).build(out).unwrap());
    }

    let devmem = File::open("/dev/mem").unwrap();
    let mut opts = MmapOptions::new();
    let opts = opts.len(end);
    let mmap = unsafe { opts.map(&devmem) }.unwrap();
    mmap.advise(Advice::Sequential).unwrap();

    let pb = ProgressBar::new(count as u64);
    pb.set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
        .unwrap()
        .progress_chars("#>-"));

    while pos < end {
        // Read in 64KiB chunks to avoid exhausting memory
        let chunk_len = min(MAX_CHUNK_LEN, end - pos);
        let chunk_start = pos;
        let chunk_end = pos + chunk_len;

        if compress {
            save_chunk(
                &mmap,
                chunk_start,
                chunk_end,
                &mut compressor.as_mut().unwrap(),
            );
        } else {
            save_chunk(&mmap, chunk_start, chunk_end, &mut out);
        }

        pos += chunk_len;
        pb.set_position((pos - start) as u64);
    }

    if compress {
        let (_, res) = compressor.unwrap().finish();
        res.unwrap();
    }
    pb.finish_with_message("Dump OK");
    println!(); /* Progress bar messes up cursor */
}

fn main() {
    let cli = Cli::parse();
    let start_addr = cli.start_addr.0;
    let count_bytes = cli.count_bytes.0;
    let out_file = cli.outfile;
    let compress = cli.compress;
    dump(start_addr, count_bytes, &out_file, compress);
}
