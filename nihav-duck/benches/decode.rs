use std::fs::File;

use criterion::{criterion_group, criterion_main, Criterion};

use nihav_commonfmt::generic_register_all_demuxers;
use nihav_core::codecs::*;
use nihav_core::demuxers::*;
use nihav_duck::duck_register_all_decoders;

type Decs = Vec<Option<(Box<NADecoderSupport>, Box<dyn NADecoder>)>>;

fn test_decoding(
    demuxer: &str,
    dec_name: &str,
    filename: &str,
    limit: Option<u64>,
    dmx_reg: &RegisteredDemuxers,
    dec_reg: &RegisteredDecoders,
) {
    let dmx_f = dmx_reg.find_demuxer(demuxer).unwrap();
    let mut file = File::open(filename).unwrap();
    let mut fr = FileReader::new_read(&mut file);
    let mut br = ByteReader::new(&mut fr);
    let mut dmx = create_demuxer(dmx_f, &mut br).unwrap();

    let mut decs: Decs = Vec::new();
    let mut found = false;
    for i in 0..dmx.get_num_streams() {
        let s = dmx.get_stream(i).unwrap();
        let info = s.get_info();
        if !found && (info.get_name() == dec_name) {
            let decfunc = dec_reg.find_decoder(info.get_name());
            if let Some(df) = decfunc {
                let mut dec = (df)();
                let mut dsupp = Box::new(NADecoderSupport::new());
                dec.init(&mut dsupp, info).unwrap();
                decs.push(Some((dsupp, dec)));
                found = true;
            } else {
                decs.push(None);
            }
        } else {
            decs.push(None);
        }
    }

    loop {
        let pktres = dmx.get_frame();
        if let Err(e) = pktres {
            if e == DemuxerError::EOF {
                break;
            }
            panic!("error");
        }
        let pkt = pktres.unwrap();
        let streamno = pkt.get_stream().get_id() as usize;
        if let Some((ref mut dsupp, ref mut dec)) = decs[streamno] {
            if limit.is_some() && pkt.get_pts().is_some() && pkt.get_pts().unwrap() > limit.unwrap()
            {
                break;
            }
            dec.decode(dsupp, &pkt).unwrap();
        }
    }
}

fn decode_vp6() {
    let mut dmx_reg = RegisteredDemuxers::new();
    generic_register_all_demuxers(&mut dmx_reg);
    let mut dec_reg = RegisteredDecoders::new();
    duck_register_all_decoders(&mut dec_reg);

    test_decoding(
        "avi",
        "vp6",
        "assets/selection_720x576_300kBit_vp60i.avi",
        Some(16),
        &dmx_reg,
        &dec_reg,
    );
}

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("decode_vp6", |b| b.iter(|| decode_vp6));
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = criterion_benchmark
}
criterion_main!(benches);
