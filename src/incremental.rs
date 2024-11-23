use std::{fs::File, hash::Hasher, io::Write};

use fxhash::FxHashMap;

pub type Hash = u64;

const HASH_CACHE_LEN: usize = 32;

/*
pub struct HashCache {
    /// Hashing is the part we spend most CPU time on by far
    hash_cache: FxHashMap<Vec<u8>, Hash>,
    /// Hashes sorted by frequency, for consideration to be promoted into the cache.
    hash_hit: FxHashMap<Hash, u32>,
    /// The most popular hashes, to
}

impl HashCache {
    pub fn new() -> Self {
        Self {
            hash_cache: FxHashMap::default(),
            hash_hit: FxHashMap::default(),
        }
    }

    pub fn query(&mut self, data: &[u8]) -> Option<Hash> {
        let h = self.hash_cache.get(data);
        if h.is_none() {
            return None;
        }
        if let Some(pop) = self.hash_popularity.get_mut(data) {
            *pop += 1;
        } else {
            self.hash_popularity.insert(data, 1);
        }
        return Some(*h.unwrap());
    }

    pub fn housekeep(&mut self) {}
}
*/

pub struct Incremental {
    /// Mapping of hashes for uncompressed data to their blocks and refcounts
    blocks: FxHashMap<Hash, (u32, u32)>,
    /// The current generation
    generation: u32,
    /// List of blocks in a given generation, by their hashes
    generations: Vec<Vec<Hash>>,
    /// File to write the snapshots and metadata to
    backing_file: Option<File>,
    /// File with compressor attached
    backing_compressor: Option<lz4::Encoder<File>>,
}

impl Incremental {
    pub fn new(compress: bool, dest_file: &str) -> Self {
        let f = File::create(dest_file).unwrap();
        if compress {
            let compressor = lz4::EncoderBuilder::new().level(1).build(f).unwrap();
            let mut incr = Incremental {
                blocks: FxHashMap::default(),
                generation: 0,
                generations: Vec::new(),
                backing_file: None,
                backing_compressor: Some(compressor),
            };
            // Special case: Gen 0 is initialized here already
            incr.generations.push(Vec::new());
            incr
        } else {
            let mut incr = Incremental {
                blocks: FxHashMap::default(),
                generation: 0,
                generations: Vec::new(),
                backing_file: Some(f),
                backing_compressor: None,
            };
            // Special case: Gen 0 is initialized here already
            incr.generations.push(Vec::new());
            incr
        }
    }

    pub fn new_generation(&mut self) {
        self.generation += 1;
        self.generations.push(Vec::new());

        // Good time to do some housekeeping around the hash cache
    }

    pub fn flush_chunk(&mut self, data: &[u8], hash: Hash) {
        // TODO: Also write block hash and len
        if self.backing_compressor.is_some() {
            // Compress block
            let mut data_reader = std::io::Cursor::new(data);
            std::io::copy(
                &mut data_reader,
                &mut self.backing_compressor.as_mut().unwrap(),
            )
            .unwrap();
        } else {
            self.backing_file.as_ref().unwrap().write_all(data).unwrap();
        }
    }

    pub fn hash_chunk(data: &[u8]) -> u64 {
        // Check if chunk is already in cache
        let mut hasher = wyhash2::WyHash::with_seed(0);
        hasher.write(data);
        hasher.finish()
    }

    pub fn add_hashed_chunk(&mut self, data: &[u8], hash: Hash) {
        let gen: usize = self.generation.try_into().unwrap();
        let gen: &mut Vec<u64> = &mut self.generations[gen];
        let mut new_block = false;
        if let Some(block) = self.blocks.get_mut(&hash) {
            block.1 += 1;
        } else {
            self.blocks.insert(hash, (0, 1));
            // Mark for write which needs to happen later to appease the borrow checker
            new_block = true;
        }
        gen.push(hash);
        if new_block {
            self.flush_chunk(data, hash);
        }
    }

    /// Output statistics to stdout
    pub fn print_stats(&self) {
        // Sum up blocks
        let total_blocks = self.blocks.len();
        println!(
            "Generation: {}, total blocks: {}",
            self.generation, total_blocks
        );
    }

    /// Flush metadata to disk
    pub fn flush(&mut self) {}
}
