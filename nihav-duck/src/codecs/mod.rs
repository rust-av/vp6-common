use nihav_core::codecs::*;

macro_rules! validate {
    ($a:expr) => {
        if !$a {
            println!("check failed at {}:{}", file!(), line!());
            return Err(DecoderError::InvalidData);
        }
    };
}

#[cfg(any(feature = "encoder_vp6", feature = "decoder_vp6"))]
#[macro_use]
#[allow(clippy::erasing_op)]
#[allow(clippy::needless_range_loop)]
#[allow(clippy::too_many_arguments)]
#[allow(clippy::useless_let_if_seq)]
mod vpcommon;
#[cfg(any(feature = "encoder_vp6", feature = "decoder_vp6"))]
#[allow(clippy::needless_range_loop)]
#[allow(clippy::useless_let_if_seq)]
#[allow(clippy::too_many_arguments)]
mod vp56;
#[cfg(any(feature = "decoder_vp6", feature = "encoder_vp6"))]
mod vp6data;
#[cfg(feature = "decoder_vp6")]
#[allow(clippy::needless_range_loop)]
mod vp6dec;
#[cfg(any(feature = "decoder_vp6", feature = "encoder_vp6"))]
mod vp6dsp;

const DUCK_CODECS: &[DecoderInfo] = &[
    #[cfg(feature = "decoder_vp6")]
    DecoderInfo {
        name: "vp6",
        get_decoder: vp6dec::get_decoder_vp6,
    },
    #[cfg(feature = "decoder_vp6")]
    DecoderInfo {
        name: "vp6f",
        get_decoder: vp6dec::get_decoder_vp6f,
    },
    #[cfg(feature = "decoder_vp6")]
    DecoderInfo {
        name: "vp6a",
        get_decoder: vp6dec::get_decoder_vp6_alpha,
    },
];

/// Registers all available codecs provided by this crate.
pub fn duck_register_all_decoders(rd: &mut RegisteredDecoders) {
    for decoder in DUCK_CODECS.iter() {
        rd.add_decoder(*decoder);
    }
}

#[cfg(feature = "encoder_vp6")]
#[allow(clippy::needless_range_loop)]
mod vp6enc;

const DUCK_ENCODERS: &[EncoderInfo] = &[
    #[cfg(feature = "encoder_vp6")]
    EncoderInfo {
        name: "vp6",
        get_encoder: vp6enc::get_encoder,
    },
    #[cfg(feature = "encoder_vp6")]
    EncoderInfo {
        name: "vp6f",
        get_encoder: vp6enc::get_encoder_flv,
    },
];

/// Registers all available encoders provided by this crate.
pub fn duck_register_all_encoders(re: &mut RegisteredEncoders) {
    for encoder in DUCK_ENCODERS.iter() {
        re.add_encoder(*encoder);
    }
}
