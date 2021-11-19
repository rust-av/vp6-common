use nihav_core::frame::*;
use nihav_codec_support::codecs::blockdsp::edge_emu;

#[allow(clippy::too_many_arguments)]
pub fn get_block(dst: &mut [u8], dstride: usize, src: NAVideoBufferRef<u8>, comp: usize,
                 dx: usize, dy: usize, mv_x: i16, mv_y: i16)
{
    let (w, h) = src.get_dimensions(comp);
    let sx = (dx as isize) + (mv_x as isize);
    let sy = (dy as isize) + (mv_y as isize);

    if (sx - 2 < 0) || (sx + 8 + 2 > (w as isize)) ||
       (sy - 2 < 0) || (sy + 8 + 2 > (h as isize)) {
        edge_emu(&src, sx - 2, sy - 2, 8 + 2 + 2, 8 + 2 + 2,
                 dst, dstride, comp, 0);
    } else {
        let sstride = src.get_stride(comp);
        let soff    = src.get_offset(comp);
        let sdta    = src.get_data();
        let sbuf: &[u8] = sdta.as_slice();
        let saddr = soff + ((sx - 2) as usize) + ((sy - 2) as usize) * sstride;
        let src = &sbuf[saddr..];
        for (dline, sline) in dst.chunks_mut(dstride).zip(src.chunks(sstride)).take(12) {
            dline[..12].copy_from_slice(&sline[..12]);
        }
    }
}

pub fn calc_variance(src: &[u8], stride: usize) -> u16 {
    let mut sum = 0;
    let mut ssum = 0;
    for line in src.chunks(stride * 2).take(4) {
        for el in line.iter().take(8).step_by(2) {
            let pix = u32::from(*el);
            sum += pix;
            ssum += pix * pix;
        }
    }
    ((ssum * 16 - sum * sum) >> 8) as u16
}

macro_rules! mc_filter {
    (bilinear; $a: expr, $b: expr, $c: expr) => {
        ((u16::from($a) * (8 - $c) + u16::from($b) * $c + 4) >> 3) as u8
    };
    (bicubic; $src: expr, $off: expr, $step: expr, $coeffs: expr) => {
        ((i32::from($src[$off - $step]    ) * i32::from($coeffs[0]) +
          i32::from($src[$off]            ) * i32::from($coeffs[1]) +
          i32::from($src[$off + $step]    ) * i32::from($coeffs[2]) +
          i32::from($src[$off + $step * 2]) * i32::from($coeffs[3]) + 64) >> 7).min(255).max(0) as u8
    }
}

//#[allow(snake_case)]
pub fn mc_bilinear<const SOFF: usize, const SSTRIDE: usize>(dst: &mut [u8], dstride: usize, src: &[u8], mx: u16, my: u16) {
    let mut soff = SOFF;
    assert!(dstride >= 8);
        
    if my == 0 {
        assert!(src.len() >= soff + SSTRIDE * 8);
        for dline in dst.chunks_exact_mut(dstride).take(8) {
            for i in 0..8 {
                dline[i] = mc_filter!(bilinear; src[soff + i], src[soff + i + 1], mx);
            }
            soff += SSTRIDE;
        }
    } else if mx == 0 {
        assert!(src.len() >= soff + SSTRIDE*8);
        for dline in dst.chunks_exact_mut(dstride).take(8) {
            for i in 0..8 {
                dline[i] = mc_filter!(bilinear; src[soff + i], src[soff + i + SSTRIDE], my);
            }
            soff += SSTRIDE;
        }
    } else {
        let mut tmp = [0u8; 8];
        for i in 0..8 {
            tmp[i] = mc_filter!(bilinear; src[soff + i], src[soff + i + 1], mx);
        }
        soff += SSTRIDE;
        
        assert!(src.len() >= soff + SSTRIDE*8);
        for dline in dst.chunks_exact_mut(dstride).take(8) {
            for i in 0..8 {
                let cur = mc_filter!(bilinear; src[soff + i], src[soff + i + 1], mx);
                dline[i] = mc_filter!(bilinear; tmp[i], cur, my);
                tmp[i] = cur;
            }
            soff += SSTRIDE;
        }
    }
}

pub fn mc_bilinear16(dst: &mut [u8], dstride: usize, src: &[u8], mx: u16, my: u16) {
    mc_bilinear::<{16 * 2 + 2}, 16>(dst, dstride, src, mx, my)
}

#[allow(clippy::trivially_copy_pass_by_ref)]
pub fn mc_bicubic(dst: &mut [u8], dstride: usize, src: &[u8], mut soff: usize, sstride: usize, coeffs_w: &[i16; 4], coeffs_h: &[i16; 4]) {
    if coeffs_h[1] == 128 {
        for dline in dst.chunks_mut(dstride).take(8) {
            for i in 0..8 {
                dline[i] = mc_filter!(bicubic; src, soff + i, 1, coeffs_w);
            }
            soff += sstride;
        }
    } else if coeffs_w[1] == 128 { // horizontal-only interpolation
        for dline in dst.chunks_mut(dstride).take(8) {
            for i in 0..8 {
                dline[i] = mc_filter!(bicubic; src, soff + i, sstride, coeffs_h);
            }
            soff += sstride;
        }
    } else {
        let mut buf = [0u8; 16 * 11];
        soff -= sstride;
        for dline in buf.chunks_mut(16) {
            for i in 0..8 {
                dline[i] = mc_filter!(bicubic; src, soff + i, 1, coeffs_w);
            }
            soff += sstride;
        }
        let mut soff = 16;
        for dline in dst.chunks_mut(dstride).take(8) {
            for i in 0..8 {
                dline[i] = mc_filter!(bicubic; buf, soff + i, 16, coeffs_h);
            }
            soff += 16;
        }
    }
}
