use crate::abi::{TIFFCIELabToRGB, TIFFDisplay, TIFFRGBAImage};
use crate::core::{
    get_tag_value, jpeg_color_mode, ojpeg_decode_full_rgb_image,
    safe_tiff_cielab_to_rgb_init, safe_tiff_logl10_to_y, safe_tiff_logl16_to_y,
    safe_tiff_logluv24_to_xyz, safe_tiff_logluv32_to_xyz, safe_tiff_xyz_to_rgb24,
    set_jpeg_color_mode, sgilog24_decode_row, sgilog32_decode_row, COMPRESSION_JPEG,
    COMPRESSION_OJPEG, JPEGCOLORMODE_RGB,
};
use crate::strile::{
    TIFFComputeStrip, TIFFGetStrileByteCount, TIFFNumberOfStrips, TIFFReadRawStrip,
    TIFFReadRawTile, TIFFReadScanline, TIFFReadTile, TIFFRawStripSize, TIFFScanlineSize,
    TIFFTileSize,
};
use crate::{emit_error_message, TIFF, TIFFIsTiled};
use libc::{c_char, c_int, c_void};
use std::cmp::min;
use std::ptr;
use std::slice;

const ORIENTATION_TOPLEFT: u16 = 1;
const ORIENTATION_BOTLEFT: u16 = 4;

const PHOTOMETRIC_MINISWHITE: u16 = 0;
const PHOTOMETRIC_MINISBLACK: u16 = 1;
const PHOTOMETRIC_RGB: u16 = 2;
const PHOTOMETRIC_PALETTE: u16 = 3;
const PHOTOMETRIC_YCBCR: u16 = 6;
const PHOTOMETRIC_CIELAB: u16 = 8;
const PHOTOMETRIC_LOGL: u16 = 32844;
const PHOTOMETRIC_LOGLUV: u16 = 32845;

const COMPRESSION_SGILOG: u16 = 34676;
const COMPRESSION_SGILOG24: u16 = 34677;

const TAG_IMAGEWIDTH: u32 = 256;
const TAG_IMAGELENGTH: u32 = 257;
const TAG_BITSPERSAMPLE: u32 = 258;
const TAG_COMPRESSION: u32 = 259;
const TAG_PHOTOMETRIC: u32 = 262;
const TAG_SAMPLESPERPIXEL: u32 = 277;
const TAG_ROWSPERSTRIP: u32 = 278;
const TAG_PLANARCONFIG: u32 = 284;
const TAG_ORIENTATION: u32 = 274;
const TAG_COLORMAP: u32 = 320;
const TAG_EXTRASAMPLES: u32 = 338;
const TAG_TILEWIDTH: u32 = 322;
const TAG_TILELENGTH: u32 = 323;

const PLANARCONFIG_CONTIG: u16 = 1;
const PLANARCONFIG_SEPARATE: u16 = 2;

const DEFAULT_DISPLAY: TIFFDisplay = TIFFDisplay {
    d_mat: [
        [3.2410, -1.5374, -0.4986],
        [-0.9692, 1.8760, 0.0416],
        [0.0556, -0.2040, 1.0570],
    ],
    d_YCR: 100.0,
    d_YCG: 100.0,
    d_YCB: 100.0,
    d_Vrwr: 255,
    d_Vrwg: 255,
    d_Vrwb: 255,
    d_Y0R: 0.0,
    d_Y0G: 0.0,
    d_Y0B: 0.0,
    d_gammaR: 2.2,
    d_gammaG: 2.2,
    d_gammaB: 2.2,
};

const D65_WHITE: [f32; 3] = [95.0470, 100.0, 108.8827];

unsafe fn get_tag_raw(
    tif: *mut TIFF,
    tag: u32,
    defaulted: bool,
) -> Option<(crate::abi::TIFFDataType, usize, *const c_void)> {
    let mut type_ = crate::abi::TIFFDataType::TIFF_NOTYPE;
    let mut count = 0u64;
    let mut data: *const c_void = ptr::null();
    if get_tag_value(tif, tag, defaulted, &mut type_, &mut count, &mut data) == 0 {
        return None;
    }
    Some((type_, usize::try_from(count).ok()?, data))
}

unsafe fn tag_u16(tif: *mut TIFF, tag: u32, defaulted: bool, fallback: u16) -> u16 {
    let Some((type_, count, data)) = get_tag_raw(tif, tag, defaulted) else {
        return fallback;
    };
    if count == 0 || data.is_null() {
        return fallback;
    }
    match type_.0 {
        x if x == crate::abi::TIFFDataType::TIFF_SHORT.0 => *data.cast::<u16>(),
        x if x == crate::abi::TIFFDataType::TIFF_LONG.0 => {
            u16::try_from(*data.cast::<u32>()).unwrap_or(fallback)
        }
        x if x == crate::abi::TIFFDataType::TIFF_SLONG.0 => {
            u16::try_from(*data.cast::<i32>()).unwrap_or(fallback)
        }
        _ => fallback,
    }
}

unsafe fn tag_u32(tif: *mut TIFF, tag: u32, defaulted: bool, fallback: u32) -> u32 {
    let Some((type_, count, data)) = get_tag_raw(tif, tag, defaulted) else {
        return fallback;
    };
    if count == 0 || data.is_null() {
        return fallback;
    }
    match type_.0 {
        x if x == crate::abi::TIFFDataType::TIFF_SHORT.0 => u32::from(*data.cast::<u16>()),
        x if x == crate::abi::TIFFDataType::TIFF_LONG.0 => *data.cast::<u32>(),
        x if x == crate::abi::TIFFDataType::TIFF_LONG8.0
            || x == crate::abi::TIFFDataType::TIFF_IFD8.0 =>
        {
            u32::try_from(*data.cast::<u64>()).unwrap_or(fallback)
        }
        x if x == crate::abi::TIFFDataType::TIFF_SLONG.0 => {
            u32::try_from(*data.cast::<i32>()).unwrap_or(fallback)
        }
        _ => fallback,
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
        x if x == crate::abi::TIFFDataType::TIFF_SHORT.0 => {
            Some(slice::from_raw_parts(data.cast::<u16>(), count).to_vec())
        }
        x if x == crate::abi::TIFFDataType::TIFF_BYTE.0 => Some(
            slice::from_raw_parts(data.cast::<u8>(), count)
                .iter()
                .map(|value| u16::from(*value))
                .collect(),
        ),
        _ => None,
    }
}

fn set_error(emsg: *mut c_char, message: &str) {
    if emsg.is_null() {
        return;
    }
    let bytes = message.as_bytes();
    let count = min(bytes.len(), 1023);
    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr().cast::<c_char>(), emsg, count);
        *emsg.add(count) = 0;
    }
}

fn scale_sample_to_u8(sample: u16, bits: u16) -> u8 {
    match bits {
        0 => 0,
        1..=7 => {
            let max_value = (1u32 << bits) - 1;
            ((u32::from(sample) * 255 + max_value / 2) / max_value) as u8
        }
        8 => sample as u8,
        16 => (sample >> 8) as u8,
        _ => ((u32::from(sample) * 255) / ((1u32 << bits.min(16)) - 1)) as u8,
    }
}

fn palette_entry_to_u8(value: u16) -> u8 {
    if value > 255 { (value >> 8) as u8 } else { value as u8 }
}

fn pack_rgba(r: u8, g: u8, b: u8, a: u8) -> u32 {
    u32::from(r) | (u32::from(g) << 8) | (u32::from(b) << 16) | (u32::from(a) << 24)
}

fn bit_sample_contig(bytes: &[u8], pixel: usize, sample: usize, bits: u16, spp: usize) -> u16 {
    if bits == 8 {
        return u16::from(bytes[pixel * spp + sample]);
    }
    if bits == 16 {
        let start = (pixel * spp + sample) * 2;
        return u16::from_ne_bytes([bytes[start], bytes[start + 1]]);
    }
    let bit_offset = (pixel * spp + sample) * bits as usize;
    let mut value = 0u16;
    for index in 0..bits as usize {
        let absolute = bit_offset + index;
        let byte = bytes[absolute / 8];
        let shift = 7 - (absolute % 8);
        value = (value << 1) | u16::from((byte >> shift) & 1);
    }
    value
}

fn bit_sample_plane(bytes: &[u8], pixel: usize, bits: u16) -> u16 {
    if bits == 8 {
        return u16::from(bytes[pixel]);
    }
    if bits == 16 {
        let start = pixel * 2;
        return u16::from_ne_bytes([bytes[start], bytes[start + 1]]);
    }
    let bit_offset = pixel * bits as usize;
    let mut value = 0u16;
    for index in 0..bits as usize {
        let absolute = bit_offset + index;
        let byte = bytes[absolute / 8];
        let shift = 7 - (absolute % 8);
        value = (value << 1) | u16::from((byte >> shift) & 1);
    }
    value
}

unsafe fn pixel_rgba_from_row(
    tif: *mut TIFF,
    row_data: &[u8],
    separate_rows: Option<&[Vec<u8>]>,
    x: usize,
) -> Option<u32> {
    let bits = tag_u16(tif, TAG_BITSPERSAMPLE, true, 1);
    let spp = usize::from(tag_u16(tif, TAG_SAMPLESPERPIXEL, true, 1).max(1));
    let photometric = tag_u16(tif, TAG_PHOTOMETRIC, true, PHOTOMETRIC_MINISBLACK);
    let planar = tag_u16(tif, TAG_PLANARCONFIG, true, PLANARCONFIG_CONTIG);
    let alpha_index = match photometric {
        PHOTOMETRIC_RGB if spp >= 4 => Some(3usize),
        PHOTOMETRIC_MINISBLACK | PHOTOMETRIC_MINISWHITE | PHOTOMETRIC_PALETTE if spp >= 2 => {
            Some(1usize)
        }
        _ => None,
    };
    let sample = |index: usize| -> u16 {
        if planar == PLANARCONFIG_SEPARATE {
            separate_rows
                .and_then(|rows| rows.get(index))
                .map(|row| bit_sample_plane(row, x, bits))
                .unwrap_or(0)
        } else {
            bit_sample_contig(row_data, x, index, bits, spp)
        }
    };
    let alpha = alpha_index
        .map(|index| scale_sample_to_u8(sample(index), bits))
        .unwrap_or(255);
    Some(match photometric {
        PHOTOMETRIC_MINISBLACK | PHOTOMETRIC_MINISWHITE => {
            let mut gray = scale_sample_to_u8(sample(0), bits);
            if photometric == PHOTOMETRIC_MINISWHITE {
                gray = 255u8.wrapping_sub(gray);
            }
            pack_rgba(gray, gray, gray, alpha)
        }
        PHOTOMETRIC_RGB | PHOTOMETRIC_YCBCR => pack_rgba(
            scale_sample_to_u8(sample(0), bits),
            scale_sample_to_u8(sample(1), bits),
            scale_sample_to_u8(sample(2), bits),
            alpha,
        ),
        PHOTOMETRIC_PALETTE => {
            let cmap = copy_u16_array_tag(tif, TAG_COLORMAP, false)?;
            let plane = 1usize << bits.min(15);
            if cmap.len() < plane * 3 {
                return None;
            }
            let index = usize::from(sample(0));
            if index >= plane {
                return None;
            }
            let r = palette_entry_to_u8(cmap[index]);
            let g = palette_entry_to_u8(cmap[index + plane]);
            let b = palette_entry_to_u8(cmap[index + plane * 2]);
            pack_rgba(r, g, b, alpha)
        }
        PHOTOMETRIC_CIELAB => {
            let mut cielab = TIFFCIELabToRGB {
                range: 0,
                rstep: 0.0,
                gstep: 0.0,
                bstep: 0.0,
                X0: 0.0,
                Y0: 0.0,
                Z0: 0.0,
                display: DEFAULT_DISPLAY,
                Yr2r: [0.0; 1501],
                Yg2g: [0.0; 1501],
                Yb2b: [0.0; 1501],
            };
            let mut rgb = [0u8; 3];
            let mut xyz = [0f32; 3];
            let mut r = 0u32;
            let mut g = 0u32;
            let mut b = 0u32;
            if safe_tiff_cielab_to_rgb_init(
                &mut cielab,
                ptr::from_ref(&DEFAULT_DISPLAY),
                D65_WHITE.as_ptr().cast_mut(),
            ) != 0
            {
                return None;
            }
            crate::core::safe_tiff_cielab_to_xyz(
                &mut cielab,
                u32::from(scale_sample_to_u8(sample(0), bits)),
                i32::from(scale_sample_to_u8(sample(1), bits)) - 128,
                i32::from(scale_sample_to_u8(sample(2), bits)) - 128,
                &mut xyz[0],
                &mut xyz[1],
                &mut xyz[2],
            );
            crate::core::safe_tiff_xyz_to_rgb(
                &mut cielab,
                xyz[0],
                xyz[1],
                xyz[2],
                &mut r,
                &mut g,
                &mut b,
            );
            rgb[0] = r as u8;
            rgb[1] = g as u8;
            rgb[2] = b as u8;
            pack_rgba(rgb[0], rgb[1], rgb[2], alpha)
        }
        _ => return None,
    })
}

unsafe fn with_rgb_jpeg_mode<T>(
    tif: *mut TIFF,
    force_rgb: bool,
    mut f: impl FnMut() -> Option<T>,
) -> Option<T> {
    if !force_rgb {
        return f();
    }
    let previous = jpeg_color_mode(tif);
    set_jpeg_color_mode(tif, JPEGCOLORMODE_RGB);
    let result = f();
    set_jpeg_color_mode(tif, previous);
    result
}

unsafe fn fill_row(
    tif: *mut TIFF,
    row_data: &[u8],
    separate_rows: Option<&[Vec<u8>]>,
    image_row: usize,
    raster: &mut [u32],
    requested_orientation: u16,
) -> bool {
    let width = tag_u32(tif, TAG_IMAGEWIDTH, true, 0) as usize;
    let height = tag_u32(tif, TAG_IMAGELENGTH, true, 0) as usize;
    let dest_row = if requested_orientation == ORIENTATION_TOPLEFT {
        image_row
    } else {
        height.saturating_sub(1).saturating_sub(image_row)
    };
    for x in 0..width {
        let Some(pixel) = pixel_rgba_from_row(tif, row_data, separate_rows, x) else {
            return false;
        };
        raster[dest_row * width + x] = pixel;
    }
    true
}

unsafe fn read_rows_into_raster(
    tif: *mut TIFF,
    raster: &mut [u32],
    requested_orientation: u16,
) -> bool {
    let width = tag_u32(tif, TAG_IMAGEWIDTH, true, 0) as usize;
    let height = tag_u32(tif, TAG_IMAGELENGTH, true, 0) as usize;
    let bits = tag_u16(tif, TAG_BITSPERSAMPLE, true, 1);
    let spp = usize::from(tag_u16(tif, TAG_SAMPLESPERPIXEL, true, 1).max(1));
    let photometric = tag_u16(tif, TAG_PHOTOMETRIC, true, PHOTOMETRIC_MINISBLACK);
    let compression = tag_u16(tif, TAG_COMPRESSION, true, 1);
    let force_rgb = photometric == PHOTOMETRIC_YCBCR
        && matches!(compression, COMPRESSION_JPEG | COMPRESSION_OJPEG);

    with_rgb_jpeg_mode(tif, force_rgb, || {
        let scanline_size = usize::try_from(TIFFScanlineSize(tif)).ok()?;
        let planar = tag_u16(tif, TAG_PLANARCONFIG, true, PLANARCONFIG_CONTIG);
        let mut row_data = vec![0u8; scanline_size];
        let mut separate = if planar == PLANARCONFIG_SEPARATE {
            Some(vec![vec![0u8; scanline_size]; spp])
        } else {
            None
        };
        for row in 0..height {
            if let Some(planes) = separate.as_mut() {
                for sample in 0..spp {
                    if TIFFReadScanline(
                        tif,
                        planes[sample].as_mut_ptr().cast::<c_void>(),
                        row as u32,
                        sample as u16,
                    ) < 0
                    {
                        return None;
                    }
                }
                if !fill_row(tif, &[], Some(planes), row, raster, requested_orientation) {
                    return None;
                }
            } else {
                if TIFFReadScanline(
                    tif,
                    row_data.as_mut_ptr().cast::<c_void>(),
                    row as u32,
                    0,
                ) < 0
                {
                    return None;
                }
                if !fill_row(tif, &row_data, None, row, raster, requested_orientation) {
                    return None;
                }
            }
        }
        if bits == 0 || width == 0 {
            return None;
        }
        Some(())
    })
    .is_some()
}

unsafe fn read_ojpeg_full_into_raster(
    tif: *mut TIFF,
    raster: &mut [u32],
    requested_orientation: u16,
) -> bool {
    let width = tag_u32(tif, TAG_IMAGEWIDTH, true, 0);
    let height = tag_u32(tif, TAG_IMAGELENGTH, true, 0);
    let Some(width_usize) = usize::try_from(width).ok() else {
        return false;
    };
    let Some(height_usize) = usize::try_from(height).ok() else {
        return false;
    };
    let Some(expected_len) = width_usize
        .checked_mul(height_usize)
        .and_then(|pixels| pixels.checked_mul(3))
    else {
        return false;
    };
    let Some(rgb) = ojpeg_decode_full_rgb_image(tif) else {
        return false;
    };
    if rgb.len() != expected_len {
        emit_error_message(tif, "OJPEG", "Decoded OJPEG RGB image has an unexpected size");
        return false;
    }

    for row in 0..height_usize {
        let dest_row = if requested_orientation == ORIENTATION_TOPLEFT {
            row
        } else {
            height_usize.saturating_sub(1).saturating_sub(row)
        };
        let src_offset = row.checked_mul(width_usize).and_then(|v| v.checked_mul(3));
        let dst_offset = dest_row.checked_mul(width_usize);
        let (Some(src_offset), Some(dst_offset)) = (src_offset, dst_offset) else {
            return false;
        };
        for x in 0..width_usize {
            let base = src_offset + x * 3;
            raster[dst_offset + x] = pack_rgba(rgb[base], rgb[base + 1], rgb[base + 2], 255);
        }
    }
    true
}

unsafe fn read_tile_region_rgba(
    tif: *mut TIFF,
    x: u32,
    y: u32,
    raster: &mut [u32],
) -> bool {
    let tile_width = tag_u32(tif, TAG_TILEWIDTH, false, 0) as usize;
    let tile_length = tag_u32(tif, TAG_TILELENGTH, false, 0) as usize;
    let photometric = tag_u16(tif, TAG_PHOTOMETRIC, true, PHOTOMETRIC_MINISBLACK);
    let compression = tag_u16(tif, TAG_COMPRESSION, true, 1);
    let force_rgb = photometric == PHOTOMETRIC_YCBCR
        && matches!(compression, COMPRESSION_JPEG | COMPRESSION_OJPEG);
    let spp = usize::from(tag_u16(tif, TAG_SAMPLESPERPIXEL, true, 1).max(1));
    let planar = tag_u16(tif, TAG_PLANARCONFIG, true, PLANARCONFIG_CONTIG);

    with_rgb_jpeg_mode(tif, force_rgb, || {
        let tile_size = usize::try_from(TIFFTileSize(tif)).ok()?;
        let mut tile_data = vec![0u8; tile_size];
        let mut separate = if planar == PLANARCONFIG_SEPARATE {
            Some(vec![vec![0u8; tile_size]; spp])
        } else {
            None
        };
        if let Some(planes) = separate.as_mut() {
            for sample in 0..spp {
                if TIFFReadTile(
                    tif,
                    planes[sample].as_mut_ptr().cast::<c_void>(),
                    x,
                    y,
                    0,
                    sample as u16,
                ) < 0
                {
                    return None;
                }
            }
        } else if TIFFReadTile(tif, tile_data.as_mut_ptr().cast::<c_void>(), x, y, 0, 0) < 0 {
            return None;
        }

        let image_width = tag_u32(tif, TAG_IMAGEWIDTH, true, 0) as usize;
        let image_height = tag_u32(tif, TAG_IMAGELENGTH, true, 0) as usize;
        let copy_width = min(tile_width, image_width.saturating_sub(x as usize));
        let copy_rows = min(tile_length, image_height.saturating_sub(y as usize));
        let bits = tag_u16(tif, TAG_BITSPERSAMPLE, true, 1);
        let row_stride = if planar == PLANARCONFIG_CONTIG {
            ((tile_width * spp * bits as usize) + 7) / 8
        } else {
            ((tile_width * bits as usize) + 7) / 8
        };

        for row in 0..copy_rows {
            let dst_row = copy_rows - 1 - row;
            for col in 0..copy_width {
                let pixel = if let Some(planes) = separate.as_ref() {
                    let plane_rows: Vec<Vec<u8>> = planes
                        .iter()
                        .map(|plane| plane[row * row_stride..(row + 1) * row_stride].to_vec())
                        .collect();
                    pixel_rgba_from_row(tif, &[], Some(&plane_rows), col)?
                } else {
                    let row_slice = &tile_data[row * row_stride..(row + 1) * row_stride];
                    pixel_rgba_from_row(tif, row_slice, None, col)?
                };
                raster[dst_row * tile_width + col] = pixel;
            }
        }
        Some(())
    })
    .is_some()
}

unsafe fn read_tiled_into_raster(
    tif: *mut TIFF,
    raster: &mut [u32],
    requested_orientation: u16,
) -> bool {
    let width = tag_u32(tif, TAG_IMAGEWIDTH, true, 0) as usize;
    let height = tag_u32(tif, TAG_IMAGELENGTH, true, 0) as usize;
    let tile_width = tag_u32(tif, TAG_TILEWIDTH, false, 0) as usize;
    let tile_length = tag_u32(tif, TAG_TILELENGTH, false, 0) as usize;
    if tile_width == 0 || tile_length == 0 {
        return false;
    }
    let Some(tile_size) = tile_width.checked_mul(tile_length) else {
        return false;
    };
    let mut tile_raster = vec![0u32; tile_size];
    let mut y = 0usize;
    while y < height {
        let mut x = 0usize;
        while x < width {
            tile_raster.fill(0);
            if !read_tile_region_rgba(tif, x as u32, y as u32, &mut tile_raster) {
                return false;
            }
            let copy_width = min(tile_width, width - x);
            let copy_rows = min(tile_length, height - y);
            for row in 0..copy_rows {
                let src_row = tile_length - 1 - row;
                let dest_row = if requested_orientation == ORIENTATION_TOPLEFT {
                    y + row
                } else {
                    height - 1 - (y + row)
                };
                let dst_offset = dest_row * width + x;
                let src_offset = src_row * tile_width;
                raster[dst_offset..dst_offset + copy_width]
                    .copy_from_slice(&tile_raster[src_offset..src_offset + copy_width]);
            }
            x += tile_width;
        }
        y += tile_length;
    }
    true
}

unsafe fn read_logluv_image(
    tif: *mut TIFF,
    raster: &mut [u32],
    requested_orientation: u16,
) -> bool {
    let width = tag_u32(tif, TAG_IMAGEWIDTH, true, 0) as usize;
    let height = tag_u32(tif, TAG_IMAGELENGTH, true, 0) as usize;
    let compression = tag_u16(tif, TAG_COMPRESSION, true, 1);
    let photometric = tag_u16(tif, TAG_PHOTOMETRIC, true, PHOTOMETRIC_LOGLUV);
    if height != 1 {
        emit_error_message(
            tif,
            "TIFFReadRGBAImage",
            "SGILog RGBA decoding currently only supports single-row images",
        );
        return false;
    }
    let strip = if TIFFIsTiled(tif) != 0 {
        0
    } else {
        if TIFFNumberOfStrips(tif) == 0 {
            return false;
        }
        TIFFComputeStrip(tif, 0, 0)
    };
    let raw_size = if TIFFIsTiled(tif) != 0 {
        usize::try_from(TIFFGetStrileByteCount(tif, strip)).ok()
    } else {
        usize::try_from(TIFFRawStripSize(tif, strip)).ok()
    };
    let Some(raw_size) = raw_size else {
        return false;
    };
    let mut raw = vec![0u8; raw_size];
    let rc = if TIFFIsTiled(tif) != 0 {
        TIFFReadRawTile(tif, strip, raw.as_mut_ptr().cast::<c_void>(), raw_size as isize)
    } else {
        TIFFReadRawStrip(tif, strip, raw.as_mut_ptr().cast::<c_void>(), raw_size as isize)
    };
    if rc < 0 {
        return false;
    }
    raw.truncate(rc as usize);

    let packed = match compression {
        COMPRESSION_SGILOG => sgilog32_decode_row(&raw, width),
        COMPRESSION_SGILOG24 => sgilog24_decode_row(&raw, width),
        _ => None,
    };
    let Some(packed) = packed else {
        emit_error_message(tif, "TIFFReadRGBAImage", "Malformed SGILog payload");
        return false;
    };

    let mut xyz = [0f32; 3];
    let mut rgb = [0u8; 3];
    let row = if requested_orientation == ORIENTATION_TOPLEFT { 0 } else { height - 1 };
    for (x, value) in packed.into_iter().enumerate() {
        if photometric == PHOTOMETRIC_LOGL {
            let y = if compression == COMPRESSION_SGILOG24 {
                safe_tiff_logl10_to_y(((value >> 14) & 0x3ff) as c_int)
            } else {
                safe_tiff_logl16_to_y((value >> 16) as c_int)
            };
            let gray = if y <= 0.0 {
                0
            } else if y >= 100.0 {
                255
            } else {
                (y * 255.0 / 100.0) as u8
            };
            raster[row * width + x] = pack_rgba(gray, gray, gray, 255);
        } else {
            if compression == COMPRESSION_SGILOG24 {
                safe_tiff_logluv24_to_xyz(value, xyz.as_mut_ptr());
            } else {
                safe_tiff_logluv32_to_xyz(value, xyz.as_mut_ptr());
            }
            safe_tiff_xyz_to_rgb24(xyz.as_mut_ptr(), rgb.as_mut_ptr());
            raster[row * width + x] = pack_rgba(rgb[0], rgb[1], rgb[2], 255);
        }
    }
    true
}

unsafe fn read_rgba_image_impl(
    tif: *mut TIFF,
    raster: &mut [u32],
    requested_orientation: u16,
) -> bool {
    let photometric = tag_u16(tif, TAG_PHOTOMETRIC, true, PHOTOMETRIC_MINISBLACK);
    let compression = tag_u16(tif, TAG_COMPRESSION, true, 1);
    if matches!(photometric, PHOTOMETRIC_LOGL | PHOTOMETRIC_LOGLUV) {
        return read_logluv_image(tif, raster, requested_orientation);
    }
    if compression == COMPRESSION_OJPEG {
        return read_ojpeg_full_into_raster(tif, raster, requested_orientation);
    }
    if TIFFIsTiled(tif) != 0 {
        read_tiled_into_raster(tif, raster, requested_orientation)
    } else {
        read_rows_into_raster(tif, raster, requested_orientation)
    }
}

unsafe fn fill_rgba_image_metadata(img: *mut TIFFRGBAImage, tif: *mut TIFF, stoponerr: c_int) {
    (*img).tif = tif;
    (*img).stoponerr = stoponerr;
    (*img).width = tag_u32(tif, TAG_IMAGEWIDTH, true, 0);
    (*img).height = tag_u32(tif, TAG_IMAGELENGTH, true, 0);
    (*img).bitspersample = tag_u16(tif, TAG_BITSPERSAMPLE, true, 1);
    (*img).samplesperpixel = tag_u16(tif, TAG_SAMPLESPERPIXEL, true, 1);
    (*img).orientation = tag_u16(tif, TAG_ORIENTATION, true, ORIENTATION_TOPLEFT);
    (*img).photometric = tag_u16(tif, TAG_PHOTOMETRIC, true, PHOTOMETRIC_MINISBLACK);
    (*img).isContig = (tag_u16(tif, TAG_PLANARCONFIG, true, PLANARCONFIG_CONTIG)
        == PLANARCONFIG_CONTIG) as c_int;
    (*img).alpha = if (*img).samplesperpixel > 1 { 1 } else { 0 };
    (*img).redcmap = ptr::null_mut();
    (*img).greencmap = ptr::null_mut();
    (*img).bluecmap = ptr::null_mut();
    (*img).Map = ptr::null_mut();
    (*img).BWmap = ptr::null_mut();
    (*img).PALmap = ptr::null_mut();
    (*img).ycbcr = ptr::null_mut();
    (*img).cielab = ptr::null_mut();
    (*img).UaToAa = ptr::null_mut();
    (*img).Bitdepth16To8 = ptr::null_mut();
    (*img).row_offset = 0;
    (*img).col_offset = 0;
}

#[no_mangle]
pub unsafe extern "C" fn safe_tiff_rgba_image_ok(tif: *mut TIFF, emsg: *mut c_char) -> c_int {
    if tif.is_null() {
        set_error(emsg, "Invalid TIFF handle");
        return 0;
    }
    let photometric = tag_u16(tif, TAG_PHOTOMETRIC, true, PHOTOMETRIC_MINISBLACK);
    let supported = matches!(
        photometric,
        PHOTOMETRIC_MINISWHITE
            | PHOTOMETRIC_MINISBLACK
            | PHOTOMETRIC_RGB
            | PHOTOMETRIC_PALETTE
            | PHOTOMETRIC_YCBCR
            | PHOTOMETRIC_CIELAB
            | PHOTOMETRIC_LOGL
            | PHOTOMETRIC_LOGLUV
    );
    if supported {
        set_error(emsg, "");
        1
    } else {
        set_error(emsg, "Unsupported photometric interpretation for RGBA conversion");
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn safe_tiff_rgba_image_begin(
    img: *mut TIFFRGBAImage,
    tif: *mut TIFF,
    stoponerr: c_int,
    emsg: *mut c_char,
) -> c_int {
    if img.is_null() || tif.is_null() {
        set_error(emsg, "Invalid RGBA image arguments");
        return 0;
    }
    if safe_tiff_rgba_image_ok(tif, emsg) == 0 {
        return 0;
    }
    ptr::write_bytes(img, 0, 1);
    fill_rgba_image_metadata(img, tif, stoponerr);
    (*img).req_orientation = ORIENTATION_BOTLEFT;
    (*img).get = Some(safe_tiff_rgba_image_get);
    set_error(emsg, "");
    1
}

#[no_mangle]
pub unsafe extern "C" fn safe_tiff_rgba_image_get(
    img: *mut TIFFRGBAImage,
    raster: *mut u32,
    width: u32,
    height: u32,
) -> c_int {
    if img.is_null() || raster.is_null() {
        return 0;
    }
    let tif = (*img).tif;
    let actual_width = (*img).width;
    let actual_height = (*img).height;
    if width < actual_width || height < actual_height {
        emit_error_message(
            tif,
            "TIFFRGBAImageGet",
            "Destination raster is smaller than the TIFF image",
        );
        return 0;
    }
    let Some(count) = usize::try_from(actual_width)
        .ok()
        .and_then(|w| usize::try_from(actual_height).ok().and_then(|h| w.checked_mul(h)))
    else {
        return 0;
    };
    let raster = slice::from_raw_parts_mut(raster, count);
    if !read_rgba_image_impl(tif, raster, (*img).req_orientation) {
        return 0;
    }
    1
}

#[no_mangle]
pub unsafe extern "C" fn safe_tiff_rgba_image_end(img: *mut TIFFRGBAImage) {
    if img.is_null() {
        return;
    }
    ptr::write_bytes(img, 0, 1);
}

#[no_mangle]
pub unsafe extern "C" fn safe_tiff_read_rgba_image(
    tif: *mut TIFF,
    width: u32,
    height: u32,
    raster: *mut u32,
    stop_on_error: c_int,
) -> c_int {
    safe_tiff_read_rgba_image_oriented(
        tif,
        width,
        height,
        raster,
        ORIENTATION_BOTLEFT as c_int,
        stop_on_error,
    )
}

#[no_mangle]
pub unsafe extern "C" fn safe_tiff_read_rgba_image_oriented(
    tif: *mut TIFF,
    width: u32,
    height: u32,
    raster: *mut u32,
    orientation: c_int,
    stop_on_error: c_int,
) -> c_int {
    let mut img: TIFFRGBAImage = std::mem::zeroed();
    let mut emsg = [0 as c_char; 1024];
    if safe_tiff_rgba_image_begin(&mut img, tif, stop_on_error, emsg.as_mut_ptr()) == 0 {
        return 0;
    }
    img.req_orientation = orientation as u16;
    let result = safe_tiff_rgba_image_get(&mut img, raster, width, height);
    safe_tiff_rgba_image_end(&mut img);
    result
}

#[no_mangle]
pub unsafe extern "C" fn safe_tiff_read_rgba_strip(
    tif: *mut TIFF,
    row: u32,
    raster: *mut u32,
) -> c_int {
    if tif.is_null() || raster.is_null() {
        return 0;
    }
    let width = tag_u32(tif, TAG_IMAGEWIDTH, true, 0) as usize;
    let height = tag_u32(tif, TAG_IMAGELENGTH, true, 0);
    let rows_per_strip = tag_u32(tif, TAG_ROWSPERSTRIP, true, height).max(1);
    let strip_rows = min(rows_per_strip, height.saturating_sub(row)) as usize;
    let Some(count) = width.checked_mul(strip_rows) else {
        return 0;
    };
    let mut full = vec![0u32; width * height as usize];
    if !read_rgba_image_impl(tif, &mut full, ORIENTATION_TOPLEFT) {
        return 0;
    }
    let out = slice::from_raw_parts_mut(raster, count);
    for r in 0..strip_rows {
        let src_row = row as usize + strip_rows - 1 - r;
        let dst_offset = r * width;
        let src_offset = src_row * width;
        out[dst_offset..dst_offset + width].copy_from_slice(&full[src_offset..src_offset + width]);
    }
    1
}

#[no_mangle]
pub unsafe extern "C" fn safe_tiff_read_rgba_tile(
    tif: *mut TIFF,
    x: u32,
    y: u32,
    raster: *mut u32,
) -> c_int {
    if tif.is_null() || raster.is_null() {
        return 0;
    }
    let tile_width = tag_u32(tif, TAG_TILEWIDTH, false, 0) as usize;
    let tile_length = tag_u32(tif, TAG_TILELENGTH, false, 0) as usize;
    let Some(count) = tile_width.checked_mul(tile_length) else {
        return 0;
    };
    let out = slice::from_raw_parts_mut(raster, count);
    out.fill(0);
    if tag_u16(tif, TAG_PHOTOMETRIC, true, PHOTOMETRIC_MINISBLACK) == PHOTOMETRIC_LOGLUV {
        let width = tag_u32(tif, TAG_IMAGEWIDTH, true, 0);
        let height = tag_u32(tif, TAG_IMAGELENGTH, true, 0);
        let Some(full_count) = usize::try_from(width)
            .ok()
            .and_then(|w| usize::try_from(height).ok().and_then(|h| w.checked_mul(h)))
        else {
            return 0;
        };
        let mut full = vec![0u32; full_count];
        if !read_logluv_image(tif, &mut full, ORIENTATION_TOPLEFT) {
            return 0;
        }
        let copy_width = min(tile_width, width.saturating_sub(x) as usize);
        let copy_rows = min(tile_length, height.saturating_sub(y) as usize);
        for row in 0..copy_rows {
            let src_row = y as usize + row;
            let dst_row = tile_length - 1 - row;
            let src_offset = src_row * width as usize + x as usize;
            let dst_offset = dst_row * tile_width;
            out[dst_offset..dst_offset + copy_width]
                .copy_from_slice(&full[src_offset..src_offset + copy_width]);
        }
        return 1;
    }
    read_tile_region_rgba(tif, x, y, out) as c_int
}
