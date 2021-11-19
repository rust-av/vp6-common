use nihav_core::codecs::*;

macro_rules! validate {
    ($a:expr) => { if !$a { println!("check failed at {}:{}", file!(), line!()); return Err(DecoderError::InvalidData); } };
}

#[cfg(feature="decoder_clearvideo")]
mod clearvideo;
#[cfg(feature="decoder_jpeg")]
mod jpeg;
#[cfg(feature="decoder_rawvideo")]
mod rawvideo;
#[cfg(feature="decoder_rawvideo_ms")]
mod rawvideo_ms;

#[cfg(feature="decoders")]
const DECODERS: &[DecoderInfo] = &[
#[cfg(feature="decoder_clearvideo")]
    DecoderInfo { name: "clearvideo", get_decoder: clearvideo::get_decoder },
#[cfg(feature="decoder_clearvideo")]
    DecoderInfo { name: "clearvideo_rm", get_decoder: clearvideo::get_decoder_rm },
#[cfg(feature="decoder_jpeg")]
    DecoderInfo { name: "jpeg", get_decoder: jpeg::get_decoder },
#[cfg(feature="decoder_rawvideo")]
    DecoderInfo { name: "rawvideo", get_decoder: rawvideo::get_decoder },
#[cfg(feature="decoder_rawvideo_ms")]
    DecoderInfo { name: "rawvideo-ms", get_decoder: rawvideo_ms::get_decoder },
];

/// Registers all available codecs provided by this crate.
#[cfg(feature="decoders")]
pub fn generic_register_all_decoders(rd: &mut RegisteredDecoders) {
    for decoder in DECODERS.iter() {
        rd.add_decoder(*decoder);
    }
}

#[cfg(feature="encoder_cinepak")]
mod cinepakenc;
#[cfg(feature="encoder_zmbv")]
mod zmbvenc;

#[cfg(feature="encoders")]
const ENCODERS: &[EncoderInfo] = &[
];
