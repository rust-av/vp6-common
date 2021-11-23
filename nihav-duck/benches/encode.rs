use std::fs::File;

use criterion::{criterion_group, criterion_main, Criterion};

use nihav_commonfmt::*;
use nihav_core::codecs::*;
use nihav_core::demuxers::*;
use nihav_core::muxers::*;
use nihav_core::scale::*;
use nihav_duck::*;

pub struct DecoderTestParams {
    /// Demuxer name e.g. `"mov"`.
    pub demuxer: &'static str,
    /// Input file name.
    pub in_name: &'static str,
    /// Timestamp for last decoded frame.
    pub limit: Option<u64>,
    /// Desired input stream type (that will be decoded and fed to the encoder).
    pub stream_type: StreamType,
    /// Registered demuxers.
    pub dmx_reg: RegisteredDemuxers,
    /// Registered decoders.
    pub dec_reg: RegisteredDecoders,
}

/// Parameters for the encoding test output.
pub struct EncoderTestParams {
    /// Muxer name e.g. `"avi"`.
    pub muxer: &'static str,
    /// Encoder name.
    pub enc_name: &'static str,
    /// Output file name.
    pub out_name: &'static str,
    /// Registered muxers.
    pub mux_reg: RegisteredMuxers,
    /// Registered encoders.
    pub enc_reg: RegisteredEncoders,
}

pub fn test_encoding(
    dec_config: &DecoderTestParams,
    enc_config: &EncoderTestParams,
    mut enc_params: EncodeParameters,
    enc_options: &[NAOption],
) {
    let dmx_f = dec_config.dmx_reg.find_demuxer(dec_config.demuxer).unwrap();
    let mut file = File::open(dec_config.in_name).unwrap();
    let mut fr = FileReader::new_read(&mut file);
    let mut br = ByteReader::new(&mut fr);
    let mut dmx = create_demuxer(dmx_f, &mut br).unwrap();

    let in_stream = dmx
        .get_streams()
        .find(|str| str.get_media_type() == dec_config.stream_type)
        .unwrap();
    let in_stream_id = in_stream.id;
    let decfunc = dec_config
        .dec_reg
        .find_decoder(in_stream.get_info().get_name())
        .unwrap();
    let mut dec = (decfunc)();
    let mut dsupp = Box::new(NADecoderSupport::new());
    dec.init(&mut dsupp, in_stream.get_info()).unwrap();

    enc_params.tb_num = in_stream.tb_num;
    enc_params.tb_den = in_stream.tb_den;

    if let (NACodecTypeInfo::Video(ref mut vinfo), Some(ref_vinfo)) = (
        &mut enc_params.format,
        in_stream.get_info().get_properties().get_video_info(),
    ) {
        if vinfo.width == 0 {
            vinfo.width = ref_vinfo.width;
            vinfo.height = ref_vinfo.height;
        }
    }

    let encfunc = enc_config
        .enc_reg
        .find_encoder(enc_config.enc_name)
        .unwrap();
    let mut encoder = (encfunc)();
    encoder.set_options(enc_options);
    let out_str = encoder.init(0, enc_params).unwrap();

    let info = out_str.get_info();
    if let NACodecTypeInfo::Video(ref vinfo) = info.get_properties() {
        let mut hdr = [0u8; 10];
        hdr[0] = (vinfo.width >> 24) as u8;
        hdr[1] = (vinfo.width >> 16) as u8;
        hdr[2] = (vinfo.width >> 8) as u8;
        hdr[3] = vinfo.width as u8;
        hdr[4] = (vinfo.height >> 24) as u8;
        hdr[5] = (vinfo.height >> 16) as u8;
        hdr[6] = (vinfo.height >> 8) as u8;
        hdr[7] = vinfo.height as u8;
        hdr[8] = vinfo.flipped as u8;
        hdr[9] = vinfo.bits;
    }

    let (mut ifmt, dst_vinfo) = if let NACodecTypeInfo::Video(vinfo) = enc_params.format {
        (
            ScaleInfo {
                fmt: vinfo.format,
                width: vinfo.width,
                height: vinfo.height,
            },
            vinfo,
        )
    } else {
        (
            ScaleInfo {
                fmt: YUV420_FORMAT,
                width: 2,
                height: 2,
            },
            NAVideoInfo {
                width: 2,
                height: 2,
                format: YUV420_FORMAT,
                flipped: false,
                bits: 12,
            },
        )
    };
    let ofmt = ifmt;
    let mut scaler = NAScale::new(ifmt, ofmt).unwrap();
    let mut cvt_buf = alloc_video_buffer(dst_vinfo, 2).unwrap();
    loop {
        let pktres = dmx.get_frame();
        if let Err(e) = pktres {
            if e == DemuxerError::EOF {
                break;
            }
            panic!("decoding error");
        }
        let pkt = pktres.unwrap();
        if pkt.get_stream().id != in_stream_id {
            continue;
        }
        let frm = dec.decode(&mut dsupp, &pkt).unwrap();
        let buf = frm.get_buffer();
        let cfrm = if let NACodecTypeInfo::Video(_) = enc_params.format {
            let cur_ifmt = get_scale_fmt_from_pic(&buf);
            if cur_ifmt != ifmt {
                ifmt = cur_ifmt;
                scaler = NAScale::new(ifmt, ofmt).unwrap();
            }
            scaler.convert(&buf, &mut cvt_buf).unwrap();
            NAFrame::new(
                frm.get_time_information(),
                frm.frame_type,
                frm.key,
                frm.get_info(),
                cvt_buf.clone(),
            )
        } else {
            panic!("unexpected format");
        };
        encoder.encode(&cfrm).unwrap();
        while let Ok(Some(_)) = encoder.get_packet() {}
        if let Some(maxts) = dec_config.limit {
            if frm.get_pts().unwrap_or(0) >= maxts {
                break;
            }
        }
    }
    encoder.flush().unwrap();
    while let Ok(Some(_)) = encoder.get_packet() {}
}

fn encode_test(out_name: &'static str, enc_options: &[NAOption]) {
    let mut dmx_reg = RegisteredDemuxers::new();
    generic_register_all_demuxers(&mut dmx_reg);
    let mut dec_reg = RegisteredDecoders::new();
    duck_register_all_decoders(&mut dec_reg);
    let mut mux_reg = RegisteredMuxers::new();
    generic_register_all_muxers(&mut mux_reg);
    let mut enc_reg = RegisteredEncoders::new();
    duck_register_all_encoders(&mut enc_reg);

    let dec_config = DecoderTestParams {
        demuxer: "avi",
        in_name: "assets/vp6_crash.avi",
        stream_type: StreamType::Video,
        limit: Some(1),
        dmx_reg,
        dec_reg,
    };
    let enc_config = EncoderTestParams {
        muxer: "avi",
        enc_name: "vp6",
        out_name,
        mux_reg,
        enc_reg,
    };
    let dst_vinfo = NAVideoInfo {
        width: 0,
        height: 0,
        format: YUV420_FORMAT,
        flipped: true,
        bits: 12,
    };
    let enc_params = EncodeParameters {
        format: NACodecTypeInfo::Video(dst_vinfo),
        quality: 0,
        bitrate: 25000,
        tb_num: 0,
        tb_den: 0,
        flags: 0,
    };
    test_encoding(&dec_config, &enc_config, enc_params, enc_options);
}

const QUANT_OPTION: &str = "quant";

fn encode_vp6() {
    let enc_options = &[NAOption {
        name: QUANT_OPTION,
        value: NAValue::Int(42),
    }];
    encode_test("vp6-bool.avi", enc_options);
}

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("encode_vp6", |b| b.iter(|| encode_vp6));
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = criterion_benchmark
}
criterion_main!(benches);
