use core::hash::{Hash, Hasher};

pub fn hash<A: Hash + Copy, B: Hash + Copy, S: Hasher>(a: A, b: B, state: &mut S) {
    // If both index and generation are smaller than 64-bits in total
    // then just pack them into a u64 and hash that since that is likely to be
    // cheaper even on simple hash functions.
    if let Some((a, b)) = extract(a).zip(extract(b)) {
        state.write_u64((a as u64) << 32 | (b as u64));
        return;
    }

    a.hash(state);
    b.hash(state);
}

// for all normal keys, this is an easily optimized out check
fn extract<A: Hash>(a: A) -> Option<u32> {
    let mut hasher = ExtractOneHasherU32 { value: 0, count: 0 };

    a.hash(&mut hasher);

    if hasher.count == 1 {
        Some(hasher.value)
    } else {
        None
    }
}

struct ExtractOneHasherU32 {
    value: u32,
    count: u32,
}

impl Hasher for ExtractOneHasherU32 {
    fn finish(&self) -> u64 {
        unreachable!()
    }

    fn write(&mut self, _bytes: &[u8]) {}

    fn write_u8(&mut self, i: u8) {
        self.write_u32(i.into())
    }

    fn write_u16(&mut self, i: u16) {
        self.write_u32(i.into())
    }

    fn write_u32(&mut self, i: u32) {
        self.value = i;
        self.count = (self.count + 1).min(2);
    }
}
