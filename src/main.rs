use indicatif::{ProgressBar, ProgressStyle};
use memmap2::{Advice, MmapOptions};
use std::cmp::min;
use std::env;
use std::fs::File;
use std::io::Write;

const MAX_CHUNK_LEN: usize = 512 * 1024;

fn parse_num(s: &str) -> usize {
    let s = s.trim();
    if let Some(stripped) = s.strip_prefix("0x") {
        usize::from_str_radix(stripped, 16).unwrap()
    } else {
        s.parse().unwrap()
    }
}

fn parse_cmdline() -> (usize, usize, String) {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        println!("Usage: {} <start_addr> <count_bytes> <outfile>", args[0]);
        std::process::exit(1);
    }

    let start_addr = parse_num(&args[1]);
    let count_bytes = parse_num(&args[2]);
    let outfile = args[3].clone();

    (start_addr, count_bytes, outfile)
}

fn dump(start: usize, count: usize, out_file: &str) {
    let mut pos: usize = start;
    let end = start + count;
    let mut out = File::create(out_file).unwrap();

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
        out.write_all(&mmap[pos..pos + chunk_len]).unwrap();
        pos += chunk_len;
        pb.set_position((pos - start) as u64);
    }
    pb.finish_with_message("Dump OK");
    println!(); /* Progress bar messes up cursor */
}

fn main() {
    let (start_addr, count_bytes, out_file) = parse_cmdline();
    dump(start_addr, count_bytes, &out_file);
}
