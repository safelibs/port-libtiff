use super::directory::get_tag_value;
use super::CodecGeometry;
use crate::abi::TIFFDataType;
use crate::{emit_error_message, read_from_proc, seek_in_proc, tif_inner, TIFF, TIFF_UPSAMPLED};
use libc::{c_char, c_int, c_void};
use std::cmp::min;
use std::ffi::CStr;
use std::ptr;
use std::slice;

pub(crate) const COMPRESSION_OJPEG: u16 = 6;
pub(crate) const COMPRESSION_JPEG: u16 = 7;

pub(crate) const TAG_JPEGQUALITY: u32 = 65537;
pub(crate) const TAG_JPEGCOLORMODE: u32 = 65538;
pub(crate) const JPEGCOLORMODE_RAW: c_int = 0;
pub(crate) const JPEGCOLORMODE_RGB: c_int = 1;

const DEFAULT_JPEG_QUALITY: c_int = 75;

const TAG_IMAGEWIDTH: u32 = 256;
const TAG_IMAGELENGTH: u32 = 257;
const TAG_BITSPERSAMPLE: u32 = 258;
const TAG_COMPRESSION: u32 = 259;
const TAG_PHOTOMETRIC: u32 = 262;
const TAG_SAMPLESPERPIXEL: u32 = 277;
const TAG_ROWSPERSTRIP: u32 = 278;
const TAG_PLANARCONFIG: u32 = 284;
const TAG_JPEGTABLES: u32 = 347;
const TAG_JPEGPROC: u32 = 512;
const TAG_JPEGIFOFFSET: u32 = 513;
const TAG_JPEGIFBYTECOUNT: u32 = 514;
const TAG_JPEGRESTARTINTERVAL: u32 = 515;
const TAG_JPEGQTABLES: u32 = 519;
const TAG_JPEGDCTABLES: u32 = 520;
const TAG_JPEGACTABLES: u32 = 521;
const TAG_YCBCRSUBSAMPLING: u32 = 530;
const TAG_TILEWIDTH: u32 = 322;
const TAG_TILELENGTH: u32 = 323;
const TAG_STRIPOFFSETS: u32 = 273;
const TAG_STRIPBYTECOUNTS: u32 = 279;
const TAG_TILEOFFSETS: u32 = 324;
const TAG_TILEBYTECOUNTS: u32 = 325;

const PLANARCONFIG_CONTIG: u16 = 1;

#[allow(improper_ctypes)]
unsafe extern "C" {
    fn safe_tiff_jpeg_decode_rgb(
        jpeg_data: *const u8,
        jpeg_len: usize,
        out: *mut u8,
        out_len: usize,
        out_width: *mut u32,
        out_height: *mut u32,
        errbuf: *mut c_char,
        errbuf_len: usize,
    ) -> c_int;
    fn safe_tiff_jpeg_decode_raw_ycbcr(
        jpeg_data: *const u8,
        jpeg_len: usize,
        out: *mut u8,
        out_len: usize,
        subsampling_h: u32,
        subsampling_v: u32,
        errbuf: *mut c_char,
        errbuf_len: usize,
    ) -> c_int;
}

pub(crate) struct JpegStream {
    pub(crate) bytes: Vec<u8>,
    pub(crate) full_width: u32,
    pub(crate) full_height: u32,
    pub(crate) crop_x: u32,
    pub(crate) crop_y: u32,
    pub(crate) crop_width: u32,
    pub(crate) crop_height: u32,
}

fn get_tag_raw(
    tif: *mut TIFF,
    tag: u32,
    defaulted: bool,
) -> Option<(TIFFDataType, usize, *const c_void)> {
    let mut type_ = TIFFDataType::TIFF_NOTYPE;
    let mut count = 0u64;
    let mut data: *const c_void = ptr::null();
    if get_tag_value(tif, tag, defaulted, &mut type_, &mut count, &mut data) == 0 {
        return None;
    }
    Some((type_, usize::try_from(count).ok()?, data))
}

fn tag_u16(tif: *mut TIFF, tag: u32, defaulted: bool, fallback: u16) -> u16 {
    unsafe {
        let Some((type_, count, data)) = get_tag_raw(tif, tag, defaulted) else {
            return fallback;
        };
        if count == 0 || data.is_null() {
            return fallback;
        }
        match type_.0 {
            x if x == TIFFDataType::TIFF_SHORT.0 => *data.cast::<u16>(),
            x if x == TIFFDataType::TIFF_LONG.0 => {
                u16::try_from(*data.cast::<u32>()).unwrap_or(fallback)
            }
            x if x == TIFFDataType::TIFF_SLONG.0 => {
                u16::try_from(*data.cast::<i32>()).unwrap_or(fallback)
            }
            _ => fallback,
        }
    }
}

fn tag_u32(tif: *mut TIFF, tag: u32, defaulted: bool, fallback: u32) -> u32 {
    unsafe {
        let Some((type_, count, data)) = get_tag_raw(tif, tag, defaulted) else {
            return fallback;
        };
        if count == 0 || data.is_null() {
            return fallback;
        }
        match type_.0 {
            x if x == TIFFDataType::TIFF_SHORT.0 => u32::from(*data.cast::<u16>()),
            x if x == TIFFDataType::TIFF_LONG.0 => *data.cast::<u32>(),
            x if x == TIFFDataType::TIFF_LONG8.0 || x == TIFFDataType::TIFF_IFD8.0 => {
                u32::try_from(*data.cast::<u64>()).unwrap_or(fallback)
            }
            x if x == TIFFDataType::TIFF_SLONG.0 => {
                u32::try_from(*data.cast::<i32>()).unwrap_or(fallback)
            }
            _ => fallback,
        }
    }
}

fn copy_u64_array_tag(tif: *mut TIFF, tag: u32) -> Option<Vec<u64>> {
    unsafe {
        let (type_, count, data) = get_tag_raw(tif, tag, false)?;
        if count == 0 {
            return Some(Vec::new());
        }
        if data.is_null() {
            return None;
        }
        match type_.0 {
            x if x == TIFFDataType::TIFF_SHORT.0 => Some(
                slice::from_raw_parts(data.cast::<u16>(), count)
                    .iter()
                    .map(|value| u64::from(*value))
                    .collect(),
            ),
            x if x == TIFFDataType::TIFF_LONG.0 || x == TIFFDataType::TIFF_IFD.0 => Some(
                slice::from_raw_parts(data.cast::<u32>(), count)
                    .iter()
                    .map(|value| u64::from(*value))
                    .collect(),
            ),
            x if x == TIFFDataType::TIFF_LONG8.0 || x == TIFFDataType::TIFF_IFD8.0 => {
                Some(slice::from_raw_parts(data.cast::<u64>(), count).to_vec())
            }
            _ => None,
        }
    }
}

fn copy_u8_array_tag(tif: *mut TIFF, tag: u32) -> Option<Vec<u8>> {
    unsafe {
        let (type_, count, data) = get_tag_raw(tif, tag, false)?;
        if count == 0 {
            return Some(Vec::new());
        }
        if data.is_null() {
            return None;
        }
        match type_.0 {
            x if x == TIFFDataType::TIFF_BYTE.0
                || x == TIFFDataType::TIFF_UNDEFINED.0
                || x == TIFFDataType::TIFF_ASCII.0 =>
            {
                Some(slice::from_raw_parts(data.cast::<u8>(), count).to_vec())
            }
            _ => None,
        }
    }
}

fn file_size(tif: *mut TIFF) -> u64 {
    unsafe {
        let inner = tif_inner(tif);
        if !(*inner).mapped_base.is_null() && (*inner).mapped_size != 0 {
            (*inner).mapped_size
        } else if let Some(sizeproc) = (*tif).tif_sizeproc {
            sizeproc((*tif).tif_clientdata)
        } else {
            0
        }
    }
}

fn read_exact_at(tif: *mut TIFF, offset: u64, bytes: &mut [u8]) -> bool {
    unsafe {
        let Some(end) = offset.checked_add(bytes.len() as u64) else {
            return false;
        };
        let size = file_size(tif);
        if size != 0 && end > size {
            return false;
        }
        let inner = tif_inner(tif);
        if !(*inner).mapped_base.is_null() && (*inner).mapped_size >= end {
            ptr::copy_nonoverlapping(
                (*inner).mapped_base.cast::<u8>().add(offset as usize),
                bytes.as_mut_ptr(),
                bytes.len(),
            );
            true
        } else if seek_in_proc(tif, offset, libc::SEEK_SET) == offset {
            read_from_proc(
                tif,
                bytes.as_mut_ptr().cast::<c_void>(),
                bytes.len() as crate::Tmsize,
            )
        } else {
            false
        }
    }
}

fn read_exact_vec_at(tif: *mut TIFF, offset: u64, len: usize) -> Option<Vec<u8>> {
    let mut bytes = vec![0u8; len];
    read_exact_at(tif, offset, &mut bytes).then_some(bytes)
}

fn read_table_at(tif: *mut TIFF, offset: u64, len: usize, label: &str) -> Option<Vec<u8>> {
    if len == 0 {
        emit_error_message(tif, label, "Referenced JPEG table is empty");
        return None;
    }
    read_exact_vec_at(tif, offset, len).or_else(|| {
        emit_error_message(tif, label, "Failed to read referenced JPEG table");
        None
    })
}

fn push_marker(out: &mut Vec<u8>, marker: u8, payload: &[u8]) -> bool {
    let Ok(length) = u16::try_from(payload.len().saturating_add(2)) else {
        return false;
    };
    out.push(0xff);
    out.push(marker);
    out.extend_from_slice(&length.to_be_bytes());
    out.extend_from_slice(payload);
    true
}

fn ensure_eoi(out: &mut Vec<u8>) {
    if out.len() < 2 || out[out.len() - 2] != 0xff || out[out.len() - 1] != 0xd9 {
        out.extend_from_slice(&[0xff, 0xd9]);
    }
}

fn jpeg_helper_error(errbuf: &[c_char]) -> String {
    unsafe {
        if errbuf.is_empty() || errbuf[0] == 0 {
            "JPEG helper failed".to_string()
        } else {
            CStr::from_ptr(errbuf.as_ptr())
                .to_string_lossy()
                .into_owned()
        }
    }
}

pub(crate) fn reset_jpeg_state(tif: *mut TIFF) {
    unsafe {
        let state = &mut (*tif_inner(tif)).codec_state;
        state.jpeg_quality = DEFAULT_JPEG_QUALITY;
        state.jpeg_colormode = JPEGCOLORMODE_RAW;
        (*tif).tif_flags &= !TIFF_UPSAMPLED;
    }
}

pub(crate) fn jpeg_quality(tif: *mut TIFF) -> c_int {
    unsafe { (*tif_inner(tif)).codec_state.jpeg_quality }
}

pub(crate) fn jpeg_default_quality() -> c_int {
    DEFAULT_JPEG_QUALITY
}

pub(crate) fn jpeg_color_mode(tif: *mut TIFF) -> c_int {
    unsafe { (*tif_inner(tif)).codec_state.jpeg_colormode }
}

pub(crate) fn set_jpeg_color_mode(tif: *mut TIFF, value: c_int) {
    unsafe {
        let mode = if value == JPEGCOLORMODE_RGB {
            JPEGCOLORMODE_RGB
        } else {
            JPEGCOLORMODE_RAW
        };
        (*tif_inner(tif)).codec_state.jpeg_colormode = mode;
        if mode == JPEGCOLORMODE_RGB {
            (*tif).tif_flags |= TIFF_UPSAMPLED;
        } else {
            (*tif).tif_flags &= !TIFF_UPSAMPLED;
        }
    }
}

pub(crate) fn unset_jpeg_pseudo_tag(tif: *mut TIFF, tag: u32) -> c_int {
    unsafe {
        match tag {
            TAG_JPEGQUALITY => {
                (*tif_inner(tif)).codec_state.jpeg_quality = DEFAULT_JPEG_QUALITY;
                1
            }
            TAG_JPEGCOLORMODE => {
                set_jpeg_color_mode(tif, JPEGCOLORMODE_RAW);
                1
            }
            _ => 0,
        }
    }
}

fn howmany_u32(value: u32, divisor: u32) -> Option<u32> {
    if divisor == 0 {
        None
    } else {
        value.checked_add(divisor - 1).map(|sum| sum / divisor)
    }
}

fn tile_crop_origin(tif: *mut TIFF, tile: u32) -> Option<(u32, u32)> {
    let width = tag_u32(tif, TAG_IMAGEWIDTH, true, 0);
    let tile_width = tag_u32(tif, TAG_TILEWIDTH, false, 0);
    let tile_length = tag_u32(tif, TAG_TILELENGTH, false, 0);
    if width == 0 || tile_width == 0 || tile_length == 0 {
        return None;
    }
    let tiles_across = howmany_u32(width, tile_width)?;
    Some((
        (tile % tiles_across) * tile_width,
        (tile / tiles_across) * tile_length,
    ))
}

fn strip_crop_origin(tif: *mut TIFF, strip: u32) -> Option<(u32, u32)> {
    let height = tag_u32(tif, TAG_IMAGELENGTH, true, 0);
    let rows_per_strip = tag_u32(tif, TAG_ROWSPERSTRIP, true, height);
    if height == 0 || rows_per_strip == 0 {
        return None;
    }
    Some((0, strip.saturating_mul(rows_per_strip)))
}

fn read_all_strile_payloads(tif: *mut TIFF, is_tile: bool) -> Option<Vec<u8>> {
    let (offset_tag, bytecount_tag, label) = if is_tile {
        (TAG_TILEOFFSETS, TAG_TILEBYTECOUNTS, "OJPEG tile")
    } else {
        (TAG_STRIPOFFSETS, TAG_STRIPBYTECOUNTS, "OJPEG strip")
    };
    let offsets = copy_u64_array_tag(tif, offset_tag)?;
    let bytecounts = copy_u64_array_tag(tif, bytecount_tag)?;
    if offsets.len() != bytecounts.len() {
        emit_error_message(tif, label, "Malformed strile arrays");
        return None;
    }
    let mut out = Vec::new();
    for (offset, bytecount) in offsets.iter().zip(bytecounts.iter()) {
        let Ok(bytecount) = usize::try_from(*bytecount) else {
            emit_error_message(tif, label, "Strile byte count is too large");
            return None;
        };
        if *offset == 0 || bytecount == 0 {
            emit_error_message(tif, label, "Strile data is absent");
            return None;
        }
        let bytes = read_exact_vec_at(tif, *offset, bytecount).or_else(|| {
            emit_error_message(tif, label, "Failed to read OJPEG strile payload");
            None
        })?;
        out.extend_from_slice(&bytes);
    }
    Some(out)
}

fn build_synthetic_ojpeg_header(tif: *mut TIFF) -> Option<Vec<u8>> {
    unsafe {
        let label = "OJPEG";
        let width = tag_u32(tif, TAG_IMAGEWIDTH, true, 0);
        let height = tag_u32(tif, TAG_IMAGELENGTH, true, 0);
        let samples_per_pixel = tag_u16(tif, TAG_SAMPLESPERPIXEL, true, 3);
        let bits_per_sample = tag_u16(tif, TAG_BITSPERSAMPLE, true, 8);
        let jpeg_proc = tag_u16(tif, TAG_JPEGPROC, true, 1);
        let restart_interval = tag_u16(tif, TAG_JPEGRESTARTINTERVAL, true, 0);
        let subsampling = get_tag_raw(tif, TAG_YCBCRSUBSAMPLING, true)
            .and_then(|(type_, count, data)| {
                if data.is_null() || count < 2 || type_.0 != TIFFDataType::TIFF_SHORT.0 {
                    None
                } else {
                    let values = slice::from_raw_parts(data.cast::<u16>(), count);
                    Some((values[0], values[1]))
                }
            })
            .unwrap_or((2, 2));
        let q_offsets = copy_u64_array_tag(tif, TAG_JPEGQTABLES)?;
        let dc_offsets = copy_u64_array_tag(tif, TAG_JPEGDCTABLES)?;
        let ac_offsets = copy_u64_array_tag(tif, TAG_JPEGACTABLES)?;

        if width == 0 || height == 0 || bits_per_sample == 0 || samples_per_pixel == 0 {
            emit_error_message(tif, label, "Malformed OJPEG image geometry");
            return None;
        }
        if bits_per_sample > 12 {
            emit_error_message(tif, label, "Unsupported OJPEG bit depth");
            return None;
        }
        if samples_per_pixel != 1 && samples_per_pixel != 3 {
            emit_error_message(tif, label, "Unsupported OJPEG sample layout");
            return None;
        }
        if subsampling.0 == 0 || subsampling.1 == 0 {
            emit_error_message(tif, label, "Invalid OJPEG YCbCr subsampling");
            return None;
        }
        if q_offsets.is_empty() || dc_offsets.is_empty() || ac_offsets.is_empty() {
            emit_error_message(tif, label, "OJPEG table offsets are missing");
            return None;
        }

        let mut out = Vec::new();
        out.extend_from_slice(&[0xff, 0xd8]);

        if jpeg_proc == 1 {
            let _ = push_marker(
                &mut out,
                0xe0,
                &[b'J', b'F', b'I', b'F', 0, 1, 1, 0, 0, 1, 0, 1, 0, 0],
            );
        }

        for (index, offset) in q_offsets.iter().enumerate() {
            let table = read_table_at(tif, *offset, 64, label)?;
            let mut payload = Vec::with_capacity(65);
            payload.push(index as u8);
            payload.extend_from_slice(&table);
            if !push_marker(&mut out, 0xdb, &payload) {
                emit_error_message(tif, label, "OJPEG quantization table is too large");
                return None;
            }
        }

        let mut sof = Vec::new();
        sof.push(bits_per_sample as u8);
        sof.extend_from_slice(&(height as u16).to_be_bytes());
        sof.extend_from_slice(&(width as u16).to_be_bytes());
        sof.push(samples_per_pixel as u8);
        if samples_per_pixel == 1 {
            sof.push(1);
            sof.push(0x11);
            sof.push(0);
        } else {
            let q1 = min(q_offsets.len().saturating_sub(1), 1) as u8;
            let q2 = min(q_offsets.len().saturating_sub(1), 2) as u8;
            sof.push(1);
            sof.push(((subsampling.0 as u8) << 4) | (subsampling.1 as u8));
            sof.push(0);
            sof.push(2);
            sof.push(0x11);
            sof.push(q1);
            sof.push(3);
            sof.push(0x11);
            sof.push(q2);
        }
        if !push_marker(&mut out, 0xc0, &sof) {
            emit_error_message(tif, label, "Malformed OJPEG SOF payload");
            return None;
        }

        if restart_interval != 0 {
            let _ = push_marker(&mut out, 0xdd, &restart_interval.to_be_bytes());
        }

        for (index, offset) in dc_offsets.iter().enumerate() {
            let counts = read_table_at(tif, *offset, 16, label)?;
            let values_len: usize = counts.iter().map(|value| usize::from(*value)).sum();
            let table = read_table_at(tif, offset.checked_add(16)?, values_len, label)?;
            let mut payload = Vec::with_capacity(17 + table.len());
            payload.push(index as u8);
            payload.extend_from_slice(&counts);
            payload.extend_from_slice(&table);
            if !push_marker(&mut out, 0xc4, &payload) {
                emit_error_message(tif, label, "OJPEG DC table is too large");
                return None;
            }
        }
        for (index, offset) in ac_offsets.iter().enumerate() {
            let counts = read_table_at(tif, *offset, 16, label)?;
            let values_len: usize = counts.iter().map(|value| usize::from(*value)).sum();
            let table = read_table_at(tif, offset.checked_add(16)?, values_len, label)?;
            let mut payload = Vec::with_capacity(17 + table.len());
            payload.push(0x10 | index as u8);
            payload.extend_from_slice(&counts);
            payload.extend_from_slice(&table);
            if !push_marker(&mut out, 0xc4, &payload) {
                emit_error_message(tif, label, "OJPEG AC table is too large");
                return None;
            }
        }

        let mut sos = Vec::new();
        sos.push(samples_per_pixel as u8);
        if samples_per_pixel == 1 {
            sos.push(1);
            sos.push(0x00);
        } else {
            let table1 = min(dc_offsets.len().saturating_sub(1), 1) as u8;
            let table2 = min(dc_offsets.len().saturating_sub(1), 2) as u8;
            sos.push(1);
            sos.push(0x00);
            sos.push(2);
            sos.push((table1 << 4) | table1);
            sos.push(3);
            sos.push((table2 << 4) | table2);
        }
        sos.extend_from_slice(&[0, 63, 0]);
        if !push_marker(&mut out, 0xda, &sos) {
            emit_error_message(tif, label, "Malformed OJPEG SOS payload");
            return None;
        }

        Some(out)
    }
}

fn maybe_reconstruct_abbreviated_jpeg_stream(tif: *mut TIFF, input: &[u8]) -> Option<Vec<u8>> {
    if input.len() >= 2 && input[0] == 0xff && input[1] == 0xd8 {
        let mut bytes = input.to_vec();
        ensure_eoi(&mut bytes);
        return Some(bytes);
    }
    let tables = copy_u8_array_tag(tif, TAG_JPEGTABLES)?;
    if tables.len() < 2 || tables[0] != 0xff || tables[1] != 0xd8 {
        return None;
    }
    let mut bytes = tables;
    if bytes.len() >= 2 && bytes[bytes.len() - 2] == 0xff && bytes[bytes.len() - 1] == 0xd9 {
        bytes.truncate(bytes.len() - 2);
    }
    if input.len() >= 2 && input[0] == 0xff && input[1] == 0xd8 {
        bytes.extend_from_slice(&input[2..]);
    } else {
        bytes.extend_from_slice(input);
    }
    ensure_eoi(&mut bytes);
    Some(bytes)
}

pub(crate) fn maybe_reconstruct_jpeg_stream(
    tif: *mut TIFF,
    input: &[u8],
    is_tile: bool,
    strile: u32,
    geometry: CodecGeometry,
) -> Option<JpegStream> {
    if tag_u16(tif, TAG_PLANARCONFIG, true, PLANARCONFIG_CONTIG) != PLANARCONFIG_CONTIG {
        emit_error_message(tif, "JPEG", "Planar separate JPEG is not supported");
        return None;
    }

    if tag_u16(tif, TAG_COMPRESSION, true, 0) == COMPRESSION_JPEG {
        let bytes = maybe_reconstruct_abbreviated_jpeg_stream(tif, input).or_else(|| {
            emit_error_message(
                tif,
                "JPEG",
                "Missing JPEGTables for abbreviated JPEG stream",
            );
            None
        })?;
        return Some(JpegStream {
            bytes,
            full_width: geometry.width,
            full_height: geometry.rows as u32,
            crop_x: 0,
            crop_y: 0,
            crop_width: geometry.width,
            crop_height: geometry.rows as u32,
        });
    }

    let width = tag_u32(tif, TAG_IMAGEWIDTH, true, 0);
    let height = tag_u32(tif, TAG_IMAGELENGTH, true, 0);
    let (crop_x, crop_y) = if is_tile {
        tile_crop_origin(tif, strile)?
    } else {
        strip_crop_origin(tif, strile)?
    };
    let crop_width = if is_tile {
        min(geometry.width, width.saturating_sub(crop_x))
    } else {
        geometry.width
    };
    let crop_height = min(geometry.rows as u32, height.saturating_sub(crop_y));

    let header = {
        let offset = tag_u32(tif, TAG_JPEGIFOFFSET, false, 0) as u64;
        let len = tag_u32(tif, TAG_JPEGIFBYTECOUNT, false, 0) as usize;
        if offset != 0 && len != 0 {
            read_exact_vec_at(tif, offset, len)
        } else {
            None
        }
    };

    let mut bytes = if let Some(header) = header {
        if header.len() < 2 || header[0] != 0xff || header[1] != 0xd8 {
            emit_error_message(tif, "OJPEG", "Malformed JPEGInterchangeFormat header");
            return None;
        }
        header
    } else {
        build_synthetic_ojpeg_header(tif)?
    };

    let entropy = read_all_strile_payloads(tif, is_tile)?;
    if entropy.is_empty() {
        emit_error_message(tif, "OJPEG", "Entropy-coded image data is absent");
        return None;
    }
    if bytes.len() >= 2 && bytes[bytes.len() - 2] == 0xff && bytes[bytes.len() - 1] == 0xd9 {
        bytes.truncate(bytes.len() - 2);
    }
    bytes.extend_from_slice(&entropy);
    ensure_eoi(&mut bytes);

    Some(JpegStream {
        bytes,
        full_width: width,
        full_height: height,
        crop_x,
        crop_y,
        crop_width,
        crop_height,
    })
}

fn decode_full_rgb_stream(tif: *mut TIFF, stream: &JpegStream) -> Option<Vec<u8>> {
    unsafe {
        let full_width = usize::try_from(stream.full_width).ok()?;
        let full_height = usize::try_from(stream.full_height).ok()?;
        let full_len = full_width.checked_mul(full_height)?.checked_mul(3)?;
        let mut full = vec![0u8; full_len];
        let mut errbuf = [0 as c_char; 256];
        let mut decoded_width = 0u32;
        let mut decoded_height = 0u32;
        if safe_tiff_jpeg_decode_rgb(
            stream.bytes.as_ptr(),
            stream.bytes.len(),
            full.as_mut_ptr(),
            full.len(),
            &mut decoded_width,
            &mut decoded_height,
            errbuf.as_mut_ptr(),
            errbuf.len(),
        ) == 0
        {
            emit_error_message(tif, "JPEG", jpeg_helper_error(&errbuf));
            return None;
        }
        if decoded_width != stream.full_width || decoded_height != stream.full_height {
            emit_error_message(
                tif,
                "JPEG",
                "Decoded JPEG dimensions do not match TIFF geometry",
            );
            return None;
        }
        Some(full)
    }
}

fn decode_rgb_stream(
    tif: *mut TIFF,
    stream: &JpegStream,
    geometry: CodecGeometry,
    expected_size: usize,
) -> Option<Vec<u8>> {
    let full = decode_full_rgb_stream(tif, stream)?;
    let row_size = geometry.row_size;
    let rows = geometry.rows;
    let mut out = vec![0u8; expected_size];
    let copy_width = min(stream.crop_width as usize, geometry.width as usize);
    let copy_rows = min(stream.crop_height as usize, rows);
    let copy_bytes = copy_width.checked_mul(3)?;
    for row in 0..copy_rows {
        let src_y = stream.crop_y as usize + row;
        let src_x = stream.crop_x as usize;
        let src_offset = (src_y * stream.full_width as usize + src_x) * 3;
        let dst_offset = row * row_size;
        out[dst_offset..dst_offset + copy_bytes]
            .copy_from_slice(&full[src_offset..src_offset + copy_bytes]);
    }
    Some(out)
}

pub(crate) fn ojpeg_decode_full_rgb_image(tif: *mut TIFF) -> Option<Vec<u8>> {
    if tag_u16(tif, TAG_COMPRESSION, true, 0) != COMPRESSION_OJPEG {
        emit_error_message(
            tif,
            "OJPEG",
            "Full-image OJPEG decode requires OJPEG compression",
        );
        return None;
    }

    let width = tag_u32(tif, TAG_IMAGEWIDTH, true, 0);
    let height = tag_u32(tif, TAG_IMAGELENGTH, true, 0);
    let full_width = usize::try_from(width).ok()?;
    let full_height = usize::try_from(height).ok()?;
    let row_size = full_width.checked_mul(3)?;
    let geometry = CodecGeometry {
        row_size,
        rows: full_height,
        width,
    };
    let is_tile =
        tag_u32(tif, TAG_TILEWIDTH, false, 0) != 0 && tag_u32(tif, TAG_TILELENGTH, false, 0) != 0;
    let stream = maybe_reconstruct_jpeg_stream(tif, &[], is_tile, 0, geometry)?;
    decode_full_rgb_stream(tif, &stream)
}

pub(crate) fn jpeg_decode_bytes(
    tif: *mut TIFF,
    input: &[u8],
    is_tile: bool,
    strile: u32,
    geometry: CodecGeometry,
    expected_size: usize,
) -> Option<Vec<u8>> {
    unsafe {
        let stream = maybe_reconstruct_jpeg_stream(tif, input, is_tile, strile, geometry)?;
        if jpeg_color_mode(tif) == JPEGCOLORMODE_RGB
            || tag_u16(tif, TAG_COMPRESSION, true, 0) == COMPRESSION_OJPEG
        {
            return decode_rgb_stream(tif, &stream, geometry, expected_size);
        }

        let (h, v) = get_tag_raw(tif, TAG_YCBCRSUBSAMPLING, true)
            .and_then(|(type_, count, data)| {
                if data.is_null() || count < 2 || type_.0 != TIFFDataType::TIFF_SHORT.0 {
                    None
                } else {
                    let values = slice::from_raw_parts(data.cast::<u16>(), count);
                    Some((u32::from(values[0]), u32::from(values[1])))
                }
            })
            .unwrap_or((2, 2));
        let mut out = vec![0u8; expected_size];
        let mut errbuf = [0 as c_char; 256];
        if safe_tiff_jpeg_decode_raw_ycbcr(
            stream.bytes.as_ptr(),
            stream.bytes.len(),
            out.as_mut_ptr(),
            out.len(),
            h,
            v,
            errbuf.as_mut_ptr(),
            errbuf.len(),
        ) == 0
        {
            emit_error_message(tif, "JPEG", jpeg_helper_error(&errbuf));
            return None;
        }
        Some(out)
    }
}

pub(crate) fn jpeg_encode_bytes(
    _tif: *mut TIFF,
    _input: &[u8],
    _geometry: CodecGeometry,
) -> Option<Vec<u8>> {
    None
}
