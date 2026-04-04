use crate::abi::{TIFFCIELabToRGB, TIFFDisplay, TIFFRGBAImage, TIFFYCbCrToRGB};
use crate::core::{
    free_ycbcr_tables, get_tag_value, jpeg_color_mode, ojpeg_decode_full_rgb_image,
    safe_tiff_cielab16_to_xyz, safe_tiff_cielab_to_rgb_init, safe_tiff_logl16_to_y,
    safe_tiff_logluv24_to_xyz, safe_tiff_logluv32_to_xyz, safe_tiff_xyz_to_rgb24,
    safe_tiff_ycbcr_to_rgb, safe_tiff_ycbcr_to_rgb_init, set_jpeg_color_mode,
    TIFFIsCODECConfigured, COMPRESSION_JPEG, COMPRESSION_OJPEG, JPEGCOLORMODE_RGB,
};
use crate::strile::{
    TIFFComputeTile, TIFFGetStrileByteCount, TIFFNumberOfStrips, TIFFNumberOfTiles,
    TIFFReadRawStrip, TIFFReadRawTile, TIFFReadScanline, TIFFReadTile, TIFFScanlineSize,
    TIFFTileSize,
};
use crate::{emit_error_message, TIFFIsTiled, TIFF};
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
const PHOTOMETRIC_SEPARATED: u16 = 5;
const PHOTOMETRIC_YCBCR: u16 = 6;
const PHOTOMETRIC_CIELAB: u16 = 8;
const PHOTOMETRIC_LOGL: u16 = 32844;
const PHOTOMETRIC_LOGLUV: u16 = 32845;

const COMPRESSION_CCITTRLE: u16 = 2;
const COMPRESSION_CCITTFAX3: u16 = 3;
const COMPRESSION_CCITTFAX4: u16 = 4;
const COMPRESSION_CCITTRLEW: u16 = 32771;
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
const TAG_WHITEPOINT: u32 = 318;
const TAG_INKSET: u32 = 332;
const TAG_EXTRASAMPLES: u32 = 338;
const TAG_SAMPLEFORMAT: u32 = 339;
const TAG_YCBCRCOEFFICIENTS: u32 = 529;
const TAG_YCBCRSUBSAMPLING: u32 = 530;
const TAG_REFERENCEBLACKWHITE: u32 = 532;
const TAG_TILEWIDTH: u32 = 322;
const TAG_TILELENGTH: u32 = 323;

const PLANARCONFIG_CONTIG: u16 = 1;
const PLANARCONFIG_SEPARATE: u16 = 2;

const INKSET_CMYK: u16 = 1;
const EXTRASAMPLE_UNSPECIFIED: u16 = 0;
const EXTRASAMPLE_ASSOCALPHA: u16 = 1;
const EXTRASAMPLE_UNASSALPHA: u16 = 2;
const SAMPLEFORMAT_IEEEFP: u16 = 3;

#[derive(Clone, Copy)]
struct PreparedRgbaImage {
    width: u32,
    height: u32,
    bitspersample: u16,
    samplesperpixel: u16,
    orientation: u16,
    photometric: u16,
    compression: u16,
    planarconfig: u16,
    alpha: c_int,
    colorchannels: i32,
    is_contig: c_int,
}

#[derive(Clone, Copy)]
struct RowDecodeState {
    bits: u16,
    samples_per_pixel: usize,
    photometric: u16,
    planar: u16,
    alpha_index: Option<usize>,
}

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

unsafe fn tag_u16_optional(tif: *mut TIFF, tag: u32, defaulted: bool) -> Option<u16> {
    let (type_, count, data) = get_tag_raw(tif, tag, defaulted)?;
    if count == 0 || data.is_null() {
        return None;
    }
    match type_.0 {
        x if x == crate::abi::TIFFDataType::TIFF_SHORT.0 => Some(*data.cast::<u16>()),
        x if x == crate::abi::TIFFDataType::TIFF_LONG.0 => u16::try_from(*data.cast::<u32>()).ok(),
        x if x == crate::abi::TIFFDataType::TIFF_SLONG.0 => u16::try_from(*data.cast::<i32>()).ok(),
        _ => None,
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

unsafe fn copy_f32_array_tag(tif: *mut TIFF, tag: u32, defaulted: bool) -> Option<Vec<f32>> {
    let (type_, count, data) = get_tag_raw(tif, tag, defaulted)?;
    if count == 0 {
        return Some(Vec::new());
    }
    if data.is_null() {
        return None;
    }
    match type_.0 {
        x if x == crate::abi::TIFFDataType::TIFF_FLOAT.0
            || x == crate::abi::TIFFDataType::TIFF_RATIONAL.0
            || x == crate::abi::TIFFDataType::TIFF_SRATIONAL.0 =>
        {
            Some(slice::from_raw_parts(data.cast::<f32>(), count).to_vec())
        }
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
    if value > 255 {
        (value >> 8) as u8
    } else {
        value as u8
    }
}

unsafe fn ycbcr_subsampling(tif: *mut TIFF) -> Option<(u16, u16)> {
    let values = copy_u16_array_tag(tif, TAG_YCBCRSUBSAMPLING, true)?;
    if values.len() < 2 {
        return None;
    }
    Some((values[0], values[1]))
}

unsafe fn validate_and_init_ycbcr_state(
    tif: *mut TIFF,
    ycbcr: *mut TIFFYCbCrToRGB,
    planarconfig: u16,
    emsg: *mut c_char,
) -> bool {
    if ycbcr.is_null() {
        set_error(emsg, "No space for YCbCr->RGB conversion state");
        return false;
    }
    let Some((hs, vs)) = ycbcr_subsampling(tif) else {
        set_error(emsg, "Missing or invalid YCbCrSubsampling tag");
        return false;
    };
    if hs == 0 || vs == 0 || vs > hs || hs > 4 || vs > 4 {
        set_error(emsg, "Invalid YCbCrSubsampling tag");
        return false;
    }
    if (planarconfig == PLANARCONFIG_CONTIG || planarconfig == PLANARCONFIG_SEPARATE)
        && (hs != 1 || vs != 1)
    {
        set_error(
            emsg,
            "Sorry, can not handle YCbCr images with subsampling other than 1,1",
        );
        return false;
    }

    let Some(luma) = copy_f32_array_tag(tif, TAG_YCBCRCOEFFICIENTS, true) else {
        set_error(emsg, "Missing YCbCrCoefficients tag");
        return false;
    };
    let Some(ref_black_white) = copy_f32_array_tag(tif, TAG_REFERENCEBLACKWHITE, true) else {
        set_error(emsg, "Missing ReferenceBlackWhite tag");
        return false;
    };
    if luma.len() < 3 || ref_black_white.len() < 6 {
        set_error(emsg, "Invalid YCbCr conversion tags");
        return false;
    }
    if !luma[..3].iter().all(|value| value.is_finite())
        || !ref_black_white[..6].iter().all(|value| value.is_finite())
    {
        set_error(emsg, "Invalid YCbCr conversion tags");
        return false;
    }
    if luma[1] == 0.0 {
        set_error(emsg, "Invalid values for YCbCrCoefficients tag");
        return false;
    }
    ptr::write_bytes(ycbcr, 0, 1);
    if safe_tiff_ycbcr_to_rgb_init(
        ycbcr,
        luma.as_ptr().cast_mut(),
        ref_black_white.as_ptr().cast_mut(),
    ) < 0
    {
        set_error(emsg, "Failed to initialize YCbCr->RGB conversion state");
        return false;
    }
    true
}

unsafe fn validate_and_init_cielab_state(
    tif: *mut TIFF,
    cielab: *mut TIFFCIELabToRGB,
    emsg: *mut c_char,
) -> bool {
    if cielab.is_null() {
        set_error(emsg, "No space for CIE L*a*b*->RGB conversion state");
        return false;
    }
    let Some(white_point) = copy_f32_array_tag(tif, TAG_WHITEPOINT, true) else {
        set_error(emsg, "Missing WhitePoint tag");
        return false;
    };
    if white_point.len() < 2 || !white_point[..2].iter().all(|value| value.is_finite()) {
        set_error(emsg, "Invalid value for WhitePoint tag.");
        return false;
    }
    if white_point[1] == 0.0 {
        set_error(emsg, "Invalid value for WhitePoint tag.");
        return false;
    }
    let ref_white = [
        white_point[0] / white_point[1] * 100.0,
        100.0,
        (1.0 - white_point[0] - white_point[1]) / white_point[1] * 100.0,
    ];
    ptr::write_bytes(cielab, 0, 1);
    if safe_tiff_cielab_to_rgb_init(
        cielab,
        ptr::from_ref(&DEFAULT_DISPLAY),
        ref_white.as_ptr().cast_mut(),
    ) < 0
    {
        set_error(
            emsg,
            "Failed to initialize CIE L*a*b*->RGB conversion state.",
        );
        return false;
    }
    true
}

unsafe fn free_rgba_conversion_state(img: *mut TIFFRGBAImage) {
    if img.is_null() {
        return;
    }
    if !(*img).ycbcr.is_null() {
        free_ycbcr_tables((*img).ycbcr);
        drop(Box::from_raw((*img).ycbcr));
        (*img).ycbcr = ptr::null_mut();
    }
    if !(*img).cielab.is_null() {
        drop(Box::from_raw((*img).cielab));
        (*img).cielab = ptr::null_mut();
    }
}

fn pack_rgba(r: u8, g: u8, b: u8, a: u8) -> u32 {
    u32::from(r) | (u32::from(g) << 8) | (u32::from(b) << 16) | (u32::from(a) << 24)
}

fn decode_logl_gray(y: f64) -> u8 {
    if y <= 0.0 {
        0
    } else if y >= 1.0 {
        255
    } else {
        (256.0 * y.sqrt()) as u8
    }
}

fn is_stop_on_error(stop_on_error: c_int) -> bool {
    stop_on_error != 0
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
    img: *mut TIFFRGBAImage,
    tif: *mut TIFF,
    state: RowDecodeState,
    row_data: &[u8],
    separate_rows: Option<&[&[u8]]>,
    x: usize,
) -> Option<u32> {
    let bits = state.bits;
    let spp = state.samples_per_pixel;
    let photometric = state.photometric;
    let planar = state.planar;
    let alpha_index = state.alpha_index;
    let sample = |index: usize| -> u16 {
        if planar == PLANARCONFIG_SEPARATE {
            separate_rows
                .and_then(|rows| rows.get(index).copied())
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
        PHOTOMETRIC_RGB => pack_rgba(
            scale_sample_to_u8(sample(0), bits),
            scale_sample_to_u8(sample(1), bits),
            scale_sample_to_u8(sample(2), bits),
            alpha,
        ),
        PHOTOMETRIC_SEPARATED => {
            let c = scale_sample_to_u8(sample(0), bits);
            let m = scale_sample_to_u8(sample(1), bits);
            let y = scale_sample_to_u8(sample(2), bits);
            let k = 255u16.saturating_sub(u16::from(scale_sample_to_u8(sample(3), bits)));
            let r = ((k * u16::from(255u8.saturating_sub(c))) / 255) as u8;
            let g = ((k * u16::from(255u8.saturating_sub(m))) / 255) as u8;
            let b = ((k * u16::from(255u8.saturating_sub(y))) / 255) as u8;
            pack_rgba(r, g, b, 255)
        }
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
            let cielab_ptr = if img.is_null() || (*img).cielab.is_null() {
                if safe_tiff_cielab_to_rgb_init(
                    &mut cielab,
                    ptr::from_ref(&DEFAULT_DISPLAY),
                    D65_WHITE.as_ptr().cast_mut(),
                ) != 0
                {
                    return None;
                }
                &mut cielab
            } else {
                (*img).cielab
            };
            if bits == 16 {
                safe_tiff_cielab16_to_xyz(
                    cielab_ptr,
                    u32::from(sample(0)),
                    i32::from(i16::from_ne_bytes(sample(1).to_ne_bytes())),
                    i32::from(i16::from_ne_bytes(sample(2).to_ne_bytes())),
                    &mut xyz[0],
                    &mut xyz[1],
                    &mut xyz[2],
                );
            } else {
                crate::core::safe_tiff_cielab_to_xyz(
                    cielab_ptr,
                    u32::from(scale_sample_to_u8(sample(0), bits)),
                    i32::from(scale_sample_to_u8(sample(1), bits)) - 128,
                    i32::from(scale_sample_to_u8(sample(2), bits)) - 128,
                    &mut xyz[0],
                    &mut xyz[1],
                    &mut xyz[2],
                );
            }
            crate::core::safe_tiff_xyz_to_rgb(
                cielab_ptr, xyz[0], xyz[1], xyz[2], &mut r, &mut g, &mut b,
            );
            rgb[0] = r as u8;
            rgb[1] = g as u8;
            rgb[2] = b as u8;
            pack_rgba(rgb[0], rgb[1], rgb[2], alpha)
        }
        PHOTOMETRIC_YCBCR => {
            let ycbcr = if img.is_null() {
                ptr::null_mut()
            } else {
                (*img).ycbcr
            };
            if ycbcr.is_null() {
                return None;
            }
            let mut r = 0u32;
            let mut g = 0u32;
            let mut b = 0u32;
            safe_tiff_ycbcr_to_rgb(
                ycbcr,
                u32::from(sample(0)),
                i32::from(scale_sample_to_u8(sample(1), bits)),
                i32::from(scale_sample_to_u8(sample(2), bits)),
                &mut r,
                &mut g,
                &mut b,
            );
            pack_rgba(r as u8, g as u8, b as u8, alpha)
        }
        _ => return None,
    })
}

unsafe fn row_decode_state(img: *mut TIFFRGBAImage, tif: *mut TIFF) -> RowDecodeState {
    let bits = if img.is_null() {
        tag_u16(tif, TAG_BITSPERSAMPLE, true, 1)
    } else {
        (*img).bitspersample
    };
    let samples_per_pixel = if img.is_null() {
        usize::from(tag_u16(tif, TAG_SAMPLESPERPIXEL, true, 1).max(1))
    } else {
        usize::from((*img).samplesperpixel.max(1))
    };
    let photometric = if img.is_null() {
        tag_u16(tif, TAG_PHOTOMETRIC, true, PHOTOMETRIC_MINISBLACK)
    } else {
        (*img).photometric
    };
    let planar = if img.is_null() {
        tag_u16(tif, TAG_PLANARCONFIG, true, PLANARCONFIG_CONTIG)
    } else if (*img).isContig != 0 {
        PLANARCONFIG_CONTIG
    } else {
        PLANARCONFIG_SEPARATE
    };
    let alpha_index = match photometric {
        PHOTOMETRIC_RGB if samples_per_pixel >= 4 => Some(3usize),
        PHOTOMETRIC_MINISBLACK | PHOTOMETRIC_MINISWHITE | PHOTOMETRIC_PALETTE
            if samples_per_pixel >= 2 =>
        {
            Some(1usize)
        }
        _ => None,
    };
    RowDecodeState {
        bits,
        samples_per_pixel,
        photometric,
        planar,
        alpha_index,
    }
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

fn window_dest_row(
    raster_height: usize,
    read_height: usize,
    requested_orientation: u16,
    window_row: usize,
) -> usize {
    let base_row = raster_height.saturating_sub(read_height);
    let dest_rel_row = if requested_orientation == ORIENTATION_TOPLEFT {
        window_row
    } else {
        read_height.saturating_sub(1).saturating_sub(window_row)
    };
    base_row + dest_rel_row
}

unsafe fn fill_row_window(
    img: *mut TIFFRGBAImage,
    tif: *mut TIFF,
    state: RowDecodeState,
    row_data: &[u8],
    separate_rows: Option<&[&[u8]]>,
    col_offset: usize,
    read_width: usize,
    window_row: usize,
    read_height: usize,
    raster: &mut [u32],
    raster_width: usize,
    raster_height: usize,
    requested_orientation: u16,
    stop_on_error: c_int,
) -> bool {
    let dest_row = window_dest_row(
        raster_height,
        read_height,
        requested_orientation,
        window_row,
    );
    let dst_offset = dest_row * raster_width;
    for x in 0..read_width {
        let pixel =
            match pixel_rgba_from_row(img, tif, state, row_data, separate_rows, col_offset + x) {
                Some(pixel) => pixel,
                None if is_stop_on_error(stop_on_error) => return false,
                None => 0,
            };
        raster[dst_offset + x] = pixel;
    }
    true
}

unsafe fn read_rows_into_raster(
    img: *mut TIFFRGBAImage,
    raster: &mut [u32],
    raster_width: usize,
    raster_height: usize,
    requested_orientation: u16,
    stop_on_error: c_int,
) -> bool {
    let tif = (*img).tif;
    let width = tag_u32(tif, TAG_IMAGEWIDTH, true, 0) as usize;
    let height = tag_u32(tif, TAG_IMAGELENGTH, true, 0) as usize;
    let Some(row_offset) = usize::try_from((*img).row_offset).ok() else {
        return false;
    };
    let Some(col_offset) = usize::try_from((*img).col_offset).ok() else {
        return false;
    };
    if row_offset >= height || col_offset >= width {
        return false;
    }
    let read_width = min(raster_width, width - col_offset);
    let read_height = min(raster_height, height - row_offset);
    let bits = tag_u16(tif, TAG_BITSPERSAMPLE, true, 1);
    let spp = usize::from(tag_u16(tif, TAG_SAMPLESPERPIXEL, true, 1).max(1));
    let photometric = tag_u16(tif, TAG_PHOTOMETRIC, true, PHOTOMETRIC_MINISBLACK);
    let compression = tag_u16(tif, TAG_COMPRESSION, true, 1);
    let force_rgb = photometric == PHOTOMETRIC_YCBCR
        && matches!(compression, COMPRESSION_JPEG | COMPRESSION_OJPEG);

    with_rgb_jpeg_mode(tif, force_rgb, || {
        let state = row_decode_state(img, tif);
        let scanline_size = usize::try_from(TIFFScanlineSize(tif)).ok()?;
        let planar = state.planar;
        let mut row_data = vec![0u8; scanline_size];
        let mut separate = if planar == PLANARCONFIG_SEPARATE {
            Some(vec![vec![0u8; scanline_size]; spp])
        } else {
            None
        };
        for rel_row in 0..read_height {
            let row = row_offset + rel_row;
            if let Some(planes) = separate.as_mut() {
                for sample in 0..spp {
                    if TIFFReadScanline(
                        tif,
                        planes[sample].as_mut_ptr().cast::<c_void>(),
                        row as u32,
                        sample as u16,
                    ) < 0
                    {
                        if is_stop_on_error(stop_on_error) {
                            return None;
                        }
                        planes[sample].fill(0);
                    }
                }
                let plane_slices: Vec<&[u8]> =
                    planes.iter().map(|plane| plane.as_slice()).collect();
                if !fill_row_window(
                    img,
                    tif,
                    state,
                    &[],
                    Some(&plane_slices),
                    col_offset,
                    read_width,
                    rel_row,
                    read_height,
                    raster,
                    raster_width,
                    raster_height,
                    requested_orientation,
                    stop_on_error,
                ) {
                    return None;
                }
            } else {
                if TIFFReadScanline(tif, row_data.as_mut_ptr().cast::<c_void>(), row as u32, 0) < 0
                {
                    if is_stop_on_error(stop_on_error) {
                        return None;
                    }
                    row_data.fill(0);
                }
                if !fill_row_window(
                    img,
                    tif,
                    state,
                    &row_data,
                    None,
                    col_offset,
                    read_width,
                    rel_row,
                    read_height,
                    raster,
                    raster_width,
                    raster_height,
                    requested_orientation,
                    stop_on_error,
                ) {
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
    stop_on_error: c_int,
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
        if is_stop_on_error(stop_on_error) {
            return false;
        }
        raster.fill(0);
        return true;
    };
    if rgb.len() != expected_len {
        emit_error_message(
            tif,
            "OJPEG",
            "Decoded OJPEG RGB image has an unexpected size",
        );
        if is_stop_on_error(stop_on_error) {
            return false;
        }
        raster.fill(0);
        return true;
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
    img: *mut TIFFRGBAImage,
    x: u32,
    y: u32,
    raster: &mut [u32],
    stop_on_error: c_int,
) -> bool {
    let tif = (*img).tif;
    let tile_width = tag_u32(tif, TAG_TILEWIDTH, false, 0) as usize;
    let tile_length = tag_u32(tif, TAG_TILELENGTH, false, 0) as usize;
    let photometric = tag_u16(tif, TAG_PHOTOMETRIC, true, PHOTOMETRIC_MINISBLACK);
    let compression = tag_u16(tif, TAG_COMPRESSION, true, 1);
    let force_rgb = photometric == PHOTOMETRIC_YCBCR
        && matches!(compression, COMPRESSION_JPEG | COMPRESSION_OJPEG);
    let state = row_decode_state(img, tif);
    let spp = state.samples_per_pixel;
    let planar = state.planar;

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
                    if is_stop_on_error(stop_on_error) {
                        return None;
                    }
                    planes[sample].fill(0);
                }
            }
        } else if TIFFReadTile(tif, tile_data.as_mut_ptr().cast::<c_void>(), x, y, 0, 0) < 0 {
            if is_stop_on_error(stop_on_error) {
                return None;
            }
            tile_data.fill(0);
        }

        let image_width = tag_u32(tif, TAG_IMAGEWIDTH, true, 0) as usize;
        let image_height = tag_u32(tif, TAG_IMAGELENGTH, true, 0) as usize;
        let copy_width = min(tile_width, image_width.saturating_sub(x as usize));
        let copy_rows = min(tile_length, image_height.saturating_sub(y as usize));
        let bits = state.bits;
        let row_stride = if planar == PLANARCONFIG_CONTIG {
            ((tile_width * spp * bits as usize) + 7) / 8
        } else {
            ((tile_width * bits as usize) + 7) / 8
        };

        for row in 0..copy_rows {
            let dst_row = copy_rows - 1 - row;
            for col in 0..copy_width {
                let pixel = if let Some(planes) = separate.as_ref() {
                    let row_start = row * row_stride;
                    let row_end = (row + 1) * row_stride;
                    let plane_rows: Vec<&[u8]> = planes
                        .iter()
                        .map(|plane| &plane[row_start..row_end])
                        .collect();
                    match pixel_rgba_from_row(img, tif, state, &[], Some(&plane_rows), col) {
                        Some(pixel) => pixel,
                        None if is_stop_on_error(stop_on_error) => return None,
                        None => 0,
                    }
                } else {
                    let row_slice = &tile_data[row * row_stride..(row + 1) * row_stride];
                    match pixel_rgba_from_row(img, tif, state, row_slice, None, col) {
                        Some(pixel) => pixel,
                        None if is_stop_on_error(stop_on_error) => return None,
                        None => 0,
                    }
                };
                raster[dst_row * tile_width + col] = pixel;
            }
        }
        Some(())
    })
    .is_some()
}

unsafe fn read_tiled_into_raster(
    img: *mut TIFFRGBAImage,
    raster: &mut [u32],
    raster_width: usize,
    raster_height: usize,
    requested_orientation: u16,
    stop_on_error: c_int,
) -> bool {
    let tif = (*img).tif;
    let width = tag_u32(tif, TAG_IMAGEWIDTH, true, 0) as usize;
    let height = tag_u32(tif, TAG_IMAGELENGTH, true, 0) as usize;
    let Some(row_offset) = usize::try_from((*img).row_offset).ok() else {
        return false;
    };
    let Some(col_offset) = usize::try_from((*img).col_offset).ok() else {
        return false;
    };
    if row_offset >= height || col_offset >= width {
        return false;
    }
    let read_width = min(raster_width, width - col_offset);
    let read_height = min(raster_height, height - row_offset);
    let tile_width = tag_u32(tif, TAG_TILEWIDTH, false, 0) as usize;
    let tile_length = tag_u32(tif, TAG_TILELENGTH, false, 0) as usize;
    if tile_width == 0 || tile_length == 0 {
        return false;
    }
    let Some(tile_size) = tile_width.checked_mul(tile_length) else {
        return false;
    };
    let mut tile_raster = vec![0u32; tile_size];
    let tile_y_start = (row_offset / tile_length) * tile_length;
    let tile_y_end = row_offset.saturating_add(read_height);
    let tile_x_start = (col_offset / tile_width) * tile_width;
    let tile_x_end = col_offset.saturating_add(read_width);
    let mut y = tile_y_start;
    while y < tile_y_end {
        let mut x = tile_x_start;
        while x < tile_x_end {
            tile_raster.fill(0);
            if !read_tile_region_rgba(img, x as u32, y as u32, &mut tile_raster, stop_on_error) {
                return false;
            }
            let copy_x0 = col_offset.max(x);
            let copy_x1 = tile_x_end.min(x + tile_width).min(width);
            let copy_y0 = row_offset.max(y);
            let copy_y1 = tile_y_end.min(y + tile_length).min(height);
            for image_row in copy_y0..copy_y1 {
                let window_row = image_row - row_offset;
                let dest_row = window_dest_row(
                    raster_height,
                    read_height,
                    requested_orientation,
                    window_row,
                );
                let dst_col = copy_x0 - col_offset;
                let src_col = copy_x0 - x;
                let count = copy_x1 - copy_x0;
                let src_row = tile_length - 1 - (image_row - y);
                let src_offset = src_row * tile_width + src_col;
                let dst_offset = dest_row * raster_width + dst_col;
                raster[dst_offset..dst_offset + count]
                    .copy_from_slice(&tile_raster[src_offset..src_offset + count]);
            }
            x += tile_width;
        }
        y += tile_length;
    }
    true
}

fn sgilog16_decode_row_consuming(input: &[u8], pixels: usize) -> Option<(Vec<u16>, usize)> {
    let mut out = vec![0u16; pixels];
    let mut offset = 0usize;
    for shift in [8u16, 0] {
        let mut written = 0usize;
        while written < pixels && offset < input.len() {
            let control = input[offset];
            offset += 1;
            if control >= 128 {
                let value = (*input.get(offset)? as u16) << shift;
                let run = control as usize + 2 - 128;
                offset += 1;
                for _ in 0..run {
                    if written >= pixels {
                        break;
                    }
                    out[written] |= value;
                    written += 1;
                }
            } else {
                for _ in 0..control as usize {
                    if written >= pixels {
                        return None;
                    }
                    out[written] |= u16::from(*input.get(offset)?) << shift;
                    offset += 1;
                    written += 1;
                }
            }
        }
        if written != pixels {
            return None;
        }
    }
    Some((out, offset))
}

fn sgilog24_decode_row_consuming(input: &[u8], pixels: usize) -> Option<(Vec<u32>, usize)> {
    let needed = pixels.checked_mul(3)?;
    if input.len() < needed {
        return None;
    }
    let mut out = Vec::with_capacity(pixels);
    for chunk in input[..needed].chunks_exact(3) {
        out.push(((chunk[0] as u32) << 16) | ((chunk[1] as u32) << 8) | chunk[2] as u32);
    }
    Some((out, needed))
}

fn sgilog32_decode_row_consuming(input: &[u8], pixels: usize) -> Option<(Vec<u32>, usize)> {
    let mut out = vec![0u32; pixels];
    let mut offset = 0usize;
    for shift in [24u32, 16, 8, 0] {
        let mut written = 0usize;
        while written < pixels && offset < input.len() {
            let control = input[offset];
            offset += 1;
            if control >= 128 {
                let value = (u32::from(*input.get(offset)?)) << shift;
                let run = control as usize + 2 - 128;
                offset += 1;
                for _ in 0..run {
                    if written >= pixels {
                        break;
                    }
                    out[written] |= value;
                    written += 1;
                }
            } else {
                for _ in 0..control as usize {
                    if written >= pixels {
                        return None;
                    }
                    out[written] |= u32::from(*input.get(offset)?) << shift;
                    offset += 1;
                    written += 1;
                }
            }
        }
        if written != pixels {
            return None;
        }
    }
    Some((out, offset))
}

unsafe fn decode_logluv_row_rgba(
    tif: *mut TIFF,
    input: &[u8],
    pixels: usize,
    photometric: u16,
    compression: u16,
) -> Option<(Vec<u32>, usize)> {
    match (photometric, compression) {
        (PHOTOMETRIC_LOGL, COMPRESSION_SGILOG) => {
            let (packed, consumed) = sgilog16_decode_row_consuming(input, pixels)?;
            let mut out = Vec::with_capacity(pixels);
            for value in packed {
                let gray = decode_logl_gray(safe_tiff_logl16_to_y(value as c_int));
                out.push(pack_rgba(gray, gray, gray, 255));
            }
            Some((out, consumed))
        }
        (PHOTOMETRIC_LOGLUV, COMPRESSION_SGILOG24) => {
            let (packed, consumed) = sgilog24_decode_row_consuming(input, pixels)?;
            let mut xyz = [0f32; 3];
            let mut rgb = [0u8; 3];
            let mut out = Vec::with_capacity(pixels);
            for value in packed {
                safe_tiff_logluv24_to_xyz(value, xyz.as_mut_ptr());
                safe_tiff_xyz_to_rgb24(xyz.as_mut_ptr(), rgb.as_mut_ptr());
                out.push(pack_rgba(rgb[0], rgb[1], rgb[2], 255));
            }
            Some((out, consumed))
        }
        (PHOTOMETRIC_LOGLUV, COMPRESSION_SGILOG) => {
            let (packed, consumed) = sgilog32_decode_row_consuming(input, pixels)?;
            let mut xyz = [0f32; 3];
            let mut rgb = [0u8; 3];
            let mut out = Vec::with_capacity(pixels);
            for value in packed {
                safe_tiff_logluv32_to_xyz(value, xyz.as_mut_ptr());
                safe_tiff_xyz_to_rgb24(xyz.as_mut_ptr(), rgb.as_mut_ptr());
                out.push(pack_rgba(rgb[0], rgb[1], rgb[2], 255));
            }
            Some((out, consumed))
        }
        (PHOTOMETRIC_LOGL, _) => {
            emit_error_message(
                tif,
                "TIFFReadRGBAImage",
                "LogL data must use SGILog compression",
            );
            None
        }
        _ => {
            emit_error_message(
                tif,
                "TIFFReadRGBAImage",
                "LogLuv data must use SGILog or SGILog24 compression",
            );
            None
        }
    }
}

unsafe fn read_logluv_into_raster(
    img: *mut TIFFRGBAImage,
    raster: &mut [u32],
    raster_width: usize,
    raster_height: usize,
    requested_orientation: u16,
    stop_on_error: c_int,
) -> bool {
    let tif = (*img).tif;
    let width = tag_u32(tif, TAG_IMAGEWIDTH, true, 0) as usize;
    let height = tag_u32(tif, TAG_IMAGELENGTH, true, 0) as usize;
    let Some(row_offset) = usize::try_from((*img).row_offset).ok() else {
        return false;
    };
    let Some(col_offset) = usize::try_from((*img).col_offset).ok() else {
        return false;
    };
    if row_offset >= height || col_offset >= width {
        return false;
    }
    let read_width = min(raster_width, width - col_offset);
    let read_height = min(raster_height, height - row_offset);
    let compression = tag_u16(tif, TAG_COMPRESSION, true, 1);
    let photometric = tag_u16(tif, TAG_PHOTOMETRIC, true, PHOTOMETRIC_LOGLUV);
    let Some(expected_pixels) = raster_width.checked_mul(raster_height) else {
        return false;
    };
    if raster.len() < expected_pixels {
        return false;
    }
    raster.fill(0);

    if TIFFIsTiled(tif) != 0 {
        let tile_width = tag_u32(tif, TAG_TILEWIDTH, false, 0) as usize;
        let tile_length = tag_u32(tif, TAG_TILELENGTH, false, 0) as usize;
        if tile_width == 0 || tile_length == 0 {
            return false;
        }
        let tiles = TIFFNumberOfTiles(tif);
        if tiles == 0 {
            return true;
        }
        let mut y = 0usize;
        while y < height {
            let mut x = 0usize;
            while x < width {
                let tile = TIFFComputeTile(tif, x as u32, y as u32, 0, 0);
                let Some(raw_size) = usize::try_from(TIFFGetStrileByteCount(tif, tile)).ok() else {
                    return false;
                };
                let mut raw = vec![0u8; raw_size];
                let rc = TIFFReadRawTile(
                    tif,
                    tile,
                    raw.as_mut_ptr().cast::<c_void>(),
                    raw_size as isize,
                );
                if rc < 0 {
                    if is_stop_on_error(stop_on_error) {
                        return false;
                    }
                    x += tile_width;
                    continue;
                }
                raw.truncate(rc as usize);
                let mut consumed = 0usize;
                let copy_width = min(tile_width, width - x);
                for rel_row in 0..tile_length {
                    let Some((row_rgba, used)) = decode_logluv_row_rgba(
                        tif,
                        &raw[consumed..],
                        tile_width,
                        photometric,
                        compression,
                    ) else {
                        if is_stop_on_error(stop_on_error) {
                            return false;
                        }
                        break;
                    };
                    consumed += used;
                    let image_row = y + rel_row;
                    if image_row >= height {
                        continue;
                    }
                    if image_row < row_offset || image_row >= row_offset + read_height {
                        continue;
                    }
                    let copy_x0 = col_offset.max(x);
                    let copy_x1 = (x + copy_width).min(col_offset + read_width);
                    if copy_x0 >= copy_x1 {
                        continue;
                    }
                    let dest_row = window_dest_row(
                        raster_height,
                        read_height,
                        requested_orientation,
                        image_row - row_offset,
                    );
                    let dst_offset = dest_row * raster_width + (copy_x0 - col_offset);
                    let src_offset = copy_x0 - x;
                    raster[dst_offset..dst_offset + (copy_x1 - copy_x0)]
                        .copy_from_slice(&row_rgba[src_offset..src_offset + (copy_x1 - copy_x0)]);
                }
                x += tile_width;
            }
            y += tile_length;
        }
    } else {
        let strips = TIFFNumberOfStrips(tif);
        if strips == 0 {
            return true;
        }
        let rows_per_strip = tag_u32(tif, TAG_ROWSPERSTRIP, true, height as u32).max(1) as usize;
        for strip in 0..strips {
            let start_row = strip as usize * rows_per_strip;
            if start_row >= height {
                break;
            }
            let rows_in_strip = min(rows_per_strip, height - start_row);
            let Some(raw_size) = usize::try_from(TIFFGetStrileByteCount(tif, strip)).ok() else {
                return false;
            };
            let mut raw = vec![0u8; raw_size];
            let rc = TIFFReadRawStrip(
                tif,
                strip,
                raw.as_mut_ptr().cast::<c_void>(),
                raw_size as isize,
            );
            if rc < 0 {
                if is_stop_on_error(stop_on_error) {
                    return false;
                }
                continue;
            }
            raw.truncate(rc as usize);
            let mut consumed = 0usize;
            for rel_row in 0..rows_in_strip {
                let Some((row_rgba, used)) =
                    decode_logluv_row_rgba(tif, &raw[consumed..], width, photometric, compression)
                else {
                    if is_stop_on_error(stop_on_error) {
                        return false;
                    }
                    break;
                };
                consumed += used;
                let image_row = start_row + rel_row;
                if image_row < row_offset || image_row >= row_offset + read_height {
                    continue;
                }
                let dest_row = window_dest_row(
                    raster_height,
                    read_height,
                    requested_orientation,
                    image_row - row_offset,
                );
                let dst_offset = dest_row * raster_width;
                raster[dst_offset..dst_offset + read_width]
                    .copy_from_slice(&row_rgba[col_offset..col_offset + read_width]);
            }
        }
    }
    true
}

unsafe fn read_rgba_image_impl(
    img: *mut TIFFRGBAImage,
    raster: &mut [u32],
    raster_width: usize,
    raster_height: usize,
    requested_orientation: u16,
    stop_on_error: c_int,
) -> bool {
    let tif = (*img).tif;
    let photometric = tag_u16(tif, TAG_PHOTOMETRIC, true, PHOTOMETRIC_MINISBLACK);
    let compression = tag_u16(tif, TAG_COMPRESSION, true, 1);
    if matches!(photometric, PHOTOMETRIC_LOGL | PHOTOMETRIC_LOGLUV) {
        return read_logluv_into_raster(
            img,
            raster,
            raster_width,
            raster_height,
            requested_orientation,
            stop_on_error,
        );
    }
    if compression == COMPRESSION_OJPEG {
        let full_image = (*img).row_offset == 0
            && (*img).col_offset == 0
            && raster_width == (*img).width as usize
            && raster_height == (*img).height as usize;
        if !full_image {
            emit_error_message(
                tif,
                "TIFFRGBAImageGet",
                "Windowed OJPEG RGBA reads are not supported",
            );
            return false;
        }
        return read_ojpeg_full_into_raster(tif, raster, requested_orientation, stop_on_error);
    }
    if TIFFIsTiled(tif) != 0 {
        read_tiled_into_raster(
            img,
            raster,
            raster_width,
            raster_height,
            requested_orientation,
            stop_on_error,
        )
    } else {
        read_rows_into_raster(
            img,
            raster,
            raster_width,
            raster_height,
            requested_orientation,
            stop_on_error,
        )
    }
}

fn is_ccitt_compression(compression: u16) -> bool {
    matches!(
        compression,
        COMPRESSION_CCITTRLE
            | COMPRESSION_CCITTFAX3
            | COMPRESSION_CCITTFAX4
            | COMPRESSION_CCITTRLEW
    )
}

fn rgba_codec_configured(compression: u16) -> bool {
    matches!(compression, COMPRESSION_SGILOG | COMPRESSION_SGILOG24)
        || unsafe { TIFFIsCODECConfigured(compression) != 0 }
}

unsafe fn prepared_rgba_image(tif: *mut TIFF, emsg: *mut c_char) -> Option<PreparedRgbaImage> {
    let compression = tag_u16(tif, TAG_COMPRESSION, true, 1);
    if !rgba_codec_configured(compression) {
        set_error(
            emsg,
            "Sorry, requested compression method is not configured",
        );
        return None;
    }

    let bitspersample = tag_u16(tif, TAG_BITSPERSAMPLE, true, 1);
    if !matches!(bitspersample, 1 | 2 | 4 | 8 | 16) {
        set_error(
            emsg,
            &format!("Sorry, can not handle images with {bitspersample}-bit samples"),
        );
        return None;
    }

    if tag_u16(tif, TAG_SAMPLEFORMAT, true, 1) == SAMPLEFORMAT_IEEEFP {
        set_error(
            emsg,
            "Sorry, can not handle images with IEEE floating-point samples",
        );
        return None;
    }

    let samplesperpixel = tag_u16(tif, TAG_SAMPLESPERPIXEL, true, 1);
    let sampleinfo = copy_u16_array_tag(tif, TAG_EXTRASAMPLES, false).unwrap_or_default();
    let extrasamples = u16::try_from(sampleinfo.len()).unwrap_or(u16::MAX);
    let mut alpha = 0;
    if let Some(sample) = sampleinfo.first().copied() {
        match sample {
            EXTRASAMPLE_UNSPECIFIED if samplesperpixel > 3 => {
                alpha = EXTRASAMPLE_ASSOCALPHA as c_int;
            }
            EXTRASAMPLE_ASSOCALPHA | EXTRASAMPLE_UNASSALPHA => {
                alpha = c_int::from(sample);
            }
            _ => {}
        }
    }

    let colorchannels = i32::from(samplesperpixel) - i32::from(extrasamples);
    let planarconfig = tag_u16(tif, TAG_PLANARCONFIG, true, PLANARCONFIG_CONTIG);
    let mut photometric = if let Some(photometric) = tag_u16_optional(tif, TAG_PHOTOMETRIC, false) {
        photometric
    } else {
        match colorchannels {
            1 => {
                if is_ccitt_compression(compression) {
                    PHOTOMETRIC_MINISWHITE
                } else {
                    PHOTOMETRIC_MINISBLACK
                }
            }
            3 => PHOTOMETRIC_RGB,
            _ => {
                set_error(emsg, "Missing needed PhotometricInterpretation tag");
                return None;
            }
        }
    };
    let mut effective_bits = bitspersample;

    match photometric {
        PHOTOMETRIC_MINISWHITE | PHOTOMETRIC_MINISBLACK | PHOTOMETRIC_PALETTE => {
            if planarconfig == PLANARCONFIG_CONTIG && samplesperpixel != 1 && bitspersample < 8 {
                set_error(
                    emsg,
                    &format!(
                        "Sorry, can not handle contiguous data with PhotometricInterpretation={photometric}, and Samples/pixel={samplesperpixel} and Bits/Sample={bitspersample}"
                    ),
                );
                return None;
            }
        }
        PHOTOMETRIC_YCBCR => {}
        PHOTOMETRIC_RGB => {
            if colorchannels < 3 {
                set_error(
                    emsg,
                    &format!("Sorry, can not handle RGB image with Color channels={colorchannels}"),
                );
                return None;
            }
            if planarconfig == PLANARCONFIG_CONTIG && alpha != 0 && samplesperpixel < 4 {
                set_error(emsg, "Sorry, can not handle image");
                return None;
            }
        }
        PHOTOMETRIC_SEPARATED => {
            let inkset = tag_u16(tif, TAG_INKSET, true, INKSET_CMYK);
            if inkset != INKSET_CMYK {
                set_error(
                    emsg,
                    &format!("Sorry, can not handle separated image with InkSet={inkset}"),
                );
                return None;
            }
            if samplesperpixel < 4 {
                set_error(
                    emsg,
                    &format!(
                        "Sorry, can not handle separated image with Samples/pixel={samplesperpixel}"
                    ),
                );
                return None;
            }
        }
        PHOTOMETRIC_LOGL => {
            if compression != COMPRESSION_SGILOG {
                set_error(
                    emsg,
                    &format!("Sorry, LogL data must have Compression={COMPRESSION_SGILOG}"),
                );
                return None;
            }
            photometric = PHOTOMETRIC_MINISBLACK;
            effective_bits = 8;
        }
        PHOTOMETRIC_LOGLUV => {
            if compression != COMPRESSION_SGILOG && compression != COMPRESSION_SGILOG24 {
                set_error(
                    emsg,
                    &format!(
                        "Sorry, LogLuv data must have Compression={COMPRESSION_SGILOG} or {COMPRESSION_SGILOG24}"
                    ),
                );
                return None;
            }
            if planarconfig != PLANARCONFIG_CONTIG {
                set_error(
                    emsg,
                    &format!(
                        "Sorry, can not handle LogLuv images with Planarconfiguration={planarconfig}"
                    ),
                );
                return None;
            }
            if samplesperpixel != 3 || colorchannels != 3 {
                set_error(
                    emsg,
                    &format!(
                        "Sorry, can not handle image with Samples/pixel={samplesperpixel}, colorchannels={colorchannels}"
                    ),
                );
                return None;
            }
            photometric = PHOTOMETRIC_RGB;
            effective_bits = 8;
        }
        PHOTOMETRIC_CIELAB => {
            if samplesperpixel != 3 || colorchannels != 3 || !matches!(bitspersample, 8 | 16) {
                set_error(
                    emsg,
                    &format!(
                        "Sorry, can not handle image with Samples/pixel={samplesperpixel}, colorchannels={colorchannels} and Bits/sample={bitspersample}"
                    ),
                );
                return None;
            }
        }
        _ => {
            set_error(
                emsg,
                &format!(
                    "Sorry, can not handle image with PhotometricInterpretation={photometric}"
                ),
            );
            return None;
        }
    }

    if photometric == PHOTOMETRIC_YCBCR
        && planarconfig == PLANARCONFIG_CONTIG
        && matches!(compression, COMPRESSION_JPEG | COMPRESSION_OJPEG)
    {
        photometric = PHOTOMETRIC_RGB;
    }

    Some(PreparedRgbaImage {
        width: tag_u32(tif, TAG_IMAGEWIDTH, true, 0),
        height: tag_u32(tif, TAG_IMAGELENGTH, true, 0),
        bitspersample: effective_bits,
        samplesperpixel,
        orientation: tag_u16(tif, TAG_ORIENTATION, true, ORIENTATION_TOPLEFT),
        photometric,
        compression,
        planarconfig,
        alpha,
        colorchannels,
        is_contig: (!(planarconfig == PLANARCONFIG_SEPARATE && samplesperpixel > 1)) as c_int,
    })
}

unsafe extern "C" fn safe_tiff_rgba_put_contig_stub(
    _: *mut TIFFRGBAImage,
    _: *mut u32,
    _: u32,
    _: u32,
    _: u32,
    _: u32,
    _: i32,
    _: i32,
    _: *mut u8,
) {
}

unsafe extern "C" fn safe_tiff_rgba_put_separate_stub(
    _: *mut TIFFRGBAImage,
    _: *mut u32,
    _: u32,
    _: u32,
    _: u32,
    _: u32,
    _: i32,
    _: i32,
    _: *mut u8,
    _: *mut u8,
    _: *mut u8,
    _: *mut u8,
) {
}

unsafe fn ycbcr_case_supported(
    tif: *mut TIFF,
    state: PreparedRgbaImage,
    emsg: *mut c_char,
) -> bool {
    if state.bitspersample != 8 || state.samplesperpixel != 3 {
        return false;
    }
    let mut ycbcr: TIFFYCbCrToRGB = std::mem::zeroed();
    let ok = validate_and_init_ycbcr_state(tif, &mut ycbcr, state.planarconfig, emsg);
    free_ycbcr_tables(&mut ycbcr);
    ok
}

unsafe fn cielab_case_supported(
    tif: *mut TIFF,
    state: PreparedRgbaImage,
    emsg: *mut c_char,
) -> bool {
    state.samplesperpixel == 3 && matches!(state.bitspersample, 8 | 16) && {
        let mut cielab: TIFFCIELabToRGB = std::mem::zeroed();
        validate_and_init_cielab_state(tif, &mut cielab, emsg)
    }
}

unsafe fn contig_case_supported(
    tif: *mut TIFF,
    state: PreparedRgbaImage,
    emsg: *mut c_char,
) -> bool {
    match state.photometric {
        PHOTOMETRIC_RGB => match state.bitspersample {
            8 | 16 => {
                if state.alpha == EXTRASAMPLE_ASSOCALPHA as c_int
                    || state.alpha == EXTRASAMPLE_UNASSALPHA as c_int
                {
                    state.samplesperpixel >= 4
                } else {
                    state.samplesperpixel >= 3
                }
            }
            _ => false,
        },
        PHOTOMETRIC_SEPARATED => state.bitspersample == 8 && state.samplesperpixel >= 4,
        PHOTOMETRIC_PALETTE => matches!(state.bitspersample, 1 | 2 | 4 | 8),
        PHOTOMETRIC_MINISWHITE | PHOTOMETRIC_MINISBLACK => match state.bitspersample {
            1 | 2 | 4 | 16 => true,
            8 => {
                if state.alpha != 0 {
                    state.samplesperpixel == 2
                } else {
                    true
                }
            }
            _ => false,
        },
        PHOTOMETRIC_YCBCR => ycbcr_case_supported(tif, state, emsg),
        PHOTOMETRIC_CIELAB => cielab_case_supported(tif, state, emsg),
        _ => false,
    }
}

unsafe fn separate_case_supported(
    tif: *mut TIFF,
    state: PreparedRgbaImage,
    emsg: *mut c_char,
) -> bool {
    match state.photometric {
        PHOTOMETRIC_MINISWHITE | PHOTOMETRIC_MINISBLACK | PHOTOMETRIC_RGB => {
            matches!(state.bitspersample, 8 | 16)
        }
        PHOTOMETRIC_SEPARATED => state.bitspersample == 8 && state.samplesperpixel == 4,
        PHOTOMETRIC_YCBCR => ycbcr_case_supported(tif, state, emsg),
        _ => false,
    }
}

unsafe fn pick_contig_case(
    img: *mut TIFFRGBAImage,
    state: PreparedRgbaImage,
    emsg: *mut c_char,
) -> bool {
    (*img).get = Some(safe_tiff_rgba_image_get);
    (*img).put.any = None;
    let supported = contig_case_supported((*img).tif, state, emsg);
    if supported {
        (*img).put.contig = Some(safe_tiff_rgba_put_contig_stub);
    }
    supported
}

unsafe fn pick_separate_case(
    img: *mut TIFFRGBAImage,
    state: PreparedRgbaImage,
    emsg: *mut c_char,
) -> bool {
    (*img).get = Some(safe_tiff_rgba_image_get);
    (*img).put.any = None;
    let supported = separate_case_supported((*img).tif, state, emsg);
    if supported {
        (*img).put.separate = Some(safe_tiff_rgba_put_separate_stub);
    }
    supported
}

unsafe fn initialize_rgba_image(
    img: *mut TIFFRGBAImage,
    tif: *mut TIFF,
    stoponerr: c_int,
    state: PreparedRgbaImage,
    emsg: *mut c_char,
) -> bool {
    ptr::write_bytes(img, 0, 1);
    (*img).row_offset = 0;
    (*img).col_offset = 0;
    (*img).req_orientation = ORIENTATION_BOTLEFT;
    (*img).tif = tif;
    (*img).stoponerr = stoponerr;
    (*img).width = state.width;
    (*img).height = state.height;
    (*img).bitspersample = state.bitspersample;
    (*img).samplesperpixel = state.samplesperpixel;
    (*img).orientation = state.orientation;
    (*img).photometric = state.photometric;
    (*img).isContig = state.is_contig;
    (*img).alpha = state.alpha;

    if state.compression == COMPRESSION_JPEG
        && state.planarconfig == PLANARCONFIG_CONTIG
        && tag_u16(tif, TAG_PHOTOMETRIC, true, PHOTOMETRIC_MINISBLACK) == PHOTOMETRIC_YCBCR
    {
        set_jpeg_color_mode(tif, JPEGCOLORMODE_RGB);
    }

    match state.photometric {
        PHOTOMETRIC_YCBCR => {
            let ycbcr = Box::into_raw(Box::new(std::mem::zeroed()));
            if !validate_and_init_ycbcr_state(tif, ycbcr, state.planarconfig, emsg) {
                drop(Box::from_raw(ycbcr));
                ptr::write_bytes(img, 0, 1);
                return false;
            }
            (*img).ycbcr = ycbcr;
        }
        PHOTOMETRIC_CIELAB => {
            let cielab = Box::into_raw(Box::new(std::mem::zeroed()));
            if !validate_and_init_cielab_state(tif, cielab, emsg) {
                drop(Box::from_raw(cielab));
                ptr::write_bytes(img, 0, 1);
                return false;
            }
            (*img).cielab = cielab;
        }
        _ => {}
    }

    let supported = if state.is_contig != 0 {
        pick_contig_case(img, state, emsg)
    } else {
        pick_separate_case(img, state, emsg)
    };
    if !supported {
        free_rgba_conversion_state(img);
        set_error(emsg, "Sorry, can not handle image");
        ptr::write_bytes(img, 0, 1);
        return false;
    }

    set_error(emsg, "");
    true
}

#[no_mangle]
pub unsafe extern "C" fn safe_tiff_rgba_image_ok(tif: *mut TIFF, emsg: *mut c_char) -> c_int {
    if tif.is_null() {
        set_error(emsg, "Invalid TIFF handle");
        return 0;
    }
    let Some(state) = prepared_rgba_image(tif, emsg) else {
        return 0;
    };
    let supported = if state.is_contig != 0 {
        contig_case_supported(tif, state, emsg)
    } else {
        separate_case_supported(tif, state, emsg)
    };
    if supported {
        set_error(emsg, "");
    }
    supported as c_int
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
    let Some(state) = prepared_rgba_image(tif, emsg) else {
        return 0;
    };
    initialize_rgba_image(img, tif, stoponerr, state, emsg) as c_int
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
    let Some(raster_width) = usize::try_from(width).ok() else {
        return 0;
    };
    let Some(raster_height) = usize::try_from(height).ok() else {
        return 0;
    };
    let Some(count) = raster_width.checked_mul(raster_height) else {
        return 0;
    };
    let actual_width = (*img).width as usize;
    let actual_height = (*img).height as usize;
    let Some(row_offset) = usize::try_from((*img).row_offset).ok() else {
        return 0;
    };
    let Some(col_offset) = usize::try_from((*img).col_offset).ok() else {
        return 0;
    };
    if row_offset >= actual_height || col_offset >= actual_width {
        emit_error_message(
            tif,
            "TIFFRGBAImageGet",
            "Requested RGBA window is outside the image",
        );
        return 0;
    }
    let read_width = min(raster_width, actual_width - col_offset);
    let read_height = min(raster_height, actual_height - row_offset);
    if read_width == 0 || read_height == 0 {
        emit_error_message(tif, "TIFFRGBAImageGet", "Requested RGBA window is empty");
        return 0;
    }
    let raster = slice::from_raw_parts_mut(raster, count);
    raster.fill(0);
    read_rgba_image_impl(
        img,
        raster,
        raster_width,
        raster_height,
        (*img).req_orientation,
        (*img).stoponerr,
    ) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn safe_tiff_rgba_image_end(img: *mut TIFFRGBAImage) {
    if img.is_null() {
        return;
    }
    free_rgba_conversion_state(img);
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
    safe_tiff_read_rgba_strip_ext(tif, row, raster, 0)
}

#[no_mangle]
pub unsafe extern "C" fn safe_tiff_read_rgba_strip_ext(
    tif: *mut TIFF,
    row: u32,
    raster: *mut u32,
    stop_on_error: c_int,
) -> c_int {
    if tif.is_null() || raster.is_null() {
        return 0;
    }
    if TIFFIsTiled(tif) != 0 {
        emit_error_message(
            tif,
            "TIFFReadRGBAStrip",
            "Can't use TIFFReadRGBAStrip() with tiled file.",
        );
        return 0;
    }

    let rows_per_strip = tag_u32(
        tif,
        TAG_ROWSPERSTRIP,
        true,
        tag_u32(tif, TAG_IMAGELENGTH, true, 0),
    )
    .max(1);
    if row % rows_per_strip != 0 {
        emit_error_message(
            tif,
            "TIFFReadRGBAStrip",
            "Row passed to TIFFReadRGBAStrip() must be first in a strip.",
        );
        return 0;
    }

    let mut img: TIFFRGBAImage = std::mem::zeroed();
    let mut emsg = [0 as c_char; 1024];
    if safe_tiff_rgba_image_begin(&mut img, tif, stop_on_error, emsg.as_mut_ptr()) == 0 {
        return 0;
    }
    if row >= img.height {
        emit_error_message(
            tif,
            "TIFFReadRGBAStrip",
            "Invalid row passed to TIFFReadRGBAStrip().",
        );
        safe_tiff_rgba_image_end(&mut img);
        return 0;
    }
    let Ok(row_offset) = i32::try_from(row) else {
        safe_tiff_rgba_image_end(&mut img);
        return 0;
    };
    img.row_offset = row_offset;
    img.col_offset = 0;
    let rows_to_read = min(rows_per_strip, img.height - row);
    let ok = safe_tiff_rgba_image_get(&mut img, raster, img.width, rows_to_read);
    safe_tiff_rgba_image_end(&mut img);
    ok
}

#[no_mangle]
pub unsafe extern "C" fn safe_tiff_read_rgba_tile(
    tif: *mut TIFF,
    x: u32,
    y: u32,
    raster: *mut u32,
) -> c_int {
    safe_tiff_read_rgba_tile_ext(tif, x, y, raster, 0)
}

#[no_mangle]
pub unsafe extern "C" fn safe_tiff_read_rgba_tile_ext(
    tif: *mut TIFF,
    x: u32,
    y: u32,
    raster: *mut u32,
    stop_on_error: c_int,
) -> c_int {
    if tif.is_null() || raster.is_null() {
        return 0;
    }
    if TIFFIsTiled(tif) == 0 {
        emit_error_message(
            tif,
            "TIFFReadRGBATile",
            "Can't use TIFFReadRGBATile() with striped file.",
        );
        return 0;
    }
    let tile_width = tag_u32(tif, TAG_TILEWIDTH, false, 0);
    let tile_length = tag_u32(tif, TAG_TILELENGTH, false, 0);
    if tile_width == 0 || tile_length == 0 {
        return 0;
    }
    if x % tile_width != 0 || y % tile_length != 0 {
        emit_error_message(
            tif,
            "TIFFReadRGBATile",
            "Row/col passed to TIFFReadRGBATile() must be topleft corner of a tile.",
        );
        return 0;
    }
    let tile_width = tile_width as usize;
    let tile_length = tile_length as usize;
    let Some(count) = tile_width.checked_mul(tile_length) else {
        return 0;
    };
    let out = slice::from_raw_parts_mut(raster, count);
    out.fill(0);
    let mut img: TIFFRGBAImage = std::mem::zeroed();
    let mut emsg = [0 as c_char; 1024];
    if safe_tiff_rgba_image_begin(&mut img, tif, stop_on_error, emsg.as_mut_ptr()) == 0 {
        return 0;
    }
    if x >= img.width || y >= img.height {
        emit_error_message(
            tif,
            "TIFFReadRGBATile",
            "Invalid row/col passed to TIFFReadRGBATile().",
        );
        safe_tiff_rgba_image_end(&mut img);
        return 0;
    }

    let read_width = min(tile_width as u32, img.width - x);
    let read_height = min(tile_length as u32, img.height - y);
    let Ok(row_offset) = i32::try_from(y) else {
        safe_tiff_rgba_image_end(&mut img);
        return 0;
    };
    let Ok(col_offset) = i32::try_from(x) else {
        safe_tiff_rgba_image_end(&mut img);
        return 0;
    };
    img.row_offset = row_offset;
    img.col_offset = col_offset;
    let ok = safe_tiff_rgba_image_get(&mut img, raster, read_width, read_height);
    safe_tiff_rgba_image_end(&mut img);
    if ok == 0 {
        return 0;
    }

    if read_width as usize == tile_width && read_height as usize == tile_length {
        return 1;
    }

    let read_width = read_width as usize;
    let read_height = read_height as usize;
    for i_row in 0..read_height {
        let src_start = (read_height - i_row - 1) * read_width;
        let dst_start = (tile_length - i_row - 1) * tile_width;
        out.copy_within(src_start..src_start + read_width, dst_start);
        out[dst_start + read_width..dst_start + tile_width].fill(0);
    }
    for i_row in read_height..tile_length {
        let dst_start = (tile_length - i_row - 1) * tile_width;
        out[dst_start..dst_start + tile_width].fill(0);
    }
    1
}
