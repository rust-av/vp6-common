extern crate nihav_allstuff;
extern crate nihav_codec_support;
extern crate nihav_core;
extern crate nihav_registry;

use nihav_core::codecs::*;
use nihav_core::demuxers::*;
use nihav_core::frame::*;
use nihav_core::io::byteio::{ByteReader, FileReader};
use nihav_core::options::*;
use nihav_core::scale::*;
use nihav_core::soundcvt::*;
use nihav_registry::detect;
use nihav_registry::register;
use std::env;
use std::fs::File;
use std::io::{BufReader, Write};

mod demux;
use crate::demux::*;
mod null;

#[derive(Default)]
#[allow(clippy::type_complexity)]
struct Transcoder {
    input_name: String,
    input_fmt: Option<String>,
    decoders: Vec<
        Option<(
            Box<NADecoderSupport>,
            Box<dyn NADecoder>,
        )>,
    >,
}

impl Transcoder {
    fn new() -> Self {
        Self::default()
    }
}

#[allow(clippy::single_match)]
fn main() {
    let args: Vec<_> = env::args().collect();

    if args.len() != 2 {
        eprintln!("File path expected");
        return;
    }

    let full_reg = FullRegister::new();

    let mut transcoder = Transcoder::new();
    transcoder.input_name = args[1].clone();

    let res = File::open(transcoder.input_name.as_str());
    if res.is_err() {
        println!("error opening input");
        return;
    }
    let file = res.unwrap();
    let file = BufReader::new(file);
    let mut fr = FileReader::new_read(file);
    let mut br = ByteReader::new(&mut fr);
    let (is_raw, start, end) = if transcoder.input_fmt.is_none() {
        detect_tags(&mut br)
    } else {
        (false, 0, None)
    };

    let mut nfr: Box<dyn ByteIO>;
    if start != 0 || end.is_some() {
        let file = fr.finish();
        nfr = Box::new(BoundedFileReader::new_read(file, start, end).unwrap());
    } else {
        nfr = Box::new(fr);
    }
    let mut br = ByteReader::new(nfr.as_mut());

    let mut dmx = DemuxerObject::create(
        &mut br,
        &full_reg,
        transcoder.input_name.as_str(),
        &transcoder.input_fmt,
        is_raw,
    );
    if dmx.is_none() {
        println!(
            "cannot find demuxer for '{}'",
            transcoder.input_name.as_str()
        );
        return;
    }

    for i in 0..dmx.get_num_streams() {
        let s = dmx.get_stream(i).unwrap();
        let info = s.get_info();
        let decfunc = full_reg.dec_reg.find_decoder(info.get_name());
        let str_id = s.get_num() as u32;
        if let Some(create_dec) = decfunc {
            let mut dec = (create_dec)();
            let mut dsupp = Box::new(NADecoderSupport::new());
            let ret = dec.init(&mut dsupp, info.clone());
            if ret.is_err() {
                println!("Error initialising decoder '{}' for stream {}", info.get_name(), str_id);
                return;
            }
            transcoder.decoders.push(Some((dsupp, dec)))
        } else {
            println!("No decoder for stream {} ({}) is found", str_id, info.get_name());
            transcoder.decoders.push(None);
        }
    }

    'main_loop: loop {
        let pktres = dmx.get_frame();
        if let Err(DemuxerError::EOF) = pktres {
            break;
        }
        if pktres.is_err() {
            println!("demuxing error");
            break;
        }
        let mut pkt = pktres.unwrap();
        let src_id = pkt.get_stream().get_num();

        if let Some((ref mut dsupp, ref mut decoder)) =
            transcoder.decoders[src_id]
        {
            let ret = decoder.decode(dsupp, &pkt);
            if ret.is_err() {
                println!("error decoding stream {}", src_id);
                break;
            }
        } else {
            println!("no decoder for stream {}", src_id);
            break;
        }
    }
}
