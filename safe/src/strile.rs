use crate::abi::TIFFDataType;
use crate::core::{get_tag_value, safe_tiff_set_field_marshaled, TIFFRewriteDirectory, TIFFWriteDirectory};
use crate::{
    emit_error_message, read_from_proc, seek_in_proc, tif_inner, write_to_proc, _TIFFcallocExt,
    _TIFFfree, _TIFFmallocExt, TIFF, Tmsize,
};
use libc::{c_int, c_void};
use std::cmp::{max, min};
use std::ptr;
use std::slice;

const COMPRESSION_NONE: u16 = 1;
const PHOTOMETRIC_YCBCR: u16 = 6;

const PLANARCONFIG_CONTIG: u16 = 1;
const PLANARCONFIG_SEPARATE: u16 = 2;

const TAG_IMAGEWIDTH: u32 = 256;
const TAG_IMAGELENGTH: u32 = 257;
const TAG_BITSPERSAMPLE: u32 = 258;
const TAG_COMPRESSION: u32 = 259;
const TAG_PHOTOMETRIC: u32 = 262;
const TAG_SAMPLESPERPIXEL: u32 = 277;
const TAG_ROWSPERSTRIP: u32 = 278;
const TAG_PLANARCONFIG: u32 = 284;
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

unsafe fn file_size(tif: *mut TIFF) -> u64 {
    let inner = tif_inner(tif);
    if !(*inner).mapped_base.is_null() && (*inner).mapped_size != 0 {
        (*inner).mapped_size
    } else if let Some(sizeproc) = (*tif).tif_sizeproc {
        sizeproc((*tif).tif_clientdata)
    } else {
        0
    }
}

unsafe fn read_exact_at(tif: *mut TIFF, offset: u64, bytes: &mut [u8]) -> bool {
    let Some(end) = offset.checked_add(bytes.len() as u64) else {
        return false;
    };
    let size = file_size(tif);
    if size != 0 && end > size {
        return false;
    }
    let inner = tif_inner(tif);
    if (*tif).tif_flags & TIFF_MAPPED != 0 && !(*inner).mapped_base.is_null() && end <= (*inner).mapped_size {
        ptr::copy_nonoverlapping(
            (*inner).mapped_base.cast::<u8>().add(offset as usize),
            bytes.as_mut_ptr(),
            bytes.len(),
        );
        true
    } else if seek_in_proc(tif, offset, libc::SEEK_SET) == offset {
        read_from_proc(tif, bytes.as_mut_ptr().cast::<c_void>(), bytes.len() as Tmsize)
    } else {
        false
    }
}

unsafe fn write_exact_at(tif: *mut TIFF, offset: u64, bytes: &[u8]) -> bool {
    if seek_in_proc(tif, offset, libc::SEEK_SET) != offset {
        return false;
    }
    if bytes.is_empty() {
        return true;
    }
    write_to_proc(tif, bytes.as_ptr().cast_mut().cast::<c_void>(), bytes.len() as Tmsize)
}

unsafe fn next_append_offset(tif: *mut TIFF) -> Option<u64> {
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

unsafe fn get_tag_raw(
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

unsafe fn get_tag_scalar_u16(tif: *mut TIFF, tag: u32, defaulted: bool) -> Option<u16> {
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

unsafe fn get_tag_scalar_u32(tif: *mut TIFF, tag: u32, defaulted: bool) -> Option<u32> {
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

unsafe fn copy_u16_array_tag(tif: *mut TIFF, tag: u32, defaulted: bool) -> Option<Vec<u16>> {
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

unsafe fn copy_u64_array_tag(tif: *mut TIFF, tag: u32, defaulted: bool) -> Option<Vec<u64>> {
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

unsafe fn set_u32_tag(tif: *mut TIFF, tag: u32, value: u32) -> bool {
    safe_tiff_set_field_marshaled(
        tif,
        tag,
        TIFFDataType::TIFF_LONG,
        1,
        ptr::from_ref(&value).cast::<c_void>(),
    ) != 0
}

unsafe fn set_u64_array_tag(tif: *mut TIFF, tag: u32, values: &[u64]) -> bool {
    safe_tiff_set_field_marshaled(
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

unsafe fn checked_add_u64(tif: *mut TIFF, module: &str, left: u64, right: u64) -> Option<u64> {
    left.checked_add(right).or_else(|| {
        emit_error_message(tif, module, "Integer overflow");
        None
    })
}

unsafe fn checked_mul_u64(tif: *mut TIFF, module: &str, left: u64, right: u64) -> Option<u64> {
    left.checked_mul(right).or_else(|| {
        emit_error_message(tif, module, "Integer overflow");
        None
    })
}

fn checked_howmany_u32(value: u32, divisor: u32) -> Option<u32> {
    if divisor == 0 {
        None
    } else {
        value
            .checked_add(divisor - 1)
            .map(|sum| sum / divisor)
    }
}

fn checked_howmany_u64(value: u64, divisor: u64) -> Option<u64> {
    if divisor == 0 {
        None
    } else {
        value.checked_add(divisor - 1).map(|sum| sum / divisor)
    }
}

unsafe fn cast_u64_to_tmsize(tif: *mut TIFF, module: &str, value: u64) -> Tmsize {
    if value > isize::MAX as u64 {
        emit_error_message(tif, module, "Integer overflow");
        0
    } else {
        value as Tmsize
    }
}

unsafe fn require_u32_tag(tif: *mut TIFF, tag: u32, module: &str, label: &str) -> Option<u32> {
    let Some(value) = get_tag_scalar_u32(tif, tag, false) else {
        emit_error_message(tif, module, format!("Must set \"{}\" before writing data", label));
        return None;
    };
    Some(value)
}

unsafe fn image_width(tif: *mut TIFF) -> Option<u32> {
    get_tag_scalar_u32(tif, TAG_IMAGEWIDTH, false)
}

unsafe fn image_length(tif: *mut TIFF) -> Option<u32> {
    get_tag_scalar_u32(tif, TAG_IMAGELENGTH, false)
}

unsafe fn image_depth(tif: *mut TIFF) -> u32 {
    get_tag_scalar_u32(tif, TAG_IMAGEDEPTH, true).unwrap_or(1)
}

unsafe fn bits_per_sample(tif: *mut TIFF) -> u16 {
    get_tag_scalar_u16(tif, TAG_BITSPERSAMPLE, true).unwrap_or(1)
}

unsafe fn samples_per_pixel(tif: *mut TIFF) -> u16 {
    get_tag_scalar_u16(tif, TAG_SAMPLESPERPIXEL, true).unwrap_or(1)
}

unsafe fn rows_per_strip(tif: *mut TIFF) -> u32 {
    get_tag_scalar_u32(tif, TAG_ROWSPERSTRIP, true).unwrap_or(u32::MAX)
}

unsafe fn planar_config(tif: *mut TIFF) -> u16 {
    get_tag_scalar_u16(tif, TAG_PLANARCONFIG, true).unwrap_or(PLANARCONFIG_CONTIG)
}

unsafe fn tile_width(tif: *mut TIFF) -> Option<u32> {
    get_tag_scalar_u32(tif, TAG_TILEWIDTH, false)
}

unsafe fn tile_length(tif: *mut TIFF) -> Option<u32> {
    get_tag_scalar_u32(tif, TAG_TILELENGTH, false)
}

unsafe fn tile_depth(tif: *mut TIFF) -> u32 {
    get_tag_scalar_u32(tif, TAG_TILEDEPTH, true).unwrap_or(1)
}

unsafe fn photometric(tif: *mut TIFF) -> u16 {
    get_tag_scalar_u16(tif, TAG_PHOTOMETRIC, true).unwrap_or(0)
}

unsafe fn compression(tif: *mut TIFF) -> u16 {
    get_tag_scalar_u16(tif, TAG_COMPRESSION, true).unwrap_or(COMPRESSION_NONE)
}

unsafe fn ycbcr_subsampling(tif: *mut TIFF) -> Option<(u16, u16)> {
    let values = copy_u16_array_tag(tif, TAG_YCBCRSUBSAMPLING, true)?;
    if values.len() >= 2 {
        Some((values[0], values[1]))
    } else {
        None
    }
}

unsafe fn is_tiled_image(tif: *mut TIFF) -> bool {
    ((*tif).tif_flags & TIFF_ISTILED) != 0
}

unsafe fn scanline_size64_internal(tif: *mut TIFF, report_errors: bool) -> Option<u64> {
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

unsafe fn raster_scanline_size64_internal(tif: *mut TIFF) -> Option<u64> {
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

unsafe fn strip_size_rows_internal(tif: *mut TIFF, strip: u32, report_errors: bool) -> Option<u32> {
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

unsafe fn vstrip_size64_internal(tif: *mut TIFF, nrows: u32, report_errors: bool) -> Option<u64> {
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
    checked_mul_u64(tif, module, u64::from(rows), scanline_size64_internal(tif, report_errors)?)
}

unsafe fn strip_size64_internal(tif: *mut TIFF) -> Option<u64> {
    let height = image_length(tif)?;
    let mut rps = rows_per_strip(tif);
    if rps == u32::MAX || rps > height {
        rps = height;
    }
    vstrip_size64_internal(tif, rps, true)
}

unsafe fn tile_row_size64_internal(tif: *mut TIFF) -> Option<u64> {
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

    let bits = checked_mul_u64(tif, module, u64::from(bits_per_sample(tif)), u64::from(width))?;
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

unsafe fn vtile_size64_internal(tif: *mut TIFF, nrows: u32) -> Option<u64> {
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
        checked_mul_u64(tif, module, u64::from(nrows), tile_row_size64_internal(tif)?)
    }
}

unsafe fn tile_size64_internal(tif: *mut TIFF) -> Option<u64> {
    vtile_size64_internal(tif, tile_length(tif)?)
}

unsafe fn number_of_strips_internal(tif: *mut TIFF) -> Option<u32> {
    let module = "TIFFNumberOfStrips";
    let height = image_length(tif)?;
    let rps = rows_per_strip(tif);
    let mut strips = if rps == u32::MAX {
        1
    } else {
        checked_howmany_u32(height, rps)?
    };
    if planar_config(tif) == PLANARCONFIG_SEPARATE {
        strips = strips.checked_mul(u32::from(samples_per_pixel(tif))).or_else(|| {
            emit_error_message(tif, module, "Integer overflow");
            None
        })?;
    }
    Some(strips)
}

unsafe fn number_of_tiles_internal(tif: *mut TIFF) -> Option<u32> {
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
        tiles = tiles.checked_mul(u32::from(samples_per_pixel(tif))).or_else(|| {
            emit_error_message(tif, module, "Integer overflow");
            None
        })?;
    }
    Some(tiles)
}

unsafe fn compute_strip_internal(tif: *mut TIFF, row: u32, sample: u16, report_errors: bool) -> Option<u32> {
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
        strip = strip.checked_add(u32::from(sample).checked_mul(strips_per_image)?).or_else(|| {
            if report_errors {
                emit_error_message(tif, module, "Integer overflow");
            }
            None
        })?;
    }
    Some(strip)
}

unsafe fn check_tile_internal(
    tif: *mut TIFF,
    x: u32,
    y: u32,
    z: u32,
    sample: u16,
) -> bool {
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

unsafe fn compute_tile_internal(
    tif: *mut TIFF,
    x: u32,
    y: u32,
    mut z: u32,
    sample: u16,
) -> Option<u32> {
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

unsafe fn expected_strile_count(tif: *mut TIFF) -> Option<usize> {
    let count = if is_tiled_image(tif) {
        number_of_tiles_internal(tif)?
    } else {
        number_of_strips_internal(tif)?
    };
    usize::try_from(count).ok()
}

unsafe fn read_strile_arrays(tif: *mut TIFF) -> Option<StrileArrays> {
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

unsafe fn write_strile_arrays(tif: *mut TIFF, arrays: &StrileArrays) -> bool {
    set_u64_array_tag(tif, arrays.offset_tag, &arrays.offsets)
        && set_u64_array_tag(tif, arrays.bytecount_tag, &arrays.bytecounts)
}

unsafe fn ensure_strile_arrays(tif: *mut TIFF, module: &str) -> Option<StrileArrays> {
    let arrays = read_strile_arrays(tif)?;
    if write_strile_arrays(tif, &arrays) {
        Some(arrays)
    } else {
        emit_error_message(tif, module, "Failed to synchronize strile arrays");
        None
    }
}

unsafe fn strile_bounds(tif: *mut TIFF, strile: u32) -> Option<(StrileArrays, usize)> {
    let arrays = read_strile_arrays(tif)?;
    let index = usize::try_from(strile).ok()?;
    if index >= arrays.offsets.len() {
        None
    } else {
        Some((arrays, index))
    }
}

unsafe fn write_appended_strile_data(tif: *mut TIFF, module: &str, strile: u32, data: &[u8]) -> bool {
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
    let Some(write_offset) = checked_add_u64(tif, module, offset, arrays.bytecounts[index]) else {
        return false;
    };
    if !write_exact_at(tif, write_offset, data) {
        emit_error_message(tif, module, "Failed to write strile payload");
        return false;
    }
    let Some(new_bytecount) = checked_add_u64(tif, module, arrays.bytecounts[index], data.len() as u64) else {
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

unsafe fn write_overwrite_strile_data(tif: *mut TIFF, module: &str, strile: u32, data: &[u8]) -> bool {
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
    let reuse_existing = arrays.offsets[index] != 0 && arrays.bytecounts[index] >= data.len() as u64;
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

unsafe fn write_scanline_data(
    tif: *mut TIFF,
    module: &str,
    row: u32,
    sample: u16,
    data: &[u8],
) -> bool {
    let Some(mut arrays) = ensure_strile_arrays(tif, module) else {
        return false;
    };
    let Some(strip) = compute_strip_internal(tif, row, sample, true) else {
        return false;
    };
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
    let Some(within_strip) = checked_mul_u64(tif, module, u64::from(strip_row), data.len() as u64) else {
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

unsafe fn read_strile_bytes(tif: *mut TIFF, module: &str, strile: u32, size: usize, buf: *mut c_void) -> Option<usize> {
    let (arrays, index) = strile_bounds(tif, strile)?;
    let offset = arrays.offsets[index];
    let bytecount = arrays.bytecounts[index];
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

unsafe fn expected_strip_size_for_index(tif: *mut TIFF, strip: u32) -> Option<u64> {
    vstrip_size64_internal(tif, strip_size_rows_internal(tif, strip, true)?, true)
}

unsafe fn read_scanline_bytes(tif: *mut TIFF, row: u32, sample: u16, out: &mut [u8]) -> bool {
    let module = "TIFFReadScanline";
    let Some(height) = image_length(tif) else {
        emit_error_message(tif, module, "Missing image length");
        return false;
    };
    if row >= height {
        emit_error_message(tif, module, "Row out of range");
        return false;
    }
    let Some(strip) = compute_strip_internal(tif, row, sample, true) else {
        return false;
    };
    let Some((arrays, index)) = strile_bounds(tif, strip) else {
        emit_error_message(tif, module, "Strip out of range");
        return false;
    };
    if arrays.offsets[index] == 0 || arrays.bytecounts[index] == 0 {
        emit_error_message(tif, module, "Strip byte count is zero");
        return false;
    }
    let rps = rows_per_strip(tif);
    let strip_row = if rps == u32::MAX { row } else { row % rps };
    let Some(within_strip) = checked_mul_u64(tif, module, u64::from(strip_row), out.len() as u64) else {
        return false;
    };
    let Some(end_offset) = checked_add_u64(tif, module, within_strip, out.len() as u64) else {
        return false;
    };
    if end_offset > arrays.bytecounts[index] {
        emit_error_message(tif, module, "Scanline extends beyond the strip byte count");
        return false;
    }
    let Some(offset) = checked_add_u64(tif, module, arrays.offsets[index], within_strip) else {
        return false;
    };
    if !read_exact_at(tif, offset, out) {
        emit_error_message(tif, module, "Failed to read scanline data");
        return false;
    }
    (*tif_inner(tif)).tif_curstrip = strip;
    (*tif).tif_row = row + 1;
    true
}

unsafe fn check_read_mode(tif: *mut TIFF, tiles: bool, module: &str) -> bool {
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

unsafe fn check_write_mode(tif: *mut TIFF, tiles: bool, module: &str) -> bool {
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
    if compression(tif) != COMPRESSION_NONE {
        emit_error_message(tif, module, "Only COMPRESSION_NONE is implemented in the safe port");
        return false;
    }
    true
}

unsafe fn free_raw_buffer_if_owned(tif: *mut TIFF) {
    if !(*tif).tif_rawdata.is_null() && ((*tif).tif_flags & TIFF_MYBUFFER) != 0 {
        _TIFFfree((*tif).tif_rawdata.cast::<c_void>());
    }
    (*tif).tif_rawdata = ptr::null_mut();
    (*tif).tif_rawdatasize = 0;
    (*tif).tif_rawcp = ptr::null_mut();
    (*tif).tif_rawcc = 0;
    (*tif).tif_flags &= !(TIFF_MYBUFFER | TIFF_BUFFERMMAP);
}

#[no_mangle]
pub unsafe extern "C" fn TIFFScanlineSize64(tif: *mut TIFF) -> u64 {
    scanline_size64_internal(tif, true).unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn TIFFScanlineSize(tif: *mut TIFF) -> Tmsize {
    cast_u64_to_tmsize(tif, "TIFFScanlineSize", TIFFScanlineSize64(tif))
}

#[no_mangle]
pub unsafe extern "C" fn TIFFRasterScanlineSize64(tif: *mut TIFF) -> u64 {
    raster_scanline_size64_internal(tif).unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn TIFFRasterScanlineSize(tif: *mut TIFF) -> Tmsize {
    cast_u64_to_tmsize(
        tif,
        "TIFFRasterScanlineSize",
        TIFFRasterScanlineSize64(tif),
    )
}

#[no_mangle]
pub unsafe extern "C" fn TIFFVStripSize64(tif: *mut TIFF, nrows: u32) -> u64 {
    vstrip_size64_internal(tif, nrows, true).unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn TIFFVStripSize(tif: *mut TIFF, nrows: u32) -> Tmsize {
    cast_u64_to_tmsize(tif, "TIFFVStripSize", TIFFVStripSize64(tif, nrows))
}

#[no_mangle]
pub unsafe extern "C" fn TIFFStripSize64(tif: *mut TIFF) -> u64 {
    strip_size64_internal(tif).unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn TIFFStripSize(tif: *mut TIFF) -> Tmsize {
    cast_u64_to_tmsize(tif, "TIFFStripSize", TIFFStripSize64(tif))
}

#[no_mangle]
pub unsafe extern "C" fn TIFFRawStripSize64(tif: *mut TIFF, strip: u32) -> u64 {
    TIFFGetStrileByteCount(tif, strip)
}

#[no_mangle]
pub unsafe extern "C" fn TIFFRawStripSize(tif: *mut TIFF, strip: u32) -> Tmsize {
    cast_u64_to_tmsize(tif, "TIFFRawStripSize", TIFFRawStripSize64(tif, strip))
}

#[no_mangle]
pub unsafe extern "C" fn TIFFTileRowSize64(tif: *mut TIFF) -> u64 {
    tile_row_size64_internal(tif).unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn TIFFTileRowSize(tif: *mut TIFF) -> Tmsize {
    cast_u64_to_tmsize(tif, "TIFFTileRowSize", TIFFTileRowSize64(tif))
}

#[no_mangle]
pub unsafe extern "C" fn TIFFVTileSize64(tif: *mut TIFF, nrows: u32) -> u64 {
    vtile_size64_internal(tif, nrows).unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn TIFFVTileSize(tif: *mut TIFF, nrows: u32) -> Tmsize {
    cast_u64_to_tmsize(tif, "TIFFVTileSize", TIFFVTileSize64(tif, nrows))
}

#[no_mangle]
pub unsafe extern "C" fn TIFFTileSize64(tif: *mut TIFF) -> u64 {
    tile_size64_internal(tif).unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn TIFFTileSize(tif: *mut TIFF) -> Tmsize {
    cast_u64_to_tmsize(tif, "TIFFTileSize", TIFFTileSize64(tif))
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
pub unsafe extern "C" fn TIFFReadBufferSetup(tif: *mut TIFF, bp: *mut c_void, size: Tmsize) -> c_int {
    if tif.is_null() {
        return 0;
    }
    free_raw_buffer_if_owned(tif);
    (*tif).tif_flags &= !TIFF_BUFFERMMAP;
    let alloc_size = if size <= 0 { 8192 } else { ((size + 1023) / 1024) * 1024 };
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

#[no_mangle]
pub unsafe extern "C" fn TIFFWriteBufferSetup(tif: *mut TIFF, bp: *mut c_void, size: Tmsize) -> c_int {
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
    let module_name = if module.is_null() {
        "TIFFWriteCheck"
    } else {
        std::ffi::CStr::from_ptr(module).to_str().unwrap_or("TIFFWriteCheck")
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

#[no_mangle]
pub unsafe extern "C" fn TIFFWriteScanline(
    tif: *mut TIFF,
    buf: *mut c_void,
    row: u32,
    sample: u16,
) -> c_int {
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
    if write_scanline_data(tif, module, row, sample, data) {
        1
    } else {
        -1
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFWriteEncodedStrip(
    tif: *mut TIFF,
    strip: u32,
    data: *mut c_void,
    cc: Tmsize,
) -> Tmsize {
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
    if write_overwrite_strile_data(tif, module, strip, bytes) {
        (*tif_inner(tif)).tif_curstrip = strip;
        cc
    } else {
        -1
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFWriteRawStrip(
    tif: *mut TIFF,
    strip: u32,
    data: *mut c_void,
    cc: Tmsize,
) -> Tmsize {
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

#[no_mangle]
pub unsafe extern "C" fn TIFFWriteTile(
    tif: *mut TIFF,
    buf: *mut c_void,
    x: u32,
    y: u32,
    z: u32,
    sample: u16,
) -> Tmsize {
    if !check_tile_internal(tif, x, y, z, sample) {
        return -1;
    }
    let tile = TIFFComputeTile(tif, x, y, z, sample);
    TIFFWriteEncodedTile(tif, tile, buf, TIFFTileSize(tif))
}

#[no_mangle]
pub unsafe extern "C" fn TIFFWriteEncodedTile(
    tif: *mut TIFF,
    tile: u32,
    data: *mut c_void,
    cc: Tmsize,
) -> Tmsize {
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
    if write_overwrite_strile_data(tif, module, tile, bytes) {
        (*tif_inner(tif)).tif_curtile = tile;
        size as Tmsize
    } else {
        -1
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFWriteRawTile(
    tif: *mut TIFF,
    tile: u32,
    data: *mut c_void,
    cc: Tmsize,
) -> Tmsize {
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

#[no_mangle]
pub unsafe extern "C" fn TIFFReadScanline(
    tif: *mut TIFF,
    buf: *mut c_void,
    row: u32,
    sample: u16,
) -> c_int {
    if !check_read_mode(tif, false, "TIFFReadScanline") || buf.is_null() {
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

#[no_mangle]
pub unsafe extern "C" fn TIFFReadEncodedStrip(
    tif: *mut TIFF,
    strip: u32,
    buf: *mut c_void,
    size: Tmsize,
) -> Tmsize {
    let module = "TIFFReadEncodedStrip";
    if !check_read_mode(tif, false, module) || buf.is_null() {
        return -1;
    }
    if compression(tif) != COMPRESSION_NONE {
        emit_error_message(tif, module, "Only COMPRESSION_NONE is implemented in the safe port");
        return -1;
    }
    let Some(expected_size) = expected_strip_size_for_index(tif, strip) else {
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
    match read_strile_bytes(tif, module, strip, requested_usize, buf) {
        Some(read_size) => read_size as Tmsize,
        None => -1,
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
    if !check_tile_internal(tif, x, y, z, sample) {
        return -1;
    }
    let tile = TIFFComputeTile(tif, x, y, z, sample);
    TIFFReadEncodedTile(tif, tile, buf, TIFFTileSize(tif))
}

#[no_mangle]
pub unsafe extern "C" fn TIFFReadEncodedTile(
    tif: *mut TIFF,
    tile: u32,
    buf: *mut c_void,
    size: Tmsize,
) -> Tmsize {
    let module = "TIFFReadEncodedTile";
    if !check_read_mode(tif, true, module) || buf.is_null() {
        return -1;
    }
    if compression(tif) != COMPRESSION_NONE {
        emit_error_message(tif, module, "Only COMPRESSION_NONE is implemented in the safe port");
        return -1;
    }
    let tile_size = TIFFTileSize64(tif);
    if tile_size == 0 {
        return -1;
    }
    let requested = if size == -1 {
        tile_size
    } else if size < 0 {
        return -1;
    } else {
        min(tile_size, size as u64)
    };
    let Ok(requested_usize) = usize::try_from(requested) else {
        emit_error_message(tif, module, "Requested tile size is too large");
        return -1;
    };
    match read_strile_bytes(tif, module, tile, requested_usize, buf) {
        Some(read_size) => read_size as Tmsize,
        None => -1,
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
    if tif.is_null() || crate::TIFFGetMode(tif) == libc::O_RDONLY {
        if !tif.is_null() {
            emit_error_message(tif, "TIFFDeferStrileArrayWriting", "File opened in read-only mode");
        }
        return 0;
    }
    (*tif_inner(tif)).strile_state.defer_array_writing = true;
    TIFFSetupStrips(tif)
}

#[no_mangle]
pub unsafe extern "C" fn TIFFForceStrileArrayWriting(tif: *mut TIFF) -> c_int {
    if tif.is_null() || crate::TIFFGetMode(tif) == libc::O_RDONLY {
        if !tif.is_null() {
            emit_error_message(tif, "TIFFForceStrileArrayWriting", "File opened in read-only mode");
        }
        return 0;
    }
    if crate::TIFFCurrentDirOffset(tif) == 0 {
        emit_error_message(tif, "TIFFForceStrileArrayWriting", "Directory has not yet been written");
        return 0;
    }
    if TIFFSetupStrips(tif) == 0 {
        return 0;
    }
    (*tif_inner(tif)).strile_state.defer_array_writing = false;
    TIFFRewriteDirectory(tif)
}

#[no_mangle]
pub unsafe extern "C" fn TIFFGetStrileOffset(tif: *mut TIFF, strile: u32) -> u64 {
    TIFFGetStrileOffsetWithErr(tif, strile, ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn TIFFGetStrileByteCount(tif: *mut TIFF, strile: u32) -> u64 {
    TIFFGetStrileByteCountWithErr(tif, strile, ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn TIFFGetStrileOffsetWithErr(
    tif: *mut TIFF,
    strile: u32,
    err: *mut c_int,
) -> u64 {
    if !err.is_null() {
        *err = 0;
    }
    let Some((arrays, index)) = strile_bounds(tif, strile) else {
        if !err.is_null() {
            *err = 1;
        }
        return 0;
    };
    arrays.offsets[index]
}

#[no_mangle]
pub unsafe extern "C" fn TIFFGetStrileByteCountWithErr(
    tif: *mut TIFF,
    strile: u32,
    err: *mut c_int,
) -> u64 {
    if !err.is_null() {
        *err = 0;
    }
    let Some((arrays, index)) = strile_bounds(tif, strile) else {
        if !err.is_null() {
            *err = 1;
        }
        return 0;
    };
    arrays.bytecounts[index]
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
    let module = "TIFFReadFromUserBuffer";
    if tif.is_null() || inbuf.is_null() || outbuf.is_null() || insize < 0 || outsize < 0 {
        return 0;
    }
    if crate::TIFFGetMode(tif) == libc::O_WRONLY {
        emit_error_message(tif, module, "File not open for reading");
        return 0;
    }
    if compression(tif) != COMPRESSION_NONE {
        emit_error_message(tif, module, "Only COMPRESSION_NONE is implemented in the safe port");
        return 0;
    }
    let expected_size = if is_tiled_image(tif) {
        TIFFTileSize64(tif)
    } else {
        expected_strip_size_for_index(tif, strile).unwrap_or(0)
    };
    let copy_size = min(expected_size, insize as u64);
    if copy_size > outsize as u64 {
        emit_error_message(tif, module, "Output buffer is too small");
        return 0;
    }
    ptr::copy_nonoverlapping(inbuf.cast::<u8>(), outbuf.cast::<u8>(), copy_size as usize);
    1
}

#[no_mangle]
pub unsafe extern "C" fn TIFFSetWriteOffset(tif: *mut TIFF, off: u64) {
    if !tif.is_null() {
        (*tif_inner(tif)).strile_state.write_offset = off;
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFFlushData(tif: *mut TIFF) -> c_int {
    if tif.is_null() {
        return 0;
    }
    if ((*tif).tif_flags & TIFF_BEENWRITING) == 0 {
        return 1;
    }
    (*tif).tif_rawcc = 0;
    (*tif).tif_rawcp = (*tif).tif_rawdata;
    1
}

#[no_mangle]
pub unsafe extern "C" fn TIFFFlush(tif: *mut TIFF) -> c_int {
    if tif.is_null() {
        return 0;
    }
    if crate::TIFFGetMode(tif) == libc::O_RDONLY {
        return 1;
    }
    if TIFFFlushData(tif) == 0 {
        return 0;
    }
    if ((*tif).tif_flags & (TIFF_DIRTYDIRECT | TIFF_DIRTYSTRIP)) != 0 {
        if crate::TIFFCurrentDirOffset(tif) == 0 {
            TIFFWriteDirectory(tif)
        } else {
            TIFFRewriteDirectory(tif)
        }
    } else {
        1
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
    if data.is_null() || count <= 0 {
        return;
    }
    for byte in slice::from_raw_parts_mut(data, count as usize) {
        *byte = TIFF_BIT_REV_TABLE[*byte as usize];
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFSwabShort(value: *mut u16) {
    if !value.is_null() {
        *value = (*value).swap_bytes();
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFSwabLong(value: *mut u32) {
    if !value.is_null() {
        *value = (*value).swap_bytes();
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFSwabLong8(value: *mut u64) {
    if !value.is_null() {
        *value = (*value).swap_bytes();
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFSwabArrayOfShort(values: *mut u16, count: Tmsize) {
    if values.is_null() || count <= 0 {
        return;
    }
    for value in slice::from_raw_parts_mut(values, count as usize) {
        *value = value.swap_bytes();
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFSwabArrayOfTriples(values: *mut u8, count: Tmsize) {
    if values.is_null() || count <= 0 {
        return;
    }
    for triple in slice::from_raw_parts_mut(values, count as usize * 3).chunks_exact_mut(3) {
        triple.swap(0, 2);
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFSwabArrayOfLong(values: *mut u32, count: Tmsize) {
    if values.is_null() || count <= 0 {
        return;
    }
    for value in slice::from_raw_parts_mut(values, count as usize) {
        *value = value.swap_bytes();
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFSwabArrayOfLong8(values: *mut u64, count: Tmsize) {
    if values.is_null() || count <= 0 {
        return;
    }
    for value in slice::from_raw_parts_mut(values, count as usize) {
        *value = value.swap_bytes();
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFSwabFloat(value: *mut f32) {
    if !value.is_null() {
        *value = f32::from_bits((*value).to_bits().swap_bytes());
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFSwabArrayOfFloat(values: *mut f32, count: Tmsize) {
    if values.is_null() || count <= 0 {
        return;
    }
    for value in slice::from_raw_parts_mut(values, count as usize) {
        *value = f32::from_bits(value.to_bits().swap_bytes());
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFSwabDouble(value: *mut f64) {
    if !value.is_null() {
        *value = f64::from_bits((*value).to_bits().swap_bytes());
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFSwabArrayOfDouble(values: *mut f64, count: Tmsize) {
    if values.is_null() || count <= 0 {
        return;
    }
    for value in slice::from_raw_parts_mut(values, count as usize) {
        *value = f64::from_bits(value.to_bits().swap_bytes());
    }
}
