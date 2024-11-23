use indicatif::{ProgressBar, ProgressStyle};
use memmap2::{Advice, Mmap, MmapOptions};
use rayon::prelude::*;
use std::cmp::min;
use std::fs::File;
use std::io::Write;
use std::sync::Mutex;

use crate::incremental::Incremental;

pub struct Dumper {
    start: usize,
    count: usize,
    compress: bool,
    mmap: Mmap,
    incr: Mutex<Option<Incremental>>,
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
            incr: Mutex::from(None),
        }
    }

    pub fn dump_incremental(&mut self, out_file: &str) {
        self.incr = Mutex::from(Some(Incremental::new(self.compress, out_file)));
        loop {
            let start_time = std::time::Instant::now();
            // Iterate in parallel via rayon over chunks, mainly to distribute hashing across cores
            self.mmap[self.start..self.start + self.count]
                .par_chunks(crate::MAX_CHUNK_LEN)
                .for_each(|chunk| {
                    let hash = Incremental::hash_chunk(chunk);
                    let mut incr_lock = self.incr.lock();
                    let incr = incr_lock.as_mut().unwrap().as_mut().unwrap();
                    incr.add_hashed_chunk(&chunk, hash);
                    drop(incr_lock);
                });
            let elapsed = start_time.elapsed();
            println!(
                "Took {} ns / {} ms",
                elapsed.as_nanos(),
                elapsed.as_millis()
            );
            let mut incr = self.incr.lock();
            let incr = incr.as_mut().unwrap().as_mut().unwrap();
            incr.print_stats();
            incr.new_generation();
        }
        // self.incr.as_mut().unwrap().flush();
    }

    fn save_chunk(&mut self, start: usize, end: usize, dest: &mut dyn Write) {
        let mut src = &self.mmap[start..end];
        std::io::copy(&mut src, dest).unwrap();
    }

    /// Save a full dump of memory to a file, one-to-one
    pub fn dump_full(&mut self, out_file: &str) {
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
            let chunk_len = min(crate::MAX_CHUNK_LEN, end - pos);
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
