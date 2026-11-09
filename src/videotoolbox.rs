//! Asks macOS which video codecs this Mac decodes in hardware.
#![allow(unsafe_code)]

/// A `CMVideoCodecType`, the four-character code of a codec.
#[derive(Clone, Copy)]
pub struct Codec(u32);

pub const AV1: Codec = Codec(u32::from_be_bytes(*b"av01"));
pub const VP9: Codec = Codec(u32::from_be_bytes(*b"vp09"));

// SAFETY: the signatures match the VideoToolbox headers; both functions take a plain
// codec code (`Boolean` is an unsigned char) and have no other preconditions.
#[link(name = "VideoToolbox", kind = "framework")]
unsafe extern "C" {
    safe fn VTRegisterSupplementalVideoDecoderIfAvailable(codec: u32);
    safe fn VTIsHardwareDecodeSupported(codec: u32) -> u8;
}

pub fn decodes_in_hardware(codec: Codec) -> bool {
    // VP9 and AV1 decoders are supplemental and only count once registered.
    VTRegisterSupplementalVideoDecoderIfAvailable(codec.0);
    VTIsHardwareDecodeSupported(codec.0) != 0
}
