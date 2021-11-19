use std::io::SeekFrom;
use nihav_core::codecs::*;
use nihav_core::demuxers::*;
use nihav_core::muxers::*;
use nihav_registry::detect;
use nihav_core::io::byteio::ByteReader;
use nihav_allstuff::*;
use crate::null::*;

pub struct FullRegister {
    pub dmx_reg:    RegisteredDemuxers,
    pub rdmx_reg:   RegisteredRawDemuxers,
    pub pkt_reg:    RegisteredPacketisers,
    pub dec_reg:    RegisteredDecoders,
    pub enc_reg:    RegisteredEncoders,
    pub mux_reg:    RegisteredMuxers,
}

impl FullRegister {
    pub fn new() -> Self {
        let mut dmx_reg = RegisteredDemuxers::new();
        nihav_register_all_demuxers(&mut dmx_reg);
        let mut rdmx_reg = RegisteredRawDemuxers::new();
        nihav_register_all_raw_demuxers(&mut rdmx_reg);
        let mut dec_reg = RegisteredDecoders::new();
        nihav_register_all_decoders(&mut dec_reg);
        let mut pkt_reg = RegisteredPacketisers::new();
        nihav_register_all_packetisers(&mut pkt_reg);
        let mut enc_reg = RegisteredEncoders::new();
        nihav_register_all_encoders(&mut enc_reg);
        enc_reg.add_encoder(NULL_ENCODER);
        let mut mux_reg = RegisteredMuxers::new();
        nihav_register_all_muxers(&mut mux_reg);
        mux_reg.add_muxer(NULL_MUXER);
        Self { dmx_reg, rdmx_reg, pkt_reg, dec_reg, enc_reg, mux_reg }
    }
}

pub struct RawStreamCtx<'a> {
    stream:     NAStreamRef,
    sm:         StreamManager,
    packetiser: Box<dyn NAPacketiser + Send>,
    br:         &'a mut ByteReader<'a>,
}

impl<'a> RawStreamCtx<'a> {
    fn new(stream: NAStreamRef, packetiser: Box<dyn NAPacketiser + Send>, br: &'a mut ByteReader<'a>) -> Self {
        let mut sm = StreamManager::new();
        sm.add_stream_ref(stream.clone());
        Self { stream, sm, packetiser, br }
    }
}

pub enum DemuxerObject<'a> {
    None,
    Normal(Demuxer<'a>),
    Raw(RawDemuxer<'a>, Vec<Option<Box<dyn NAPacketiser + Send>>>, bool),
    RawStream(RawStreamCtx<'a>),
}

impl<'a> DemuxerObject<'a> {
    pub fn create(br: &'a mut ByteReader<'a>, reg: &FullRegister, name: &str, ifmt: &Option<String>, is_raw: bool) -> DemuxerObject<'a> {
        if !is_raw {
            let dmx_name = if let Some(ref dname) = ifmt {
                    dname.as_str()
                } else if let Some((dname, score)) = detect::detect_format(name, br) {
                    println!("detected {} with score {:?}", dname, score);
                    dname
                } else {
                    ""
                };
            if dmx_name != "" {
                println!("trying demuxer {} on {}", dmx_name, name);
                if let Some(dmx_fact) = reg.dmx_reg.find_demuxer(dmx_name) {
                    br.seek(SeekFrom::Start(0)).unwrap();
                    let dmx = create_demuxer(dmx_fact, br).unwrap();
                    return DemuxerObject::Normal(dmx);
                }
            }
            if ifmt.is_some() {
                return DemuxerObject::None;
            }
            if dmx_name != "" {
                println!("trying raw demuxer {} on {}", dmx_name, name);
                if let Some(rdmx_fact) = reg.rdmx_reg.find_demuxer(dmx_name) {
                    br.seek(SeekFrom::Start(0)).unwrap();
                    let dmx = create_raw_demuxer(rdmx_fact, br).unwrap();
                    let mut pkts = Vec::new();
                    for stream in dmx.get_streams() {
                        if let Some(pcreate) = reg.pkt_reg.find_packetiser(stream.get_info().get_name()) {
                            let packetiser = (pcreate)();
                            pkts.push(Some(packetiser));
                        } else {
                            pkts.push(None);
                        }
                    }
                    return DemuxerObject::Raw(dmx, pkts, false);
                }
            }
            for rdmx in reg.rdmx_reg.iter() {
                if rdmx.check_format(br) {
                    println!("detected {} as {}", name, rdmx.get_name());
                    br.seek(SeekFrom::Start(0)).unwrap();
                    let dmx = create_raw_demuxer(*rdmx, br).unwrap();
                    let mut pkts = Vec::new();
                    for stream in dmx.get_streams() {
                        if let Some(pcreate) = reg.pkt_reg.find_packetiser(stream.get_info().get_name()) {
                            let packetiser = (pcreate)();
                            pkts.push(Some(packetiser));
                        } else {
                            pkts.push(None);
                        }
                    }
                    return DemuxerObject::Raw(dmx, pkts, false);
                }
            }
        }
        br.seek(SeekFrom::Start(0)).unwrap();
        let mut buf = vec![0; 1048576];
        let size = br.peek_buf(&mut buf).unwrap();
        let mut pname = "";

        for pinfo in reg.pkt_reg.iter() {
            let mut packetiser = (pinfo.get_packetiser)();
            packetiser.add_data(&buf[..size]);
            if packetiser.parse_stream(0).is_ok() {
                pname = pinfo.name;
                break;
            }
        }
        if pname != "" {
            println!("found raw stream of type {} for {}", pname, name);
            let pcreate = reg.pkt_reg.find_packetiser(pname).unwrap();
            let mut packetiser = (pcreate)();
            packetiser.add_data(&buf[..size]);
            let stream = packetiser.parse_stream(0).unwrap();
            packetiser.reset();
            DemuxerObject::RawStream(RawStreamCtx::new(stream, packetiser, br))
        } else {
            DemuxerObject::None
        }
    }
    pub fn is_none(&self) -> bool {
        match *self {
            DemuxerObject::None => true,
            _ => false,
        }
    }
    pub fn get_duration(&self) -> u64 {
        match *self {
            DemuxerObject::Normal(ref dmx) => dmx.get_duration(),
            DemuxerObject::Raw(ref dmx, _, _) => dmx.get_duration(),
            _ => 0,
        }
    }
    pub fn get_num_streams(&self) -> usize {
        match *self {
            DemuxerObject::None => 0,
            DemuxerObject::Normal(ref dmx) => dmx.get_num_streams(),
            DemuxerObject::Raw(ref dmx, _, _) => dmx.get_num_streams(),
            DemuxerObject::RawStream(_) => 1,
        }
    }
    pub fn get_stream(&self, idx: usize) -> Option<NAStreamRef> {
        match *self {
            DemuxerObject::Normal(ref dmx) => dmx.get_stream(idx),
            DemuxerObject::Raw(ref dmx, _, _) => dmx.get_stream(idx),
            DemuxerObject::RawStream(ref ctx) if idx == 0 => Some(ctx.stream.clone()),
            _ => None,
        }
    }
    pub fn get_streams(&self) -> StreamIter {
        match *self {
            DemuxerObject::Normal(ref dmx) => dmx.get_streams(),
            DemuxerObject::Raw(ref dmx, _, _) => dmx.get_streams(),
            DemuxerObject::RawStream(ref ctx) => ctx.sm.iter(),
            _ => unreachable!(),
        }
    }
    pub fn get_stream_manager(&self) -> &StreamManager {
        match *self {
            DemuxerObject::Normal(ref dmx) => dmx.get_stream_manager(),
            DemuxerObject::Raw(ref dmx, _, _) => dmx.get_stream_manager(),
            DemuxerObject::RawStream(ref ctx) => &ctx.sm,
            _ => unreachable!(),
        }
    }
    pub fn get_frame(&mut self) -> DemuxerResult<NAPacket> {
        match *self {
            DemuxerObject::Normal(ref mut dmx) => dmx.get_frame(),
            DemuxerObject::Raw(ref mut dmx, ref mut packetisers, ref mut eof) => {
                loop {
                    let mut has_some = false;
                    for (stream, p) in dmx.get_streams().zip(packetisers.iter_mut()) {
                        if let Some(ref mut pkts) = p {
                            match pkts.get_packet(stream.clone()) {
                                Ok(Some(pkt)) => return Ok(pkt),
                                Ok(None) | Err(DecoderError::ShortData) => {
                                    if *eof {
                                        *p = None;
                                    }
                                },
                                Err(err) => {
                                    println!("packetisation error {:?}", err);
                                    return Err(DemuxerError::InvalidData);
                                }
                            };
                            has_some |= p.is_some();
                        }
                    }
                    if !has_some {
                        return Err(DemuxerError::EOF);
                    }
                    if let Ok(data) = dmx.get_data() {
                        let id = data.get_stream().get_id();
                        for (i, stream) in dmx.get_streams().enumerate() {
                            if stream.get_id() == id {
                                if let Some(ref mut pkts) = packetisers[i] {
                                    pkts.add_data(&data.get_buffer());
                                }
                                break;
                            }
                        }
                    } else {
                        *eof = true;
                    }
                }
            },
            DemuxerObject::RawStream(ref mut ctx) => {
                let mut buf = [0; 65536];
                loop {
                    match ctx.packetiser.get_packet(ctx.stream.clone()) {
                        Ok(Some(packet)) => return Ok(packet),
                        Ok(None) => {},
                        Err(DecoderError::ShortData) => {},
                        _ => return Err(DemuxerError::InvalidData),
                    };
                    match ctx.br.read_buf_some(&mut buf) {
                        Ok(size) => {
                            ctx.packetiser.add_data(&buf[..size]);
                        },
                        Err(_) => {
                            match ctx.packetiser.get_packet(ctx.stream.clone()) {
                                Ok(Some(packet)) => return Ok(packet),
                                Ok(None) | Err(DecoderError::ShortData) => return Err(DemuxerError::EOF),
                                _ => return Err(DemuxerError::InvalidData),
                            };
                        },
                    };
                }
            },
            _ => unreachable!(),
        }
    }
    pub fn seek(&mut self, seek_time: NATimePoint) -> DemuxerResult<()> {
        match *self {
            DemuxerObject::Normal(ref mut dmx) => dmx.seek(seek_time),
            DemuxerObject::Raw(ref mut dmx, _, _) => dmx.seek(seek_time),
            _ => Err(DemuxerError::NotImplemented),
        }
    }
}

impl<'a> NAOptionHandler for DemuxerObject<'a> {
    fn get_supported_options(&self) -> &[NAOptionDefinition] {
        match *self {
            DemuxerObject::Normal(ref dmx) => dmx.get_supported_options(),
            DemuxerObject::Raw(ref dmx, _, _) => dmx.get_supported_options(),
            _ => &[],
        }
    }
    fn set_options(&mut self, options: &[NAOption]) {
        match *self {
            DemuxerObject::Normal(ref mut dmx) => dmx.set_options(options),
            DemuxerObject::Raw(ref mut dmx, _, _) => dmx.set_options(options),
            _ => {},
        }
    }
    fn query_option_value(&self, name: &str) -> Option<NAValue> {
        match *self {
            DemuxerObject::Normal(ref dmx) => dmx.query_option_value(name),
            DemuxerObject::Raw(ref dmx, _, _) => dmx.query_option_value(name),
            _ => None,
        }
    }
}

pub fn detect_tags(br: &mut ByteReader) -> (bool, u64, Option<u64>) {
    let mut is_raw = false;
    let mut start = 0;
    let mut end = None;

    // check for ID3v{2-4}
    let mut buf = [0; 5];
    br.peek_buf(&mut buf).unwrap();
    if &buf[0..3] == b"ID3" && buf[3] > 0 && buf[3] < 5 && buf[4] == 0 { //ID3 tag found, must be a raw stream
        br.read_skip(6).unwrap();
        let mut size = 0;
        for _ in 0..4 {
            let b = br.read_byte().unwrap();
            if (b & 0x80) != 0 {
                println!("Invalid ID3 size");
                break;
            }
            size = (size << 7) | u64::from(b);
        }
        start = size + 10;
        is_raw = true;
    }
    // check for ID3v1
    br.seek(SeekFrom::End(-128)).unwrap();
    let off = br.tell();
    br.peek_buf(&mut buf[..3]).unwrap();
    if &buf[0..3] == b"TAG" {
        end = Some(off);
    }
    // check for APETAG
    let mut buf = [0; 8];
    if let Some(off) = end {
        br.seek(SeekFrom::Start(off - 32)).unwrap();
    } else {
        br.seek(SeekFrom::End(-32)).unwrap();
    }
    let off = br.tell();
    br.read_buf(&mut buf).unwrap();
    if &buf == b"APETAGEX" {
        let ver     = br.read_u32le().unwrap();
        let size    = u64::from(br.read_u32le().unwrap());
        let _items  = br.read_u32le().unwrap();
        let flags   = br.read_u32le().unwrap();
        if ver == 1000 || (flags & 0x80000000) == 0 {
            end = Some(off - size + 32);
        } else {
            end = Some(off - size);
        }
    }

    (is_raw, start, end)
}
