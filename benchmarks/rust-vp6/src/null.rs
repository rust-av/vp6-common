use nihav_core::codecs::*;
use nihav_core::muxers::*;
use std::marker::PhantomData;

#[derive(Default)]
struct NullEncoder {
    stream: Option<NAStreamRef>,
    pkt:    Option<NAPacket>,
}

impl NullEncoder {
    fn new() -> Self { Self::default() }
}

impl NAEncoder for NullEncoder {
    fn negotiate_format(&self, encinfo: &EncodeParameters) -> EncoderResult<EncodeParameters> {
        Ok(*encinfo)
    }
    fn init(&mut self, stream_id: u32, encinfo: EncodeParameters) -> EncoderResult<NAStreamRef> {
        let stype = match encinfo.format {
                NACodecTypeInfo::Audio(_) => StreamType::Audio,
                NACodecTypeInfo::Video(_) => StreamType::Video,
                NACodecTypeInfo::None => StreamType::Data,
            };
        let info = NACodecInfo::new("null", encinfo.format, None);
        let mut stream = NAStream::new(stype, stream_id, info, encinfo.tb_num, encinfo.tb_den, 0);
        stream.set_num(stream_id as usize);
        let stream = stream.into_ref();
        self.stream = Some(stream.clone());

        Ok(stream)
    }
    fn encode(&mut self, frm: &NAFrame) -> EncoderResult<()> {
        self.pkt = Some(NAPacket::new(self.stream.clone().unwrap(), frm.ts, true, Vec::new()));
        Ok(())
    }
    fn get_packet(&mut self) -> EncoderResult<Option<NAPacket>> {
        let mut npkt = None;
        std::mem::swap(&mut self.pkt, &mut npkt);
        Ok(npkt)
    }
    fn flush(&mut self) -> EncoderResult<()> {
        Ok(())
    }
}

impl NAOptionHandler for NullEncoder {
    fn get_supported_options(&self) -> &[NAOptionDefinition] { &[] }
    fn set_options(&mut self, _options: &[NAOption]) { }
    fn query_option_value(&self, _name: &str) -> Option<NAValue> { None }
}

fn get_encoder() -> Box<dyn NAEncoder + Send> {
    Box::new(NullEncoder::new())
}

pub const NULL_ENCODER: EncoderInfo = EncoderInfo { name: "null", get_encoder };

struct NullMuxer<'a> {
    bw:             PhantomData<&'a mut ByteWriter<'a>>,
}

impl<'a> NullMuxer<'a> {
    fn new(_bw: &'a mut ByteWriter<'a>) -> Self {
        Self {
            bw: PhantomData::default(),
        }
    }
}

impl<'a> MuxCore<'a> for NullMuxer<'a> {
    fn create(&mut self, _strmgr: &StreamManager) -> MuxerResult<()> {
        Ok(())
    }
    fn mux_frame(&mut self, _strmgr: &StreamManager, _pkt: NAPacket) -> MuxerResult<()> {
        Ok(())
    }
    fn flush(&mut self) -> MuxerResult<()> {
        Ok(())
    }
    fn end(&mut self) -> MuxerResult<()> {
        Ok(())
    }
}

impl<'a> NAOptionHandler for NullMuxer<'a> {
    fn get_supported_options(&self) -> &[NAOptionDefinition] { &[] }
    fn set_options(&mut self, _options: &[NAOption]) { }
    fn query_option_value(&self, _name: &str) -> Option<NAValue> { None }
}

pub struct NullMuxerCreator {}

impl MuxerCreator for NullMuxerCreator {
    fn new_muxer<'a>(&self, bw: &'a mut ByteWriter<'a>) -> Box<dyn MuxCore<'a> + 'a> {
        Box::new(NullMuxer::new(bw))
    }
    fn get_name(&self) -> &'static str { "null" }
    fn get_capabilities(&self) -> MuxerCapabilities { MuxerCapabilities::Universal }
}

pub const NULL_MUXER: &dyn MuxerCreator = &NullMuxerCreator{};
