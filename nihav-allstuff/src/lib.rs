//! Umbrella crate to register decoders and demuxers from all known NihAV crates.
extern crate nihav_core;
extern crate nihav_commonfmt;
extern crate nihav_duck;

use nihav_core::codecs::RegisteredDecoders;
use nihav_core::codecs::RegisteredPacketisers;
use nihav_core::codecs::RegisteredEncoders;
use nihav_core::demuxers::RegisteredDemuxers;
use nihav_core::demuxers::RegisteredRawDemuxers;
use nihav_core::muxers::RegisteredMuxers;

use nihav_commonfmt::*;
use nihav_duck::*;

/// Registers all known decoders.
pub fn nihav_register_all_decoders(rd: &mut RegisteredDecoders) {
    generic_register_all_decoders(rd);
    duck_register_all_decoders(rd);
}

/// Registers all known packetisers.
pub fn nihav_register_all_packetisers(rp: &mut RegisteredPacketisers) {
}

/// Registers all known demuxers.
pub fn nihav_register_all_demuxers(rd: &mut RegisteredDemuxers) {
    duck_register_all_demuxers(rd);
    generic_register_all_demuxers(rd);
}

/// Registers all known raw stream demuxers.
pub fn nihav_register_all_raw_demuxers(rd: &mut RegisteredRawDemuxers) {
}

/// Registers all known encoders.
pub fn nihav_register_all_encoders(re: &mut RegisteredEncoders) {
    duck_register_all_encoders(re);
}

/// Registers all known demuxers.
pub fn nihav_register_all_muxers(rm: &mut RegisteredMuxers) {
    generic_register_all_muxers(rm);
}

#[cfg(test)]
extern crate nihav_registry;

#[cfg(test)]
mod test {
    use super::*;
    use nihav_registry::register::get_codec_description;

    #[test]
    fn test_descriptions() {
        let mut rd = RegisteredDecoders::new();
        nihav_register_all_decoders(&mut rd);
        let mut has_missing = false;
        for dec in rd.iter() {
            print!("decoder {} - ", dec.name);
            let ret = get_codec_description(dec.name);
            if let Some(desc) = ret {
                println!("{}", desc);
            } else {
                println!("missing!");
                has_missing = true;
            }
        }
        assert!(!has_missing);
    }
}
