#![allow(missing_docs)]

#[cfg(feature="unoptimised_masking")]
pub fn apply_mask(mask: [u8; 4], payload_chunk: &mut [u8], mut phase: u8) {
    for b in payload_chunk.iter_mut() {
        let index = (phase & 0x03) as usize;
        *b ^= mask[index];
        phase = (phase + 1) & 0x03;
    }
}

#[cfg(feature="masking_slice_size_4")]
const MASKING_SLICE_SIZE : usize = 4;
#[cfg(feature="masking_slice_size_8")]
const MASKING_SLICE_SIZE : usize = 8;
#[cfg(feature="masking_slice_size_16")]
const MASKING_SLICE_SIZE : usize = 16;
#[cfg(feature="masking_slice_size_32")]
const MASKING_SLICE_SIZE : usize = 32;

#[cfg(not(any(
    feature="masking_slice_size_4",
    feature="masking_slice_size_8",
    feature="masking_slice_size_16",
    feature="masking_slice_size_32",
)))]
const MASKING_SLICE_SIZE : usize = 32;

#[cfg(not(any(feature="unoptimised_masking", feature="explicitly_aligned_masking")))]
pub fn apply_mask(mask: [u8; 4], payload_chunk: &mut [u8], phase: u8) {
    let mut m = [0; MASKING_SLICE_SIZE];
    for (i, mb) in m.iter_mut().enumerate() {
        *mb = mask[(i + phase as usize) % 4];
    }
    let mut chunks = payload_chunk.chunks_exact_mut(m.len());
    for chunk in &mut chunks {
        for (b, maskbyte) in chunk.iter_mut().zip(m) {
            *b ^= maskbyte;
        }
    }
    for (b, maskbyte) in chunks.into_remainder().iter_mut().zip(m) {
        *b ^= maskbyte;
    }
}

#[cfg(feature="explicitly_aligned_masking")]
pub fn apply_mask(mask: [u8; 4], payload_chunk: &mut [u8], phase: u8) {
    #[cfg_attr(feature="masking_slice_size_4", repr(align(4)))]
    #[cfg_attr(feature="masking_slice_size_8", repr(align(8)))]
    #[cfg_attr(feature="masking_slice_size_16", repr(align(16)))]
    #[cfg_attr(feature="masking_slice_size_32", repr(align(32)))]
    struct Slice([u8; MASKING_SLICE_SIZE]);

    let (prefix, main_part, suffix) : (&mut [u8], &mut [Slice], &mut [u8]) = unsafe { payload_chunk.align_to_mut() };

    let mut m = Slice([0; MASKING_SLICE_SIZE]);
    for (i, b) in prefix.iter_mut().enumerate() {
        *b ^= mask[(i + phase as usize) % 4];
    }
    for (i, mb) in m.0.iter_mut().enumerate() {
        *mb = mask[(i + phase as usize + prefix.len()) % 4];
    }

    for slice in main_part.iter_mut() {
        for (mb, b) in m.0.iter().zip(slice.0.iter_mut()) {
            *b ^= *mb;
        }
    }

    for (mb, b) in m.0.iter().zip(suffix.iter_mut()) {
        *b ^= *mb;
    }
    
}
