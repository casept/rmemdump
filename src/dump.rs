use indicatif::{ProgressBar, ProgressStyle};
use memmap2::{Advice, Mmap, MmapOptions};
use std::cmp::min;
use std::fs::File;
use std::io::Write;

const MAX_CHUNK_LEN: usize = 512 * 1024;

pub struct Dumper {
    start: usize,
    count: usize,
    compress: bool,
    mmap: Mmap,
}

impl Dumper {
    pub fn new(start: usize, count: usize, compress: bool) -> Self {
        let devmem = File::open("/dev/mem").unwrap();
        let mut opts = MmapOptions::new();
        opts.len(start + count);
        let mmap = unsafe { opts.map(&devmem) }.unwrap();
        mmap.advise(Advice::Sequential).unwrap();
        Dumper {
            start,
            count,
            compress,
            mmap,
        }
    }

    fn save_chunk(&mut self, start: usize, end: usize, dest: &mut dyn Write) {
        let mut src = &self.mmap[start..end];
        std::io::copy(&mut src, dest).unwrap();
    }

    pub fn dump(&mut self, out_file: &str) {
        let mut pos: usize = self.start;
        let end = self.start + self.count;
        let mut out = File::create(out_file).unwrap();
        let mut compressor = None;
        if self.compress {
            let out = out.try_clone().unwrap();
            compressor = Some(lz4::EncoderBuilder::new().level(1).build(out).unwrap());
        }

        let pb = ProgressBar::new(self.count as u64);
        pb.set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
        .unwrap()
        .progress_chars("#>-"));

        while pos < end {
            // Read in chunks to avoid exhausting memory
            let chunk_len = min(MAX_CHUNK_LEN, end - pos);
            let chunk_start = pos;
            let chunk_end = pos + chunk_len;

            if self.compress {
                self.save_chunk(chunk_start, chunk_end, &mut compressor.as_mut().unwrap());
            } else {
                self.save_chunk(chunk_start, chunk_end, &mut out);
            }

            pos += chunk_len;
            pb.set_position((pos - self.start) as u64);
        }

        if self.compress {
            let (_, res) = compressor.unwrap().finish();
            res.unwrap();
        }
        pb.finish_with_message("Dump OK");
        println!(); /* Progress bar messes up cursor */
    }
}
