use crate::abi::TIFFDataType;
use crate::core::{
    _TIFFRewriteField, get_strile_tag_value_u64, get_tag_value, safe_tiff_codec_decode_bytes,
    safe_tiff_codec_encode_bytes, safe_tiff_directory_entry_is_dummy,
    safe_tiff_set_field_marshaled, safe_tiff_set_field_marshaled_nondirty, CodecGeometry,
    DecodedStrileCache, PendingStrileWrite, TIFFRewriteDirectory,
};
use crate::{
    _TIFFcallocExt, _TIFFfree, _TIFFmallocExt, emit_error_message, read_from_proc, seek_in_proc,
    tif_inner, write_to_proc, Tmsize, TIFF,
};
use libc::{c_int, c_void};
use std::cmp::{max, min};
use std::ptr;
use std::slice;

const COMPRESSION_NONE: u16 = 1;
const FILLORDER_MSB2LSB: u16 = 1;
const FILLORDER_LSB2MSB: u16 = 2;
const PHOTOMETRIC_YCBCR: u16 = 6;

const PLANARCONFIG_CONTIG: u16 = 1;
const PLANARCONFIG_SEPARATE: u16 = 2;
const SAMPLEFORMAT_UINT: u16 = 1;
const SAMPLEFORMAT_COMPLEXINT: u16 = 5;
const SAMPLEFORMAT_COMPLEXIEEEFP: u16 = 6;

const TAG_IMAGEWIDTH: u32 = 256;
const TAG_IMAGELENGTH: u32 = 257;
const TAG_BITSPERSAMPLE: u32 = 258;
const TAG_COMPRESSION: u32 = 259;
const TAG_PHOTOMETRIC: u32 = 262;
const TAG_FILLORDER: u32 = 266;
const TAG_SAMPLESPERPIXEL: u32 = 277;
const TAG_ROWSPERSTRIP: u32 = 278;
const TAG_PLANARCONFIG: u32 = 284;
const TAG_SAMPLEFORMAT: u32 = 339;
const TAG_TILEWIDTH: u32 = 322;
const TAG_TILELENGTH: u32 = 323;
const TAG_TILEOFFSETS: u32 = 324;
const TAG_TILEBYTECOUNTS: u32 = 325;
const TAG_YCBCRSUBSAMPLING: u32 = 530;
const TAG_STRIPOFFSETS: u32 = 273;
const TAG_STRIPBYTECOUNTS: u32 = 279;
const TAG_IMAGEDEPTH: u32 = 32997;
const TAG_TILEDEPTH: u32 = 32998;

const TIFF_DIRTYDIRECT: u32 = 0x00008;
const TIFF_BUFFERSETUP: u32 = 0x00010;
const TIFF_BEENWRITING: u32 = 0x00040;
const TIFF_SWAB: u32 = 0x00080;
const TIFF_NOBITREV: u32 = 0x00100;
const TIFF_MYBUFFER: u32 = 0x00200;
const TIFF_ISTILED: u32 = 0x00400;
const TIFF_MAPPED: u32 = 0x00800;
const TIFF_DIRTYSTRIP: u32 = 0x200000;
const TIFF_BUFFERMMAP: u32 = 0x800000;

#[derive(Default)]
pub(crate) struct StrileState {
    pub(crate) defer_array_writing: bool,
    pub(crate) write_offset: u64,
}

struct StrileArrays {
    offset_tag: u32,
    bytecount_tag: u32,
    offsets: Vec<u64>,
    bytecounts: Vec<u64>,
}

const fn reverse_byte_const(mut value: u8) -> u8 {
    let mut reversed = 0u8;
    let mut index = 0usize;
    while index < 8 {
        reversed = (reversed << 1) | (value & 1);
        value >>= 1;
        index += 1;
    }
    reversed
}

const fn build_bit_rev_table(reverse: bool) -> [u8; 256] {
    let mut table = [0u8; 256];
    let mut index = 0usize;
    while index < 256 {
        table[index] = if reverse {
            reverse_byte_const(index as u8)
        } else {
            index as u8
        };
        index += 1;
    }
    table
}

static TIFF_BIT_REV_TABLE: [u8; 256] = build_bit_rev_table(true);
static TIFF_NO_BIT_REV_TABLE: [u8; 256] = build_bit_rev_table(false);

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
        if (*tif).tif_flags & TIFF_MAPPED != 0
            && !(*inner).mapped_base.is_null()
            && end <= (*inner).mapped_size
        {
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
                bytes.len() as Tmsize,
            )
        } else {
            false
        }
    }
}

fn write_exact_at(tif: *mut TIFF, offset: u64, bytes: &[u8]) -> bool {
    if seek_in_proc(tif, offset, libc::SEEK_SET) != offset {
        return false;
    }
    if bytes.is_empty() {
        return true;
    }
    write_to_proc(
        tif,
        bytes.as_ptr().cast_mut().cast::<c_void>(),
        bytes.len() as Tmsize,
    )
}

fn next_append_offset(tif: *mut TIFF) -> Option<u64> {
    unsafe {
        let inner = tif_inner(tif);
        if (*inner).strile_state.write_offset != 0 {
            let offset = (*inner).strile_state.write_offset;
            (*inner).strile_state.write_offset = 0;
            Some(offset)
        } else {
            let offset = seek_in_proc(tif, 0, libc::SEEK_END);
            if offset == u64::MAX {
                None
            } else {
                Some(offset)
            }
        }
    }
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

fn get_tag_scalar_u16(tif: *mut TIFF, tag: u32, defaulted: bool) -> Option<u16> {
    unsafe {
        let (type_, count, data) = get_tag_raw(tif, tag, defaulted)?;
        if count == 0 || data.is_null() {
            return None;
        }
        match type_.0 {
            x if x == TIFFDataType::TIFF_BYTE.0 || x == TIFFDataType::TIFF_UNDEFINED.0 => {
                Some(*data.cast::<u8>() as u16)
            }
            x if x == TIFFDataType::TIFF_SHORT.0 => Some(*data.cast::<u16>()),
            x if x == TIFFDataType::TIFF_LONG.0 => u16::try_from(*data.cast::<u32>()).ok(),
            x if x == TIFFDataType::TIFF_LONG8.0 || x == TIFFDataType::TIFF_IFD8.0 => {
                u16::try_from(*data.cast::<u64>()).ok()
            }
            _ => None,
        }
    }
}

fn get_tag_scalar_u32(tif: *mut TIFF, tag: u32, defaulted: bool) -> Option<u32> {
    unsafe {
        let (type_, count, data) = get_tag_raw(tif, tag, defaulted)?;
        if count == 0 || data.is_null() {
            return None;
        }
        match type_.0 {
            x if x == TIFFDataType::TIFF_BYTE.0 || x == TIFFDataType::TIFF_UNDEFINED.0 => {
                Some(*data.cast::<u8>() as u32)
            }
            x if x == TIFFDataType::TIFF_SHORT.0 => Some(*data.cast::<u16>() as u32),
            x if x == TIFFDataType::TIFF_LONG.0 => Some(*data.cast::<u32>()),
            x if x == TIFFDataType::TIFF_LONG8.0 || x == TIFFDataType::TIFF_IFD8.0 => {
                u32::try_from(*data.cast::<u64>()).ok()
            }
            _ => None,
        }
    }
}

fn copy_u16_array_tag(tif: *mut TIFF, tag: u32, defaulted: bool) -> Option<Vec<u16>> {
    unsafe {
        let (type_, count, data) = get_tag_raw(tif, tag, defaulted)?;
        if count == 0 {
            return Some(Vec::new());
        }
        if data.is_null() {
            return None;
        }
        match type_.0 {
            x if x == TIFFDataType::TIFF_BYTE.0 || x == TIFFDataType::TIFF_UNDEFINED.0 => Some(
                slice::from_raw_parts(data.cast::<u8>(), count)
                    .iter()
                    .map(|value| *value as u16)
                    .collect(),
            ),
            x if x == TIFFDataType::TIFF_SHORT.0 => {
                Some(slice::from_raw_parts(data.cast::<u16>(), count).to_vec())
            }
            _ => None,
        }
    }
}

fn copy_u64_array_tag(tif: *mut TIFF, tag: u32, defaulted: bool) -> Option<Vec<u64>> {
    unsafe {
        let (type_, count, data) = get_tag_raw(tif, tag, defaulted)?;
        if count == 0 {
            return Some(Vec::new());
        }
        if data.is_null() {
            return None;
        }
        match type_.0 {
            x if x == TIFFDataType::TIFF_BYTE.0 || x == TIFFDataType::TIFF_UNDEFINED.0 => Some(
                slice::from_raw_parts(data.cast::<u8>(), count)
                    .iter()
                    .map(|value| *value as u64)
                    .collect(),
            ),
            x if x == TIFFDataType::TIFF_SHORT.0 => Some(
                slice::from_raw_parts(data.cast::<u16>(), count)
                    .iter()
                    .map(|value| *value as u64)
                    .collect(),
            ),
            x if x == TIFFDataType::TIFF_LONG.0 || x == TIFFDataType::TIFF_IFD.0 => Some(
                slice::from_raw_parts(data.cast::<u32>(), count)
                    .iter()
                    .map(|value| *value as u64)
                    .collect(),
            ),
            x if x == TIFFDataType::TIFF_LONG8.0 || x == TIFFDataType::TIFF_IFD8.0 => {
                Some(slice::from_raw_parts(data.cast::<u64>(), count).to_vec())
            }
            _ => None,
        }
    }
}

fn set_u32_tag(tif: *mut TIFF, tag: u32, value: u32) -> bool {
    unsafe {
        safe_tiff_set_field_marshaled(
            tif,
            tag,
            TIFFDataType::TIFF_LONG,
            1,
            ptr::from_ref(&value).cast::<c_void>(),
        ) != 0
    }
}

fn set_u64_array_tag(tif: *mut TIFF, tag: u32, values: &[u64]) -> bool {
    safe_tiff_set_field_marshaled_nondirty(
        tif,
        tag,
        TIFFDataType::TIFF_LONG8,
        values.len() as u64,
        if values.is_empty() {
            ptr::null()
        } else {
            values.as_ptr().cast::<c_void>()
        },
    ) != 0
}

fn checked_add_u64(tif: *mut TIFF, module: &str, left: u64, right: u64) -> Option<u64> {
    left.checked_add(right).or_else(|| {
        emit_error_message(tif, module, "Integer overflow");
        None
    })
}

fn checked_mul_u64(tif: *mut TIFF, module: &str, left: u64, right: u64) -> Option<u64> {
    left.checked_mul(right).or_else(|| {
        emit_error_message(tif, module, "Integer overflow");
        None
    })
}

fn checked_howmany_u32(value: u32, divisor: u32) -> Option<u32> {
    if divisor == 0 {
        None
    } else {
        value.checked_add(divisor - 1).map(|sum| sum / divisor)
    }
}

fn checked_howmany_u64(value: u64, divisor: u64) -> Option<u64> {
    if divisor == 0 {
        None
    } else {
        value.checked_add(divisor - 1).map(|sum| sum / divisor)
    }
}

fn cast_u64_to_tmsize(tif: *mut TIFF, module: &str, value: u64) -> Tmsize {
    if value > isize::MAX as u64 {
        emit_error_message(tif, module, "Integer overflow");
        0
    } else {
        value as Tmsize
    }
}

fn require_u32_tag(tif: *mut TIFF, tag: u32, module: &str, label: &str) -> Option<u32> {
    let Some(value) = get_tag_scalar_u32(tif, tag, false) else {
        emit_error_message(
            tif,
            module,
            format!("Must set \"{}\" before writing data", label),
        );
        return None;
    };
    Some(value)
}

fn image_width(tif: *mut TIFF) -> Option<u32> {
    get_tag_scalar_u32(tif, TAG_IMAGEWIDTH, false)
}

fn image_length(tif: *mut TIFF) -> Option<u32> {
    get_tag_scalar_u32(tif, TAG_IMAGELENGTH, false)
}

fn image_depth(tif: *mut TIFF) -> u32 {
    get_tag_scalar_u32(tif, TAG_IMAGEDEPTH, true).unwrap_or(1)
}

fn bits_per_sample(tif: *mut TIFF) -> u16 {
    get_tag_scalar_u16(tif, TAG_BITSPERSAMPLE, true).unwrap_or(1)
}

fn samples_per_pixel(tif: *mut TIFF) -> u16 {
    get_tag_scalar_u16(tif, TAG_SAMPLESPERPIXEL, true).unwrap_or(1)
}

fn rows_per_strip(tif: *mut TIFF) -> u32 {
    get_tag_scalar_u32(tif, TAG_ROWSPERSTRIP, true).unwrap_or(u32::MAX)
}

fn planar_config(tif: *mut TIFF) -> u16 {
    get_tag_scalar_u16(tif, TAG_PLANARCONFIG, true).unwrap_or(PLANARCONFIG_CONTIG)
}

fn tile_width(tif: *mut TIFF) -> Option<u32> {
    get_tag_scalar_u32(tif, TAG_TILEWIDTH, false)
}

fn tile_length(tif: *mut TIFF) -> Option<u32> {
    get_tag_scalar_u32(tif, TAG_TILELENGTH, false)
}

fn tile_depth(tif: *mut TIFF) -> u32 {
    get_tag_scalar_u32(tif, TAG_TILEDEPTH, true).unwrap_or(1)
}

fn photometric(tif: *mut TIFF) -> u16 {
    get_tag_scalar_u16(tif, TAG_PHOTOMETRIC, true).unwrap_or(0)
}

fn compression(tif: *mut TIFF) -> u16 {
    get_tag_scalar_u16(tif, TAG_COMPRESSION, true).unwrap_or(COMPRESSION_NONE)
}

fn sample_format(tif: *mut TIFF) -> u16 {
    get_tag_scalar_u16(tif, TAG_SAMPLEFORMAT, true).unwrap_or(SAMPLEFORMAT_UINT)
}

fn fill_order(tif: *mut TIFF) -> u16 {
    get_tag_scalar_u16(tif, TAG_FILLORDER, true).unwrap_or(FILLORDER_MSB2LSB)
}

fn should_reverse_bits(tif: *mut TIFF) -> bool {
    unsafe {
        let order = fill_order(tif);
        (order == FILLORDER_MSB2LSB || order == FILLORDER_LSB2MSB)
            && ((*tif).tif_flags & TIFF_NOBITREV) == 0
            && ((*tif).tif_flags & order as u32) == 0
    }
}

fn apply_postdecode_bytes(tif: *mut TIFF, data: &mut [u8]) {
    unsafe {
        if ((*tif).tif_flags & TIFF_SWAB) == 0 {
            return;
        }

        match bits_per_sample(tif) {
            8 => {}
            16 => TIFFSwabArrayOfShort(data.as_mut_ptr().cast::<u16>(), (data.len() / 2) as Tmsize),
            24 => TIFFSwabArrayOfTriples(data.as_mut_ptr(), (data.len() / 3) as Tmsize),
            32 => {
                if sample_format(tif) == SAMPLEFORMAT_COMPLEXINT {
                    TIFFSwabArrayOfShort(
                        data.as_mut_ptr().cast::<u16>(),
                        (data.len() / 2) as Tmsize,
                    );
                } else {
                    TIFFSwabArrayOfLong(
                        data.as_mut_ptr().cast::<u32>(),
                        (data.len() / 4) as Tmsize,
                    );
                }
            }
            64 => {
                if matches!(
                    sample_format(tif),
                    SAMPLEFORMAT_COMPLEXINT | SAMPLEFORMAT_COMPLEXIEEEFP
                ) {
                    TIFFSwabArrayOfLong(
                        data.as_mut_ptr().cast::<u32>(),
                        (data.len() / 4) as Tmsize,
                    );
                } else {
                    TIFFSwabArrayOfDouble(
                        data.as_mut_ptr().cast::<f64>(),
                        (data.len() / 8) as Tmsize,
                    );
                }
            }
            128 => {
                TIFFSwabArrayOfDouble(data.as_mut_ptr().cast::<f64>(), (data.len() / 8) as Tmsize)
            }
            _ => {}
        }
    }
}

fn encode_uncompressed_bytes(tif: *mut TIFF, data: &[u8]) -> Vec<u8> {
    unsafe {
        let mut encoded = data.to_vec();
        if !encoded.is_empty() {
            apply_postdecode_bytes(tif, &mut encoded);
            if should_reverse_bits(tif) {
                TIFFReverseBits(encoded.as_mut_ptr(), encoded.len() as Tmsize);
            }
        }
        encoded
    }
}

fn ycbcr_subsampling(tif: *mut TIFF) -> Option<(u16, u16)> {
    let values = copy_u16_array_tag(tif, TAG_YCBCRSUBSAMPLING, true)?;
    if values.len() >= 2 {
        Some((values[0], values[1]))
    } else {
        None
    }
}

fn is_tiled_image(tif: *mut TIFF) -> bool {
    unsafe { ((*tif).tif_flags & TIFF_ISTILED) != 0 }
}

fn scanline_size64_internal(tif: *mut TIFF, report_errors: bool) -> Option<u64> {
    unsafe {
        let module = "TIFFScanlineSize64";
        let Some(width) = image_width(tif) else {
            if report_errors {
                emit_error_message(tif, module, "Missing image width");
            }
            return None;
        };
        let bits = bits_per_sample(tif) as u64;
        if bits == 0 || width == 0 {
            if report_errors {
                emit_error_message(tif, module, "Computed scanline size is zero");
            }
            return None;
        }
        if planar_config(tif) == PLANARCONFIG_CONTIG {
            if photometric(tif) == PHOTOMETRIC_YCBCR
                && samples_per_pixel(tif) == 3
                && ((*tif).tif_flags & crate::TIFF_UPSAMPLED) == 0
            {
                let (h, v) = ycbcr_subsampling(tif).unwrap_or((2, 2));
                if h == 0 || v == 0 {
                    if report_errors {
                        emit_error_message(tif, module, "Invalid YCbCr subsampling");
                    }
                    return None;
                }
                let block_samples = u64::from(h) * u64::from(v) + 2;
                let blocks_hor = u64::from(checked_howmany_u32(width, u32::from(h))?);
                let row_samples = checked_mul_u64(tif, module, blocks_hor, block_samples)?;
                let row_bits = checked_mul_u64(tif, module, row_samples, bits)?;
                let row_size = checked_howmany_u64(row_bits, 8)?;
                let scanline = row_size / u64::from(v);
                if scanline == 0 {
                    if report_errors {
                        emit_error_message(tif, module, "Computed scanline size is zero");
                    }
                    return None;
                }
                return Some(scanline);
            }

            let samples = checked_mul_u64(
                tif,
                module,
                u64::from(width),
                u64::from(samples_per_pixel(tif)),
            )?;
            let bits_total = checked_mul_u64(tif, module, samples, bits)?;
            let scanline = checked_howmany_u64(bits_total, 8)?;
            if scanline == 0 && report_errors {
                emit_error_message(tif, module, "Computed scanline size is zero");
            }
            (scanline != 0).then_some(scanline)
        } else {
            let bits_total = checked_mul_u64(tif, module, u64::from(width), bits)?;
            let scanline = checked_howmany_u64(bits_total, 8)?;
            if scanline == 0 && report_errors {
                emit_error_message(tif, module, "Computed scanline size is zero");
            }
            (scanline != 0).then_some(scanline)
        }
    }
}

fn raster_scanline_size64_internal(tif: *mut TIFF) -> Option<u64> {
    let module = "TIFFRasterScanlineSize64";
    let width = u64::from(image_width(tif)?);
    let bits = u64::from(bits_per_sample(tif));
    let spp = u64::from(samples_per_pixel(tif));
    let scanline_bits = checked_mul_u64(tif, module, width, bits)?;
    if planar_config(tif) == PLANARCONFIG_CONTIG {
        checked_howmany_u64(checked_mul_u64(tif, module, scanline_bits, spp)?, 8)
    } else {
        checked_mul_u64(tif, module, checked_howmany_u64(scanline_bits, 8)?, spp)
    }
}

fn strip_size_rows_internal(tif: *mut TIFF, strip: u32, report_errors: bool) -> Option<u32> {
    let module = "TIFFReadEncodedStrip";
    let height = image_length(tif)?;
    let mut rps = rows_per_strip(tif);
    if rps == u32::MAX || rps > height {
        rps = height;
    }
    if rps == 0 {
        if report_errors {
            emit_error_message(tif, module, "Rows per strip is zero");
        }
        return None;
    }
    let strips_per_plane = checked_howmany_u32(height, rps)?;
    if strips_per_plane == 0 {
        if report_errors {
            emit_error_message(tif, module, "Zero strips per image");
        }
        return None;
    }
    let strip_in_plane = strip % strips_per_plane;
    let consumed_rows = strip_in_plane.checked_mul(rps)?;
    let remaining_rows = height.saturating_sub(consumed_rows);
    Some(min(rps, remaining_rows))
}

fn vstrip_size64_internal(tif: *mut TIFF, nrows: u32, report_errors: bool) -> Option<u64> {
    unsafe {
        let module = "TIFFVStripSize64";
        let rows = if nrows == u32::MAX {
            image_length(tif)?
        } else {
            nrows
        };
        if planar_config(tif) == PLANARCONFIG_CONTIG
            && photometric(tif) == PHOTOMETRIC_YCBCR
            && samples_per_pixel(tif) == 3
            && ((*tif).tif_flags & crate::TIFF_UPSAMPLED) == 0
        {
            let (h, v) = ycbcr_subsampling(tif).unwrap_or((2, 2));
            if h == 0 || v == 0 {
                if report_errors {
                    emit_error_message(tif, module, "Invalid YCbCr subsampling");
                }
                return None;
            }
            let block_samples = u64::from(h) * u64::from(v) + 2;
            let blocks_hor = u64::from(checked_howmany_u32(image_width(tif)?, u32::from(h))?);
            let blocks_ver = u64::from(checked_howmany_u32(rows, u32::from(v))?);
            let row_samples = checked_mul_u64(tif, module, blocks_hor, block_samples)?;
            let row_size = checked_howmany_u64(
                checked_mul_u64(tif, module, row_samples, u64::from(bits_per_sample(tif)))?,
                8,
            )?;
            return checked_mul_u64(tif, module, row_size, blocks_ver);
        }
        checked_mul_u64(
            tif,
            module,
            u64::from(rows),
            scanline_size64_internal(tif, report_errors)?,
        )
    }
}

fn strip_size64_internal(tif: *mut TIFF) -> Option<u64> {
    let height = image_length(tif)?;
    let mut rps = rows_per_strip(tif);
    if rps == u32::MAX || rps > height {
        rps = height;
    }
    vstrip_size64_internal(tif, rps, true)
}

fn tile_row_size64_internal(tif: *mut TIFF) -> Option<u64> {
    let module = "TIFFTileRowSize64";
    let width = tile_width(tif)?;
    let length = tile_length(tif)?;
    if width == 0 {
        emit_error_message(tif, module, "Tile width is zero");
        return None;
    }
    if length == 0 {
        emit_error_message(tif, module, "Tile length is zero");
        return None;
    }

    let bits = checked_mul_u64(
        tif,
        module,
        u64::from(bits_per_sample(tif)),
        u64::from(width),
    )?;
    let row_bits = if planar_config(tif) == PLANARCONFIG_CONTIG {
        let spp = samples_per_pixel(tif);
        if spp == 0 {
            emit_error_message(tif, module, "Samples per pixel is zero");
            return None;
        }
        checked_mul_u64(tif, module, bits, u64::from(spp))?
    } else {
        bits
    };
    let row_size = checked_howmany_u64(row_bits, 8)?;
    if row_size == 0 {
        emit_error_message(tif, module, "Computed tile row size is zero");
        None
    } else {
        Some(row_size)
    }
}

fn vtile_size64_internal(tif: *mut TIFF, nrows: u32) -> Option<u64> {
    unsafe {
        let module = "TIFFVTileSize64";
        let width = tile_width(tif)?;
        let length = tile_length(tif)?;
        let depth = tile_depth(tif);
        if width == 0 || length == 0 || depth == 0 {
            emit_error_message(tif, module, "Tile dimensions must be non-zero");
            return None;
        }
        if planar_config(tif) == PLANARCONFIG_CONTIG
            && photometric(tif) == PHOTOMETRIC_YCBCR
            && samples_per_pixel(tif) == 3
            && ((*tif).tif_flags & crate::TIFF_UPSAMPLED) == 0
        {
            let (h, v) = ycbcr_subsampling(tif).unwrap_or((2, 2));
            if h == 0 || v == 0 {
                emit_error_message(tif, module, "Invalid YCbCr subsampling");
                return None;
            }
            let block_samples = u64::from(h) * u64::from(v) + 2;
            let blocks_hor = u64::from(checked_howmany_u32(width, u32::from(h))?);
            let blocks_ver = u64::from(checked_howmany_u32(nrows, u32::from(v))?);
            let row_samples = checked_mul_u64(tif, module, blocks_hor, block_samples)?;
            let row_size = checked_howmany_u64(
                checked_mul_u64(tif, module, row_samples, u64::from(bits_per_sample(tif)))?,
                8,
            )?;
            checked_mul_u64(tif, module, row_size, blocks_ver)
        } else {
            checked_mul_u64(
                tif,
                module,
                u64::from(nrows),
                tile_row_size64_internal(tif)?,
            )
        }
    }
}

fn tile_size64_internal(tif: *mut TIFF) -> Option<u64> {
    vtile_size64_internal(tif, tile_length(tif)?)
}

fn number_of_strips_internal(tif: *mut TIFF) -> Option<u32> {
    let module = "TIFFNumberOfStrips";
    let height = image_length(tif)?;
    let rps = rows_per_strip(tif);
    let mut strips = if rps == u32::MAX {
        1
    } else {
        checked_howmany_u32(height, rps)?
    };
    if planar_config(tif) == PLANARCONFIG_SEPARATE {
        strips = strips
            .checked_mul(u32::from(samples_per_pixel(tif)))
            .or_else(|| {
                emit_error_message(tif, module, "Integer overflow");
                None
            })?;
    }
    Some(strips)
}

fn number_of_tiles_internal(tif: *mut TIFF) -> Option<u32> {
    let module = "TIFFNumberOfTiles";
    let width = image_width(tif)?;
    let height = image_length(tif)?;
    let depth = image_depth(tif);
    let mut dx = tile_width(tif)?;
    let mut dy = tile_length(tif)?;
    let mut dz = tile_depth(tif);
    if dx == u32::MAX {
        dx = width;
    }
    if dy == u32::MAX {
        dy = height;
    }
    if dz == u32::MAX {
        dz = depth;
    }
    if dx == 0 || dy == 0 || dz == 0 {
        return Some(0);
    }
    let xpt = checked_howmany_u32(width, dx)?;
    let ypt = checked_howmany_u32(height, dy)?;
    let zpt = checked_howmany_u32(depth, dz)?;
    let mut tiles = xpt
        .checked_mul(ypt)
        .and_then(|value| value.checked_mul(zpt))
        .or_else(|| {
            emit_error_message(tif, module, "Integer overflow");
            None
        })?;
    if planar_config(tif) == PLANARCONFIG_SEPARATE {
        tiles = tiles
            .checked_mul(u32::from(samples_per_pixel(tif)))
            .or_else(|| {
                emit_error_message(tif, module, "Integer overflow");
                None
            })?;
    }
    Some(tiles)
}

fn compute_strip_internal(
    tif: *mut TIFF,
    row: u32,
    sample: u16,
    report_errors: bool,
) -> Option<u32> {
    let module = "TIFFComputeStrip";
    let rps = rows_per_strip(tif);
    if rps == 0 {
        if report_errors {
            emit_error_message(tif, module, "Rows per strip is zero");
        }
        return None;
    }
    let mut strip = row / rps;
    if planar_config(tif) == PLANARCONFIG_SEPARATE {
        let spp = samples_per_pixel(tif);
        if sample >= spp {
            if report_errors {
                emit_error_message(tif, module, "Sample out of range");
            }
            return None;
        }
        let strips_per_image = if rps == u32::MAX {
            1
        } else {
            checked_howmany_u32(image_length(tif)?, rps)?
        };
        strip = strip
            .checked_add(u32::from(sample).checked_mul(strips_per_image)?)
            .or_else(|| {
                if report_errors {
                    emit_error_message(tif, module, "Integer overflow");
                }
                None
            })?;
    }
    Some(strip)
}

fn check_tile_internal(tif: *mut TIFF, x: u32, y: u32, z: u32, sample: u16) -> bool {
    let Some(width) = image_width(tif) else {
        emit_error_message(tif, "TIFFCheckTile", "Missing image width");
        return false;
    };
    let Some(height) = image_length(tif) else {
        emit_error_message(tif, "TIFFCheckTile", "Missing image length");
        return false;
    };
    let depth = image_depth(tif);
    if x >= width {
        emit_error_message(tif, "TIFFCheckTile", "Column out of range");
        return false;
    }
    if y >= height {
        emit_error_message(tif, "TIFFCheckTile", "Row out of range");
        return false;
    }
    if z >= depth {
        emit_error_message(tif, "TIFFCheckTile", "Depth out of range");
        return false;
    }
    if planar_config(tif) == PLANARCONFIG_SEPARATE && sample >= samples_per_pixel(tif) {
        emit_error_message(tif, "TIFFCheckTile", "Sample out of range");
        return false;
    }
    true
}

fn compute_tile_internal(tif: *mut TIFF, x: u32, y: u32, mut z: u32, sample: u16) -> Option<u32> {
    let width = image_width(tif)?;
    let height = image_length(tif)?;
    let depth = image_depth(tif);
    let mut dx = tile_width(tif)?;
    let mut dy = tile_length(tif)?;
    let mut dz = tile_depth(tif);
    if depth == 1 {
        z = 0;
    }
    if dx == u32::MAX {
        dx = width;
    }
    if dy == u32::MAX {
        dy = height;
    }
    if dz == u32::MAX {
        dz = depth;
    }
    if dx == 0 || dy == 0 || dz == 0 {
        return Some(0);
    }
    let xpt = checked_howmany_u32(width, dx)?;
    let ypt = checked_howmany_u32(height, dy)?;
    let zpt = checked_howmany_u32(depth, dz)?;
    let tile = if planar_config(tif) == PLANARCONFIG_SEPARATE {
        let tiles_per_plane = xpt.checked_mul(ypt)?.checked_mul(zpt)?;
        u32::from(sample)
            .checked_mul(tiles_per_plane)?
            .checked_add(xpt.checked_mul(ypt)?.checked_mul(z / dz)?)?
            .checked_add(xpt.checked_mul(y / dy)?)?
            .checked_add(x / dx)?
    } else {
        xpt.checked_mul(ypt)?
            .checked_mul(z / dz)?
            .checked_add(xpt.checked_mul(y / dy)?)?
            .checked_add(x / dx)?
    };
    Some(tile)
}

fn expected_strile_count(tif: *mut TIFF) -> Option<usize> {
    let count = if is_tiled_image(tif) {
        number_of_tiles_internal(tif)?
    } else {
        number_of_strips_internal(tif)?
    };
    usize::try_from(count).ok()
}

fn read_strile_arrays(tif: *mut TIFF) -> Option<StrileArrays> {
    let (offset_tag, bytecount_tag) = if is_tiled_image(tif) {
        (TAG_TILEOFFSETS, TAG_TILEBYTECOUNTS)
    } else {
        (TAG_STRIPOFFSETS, TAG_STRIPBYTECOUNTS)
    };
    let count = expected_strile_count(tif)?;
    let mut offsets = copy_u64_array_tag(tif, offset_tag, false).unwrap_or_default();
    let mut bytecounts = copy_u64_array_tag(tif, bytecount_tag, false).unwrap_or_default();
    offsets.resize(count, 0);
    bytecounts.resize(count, 0);
    Some(StrileArrays {
        offset_tag,
        bytecount_tag,
        offsets,
        bytecounts,
    })
}

fn write_strile_arrays(tif: *mut TIFF, arrays: &StrileArrays) -> bool {
    set_u64_array_tag(tif, arrays.offset_tag, &arrays.offsets)
        && set_u64_array_tag(tif, arrays.bytecount_tag, &arrays.bytecounts)
}

fn ensure_strile_arrays(tif: *mut TIFF, module: &str) -> Option<StrileArrays> {
    let arrays = read_strile_arrays(tif)?;
    if write_strile_arrays(tif, &arrays) {
        Some(arrays)
    } else {
        emit_error_message(tif, module, "Failed to synchronize strile arrays");
        None
    }
}

fn read_strile_value_pair(tif: *mut TIFF, strile: u32) -> Option<(u64, u64)> {
    let (offset_tag, bytecount_tag) = if is_tiled_image(tif) {
        (TAG_TILEOFFSETS, TAG_TILEBYTECOUNTS)
    } else {
        (TAG_STRIPOFFSETS, TAG_STRIPBYTECOUNTS)
    };
    let mut err = 0;
    let offset = get_strile_tag_value_u64(tif, offset_tag, strile, &mut err)?;
    if err != 0 {
        return None;
    }
    let bytecount = get_strile_tag_value_u64(tif, bytecount_tag, strile, &mut err)?;
    if err != 0 {
        return None;
    }
    Some((offset, bytecount))
}

fn write_appended_strile_data(tif: *mut TIFF, module: &str, strile: u32, data: &[u8]) -> bool {
    unsafe {
        (*tif_inner(tif))
            .codec_state
            .pending_striles
            .remove(&strile);
        (*tif_inner(tif)).codec_state.decoded_cache = None;
        let Some(mut arrays) = ensure_strile_arrays(tif, module) else {
            return false;
        };
        let Ok(index) = usize::try_from(strile) else {
            return false;
        };
        if index >= arrays.offsets.len() {
            emit_error_message(tif, module, "Strile index out of range");
            return false;
        }
        let offset = if arrays.offsets[index] == 0 {
            let Some(start) = next_append_offset(tif) else {
                emit_error_message(tif, module, "Failed to seek to the end of the file");
                return false;
            };
            arrays.offsets[index] = start;
            start
        } else {
            arrays.offsets[index]
        };
        let Some(write_offset) = checked_add_u64(tif, module, offset, arrays.bytecounts[index])
        else {
            return false;
        };
        if !write_exact_at(tif, write_offset, data) {
            emit_error_message(tif, module, "Failed to write strile payload");
            return false;
        }
        let Some(new_bytecount) =
            checked_add_u64(tif, module, arrays.bytecounts[index], data.len() as u64)
        else {
            return false;
        };
        arrays.bytecounts[index] = new_bytecount;
        if write_strile_arrays(tif, &arrays) {
            (*tif).tif_flags |= TIFF_DIRTYSTRIP | TIFF_BEENWRITING;
            true
        } else {
            emit_error_message(tif, module, "Failed to update strile arrays");
            false
        }
    }
}

fn write_overwrite_strile_data(tif: *mut TIFF, module: &str, strile: u32, data: &[u8]) -> bool {
    unsafe {
        (*tif_inner(tif))
            .codec_state
            .pending_striles
            .remove(&strile);
        (*tif_inner(tif)).codec_state.decoded_cache = None;
        let Some(mut arrays) = ensure_strile_arrays(tif, module) else {
            return false;
        };
        let Ok(index) = usize::try_from(strile) else {
            return false;
        };
        if index >= arrays.offsets.len() {
            emit_error_message(tif, module, "Strile index out of range");
            return false;
        }
        let reuse_existing =
            arrays.offsets[index] != 0 && arrays.bytecounts[index] >= data.len() as u64;
        let offset = if reuse_existing {
            arrays.offsets[index]
        } else {
            let Some(start) = next_append_offset(tif) else {
                emit_error_message(tif, module, "Failed to seek to the end of the file");
                return false;
            };
            arrays.offsets[index] = start;
            start
        };
        if !write_exact_at(tif, offset, data) {
            emit_error_message(tif, module, "Failed to write strile payload");
            return false;
        }
        arrays.bytecounts[index] = data.len() as u64;
        if write_strile_arrays(tif, &arrays) {
            (*tif).tif_flags |= TIFF_DIRTYSTRIP | TIFF_BEENWRITING;
            true
        } else {
            emit_error_message(tif, module, "Failed to update strile arrays");
            false
        }
    }
}

fn write_scanline_data(tif: *mut TIFF, module: &str, row: u32, sample: u16, data: &[u8]) -> bool {
    unsafe {
        let Some(mut arrays) = ensure_strile_arrays(tif, module) else {
            return false;
        };
        let Some(strip) = compute_strip_internal(tif, row, sample, true) else {
            return false;
        };
        (*tif_inner(tif)).codec_state.pending_striles.remove(&strip);
        (*tif_inner(tif)).codec_state.decoded_cache = None;
        let Ok(index) = usize::try_from(strip) else {
            emit_error_message(tif, module, "Strip index out of range");
            return false;
        };
        if index >= arrays.offsets.len() {
            emit_error_message(tif, module, "Strip index out of range");
            return false;
        }
        let strip_offset = if arrays.offsets[index] == 0 {
            let Some(start) = next_append_offset(tif) else {
                emit_error_message(tif, module, "Failed to seek to the end of the file");
                return false;
            };
            arrays.offsets[index] = start;
            start
        } else {
            arrays.offsets[index]
        };

        let rps = rows_per_strip(tif);
        let strip_row = if rps == u32::MAX { row } else { row % rps };
        let Some(within_strip) =
            checked_mul_u64(tif, module, u64::from(strip_row), data.len() as u64)
        else {
            return false;
        };
        let Some(write_offset) = checked_add_u64(tif, module, strip_offset, within_strip) else {
            return false;
        };
        if !write_exact_at(tif, write_offset, data) {
            emit_error_message(tif, module, "Failed to write scanline data");
            return false;
        }
        arrays.bytecounts[index] = max(arrays.bytecounts[index], within_strip + data.len() as u64);
        if write_strile_arrays(tif, &arrays) {
            (*tif).tif_flags |= TIFF_DIRTYSTRIP | TIFF_BEENWRITING;
            (*tif_inner(tif)).tif_curstrip = strip;
            (*tif).tif_row = row + 1;
            true
        } else {
            emit_error_message(tif, module, "Failed to update strip arrays");
            false
        }
    }
}

fn read_strile_bytes(
    tif: *mut TIFF,
    module: &str,
    strile: u32,
    size: usize,
    buf: *mut c_void,
) -> Option<usize> {
    unsafe {
        let (offset, bytecount) = read_strile_value_pair(tif, strile)?;
        if offset == 0 || bytecount == 0 {
            return Some(0);
        }
        let to_read = min(size as u64, bytecount);
        let to_read_usize = usize::try_from(to_read).ok()?;
        if to_read_usize == 0 {
            return Some(0);
        }
        let out = slice::from_raw_parts_mut(buf.cast::<u8>(), to_read_usize);
        if read_exact_at(tif, offset, out) {
            Some(to_read_usize)
        } else {
            emit_error_message(tif, module, "Failed to read strile payload");
            None
        }
    }
}

fn read_encoded_strile_bytes(
    tif: *mut TIFF,
    module: &str,
    is_tile: bool,
    strile: u32,
    geometry: CodecGeometry,
    expected_size: usize,
    out: &mut [u8],
) -> Option<usize> {
    unsafe {
        if out.is_empty() {
            return Some(0);
        }
        if !load_decoded_strile_cache(tif, module, is_tile, strile, geometry, expected_size) {
            None
        } else {
            let cache = (*tif_inner(tif)).codec_state.decoded_cache.as_ref()?;
            if cache.decoded.len() < out.len() {
                emit_error_message(tif, module, "Decoded strile is smaller than requested");
                return None;
            }
            out.copy_from_slice(&cache.decoded[..out.len()]);
            if is_tile {
                (*tif_inner(tif)).tif_curtile = strile;
            } else {
                (*tif_inner(tif)).tif_curstrip = strile;
            }
            Some(out.len())
        }
    }
}

fn expected_strip_size_for_index(tif: *mut TIFF, strip: u32) -> Option<u64> {
    vstrip_size64_internal(tif, strip_size_rows_internal(tif, strip, true)?, true)
}

fn expected_tile_size(tif: *mut TIFF) -> Option<usize> {
    unsafe { usize::try_from(TIFFTileSize64(tif)).ok() }
}

fn codec_geometry_for_strip(tif: *mut TIFF, strip: u32) -> Option<CodecGeometry> {
    unsafe {
        let row_size = usize::try_from(TIFFScanlineSize64(tif)).ok()?;
        let rows = usize::try_from(strip_size_rows_internal(tif, strip, true)?).ok()?;
        Some(CodecGeometry {
            row_size,
            rows,
            width: image_width(tif)?,
        })
    }
}

fn codec_geometry_for_tile(tif: *mut TIFF) -> Option<CodecGeometry> {
    unsafe {
        let row_size = usize::try_from(TIFFTileRowSize64(tif)).ok()?;
        let rows = usize::try_from(tile_length(tif)?).ok()?;
        Some(CodecGeometry {
            row_size,
            rows,
            width: tile_width(tif)?,
        })
    }
}

fn load_decoded_strile_cache(
    tif: *mut TIFF,
    module: &str,
    is_tile: bool,
    strile: u32,
    geometry: CodecGeometry,
    expected_size: usize,
) -> bool {
    unsafe {
        if let Some(cache) = (*tif_inner(tif)).codec_state.decoded_cache.as_ref() {
            if cache.is_tile == is_tile
                && cache.index == strile
                && cache.decoded.len() >= expected_size
            {
                return true;
            }
        }
        if let Some(staged) = (*tif_inner(tif)).codec_state.pending_striles.get(&strile) {
            if staged.decoded.len() < expected_size {
                emit_error_message(tif, module, "Pending strile buffer is truncated");
                return false;
            }
            (*tif_inner(tif)).codec_state.decoded_cache = Some(DecodedStrileCache {
                is_tile,
                index: strile,
                decoded: staged.decoded.clone(),
            });
            return true;
        }
        let Some((offset, bytecount)) = read_strile_value_pair(tif, strile) else {
            emit_error_message(tif, module, "Strile out of range");
            return false;
        };
        if offset == 0 || bytecount == 0 {
            emit_error_message(tif, module, "Strile byte count is zero");
            return false;
        }
        let Ok(raw_len) = usize::try_from(bytecount) else {
            emit_error_message(tif, module, "Strile byte count is too large");
            return false;
        };
        let mut raw = vec![0u8; raw_len];
        if !read_exact_at(tif, offset, &mut raw) {
            emit_error_message(tif, module, "Failed to read strile payload");
            return false;
        }
        let Some(decoded) =
            safe_tiff_codec_decode_bytes(tif, &raw, is_tile, strile, geometry, expected_size)
        else {
            emit_error_message(tif, module, "Codec decode failed");
            return false;
        };
        (*tif_inner(tif)).codec_state.decoded_cache = Some(DecodedStrileCache {
            is_tile,
            index: strile,
            decoded,
        });
        true
    }
}

fn flush_pending_codec_striles(tif: *mut TIFF, module: &str) -> bool {
    unsafe {
        let inner = tif_inner(tif);
        let mut pending = std::mem::take(&mut (*inner).codec_state.pending_striles);
        let keys: Vec<u32> = pending.keys().copied().collect();
        for strile in keys {
            let Some(staged) = pending.remove(&strile) else {
                continue;
            };
            let geometry = CodecGeometry {
                row_size: staged.row_size,
                rows: staged.rows,
                width: staged.width,
            };
            let Some(encoded) = safe_tiff_codec_encode_bytes(tif, &staged.decoded, geometry) else {
                emit_error_message(tif, module, "Codec encode failed");
                pending.insert(strile, staged);
                (*inner).codec_state.pending_striles.append(&mut pending);
                return false;
            };
            if !write_overwrite_strile_data(tif, module, strile, &encoded) {
                pending.insert(strile, staged);
                (*inner).codec_state.pending_striles.append(&mut pending);
                return false;
            }
        }
        (*inner).codec_state.decoded_cache = None;
        true
    }
}

fn stage_pending_codec_scanline(
    tif: *mut TIFF,
    module: &str,
    row: u32,
    sample: u16,
    data: &[u8],
) -> bool {
    unsafe {
        let Some(strip) = compute_strip_internal(tif, row, sample, true) else {
            return false;
        };
        let Some(geometry) = codec_geometry_for_strip(tif, strip) else {
            emit_error_message(tif, module, "Failed to compute strip geometry");
            return false;
        };
        let Some(expected_size) = geometry.row_size.checked_mul(geometry.rows) else {
            emit_error_message(tif, module, "Strip decode size is too large");
            return false;
        };
        let rps = rows_per_strip(tif);
        let strip_row = if rps == u32::MAX { row } else { row % rps };
        let Some(within_strip) =
            checked_mul_u64(tif, module, u64::from(strip_row), geometry.row_size as u64)
        else {
            return false;
        };
        let Some(end_offset) = checked_add_u64(tif, module, within_strip, data.len() as u64) else {
            return false;
        };
        if end_offset > expected_size as u64 {
            emit_error_message(tif, module, "Scanline extends beyond the strip byte count");
            return false;
        }
        let start = within_strip as usize;
        let end = end_offset as usize;
        let inner = tif_inner(tif);
        let entry = (*inner)
            .codec_state
            .pending_striles
            .entry(strip)
            .or_insert_with(|| PendingStrileWrite {
                decoded: vec![0; expected_size],
                row_size: geometry.row_size,
                rows: geometry.rows,
                width: geometry.width,
            });
        if entry.row_size != geometry.row_size
            || entry.rows != geometry.rows
            || entry.width != geometry.width
            || entry.decoded.len() != expected_size
        {
            *entry = PendingStrileWrite {
                decoded: vec![0; expected_size],
                row_size: geometry.row_size,
                rows: geometry.rows,
                width: geometry.width,
            };
        }
        entry.decoded[start..end].copy_from_slice(data);
        (*inner).codec_state.decoded_cache = None;
        (*tif).tif_flags |= TIFF_DIRTYSTRIP | TIFF_BEENWRITING;
        (*inner).tif_curstrip = strip;
        (*tif).tif_row = row + 1;
        true
    }
}

fn read_scanline_bytes(tif: *mut TIFF, row: u32, sample: u16, out: &mut [u8]) -> bool {
    unsafe {
        let module = "TIFFReadScanline";
        let Some(height) = image_length(tif) else {
            emit_error_message(tif, module, "Missing image length");
            return false;
        };
        if row >= height {
            emit_error_message(tif, module, "Row out of range");
            return false;
        }
        if is_tiled_image(tif) {
            let Some(width) = image_width(tif) else {
                emit_error_message(tif, module, "Missing image width");
                return false;
            };
            let Some(tile_width) = tile_width(tif) else {
                emit_error_message(tif, module, "Missing tile width");
                return false;
            };
            let Some(tile_length) = tile_length(tif) else {
                emit_error_message(tif, module, "Missing tile length");
                return false;
            };
            if tile_width == 0 || tile_length == 0 {
                emit_error_message(tif, module, "Tile dimensions are invalid");
                return false;
            }
            let samples = if planar_config(tif) == PLANARCONFIG_CONTIG {
                usize::from(samples_per_pixel(tif).max(1))
            } else {
                1
            };
            let pixel_bits = usize::from(bits_per_sample(tif))
                .checked_mul(samples)
                .unwrap_or(0);
            if pixel_bits == 0 || (pixel_bits % 8) != 0 {
                emit_error_message(
                    tif,
                    module,
                    "Tiled scanline reads require byte-aligned pixels",
                );
                return false;
            }
            let bytes_per_pixel = pixel_bits / 8;
            let Some(geometry) = codec_geometry_for_tile(tif) else {
                emit_error_message(tif, module, "Failed to compute tile geometry");
                return false;
            };
            let Some(expected_size) = geometry.row_size.checked_mul(geometry.rows) else {
                emit_error_message(tif, module, "Tile decode size is too large");
                return false;
            };
            let row_in_tile = usize::try_from(row % tile_length).unwrap_or(0);
            let mut out_offset = 0usize;
            let mut x = 0u32;
            while x < width {
                let Some(tile) = compute_tile_internal(tif, x, row, 0, sample) else {
                    emit_error_message(tif, module, "Failed to compute tile index");
                    return false;
                };
                if !load_decoded_strile_cache(tif, module, true, tile, geometry, expected_size) {
                    return false;
                }
                let Some(cache) = (*tif_inner(tif)).codec_state.decoded_cache.as_ref() else {
                    emit_error_message(tif, module, "Decoded tile cache is unavailable");
                    return false;
                };
                let row_start = match row_in_tile.checked_mul(geometry.row_size) {
                    Some(value) => value,
                    None => {
                        emit_error_message(tif, module, "Tile row offset overflow");
                        return false;
                    }
                };
                let tile_pixels = usize::try_from(tile_width.min(width - x)).unwrap_or(0);
                let tile_bytes = match tile_pixels.checked_mul(bytes_per_pixel) {
                    Some(value) => value,
                    None => {
                        emit_error_message(tif, module, "Tile row size overflow");
                        return false;
                    }
                };
                let row_end = match row_start.checked_add(tile_bytes) {
                    Some(value) => value,
                    None => {
                        emit_error_message(tif, module, "Tile row bounds overflow");
                        return false;
                    }
                };
                let out_end = match out_offset.checked_add(tile_bytes) {
                    Some(value) => value,
                    None => {
                        emit_error_message(tif, module, "Scanline assembly overflow");
                        return false;
                    }
                };
                if row_end > cache.decoded.len() || out_end > out.len() {
                    emit_error_message(tif, module, "Tile row extends beyond decoded data");
                    return false;
                }
                out[out_offset..out_end].copy_from_slice(&cache.decoded[row_start..row_end]);
                out_offset = out_end;
                x = match x.checked_add(tile_width) {
                    Some(value) => value,
                    None => {
                        emit_error_message(tif, module, "Tile iteration overflow");
                        return false;
                    }
                };
                (*tif_inner(tif)).tif_curtile = tile;
            }
            if out_offset != out.len() {
                emit_error_message(tif, module, "Assembled scanline size is invalid");
                return false;
            }
            (*tif).tif_row = row + 1;
            return true;
        }
        let Some(strip) = compute_strip_internal(tif, row, sample, true) else {
            return false;
        };
        let Some(geometry) = codec_geometry_for_strip(tif, strip) else {
            emit_error_message(tif, module, "Failed to compute strip geometry");
            return false;
        };
        let Some(expected_size) = geometry.row_size.checked_mul(geometry.rows) else {
            emit_error_message(tif, module, "Strip decode size is too large");
            return false;
        };
        if !load_decoded_strile_cache(tif, module, false, strip, geometry, expected_size) {
            return false;
        }
        let Some(cache) = (*tif_inner(tif)).codec_state.decoded_cache.as_ref() else {
            emit_error_message(tif, module, "Decoded strip cache is unavailable");
            return false;
        };
        let rps = rows_per_strip(tif);
        let strip_row = if rps == u32::MAX { row } else { row % rps };
        let Some(within_strip) =
            checked_mul_u64(tif, module, u64::from(strip_row), geometry.row_size as u64)
        else {
            return false;
        };
        let Some(end_offset) = checked_add_u64(tif, module, within_strip, out.len() as u64) else {
            return false;
        };
        if end_offset > expected_size as u64 || end_offset as usize > cache.decoded.len() {
            emit_error_message(tif, module, "Scanline extends beyond the strip byte count");
            return false;
        }
        out.copy_from_slice(&cache.decoded[within_strip as usize..end_offset as usize]);
        (*tif_inner(tif)).tif_curstrip = strip;
        (*tif).tif_row = row + 1;
        true
    }
}

fn check_read_mode(tif: *mut TIFF, tiles: bool, module: &str) -> bool {
    unsafe {
        if tif.is_null() {
            return false;
        }
        if crate::TIFFGetMode(tif) == libc::O_WRONLY {
            emit_error_message(tif, module, "File not open for reading");
            return false;
        }
        if tiles ^ is_tiled_image(tif) {
            emit_error_message(
                tif,
                module,
                if tiles {
                    "Can not read tiles from a striped image"
                } else {
                    "Can not read scanlines from a tiled image"
                },
            );
            return false;
        }
        true
    }
}

fn check_write_mode(tif: *mut TIFF, tiles: bool, module: &str) -> bool {
    unsafe {
        if tif.is_null() {
            return false;
        }
        if crate::TIFFGetMode(tif) == libc::O_RDONLY {
            emit_error_message(tif, module, "File not open for writing");
            return false;
        }
        if tiles ^ is_tiled_image(tif) {
            emit_error_message(
                tif,
                module,
                if tiles {
                    "Can not write tiles to a striped image"
                } else {
                    "Can not write scanlines to a tiled image"
                },
            );
            return false;
        }
        true
    }
}

fn free_raw_buffer_if_owned(tif: *mut TIFF) {
    unsafe {
        if !(*tif).tif_rawdata.is_null() && ((*tif).tif_flags & TIFF_MYBUFFER) != 0 {
            _TIFFfree((*tif).tif_rawdata.cast::<c_void>());
        }
        (*tif).tif_rawdata = ptr::null_mut();
        (*tif).tif_rawdatasize = 0;
        (*tif).tif_rawcp = ptr::null_mut();
        (*tif).tif_rawcc = 0;
        (*tif).tif_flags &= !(TIFF_MYBUFFER | TIFF_BUFFERMMAP);
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFScanlineSize64(tif: *mut TIFF) -> u64 {
    scanline_size64_internal(tif, true).unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn TIFFScanlineSize(tif: *mut TIFF) -> Tmsize {
    unsafe { cast_u64_to_tmsize(tif, "TIFFScanlineSize", TIFFScanlineSize64(tif)) }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFRasterScanlineSize64(tif: *mut TIFF) -> u64 {
    raster_scanline_size64_internal(tif).unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn TIFFRasterScanlineSize(tif: *mut TIFF) -> Tmsize {
    unsafe { cast_u64_to_tmsize(tif, "TIFFRasterScanlineSize", TIFFRasterScanlineSize64(tif)) }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFVStripSize64(tif: *mut TIFF, nrows: u32) -> u64 {
    vstrip_size64_internal(tif, nrows, true).unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn TIFFVStripSize(tif: *mut TIFF, nrows: u32) -> Tmsize {
    unsafe { cast_u64_to_tmsize(tif, "TIFFVStripSize", TIFFVStripSize64(tif, nrows)) }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFStripSize64(tif: *mut TIFF) -> u64 {
    strip_size64_internal(tif).unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn TIFFStripSize(tif: *mut TIFF) -> Tmsize {
    unsafe { cast_u64_to_tmsize(tif, "TIFFStripSize", TIFFStripSize64(tif)) }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFRawStripSize64(tif: *mut TIFF, strip: u32) -> u64 {
    unsafe { TIFFGetStrileByteCount(tif, strip) }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFRawStripSize(tif: *mut TIFF, strip: u32) -> Tmsize {
    unsafe { cast_u64_to_tmsize(tif, "TIFFRawStripSize", TIFFRawStripSize64(tif, strip)) }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFTileRowSize64(tif: *mut TIFF) -> u64 {
    tile_row_size64_internal(tif).unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn TIFFTileRowSize(tif: *mut TIFF) -> Tmsize {
    unsafe { cast_u64_to_tmsize(tif, "TIFFTileRowSize", TIFFTileRowSize64(tif)) }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFVTileSize64(tif: *mut TIFF, nrows: u32) -> u64 {
    vtile_size64_internal(tif, nrows).unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn TIFFVTileSize(tif: *mut TIFF, nrows: u32) -> Tmsize {
    unsafe { cast_u64_to_tmsize(tif, "TIFFVTileSize", TIFFVTileSize64(tif, nrows)) }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFTileSize64(tif: *mut TIFF) -> u64 {
    tile_size64_internal(tif).unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn TIFFTileSize(tif: *mut TIFF) -> Tmsize {
    unsafe { cast_u64_to_tmsize(tif, "TIFFTileSize", TIFFTileSize64(tif)) }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFDefaultStripSize(tif: *mut TIFF, request: u32) -> u32 {
    if request != 0 {
        request
    } else {
        let scanline_size = scanline_size64_internal(tif, false).unwrap_or(1).max(1);
        let rows = (8192u64 / scanline_size).max(1);
        min(rows, u32::MAX as u64) as u32
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFDefaultTileSize(_: *mut TIFF, tw: *mut u32, th: *mut u32) {
    unsafe {
        if !tw.is_null() {
            if *tw == 0 {
                *tw = 256;
            }
            if (*tw & 0x0f) != 0 {
                *tw = (*tw + 15) & !15;
            }
        }
        if !th.is_null() {
            if *th == 0 {
                *th = 256;
            }
            if (*th & 0x0f) != 0 {
                *th = (*th + 15) & !15;
            }
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFComputeStrip(tif: *mut TIFF, row: u32, sample: u16) -> u32 {
    compute_strip_internal(tif, row, sample, true).unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn TIFFNumberOfStrips(tif: *mut TIFF) -> u32 {
    number_of_strips_internal(tif).unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn TIFFComputeTile(
    tif: *mut TIFF,
    x: u32,
    y: u32,
    z: u32,
    sample: u16,
) -> u32 {
    compute_tile_internal(tif, x, y, z, sample).unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn TIFFCheckTile(
    tif: *mut TIFF,
    x: u32,
    y: u32,
    z: u32,
    sample: u16,
) -> c_int {
    check_tile_internal(tif, x, y, z, sample) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn TIFFNumberOfTiles(tif: *mut TIFF) -> u32 {
    if !is_tiled_image(tif) {
        0
    } else {
        number_of_tiles_internal(tif).unwrap_or(0)
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFReadBufferSetup(
    tif: *mut TIFF,
    bp: *mut c_void,
    size: Tmsize,
) -> c_int {
    unsafe {
        if tif.is_null() {
            return 0;
        }
        free_raw_buffer_if_owned(tif);
        (*tif).tif_flags &= !TIFF_BUFFERMMAP;
        let alloc_size = if size <= 0 {
            8192
        } else {
            ((size + 1023) / 1024) * 1024
        };
        if bp.is_null() {
            let buffer = _TIFFcallocExt(tif, 1, alloc_size);
            if buffer.is_null() {
                emit_error_message(tif, "TIFFReadBufferSetup", "No space for data buffer");
                return 0;
            }
            (*tif).tif_rawdata = buffer.cast::<u8>();
            (*tif).tif_flags |= TIFF_MYBUFFER;
        } else {
            (*tif).tif_rawdata = bp.cast::<u8>();
            (*tif).tif_flags &= !TIFF_MYBUFFER;
        }
        (*tif).tif_rawdatasize = alloc_size;
        (*tif).tif_rawcp = (*tif).tif_rawdata;
        (*tif).tif_rawcc = 0;
        1
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFWriteBufferSetup(
    tif: *mut TIFF,
    bp: *mut c_void,
    size: Tmsize,
) -> c_int {
    unsafe {
        if tif.is_null() {
            return 0;
        }
        free_raw_buffer_if_owned(tif);
        let mut alloc_size = size;
        if alloc_size == -1 {
            alloc_size = if is_tiled_image(tif) {
                TIFFTileSize(tif)
            } else {
                TIFFStripSize(tif)
            };
            if alloc_size <= 0 {
                alloc_size = 8192;
            } else {
                alloc_size = alloc_size + alloc_size / 10;
                if alloc_size < 8192 {
                    alloc_size = 8192;
                }
            }
        }
        if bp.is_null() {
            let buffer = _TIFFmallocExt(tif, alloc_size);
            if buffer.is_null() {
                emit_error_message(tif, "TIFFWriteBufferSetup", "No space for output buffer");
                return 0;
            }
            (*tif).tif_rawdata = buffer.cast::<u8>();
            (*tif).tif_flags |= TIFF_MYBUFFER;
        } else {
            (*tif).tif_rawdata = bp.cast::<u8>();
            (*tif).tif_flags &= !TIFF_MYBUFFER;
        }
        (*tif).tif_rawdatasize = alloc_size;
        (*tif).tif_rawcp = (*tif).tif_rawdata;
        (*tif).tif_rawcc = 0;
        (*tif).tif_flags |= TIFF_BUFFERSETUP;
        1
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFSetupStrips(tif: *mut TIFF) -> c_int {
    if tif.is_null() {
        return 0;
    }
    ensure_strile_arrays(tif, "TIFFSetupStrips").is_some() as c_int
}

#[no_mangle]
pub unsafe extern "C" fn TIFFWriteCheck(
    tif: *mut TIFF,
    tiles: c_int,
    module: *const libc::c_char,
) -> c_int {
    unsafe {
        let module_name = if module.is_null() {
            "TIFFWriteCheck"
        } else {
            std::ffi::CStr::from_ptr(module)
                .to_str()
                .unwrap_or("TIFFWriteCheck")
        };
        if !check_write_mode(tif, tiles != 0, module_name) {
            return 0;
        }
        if require_u32_tag(tif, TAG_IMAGEWIDTH, module_name, "ImageWidth").is_none() {
            return 0;
        }
        if require_u32_tag(tif, TAG_IMAGELENGTH, module_name, "ImageLength").is_none() {
            return 0;
        }
        if TIFFSetupStrips(tif) == 0 {
            emit_error_message(tif, module_name, "No space for strip/tile arrays");
            return 0;
        }
        if TIFFScanlineSize64(tif) == 0 {
            return 0;
        }
        if tiles != 0 && TIFFTileSize64(tif) == 0 {
            return 0;
        }
        (*tif).tif_flags |= TIFF_BEENWRITING;
        1
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFWriteScanline(
    tif: *mut TIFF,
    buf: *mut c_void,
    row: u32,
    sample: u16,
) -> c_int {
    unsafe {
        let module = "TIFFWriteScanline";
        if !check_write_mode(tif, false, module) {
            return -1;
        }
        if buf.is_null() {
            emit_error_message(tif, module, "Input buffer is NULL");
            return -1;
        }
        let mut height = image_length(tif).unwrap_or(0);
        if row >= height {
            if planar_config(tif) == PLANARCONFIG_SEPARATE {
                emit_error_message(
                    tif,
                    module,
                    "Can not change \"ImageLength\" when using separate planes",
                );
                return -1;
            }
            height = row.saturating_add(1);
            if !set_u32_tag(tif, TAG_IMAGELENGTH, height) {
                return -1;
            }
        }
        if TIFFWriteCheck(tif, 0, ptr::null()) == 0 {
            return -1;
        }
        let scanline_size = TIFFScanlineSize64(tif);
        if scanline_size == 0 {
            return -1;
        }
        let Ok(scanline_size) = usize::try_from(scanline_size) else {
            emit_error_message(tif, module, "Scanline size is too large");
            return -1;
        };
        let data = slice::from_raw_parts(buf.cast::<u8>(), scanline_size);
        if compression(tif) == COMPRESSION_NONE {
            let encoded = encode_uncompressed_bytes(tif, data);
            if write_scanline_data(tif, module, row, sample, &encoded) {
                1
            } else {
                -1
            }
        } else if stage_pending_codec_scanline(tif, module, row, sample, data) {
            1
        } else {
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFWriteEncodedStrip(
    tif: *mut TIFF,
    strip: u32,
    data: *mut c_void,
    cc: Tmsize,
) -> Tmsize {
    unsafe {
        let module = "TIFFWriteEncodedStrip";
        if !check_write_mode(tif, false, module) || data.is_null() || cc < 0 {
            return -1;
        }
        if TIFFWriteCheck(tif, 0, ptr::null()) == 0 {
            return -1;
        }
        let Ok(size) = usize::try_from(cc) else {
            emit_error_message(tif, module, "Encoded strip size is invalid");
            return -1;
        };
        let bytes = slice::from_raw_parts(data.cast::<u8>(), size);
        let Some(geometry) = codec_geometry_for_strip(tif, strip) else {
            emit_error_message(tif, module, "Failed to compute strip geometry");
            return -1;
        };
        let encoded = if compression(tif) == COMPRESSION_NONE {
            encode_uncompressed_bytes(tif, bytes)
        } else if let Some(encoded) = safe_tiff_codec_encode_bytes(tif, bytes, geometry) {
            encoded
        } else {
            emit_error_message(tif, module, "Codec encode failed");
            return -1;
        };
        if write_overwrite_strile_data(tif, module, strip, &encoded) {
            (*tif_inner(tif)).tif_curstrip = strip;
            cc
        } else {
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFWriteRawStrip(
    tif: *mut TIFF,
    strip: u32,
    data: *mut c_void,
    cc: Tmsize,
) -> Tmsize {
    unsafe {
        let module = "TIFFWriteRawStrip";
        if !check_write_mode(tif, false, module) || data.is_null() || cc < 0 {
            return -1;
        }
        if TIFFWriteCheck(tif, 0, ptr::null()) == 0 {
            return -1;
        }
        let Ok(size) = usize::try_from(cc) else {
            emit_error_message(tif, module, "Raw strip size is invalid");
            return -1;
        };
        let bytes = slice::from_raw_parts(data.cast::<u8>(), size);
        if write_appended_strile_data(tif, module, strip, bytes) {
            (*tif_inner(tif)).tif_curstrip = strip;
            cc
        } else {
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFWriteTile(
    tif: *mut TIFF,
    buf: *mut c_void,
    x: u32,
    y: u32,
    z: u32,
    sample: u16,
) -> Tmsize {
    unsafe {
        if !check_tile_internal(tif, x, y, z, sample) {
            return -1;
        }
        let tile = TIFFComputeTile(tif, x, y, z, sample);
        TIFFWriteEncodedTile(tif, tile, buf, TIFFTileSize(tif))
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFWriteEncodedTile(
    tif: *mut TIFF,
    tile: u32,
    data: *mut c_void,
    cc: Tmsize,
) -> Tmsize {
    unsafe {
        let module = "TIFFWriteEncodedTile";
        if !check_write_mode(tif, true, module) || data.is_null() || cc < -1 {
            return -1;
        }
        if TIFFWriteCheck(tif, 1, ptr::null()) == 0 {
            return -1;
        }
        let tile_size = TIFFTileSize64(tif);
        if tile_size == 0 {
            return -1;
        }
        let mut size = if cc < 0 { tile_size } else { cc as u64 };
        size = min(size, tile_size);
        let Ok(size_usize) = usize::try_from(size) else {
            emit_error_message(tif, module, "Encoded tile size is invalid");
            return -1;
        };
        let bytes = slice::from_raw_parts(data.cast::<u8>(), size_usize);
        let Some(geometry) = codec_geometry_for_tile(tif) else {
            emit_error_message(tif, module, "Failed to compute tile geometry");
            return -1;
        };
        let encoded = if compression(tif) == COMPRESSION_NONE {
            encode_uncompressed_bytes(tif, bytes)
        } else if let Some(encoded) = safe_tiff_codec_encode_bytes(tif, bytes, geometry) {
            encoded
        } else {
            emit_error_message(tif, module, "Codec encode failed");
            return -1;
        };
        if write_overwrite_strile_data(tif, module, tile, &encoded) {
            (*tif_inner(tif)).tif_curtile = tile;
            size as Tmsize
        } else {
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFWriteRawTile(
    tif: *mut TIFF,
    tile: u32,
    data: *mut c_void,
    cc: Tmsize,
) -> Tmsize {
    unsafe {
        let module = "TIFFWriteRawTile";
        if !check_write_mode(tif, true, module) || data.is_null() || cc < 0 {
            return -1;
        }
        if TIFFWriteCheck(tif, 1, ptr::null()) == 0 {
            return -1;
        }
        let Ok(size) = usize::try_from(cc) else {
            emit_error_message(tif, module, "Raw tile size is invalid");
            return -1;
        };
        let bytes = slice::from_raw_parts(data.cast::<u8>(), size);
        if write_appended_strile_data(tif, module, tile, bytes) {
            (*tif_inner(tif)).tif_curtile = tile;
            cc
        } else {
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFReadScanline(
    tif: *mut TIFF,
    buf: *mut c_void,
    row: u32,
    sample: u16,
) -> c_int {
    unsafe {
        if tif.is_null() || buf.is_null() {
            return -1;
        }
        if crate::TIFFGetMode(tif) == libc::O_WRONLY {
            emit_error_message(tif, "TIFFReadScanline", "File not open for reading");
            return -1;
        }
        let size = TIFFScanlineSize64(tif);
        if size == 0 {
            return -1;
        }
        let Ok(size_usize) = usize::try_from(size) else {
            emit_error_message(tif, "TIFFReadScanline", "Scanline size is too large");
            return -1;
        };
        let out = slice::from_raw_parts_mut(buf.cast::<u8>(), size_usize);
        if read_scanline_bytes(tif, row, sample, out) {
            1
        } else {
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFReadEncodedStrip(
    tif: *mut TIFF,
    strip: u32,
    buf: *mut c_void,
    size: Tmsize,
) -> Tmsize {
    unsafe {
        let module = "TIFFReadEncodedStrip";
        if !check_read_mode(tif, false, module) || buf.is_null() {
            return -1;
        }
        let Some(expected_size) = expected_strip_size_for_index(tif, strip) else {
            return -1;
        };
        let Some(geometry) = codec_geometry_for_strip(tif, strip) else {
            emit_error_message(tif, module, "Failed to compute strip geometry");
            return -1;
        };
        let requested = if size == -1 {
            expected_size
        } else if size < 0 {
            return -1;
        } else {
            min(expected_size, size as u64)
        };
        let Ok(requested_usize) = usize::try_from(requested) else {
            emit_error_message(tif, module, "Requested strip size is too large");
            return -1;
        };
        let out = slice::from_raw_parts_mut(buf.cast::<u8>(), requested_usize);
        match read_encoded_strile_bytes(
            tif,
            module,
            false,
            strip,
            geometry,
            expected_size as usize,
            out,
        ) {
            Some(read_size) => read_size as Tmsize,
            None => -1,
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFReadRawStrip(
    tif: *mut TIFF,
    strip: u32,
    buf: *mut c_void,
    size: Tmsize,
) -> Tmsize {
    let module = "TIFFReadRawStrip";
    if !check_read_mode(tif, false, module) || buf.is_null() || size < 0 {
        return -1;
    }
    let Ok(requested) = usize::try_from(size) else {
        emit_error_message(tif, module, "Requested strip size is too large");
        return -1;
    };
    match read_strile_bytes(tif, module, strip, requested, buf) {
        Some(read_size) => read_size as Tmsize,
        None => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFReadTile(
    tif: *mut TIFF,
    buf: *mut c_void,
    x: u32,
    y: u32,
    z: u32,
    sample: u16,
) -> Tmsize {
    unsafe {
        if !check_tile_internal(tif, x, y, z, sample) {
            return -1;
        }
        let tile = TIFFComputeTile(tif, x, y, z, sample);
        TIFFReadEncodedTile(tif, tile, buf, TIFFTileSize(tif))
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFReadEncodedTile(
    tif: *mut TIFF,
    tile: u32,
    buf: *mut c_void,
    size: Tmsize,
) -> Tmsize {
    unsafe {
        let module = "TIFFReadEncodedTile";
        if !check_read_mode(tif, true, module) || buf.is_null() {
            return -1;
        }
        let Some(geometry) = codec_geometry_for_tile(tif) else {
            emit_error_message(tif, module, "Failed to compute tile geometry");
            return -1;
        };
        let Some(tile_size) = expected_tile_size(tif) else {
            emit_error_message(tif, module, "Tile decode size is too large");
            return -1;
        };
        let requested = if size == -1 {
            tile_size as u64
        } else if size < 0 {
            return -1;
        } else {
            min(tile_size as u64, size as u64)
        };
        let Ok(requested_usize) = usize::try_from(requested) else {
            emit_error_message(tif, module, "Requested tile size is too large");
            return -1;
        };
        let out = slice::from_raw_parts_mut(buf.cast::<u8>(), requested_usize);
        match read_encoded_strile_bytes(tif, module, true, tile, geometry, tile_size, out) {
            Some(read_size) => read_size as Tmsize,
            None => -1,
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFReadRawTile(
    tif: *mut TIFF,
    tile: u32,
    buf: *mut c_void,
    size: Tmsize,
) -> Tmsize {
    let module = "TIFFReadRawTile";
    if !check_read_mode(tif, true, module) || buf.is_null() || size < 0 {
        return -1;
    }
    let Ok(requested) = usize::try_from(size) else {
        emit_error_message(tif, module, "Requested tile size is too large");
        return -1;
    };
    match read_strile_bytes(tif, module, tile, requested, buf) {
        Some(read_size) => read_size as Tmsize,
        None => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFDeferStrileArrayWriting(tif: *mut TIFF) -> c_int {
    unsafe {
        if tif.is_null() || crate::TIFFGetMode(tif) == libc::O_RDONLY {
            if !tif.is_null() {
                emit_error_message(
                    tif,
                    "TIFFDeferStrileArrayWriting",
                    "File opened in read-only mode",
                );
            }
            return 0;
        }
        if crate::TIFFCurrentDirOffset(tif) != 0 {
            emit_error_message(
                tif,
                "TIFFDeferStrileArrayWriting",
                "Directory has already been written",
            );
            return 0;
        }
        (*tif_inner(tif)).strile_state.defer_array_writing = true;
        1
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFForceStrileArrayWriting(tif: *mut TIFF) -> c_int {
    unsafe {
        let module = "TIFFForceStrileArrayWriting";
        if tif.is_null() || crate::TIFFGetMode(tif) == libc::O_RDONLY {
            if !tif.is_null() {
                emit_error_message(tif, module, "File opened in read-only mode");
            }
            return 0;
        }
        let dir_offset = crate::TIFFCurrentDirOffset(tif);
        if dir_offset == 0 {
            emit_error_message(tif, module, "Directory has not yet been written");
            return 0;
        }
        if ((*tif).tif_flags & TIFF_DIRTYDIRECT) != 0 {
            emit_error_message(
            tif,
            module,
            "Directory has changes other than the strile arrays. TIFFRewriteDirectory() should be called instead",
        );
            return 0;
        }
        if TIFFSetupStrips(tif) == 0 {
            return 0;
        }
        let (offset_tag, bytecount_tag) = if is_tiled_image(tif) {
            (TAG_TILEOFFSETS as u16, TAG_TILEBYTECOUNTS as u16)
        } else {
            (TAG_STRIPOFFSETS as u16, TAG_STRIPBYTECOUNTS as u16)
        };
        if ((*tif).tif_flags & TIFF_DIRTYSTRIP) == 0
            && !(safe_tiff_directory_entry_is_dummy(tif, dir_offset, offset_tag)
                && safe_tiff_directory_entry_is_dummy(tif, dir_offset, bytecount_tag))
        {
            emit_error_message(
                tif,
                module,
                "Function not called together with TIFFDeferStrileArrayWriting()",
            );
            return 0;
        }

        let Some(arrays) = read_strile_arrays(tif) else {
            emit_error_message(tif, module, "Failed to initialize strile arrays");
            return 0;
        };
        let Ok(count) = isize::try_from(arrays.offsets.len()) else {
            emit_error_message(tif, module, "Strile array is too large to rewrite safely");
            return 0;
        };
        let offsets_ptr = if arrays.offsets.is_empty() {
            ptr::null_mut()
        } else {
            arrays.offsets.as_ptr().cast_mut().cast::<c_void>()
        };
        let bytecounts_ptr = if arrays.bytecounts.is_empty() {
            ptr::null_mut()
        } else {
            arrays.bytecounts.as_ptr().cast_mut().cast::<c_void>()
        };

        if _TIFFRewriteField(
            tif,
            offset_tag,
            TIFFDataType::TIFF_LONG8,
            count,
            offsets_ptr,
        ) == 0
            || _TIFFRewriteField(
                tif,
                bytecount_tag,
                TIFFDataType::TIFF_LONG8,
                count,
                bytecounts_ptr,
            ) == 0
        {
            return 0;
        }

        (*tif).tif_flags &= !TIFF_DIRTYSTRIP;
        (*tif).tif_flags &= !TIFF_BEENWRITING;
        1
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFGetStrileOffset(tif: *mut TIFF, strile: u32) -> u64 {
    unsafe { TIFFGetStrileOffsetWithErr(tif, strile, ptr::null_mut()) }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFGetStrileByteCount(tif: *mut TIFF, strile: u32) -> u64 {
    unsafe { TIFFGetStrileByteCountWithErr(tif, strile, ptr::null_mut()) }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFGetStrileOffsetWithErr(
    tif: *mut TIFF,
    strile: u32,
    err: *mut c_int,
) -> u64 {
    let tag = if is_tiled_image(tif) {
        TAG_TILEOFFSETS
    } else {
        TAG_STRIPOFFSETS
    };
    get_strile_tag_value_u64(tif, tag, strile, err).unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn TIFFGetStrileByteCountWithErr(
    tif: *mut TIFF,
    strile: u32,
    err: *mut c_int,
) -> u64 {
    let tag = if is_tiled_image(tif) {
        TAG_TILEBYTECOUNTS
    } else {
        TAG_STRIPBYTECOUNTS
    };
    get_strile_tag_value_u64(tif, tag, strile, err).unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn TIFFReadFromUserBuffer(
    tif: *mut TIFF,
    strile: u32,
    inbuf: *mut c_void,
    insize: Tmsize,
    outbuf: *mut c_void,
    outsize: Tmsize,
) -> c_int {
    unsafe {
        let module = "TIFFReadFromUserBuffer";
        if tif.is_null() || inbuf.is_null() || outbuf.is_null() || insize < 0 || outsize < 0 {
            return 0;
        }
        if crate::TIFFGetMode(tif) == libc::O_WRONLY {
            emit_error_message(tif, module, "File not open for reading");
            return 0;
        }
        let is_tile = is_tiled_image(tif);
        let (geometry, expected_size) = if is_tile {
            let mut err = 0;
            let _ = TIFFGetStrileByteCountWithErr(tif, strile, &mut err);
            if err != 0 {
                emit_error_message(tif, module, "Strile index out of range");
                return 0;
            }
            let Some(geometry) = codec_geometry_for_tile(tif) else {
                emit_error_message(tif, module, "Failed to compute tile geometry");
                return 0;
            };
            let Some(expected_size) = expected_tile_size(tif) else {
                emit_error_message(tif, module, "Tile decode size is too large");
                return 0;
            };
            (geometry, expected_size as u64)
        } else {
            let Some(expected_size) = expected_strip_size_for_index(tif, strile) else {
                emit_error_message(tif, module, "Strile index out of range");
                return 0;
            };
            let Some(geometry) = codec_geometry_for_strip(tif, strile) else {
                emit_error_message(tif, module, "Failed to compute strip geometry");
                return 0;
            };
            (geometry, expected_size)
        };
        if outsize as u64 > expected_size {
            emit_error_message(tif, module, "Requested decode size is too large");
            return 0;
        }
        let Ok(input_size) = usize::try_from(insize) else {
            return 0;
        };
        let Ok(output_size) = usize::try_from(outsize) else {
            return 0;
        };
        let input = slice::from_raw_parts(inbuf.cast::<u8>(), input_size);
        let output = slice::from_raw_parts_mut(outbuf.cast::<u8>(), output_size);
        let Some(decoded) = safe_tiff_codec_decode_bytes(
            tif,
            input,
            is_tile,
            strile,
            geometry,
            expected_size as usize,
        ) else {
            emit_error_message(tif, module, "Codec decode failed");
            return 0;
        };
        if decoded.len() < output.len() {
            emit_error_message(tif, module, "Decoded strile is smaller than requested");
            return 0;
        }
        output.copy_from_slice(&decoded[..output.len()]);
        1
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFSetWriteOffset(tif: *mut TIFF, off: u64) {
    unsafe {
        if !tif.is_null() {
            (*tif_inner(tif)).strile_state.write_offset = off;
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFFlushData(tif: *mut TIFF) -> c_int {
    unsafe {
        if tif.is_null() {
            return 0;
        }
        if ((*tif).tif_flags & TIFF_BEENWRITING) == 0 {
            return 1;
        }
        if !(*tif_inner(tif)).codec_state.pending_striles.is_empty()
            && !flush_pending_codec_striles(tif, "TIFFFlushData")
        {
            return 0;
        }
        (*tif).tif_rawcc = 0;
        (*tif).tif_rawcp = (*tif).tif_rawdata;
        1
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFFlush(tif: *mut TIFF) -> c_int {
    unsafe {
        if tif.is_null() {
            return 0;
        }
        if crate::TIFFGetMode(tif) == libc::O_RDONLY {
            return 1;
        }
        if TIFFFlushData(tif) == 0 {
            return 0;
        }
        if ((*tif).tif_flags & TIFF_DIRTYSTRIP) != 0
            && ((*tif).tif_flags & TIFF_DIRTYDIRECT) == 0
            && crate::TIFFGetMode(tif) == libc::O_RDWR
        {
            if TIFFForceStrileArrayWriting(tif) != 0 {
                return 1;
            }
        }
        if ((*tif).tif_flags & (TIFF_DIRTYDIRECT | TIFF_DIRTYSTRIP)) != 0 {
            TIFFRewriteDirectory(tif)
        } else {
            1
        }
    }
}

#[no_mangle]
pub extern "C" fn TIFFGetBitRevTable(reversed: c_int) -> *const u8 {
    if reversed != 0 {
        TIFF_BIT_REV_TABLE.as_ptr()
    } else {
        TIFF_NO_BIT_REV_TABLE.as_ptr()
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFReverseBits(data: *mut u8, count: Tmsize) {
    unsafe {
        if data.is_null() || count <= 0 {
            return;
        }
        for byte in slice::from_raw_parts_mut(data, count as usize) {
            *byte = TIFF_BIT_REV_TABLE[*byte as usize];
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFSwabShort(value: *mut u16) {
    unsafe {
        if !value.is_null() {
            *value = (*value).swap_bytes();
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFSwabLong(value: *mut u32) {
    unsafe {
        if !value.is_null() {
            *value = (*value).swap_bytes();
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFSwabLong8(value: *mut u64) {
    unsafe {
        if !value.is_null() {
            *value = (*value).swap_bytes();
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFSwabArrayOfShort(values: *mut u16, count: Tmsize) {
    unsafe {
        if values.is_null() || count <= 0 {
            return;
        }
        for value in slice::from_raw_parts_mut(values, count as usize) {
            *value = value.swap_bytes();
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFSwabArrayOfTriples(values: *mut u8, count: Tmsize) {
    unsafe {
        if values.is_null() || count <= 0 {
            return;
        }
        for triple in slice::from_raw_parts_mut(values, count as usize * 3).chunks_exact_mut(3) {
            triple.swap(0, 2);
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFSwabArrayOfLong(values: *mut u32, count: Tmsize) {
    unsafe {
        if values.is_null() || count <= 0 {
            return;
        }
        for value in slice::from_raw_parts_mut(values, count as usize) {
            *value = value.swap_bytes();
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFSwabArrayOfLong8(values: *mut u64, count: Tmsize) {
    unsafe {
        if values.is_null() || count <= 0 {
            return;
        }
        for value in slice::from_raw_parts_mut(values, count as usize) {
            *value = value.swap_bytes();
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFSwabFloat(value: *mut f32) {
    unsafe {
        if !value.is_null() {
            *value = f32::from_bits((*value).to_bits().swap_bytes());
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFSwabArrayOfFloat(values: *mut f32, count: Tmsize) {
    unsafe {
        if values.is_null() || count <= 0 {
            return;
        }
        for value in slice::from_raw_parts_mut(values, count as usize) {
            *value = f32::from_bits(value.to_bits().swap_bytes());
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFSwabDouble(value: *mut f64) {
    unsafe {
        if !value.is_null() {
            *value = f64::from_bits((*value).to_bits().swap_bytes());
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFSwabArrayOfDouble(values: *mut f64, count: Tmsize) {
    unsafe {
        if values.is_null() || count <= 0 {
            return;
        }
        for value in slice::from_raw_parts_mut(values, count as usize) {
            *value = f64::from_bits(value.to_bits().swap_bytes());
        }
    }
}
