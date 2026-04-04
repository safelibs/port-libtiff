use crate::abi::{TIFFCodec, TIFFDataType, TIFFInitMethod};
use crate::{
    stub_bool_method, stub_decoderow_method, stub_predecode_method, stub_void_method, TIFF,
};
use crate::strile::{
    TIFFSwabArrayOfDouble, TIFFSwabArrayOfLong, TIFFSwabArrayOfLong8, TIFFSwabArrayOfShort,
    TIFFSwabArrayOfTriples,
};
use fax::{maps, BitReader as FaxBitReader, BitWriter as FaxBitWriter, Color as FaxColor, VecWriter};
use flate2::{bufread::ZlibDecoder, write::ZlibEncoder, Compression as FlateCompression};
use libc::{c_char, c_int, c_void};
use std::collections::BTreeMap;
use std::ffi::CString;
use std::io::{Cursor, Read, Write};
use std::ptr;
use std::slice;
use std::sync::{Mutex, OnceLock};
use weezl::{
    decode::Configuration as LzwDecodeConfig,
    encode::Encoder as LzwEncoder,
    BitOrder, LzwStatus,
};

const COMPRESSION_NONE: u16 = 1;
const COMPRESSION_CCITTRLE: u16 = 2;
const COMPRESSION_CCITTFAX3: u16 = 3;
const COMPRESSION_CCITTFAX4: u16 = 4;
const COMPRESSION_LZW: u16 = 5;
const COMPRESSION_ADOBE_DEFLATE: u16 = 8;
const COMPRESSION_NEXT: u16 = 32766;
const COMPRESSION_CCITTRLEW: u16 = 32771;
const COMPRESSION_PACKBITS: u16 = 32773;
const COMPRESSION_THUNDERSCAN: u16 = 32809;
const COMPRESSION_DEFLATE: u16 = 32946;

const TAG_FAXMODE: u32 = 65536;
const TAG_ZIPQUALITY: u32 = 65557;
const TAG_DEFLATE_SUBCODEC: u32 = 65570;
const TAG_COMPRESSION: u32 = 259;
const TAG_BITSPERSAMPLE: u32 = 258;
const TAG_IMAGEWIDTH: u32 = 256;
const TAG_GROUP3OPTIONS: u32 = 292;
const TAG_PREDICTOR: u32 = 317;
const TAG_FILLORDER: u32 = 266;
const TAG_PHOTOMETRIC: u32 = 262;
const TAG_SAMPLESPERPIXEL: u32 = 277;
const TAG_SAMPLEFORMAT: u32 = 339;
const TAG_PLANARCONFIG: u32 = 284;

const FAXMODE_CLASSIC: i32 = 0;
const FAXMODE_NORTC: i32 = 0x0001;
const DEFLATE_SUBCODEC_ZLIB: i32 = 0;
const FILLORDER_MSB2LSB: u16 = 1;
const FILLORDER_LSB2MSB: u16 = 2;
const PHOTOMETRIC_MINISWHITE: u16 = 0;
const PHOTOMETRIC_MINISBLACK: u16 = 1;
const PLANARCONFIG_CONTIG: u16 = 1;
const SAMPLEFORMAT_IEEEFP: u16 = 3;
const PREDICTOR_NONE: u16 = 1;
const PREDICTOR_HORIZONTAL: u16 = 2;
const PREDICTOR_FLOATINGPOINT: u16 = 3;
const GROUP3OPT_FILLBITS: u32 = 0x0004;
const TIFF_FILLORDER_MASK: u32 = 0x00003;
const TIFF_DIRTYDIRECT: u32 = 0x00008;
const TIFF_SWAB: u32 = 0x00080;
const TIFF_NOBITREV: u32 = 0x00100;

const NAME_NONE: &[u8] = b"None\0";
const NAME_LZW: &[u8] = b"LZW\0";
const NAME_PACKBITS: &[u8] = b"PackBits\0";
const NAME_THUNDER: &[u8] = b"ThunderScan\0";
const NAME_NEXT: &[u8] = b"NeXT\0";
const NAME_JPEG: &[u8] = b"JPEG\0";
const NAME_OJPEG: &[u8] = b"Old-style JPEG\0";
const NAME_CCITT_RLE: &[u8] = b"CCITT RLE\0";
const NAME_CCITT_RLEW: &[u8] = b"CCITT RLE/W\0";
const NAME_CCITT_G3: &[u8] = b"CCITT Group 3\0";
const NAME_CCITT_G4: &[u8] = b"CCITT Group 4\0";
const NAME_DEFLATE: &[u8] = b"Deflate\0";
const NAME_ADOBE_DEFLATE: &[u8] = b"AdobeDeflate\0";

#[derive(Default)]
pub(crate) struct PendingStrileWrite {
    pub(crate) decoded: Vec<u8>,
    pub(crate) row_size: usize,
    pub(crate) rows: usize,
    pub(crate) width: u32,
}

#[derive(Default)]
pub(crate) struct DecodedStrileCache {
    pub(crate) is_tile: bool,
    pub(crate) index: u32,
    pub(crate) decoded: Vec<u8>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct CodecGeometry {
    pub(crate) row_size: usize,
    pub(crate) rows: usize,
    pub(crate) width: u32,
}

#[derive(Default)]
struct RawFaxDecoderState {
    rows: Vec<Vec<u8>>,
    next_row: usize,
    bytes: Vec<u8>,
    bit_pos: usize,
    width: u32,
    photometric: u16,
    memory_lsb: bool,
    ended: bool,
}

#[derive(Default)]
pub(crate) struct CodecState {
    pub(crate) active_scheme: u16,
    pub(crate) fax_mode: c_int,
    pub(crate) zip_quality: c_int,
    pub(crate) deflate_subcodec: c_int,
    pub(crate) pending_striles: BTreeMap<u32, PendingStrileWrite>,
    pub(crate) decoded_cache: Option<DecodedStrileCache>,
    raw_fax_decoder: Option<RawFaxDecoderState>,
}

struct RegisteredCodec {
    codec: TIFFCodec,
    name: CString,
}

unsafe impl Send for RegisteredCodec {}

struct CodecRegistry {
    codecs: Vec<*mut RegisteredCodec>,
}

unsafe impl Send for CodecRegistry {}

impl CodecRegistry {
    fn new() -> Self {
        Self { codecs: Vec::new() }
    }
}

fn registry() -> &'static Mutex<CodecRegistry> {
    static REGISTRY: OnceLock<Mutex<CodecRegistry>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(CodecRegistry::new()))
}

pub(crate) unsafe fn set_default_codec_methods(tif: *mut TIFF) {
    (*tif).tif_setupdecode = Some(stub_bool_method);
    (*tif).tif_predecode = Some(stub_predecode_method);
    (*tif).tif_decoderow = Some(stub_decoderow_method);
    (*tif).tif_close = Some(stub_void_method);
    (*tif).tif_cleanup = Some(stub_void_method);
    (*(*tif).inner).codec_state.raw_fax_decoder = None;
}

pub(crate) unsafe fn safe_tiff_codec_reset_for_current_directory(tif: *mut TIFF, scheme: u16) {
    let inner = (*tif).inner;
    (*inner).codec_state.active_scheme = scheme;
    (*inner).codec_state.decoded_cache = None;
    (*inner).codec_state.pending_striles.clear();
    (*inner).codec_state.fax_mode = FAXMODE_CLASSIC;
    (*inner).codec_state.zip_quality = 0;
    (*inner).codec_state.deflate_subcodec = DEFLATE_SUBCODEC_ZLIB;
    (*inner).codec_state.raw_fax_decoder = None;
}

pub(crate) unsafe fn safe_tiff_codec_set_scheme(tif: *mut TIFF, scheme: c_int) -> c_int {
    if tif.is_null() {
        return 0;
    }
    set_default_codec_methods(tif);
    safe_tiff_codec_reset_for_current_directory(tif, scheme as u16);
    let codec = TIFFFindCODEC(scheme as u16);
    if codec.is_null() {
        1
    } else if let Some(init) = (*codec).init {
        init(tif, scheme)
    } else {
        1
    }
}

fn is_pseudo_tag(tag: u32) -> bool {
    matches!(tag, TAG_FAXMODE | TAG_ZIPQUALITY | TAG_DEFLATE_SUBCODEC)
}

pub(crate) unsafe fn safe_tiff_codec_default_tag_value(
    tif: *mut TIFF,
    tag: u32,
    out_type: *mut TIFFDataType,
    out_count: *mut u64,
    out_data: *mut *const c_void,
) -> c_int {
    if tif.is_null() || !is_pseudo_tag(tag) {
        return 0;
    }
    safe_tiff_codec_get_tag_value(tif, tag, out_type, out_count, out_data)
}

pub(crate) unsafe fn safe_tiff_codec_get_tag_value(
    tif: *mut TIFF,
    tag: u32,
    out_type: *mut TIFFDataType,
    out_count: *mut u64,
    out_data: *mut *const c_void,
) -> c_int {
    if tif.is_null() || !is_pseudo_tag(tag) {
        return 0;
    }
    let state = &(*(*tif).inner).codec_state;
    *out_count = 1;
    match tag {
        TAG_FAXMODE => {
            *out_type = TIFFDataType::TIFF_SLONG;
            *out_data = (&state.fax_mode as *const c_int).cast();
            1
        }
        TAG_ZIPQUALITY => {
            *out_type = TIFFDataType::TIFF_SLONG;
            *out_data = (&state.zip_quality as *const c_int).cast();
            1
        }
        TAG_DEFLATE_SUBCODEC => {
            *out_type = TIFFDataType::TIFF_SLONG;
            *out_data = (&state.deflate_subcodec as *const c_int).cast();
            1
        }
        _ => 0,
    }
}

pub(crate) unsafe fn safe_tiff_codec_set_field_marshaled(
    tif: *mut TIFF,
    tag: u32,
    storage_type: TIFFDataType,
    count: u64,
    data: *const c_void,
) -> c_int {
    if tif.is_null() || !is_pseudo_tag(tag) || count != 1 || data.is_null() {
        return 0;
    }
    let state = &mut (*(*tif).inner).codec_state;
    let value = match storage_type.0 {
        x if x == TIFFDataType::TIFF_SLONG.0 => *data.cast::<c_int>(),
        x if x == TIFFDataType::TIFF_LONG.0 => i32::try_from(*data.cast::<u32>()).unwrap_or(0),
        x if x == TIFFDataType::TIFF_SHORT.0 => i32::from(*data.cast::<u16>()),
        _ => return 0,
    };
    match tag {
        TAG_FAXMODE => state.fax_mode = value,
        TAG_ZIPQUALITY => state.zip_quality = value,
        TAG_DEFLATE_SUBCODEC => state.deflate_subcodec = value,
        _ => return 0,
    }
    (*tif).tif_flags |= TIFF_DIRTYDIRECT;
    1
}

struct TrackingWriter {
    inner: VecWriter,
    bits_written: usize,
}

impl TrackingWriter {
    fn with_capacity(bits: usize) -> Self {
        Self {
            inner: VecWriter::with_capacity(bits),
            bits_written: 0,
        }
    }

    fn finish(self) -> Vec<u8> {
        self.inner.finish()
    }
}

impl FaxBitWriter for TrackingWriter {
    type Error = std::convert::Infallible;

    fn write(&mut self, bits: fax::Bits) -> Result<(), Self::Error> {
        self.bits_written += usize::from(bits.len);
        self.inner.write(bits)
    }
}

fn consume_expected<R: FaxBitReader>(reader: &mut R, bits: fax::Bits) -> bool {
    reader.expect(bits).is_ok() && reader.consume(bits.len).is_ok()
}

fn sync_to_eol(reader: &mut CcittBitReader<'_>, max_skip: usize) -> bool {
    if consume_expected(reader, maps::EOL) {
        return true;
    }
    for _ in 0..max_skip {
        if reader.peek(1) != Some(0) || reader.consume(1).is_err() {
            return false;
        }
        if consume_expected(reader, maps::EOL) {
            return true;
        }
    }
    false
}

pub(crate) unsafe fn safe_tiff_codec_unset_field(tif: *mut TIFF, tag: u32) -> c_int {
    if tif.is_null() || !is_pseudo_tag(tag) {
        return 0;
    }
    let state = &mut (*(*tif).inner).codec_state;
    match tag {
        TAG_FAXMODE => state.fax_mode = FAXMODE_CLASSIC,
        TAG_ZIPQUALITY => state.zip_quality = 0,
        TAG_DEFLATE_SUBCODEC => state.deflate_subcodec = DEFLATE_SUBCODEC_ZLIB,
        _ => return 0,
    }
    (*tif).tif_flags |= TIFF_DIRTYDIRECT;
    1
}

unsafe fn get_tag_raw(
    tif: *mut TIFF,
    tag: u32,
    defaulted: bool,
) -> Option<(TIFFDataType, usize, *const c_void)> {
    let mut type_ = TIFFDataType::TIFF_NOTYPE;
    let mut count = 0u64;
    let mut data: *const c_void = ptr::null();
    if super::directory::get_tag_value(tif, tag, defaulted, &mut type_, &mut count, &mut data) == 0 {
        return None;
    }
    Some((type_, usize::try_from(count).ok()?, data))
}

unsafe fn tag_u16(tif: *mut TIFF, tag: u32, defaulted: bool, default: u16) -> u16 {
    let Some((type_, count, data)) = get_tag_raw(tif, tag, defaulted) else {
        return default;
    };
    if count == 0 || data.is_null() {
        return default;
    }
    match type_.0 {
        x if x == TIFFDataType::TIFF_SHORT.0 => *data.cast::<u16>(),
        x if x == TIFFDataType::TIFF_LONG.0 => u16::try_from(*data.cast::<u32>()).unwrap_or(default),
        x if x == TIFFDataType::TIFF_SLONG.0 => u16::try_from(*data.cast::<i32>()).unwrap_or(default),
        _ => default,
    }
}

unsafe fn tag_u32(tif: *mut TIFF, tag: u32, defaulted: bool, default: u32) -> u32 {
    let Some((type_, count, data)) = get_tag_raw(tif, tag, defaulted) else {
        return default;
    };
    if count == 0 || data.is_null() {
        return default;
    }
    match type_.0 {
        x if x == TIFFDataType::TIFF_SHORT.0 => u32::from(*data.cast::<u16>()),
        x if x == TIFFDataType::TIFF_LONG.0 => *data.cast::<u32>(),
        x if x == TIFFDataType::TIFF_SLONG.0 => u32::try_from(*data.cast::<i32>()).unwrap_or(default),
        _ => default,
    }
}

unsafe fn active_scheme(tif: *mut TIFF) -> u16 {
    let state = &(*(*tif).inner).codec_state;
    if state.active_scheme != 0 {
        state.active_scheme
    } else {
        tag_u16(tif, TAG_COMPRESSION, true, COMPRESSION_NONE)
    }
}

unsafe fn predictor(tif: *mut TIFF) -> u16 {
    tag_u16(tif, TAG_PREDICTOR, true, PREDICTOR_NONE)
}

unsafe fn bits_per_sample(tif: *mut TIFF) -> u16 {
    tag_u16(tif, TAG_BITSPERSAMPLE, true, 1)
}

unsafe fn samples_per_pixel(tif: *mut TIFF) -> u16 {
    tag_u16(tif, TAG_SAMPLESPERPIXEL, true, 1)
}

unsafe fn sample_format(tif: *mut TIFF) -> u16 {
    tag_u16(tif, TAG_SAMPLEFORMAT, true, 1)
}

unsafe fn planar_config(tif: *mut TIFF) -> u16 {
    tag_u16(tif, TAG_PLANARCONFIG, true, PLANARCONFIG_CONTIG)
}

unsafe fn fill_order(tif: *mut TIFF) -> u16 {
    tag_u16(tif, TAG_FILLORDER, true, FILLORDER_MSB2LSB)
}

unsafe fn photometric(tif: *mut TIFF) -> u16 {
    tag_u16(tif, TAG_PHOTOMETRIC, true, PHOTOMETRIC_MINISWHITE)
}

unsafe fn memory_fillorder_lsb(tif: *mut TIFF) -> bool {
    ((*tif).tif_flags & TIFF_FILLORDER_MASK) == u32::from(FILLORDER_LSB2MSB)
}

unsafe fn should_reverse_bits(tif: *mut TIFF) -> bool {
    let order = fill_order(tif);
    (order == FILLORDER_MSB2LSB || order == FILLORDER_LSB2MSB)
        && ((*tif).tif_flags & TIFF_NOBITREV) == 0
        && ((*tif).tif_flags & u32::from(order)) == 0
}

fn reverse_byte(mut value: u8) -> u8 {
    let mut reversed = 0u8;
    for _ in 0..8 {
        reversed = (reversed << 1) | (value & 1);
        value >>= 1;
    }
    reversed
}

fn reverse_bits_in_place(bytes: &mut [u8]) {
    for byte in bytes {
        *byte = reverse_byte(*byte);
    }
}

unsafe fn apply_swab_in_place(tif: *mut TIFF, bytes: &mut [u8]) {
    if ((*tif).tif_flags & TIFF_SWAB) == 0 {
        return;
    }
    match bits_per_sample(tif) {
        8 => {}
        16 => TIFFSwabArrayOfShort(bytes.as_mut_ptr().cast::<u16>(), (bytes.len() / 2) as crate::Tmsize),
        24 => TIFFSwabArrayOfTriples(bytes.as_mut_ptr(), (bytes.len() / 3) as crate::Tmsize),
        32 => TIFFSwabArrayOfLong(bytes.as_mut_ptr().cast::<u32>(), (bytes.len() / 4) as crate::Tmsize),
        64 => {
            if sample_format(tif) == SAMPLEFORMAT_IEEEFP {
                TIFFSwabArrayOfDouble(bytes.as_mut_ptr().cast::<f64>(), (bytes.len() / 8) as crate::Tmsize)
            } else {
                TIFFSwabArrayOfLong8(bytes.as_mut_ptr().cast::<u64>(), (bytes.len() / 8) as crate::Tmsize)
            }
        }
        _ => {}
    }
}

fn sample_size_bytes(bits: u16) -> Option<usize> {
    match bits {
        8 => Some(1),
        16 => Some(2),
        24 => Some(3),
        32 => Some(4),
        64 => Some(8),
        _ => None,
    }
}

fn load_native_sample(bytes: &[u8]) -> Option<u64> {
    match bytes.len() {
        1 => Some(u64::from(bytes[0])),
        2 => Some(u64::from(u16::from_ne_bytes([bytes[0], bytes[1]]))),
        4 => Some(u64::from(u32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))),
        8 => Some(u64::from_ne_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ])),
        _ => None,
    }
}

fn store_native_sample(bytes: &mut [u8], value: u64) -> bool {
    match bytes.len() {
        1 => bytes[0] = value as u8,
        2 => bytes.copy_from_slice(&(value as u16).to_ne_bytes()),
        4 => bytes.copy_from_slice(&(value as u32).to_ne_bytes()),
        8 => bytes.copy_from_slice(&value.to_ne_bytes()),
        _ => return false,
    }
    true
}

fn horizontal_accumulate(bytes: &mut [u8], rowsize: usize, stride: usize, sample_bytes: usize) -> bool {
    if rowsize == 0 || bytes.len() % rowsize != 0 || rowsize % sample_bytes != 0 || stride == 0 {
        return false;
    }
    for row in bytes.chunks_exact_mut(rowsize) {
        let samples = row.len() / sample_bytes;
        for index in stride..samples {
            let prev_offset = (index - stride) * sample_bytes;
            let curr_offset = index * sample_bytes;
            let Some(prev) = load_native_sample(&row[prev_offset..prev_offset + sample_bytes]) else {
                return false;
            };
            let Some(curr) = load_native_sample(&row[curr_offset..curr_offset + sample_bytes]) else {
                return false;
            };
            if !store_native_sample(
                &mut row[curr_offset..curr_offset + sample_bytes],
                curr.wrapping_add(prev),
            ) {
                return false;
            }
        }
    }
    true
}

fn horizontal_differentiate(bytes: &mut [u8], rowsize: usize, stride: usize, sample_bytes: usize) -> bool {
    if rowsize == 0 || bytes.len() % rowsize != 0 || rowsize % sample_bytes != 0 || stride == 0 {
        return false;
    }
    for row in bytes.chunks_exact_mut(rowsize) {
        let samples = row.len() / sample_bytes;
        for index in (stride..samples).rev() {
            let prev_offset = (index - stride) * sample_bytes;
            let curr_offset = index * sample_bytes;
            let Some(prev) = load_native_sample(&row[prev_offset..prev_offset + sample_bytes]) else {
                return false;
            };
            let Some(curr) = load_native_sample(&row[curr_offset..curr_offset + sample_bytes]) else {
                return false;
            };
            if !store_native_sample(
                &mut row[curr_offset..curr_offset + sample_bytes],
                curr.wrapping_sub(prev),
            ) {
                return false;
            }
        }
    }
    true
}

fn floating_accumulate(bytes: &mut [u8], rowsize: usize, stride: usize, sample_bytes: usize) -> bool {
    if rowsize == 0 || bytes.len() % rowsize != 0 || rowsize % (sample_bytes * stride) != 0 {
        return false;
    }
    let little_endian = cfg!(target_endian = "little");
    for row in bytes.chunks_exact_mut(rowsize) {
        for index in stride..row.len() {
            row[index] = row[index].wrapping_add(row[index - stride]);
        }
        let shuffled = row.to_vec();
        let samples = row.len() / sample_bytes;
        for sample_index in 0..samples {
            for byte_index in 0..sample_bytes {
                let plane = if little_endian {
                    sample_bytes - byte_index - 1
                } else {
                    byte_index
                };
                row[sample_index * sample_bytes + byte_index] =
                    shuffled[plane * samples + sample_index];
            }
        }
    }
    true
}

fn floating_differentiate(bytes: &mut [u8], rowsize: usize, stride: usize, sample_bytes: usize) -> bool {
    if rowsize == 0 || bytes.len() % rowsize != 0 || rowsize % (sample_bytes * stride) != 0 {
        return false;
    }
    let little_endian = cfg!(target_endian = "little");
    for row in bytes.chunks_exact_mut(rowsize) {
        let original = row.to_vec();
        let samples = row.len() / sample_bytes;
        for sample_index in 0..samples {
            for byte_index in 0..sample_bytes {
                let plane = if little_endian {
                    sample_bytes - byte_index - 1
                } else {
                    byte_index
                };
                row[plane * samples + sample_index] =
                    original[sample_index * sample_bytes + byte_index];
            }
        }
        for index in (stride..row.len()).rev() {
            row[index] = row[index].wrapping_sub(row[index - stride]);
        }
    }
    true
}

unsafe fn predictor_stride(tif: *mut TIFF) -> usize {
    if planar_config(tif) == PLANARCONFIG_CONTIG {
        usize::from(samples_per_pixel(tif).max(1))
    } else {
        1
    }
}

unsafe fn decode_predictor_bytes(
    tif: *mut TIFF,
    geometry: CodecGeometry,
    bytes: &mut [u8],
) -> bool {
    let predictor = predictor(tif);
    if active_scheme(tif) == COMPRESSION_CCITTRLE
        || active_scheme(tif) == COMPRESSION_CCITTRLEW
        || active_scheme(tif) == COMPRESSION_CCITTFAX3
        || active_scheme(tif) == COMPRESSION_CCITTFAX4
    {
        return true;
    }
    if should_reverse_bits(tif) {
        reverse_bits_in_place(bytes);
    }
    match predictor {
        PREDICTOR_NONE => {
            apply_swab_in_place(tif, bytes);
            true
        }
        PREDICTOR_HORIZONTAL => {
            let Some(sample_bytes) = sample_size_bytes(bits_per_sample(tif)) else {
                return false;
            };
            apply_swab_in_place(tif, bytes);
            horizontal_accumulate(bytes, geometry.row_size, predictor_stride(tif), sample_bytes)
        }
        PREDICTOR_FLOATINGPOINT => {
            if sample_format(tif) != SAMPLEFORMAT_IEEEFP {
                return false;
            }
            let Some(sample_bytes) = sample_size_bytes(bits_per_sample(tif)) else {
                return false;
            };
            floating_accumulate(bytes, geometry.row_size, predictor_stride(tif), sample_bytes)
        }
        _ => false,
    }
}

unsafe fn encode_predictor_bytes(
    tif: *mut TIFF,
    geometry: CodecGeometry,
    input: &[u8],
) -> Option<Vec<u8>> {
    let predictor = predictor(tif);
    let mut bytes = input.to_vec();
    match predictor {
        PREDICTOR_NONE => {
            apply_swab_in_place(tif, &mut bytes);
        }
        PREDICTOR_HORIZONTAL => {
            let sample_bytes = sample_size_bytes(bits_per_sample(tif))?;
            if !horizontal_differentiate(&mut bytes, geometry.row_size, predictor_stride(tif), sample_bytes) {
                return None;
            }
            apply_swab_in_place(tif, &mut bytes);
        }
        PREDICTOR_FLOATINGPOINT => {
            if sample_format(tif) != SAMPLEFORMAT_IEEEFP {
                return None;
            }
            let sample_bytes = sample_size_bytes(bits_per_sample(tif))?;
            if !floating_differentiate(&mut bytes, geometry.row_size, predictor_stride(tif), sample_bytes) {
                return None;
            }
        }
        _ => return None,
    }
    if should_reverse_bits(tif) {
        reverse_bits_in_place(&mut bytes);
    }
    Some(bytes)
}

fn decode_packbits(input: &[u8], expected_size: usize) -> Option<Vec<u8>> {
    let mut output = Vec::with_capacity(expected_size);
    let mut index = 0usize;
    while index < input.len() && output.len() < expected_size {
        let control = input[index] as i8;
        index += 1;
        if control >= 0 {
            let literal_len = usize::from(control as u8) + 1;
            let end = index.checked_add(literal_len)?;
            if end > input.len() {
                return None;
            }
            output.extend_from_slice(&input[index..end]);
            index = end;
        } else if control != -128 {
            let run_len = usize::from(control.unsigned_abs()) + 1;
            let value = *input.get(index)?;
            index += 1;
            output.resize(output.len().checked_add(run_len)?, value);
        }
    }
    (output.len() >= expected_size).then(|| {
        output.truncate(expected_size);
        output
    })
}

fn encode_packbits_row(row: &[u8], output: &mut Vec<u8>) {
    let mut index = 0usize;
    while index < row.len() {
        let mut run_len = 1usize;
        while index + run_len < row.len() && row[index] == row[index + run_len] && run_len < 128 {
            run_len += 1;
        }
        if run_len >= 3 {
            output.push((1i16 - run_len as i16) as u8);
            output.push(row[index]);
            index += run_len;
            continue;
        }

        let literal_start = index;
        index += run_len;
        while index < row.len() {
            let mut next_run = 1usize;
            while index + next_run < row.len()
                && row[index] == row[index + next_run]
                && next_run < 128
            {
                next_run += 1;
            }
            if next_run >= 3 || index - literal_start + next_run > 128 {
                break;
            }
            index += next_run;
        }

        let literal_len = index - literal_start;
        output.push((literal_len - 1) as u8);
        output.extend_from_slice(&row[literal_start..index]);
    }
}

fn encode_packbits(input: &[u8], row_size: usize) -> Vec<u8> {
    let mut output = Vec::with_capacity(input.len());
    if row_size == 0 {
        encode_packbits_row(input, &mut output);
        return output;
    }
    for row in input.chunks(row_size) {
        encode_packbits_row(row, &mut output);
    }
    output
}

fn decode_lzw(input: &[u8], expected_size: usize) -> Option<Vec<u8>> {
    let attempts = [
        (BitOrder::Msb, true),
        (BitOrder::Msb, false),
        (BitOrder::Lsb, true),
        (BitOrder::Lsb, false),
    ];
    for (order, tiff_size_switch) in attempts {
        let config = if tiff_size_switch {
            LzwDecodeConfig::with_tiff_size_switch(order, 8)
        } else {
            LzwDecodeConfig::new(order, 8)
        }
        .with_yield_on_full_buffer(true);
        let mut decoder = config.build();
        let mut output = vec![0u8; expected_size];
        let mut input_offset = 0usize;
        let mut output_offset = 0usize;
        loop {
            let result = decoder.decode_bytes(&input[input_offset..], &mut output[output_offset..]);
            input_offset += result.consumed_in;
            output_offset += result.consumed_out;
            if output_offset == expected_size {
                return Some(output);
            }
            if result.consumed_in == 0 && result.consumed_out == 0 {
                break;
            }
            match result.status {
                Ok(LzwStatus::Ok) => {}
                Ok(LzwStatus::Done) | Ok(LzwStatus::NoProgress) | Err(_) => break,
            }
        }
    }
    None
}

fn encode_lzw(input: &[u8]) -> Option<Vec<u8>> {
    LzwEncoder::with_tiff_size_switch(BitOrder::Msb, 8)
        .encode(input)
        .ok()
}

fn decode_deflate(input: &[u8]) -> Option<Vec<u8>> {
    let mut decoder = ZlibDecoder::new(Cursor::new(input));
    let mut output = Vec::new();
    decoder.read_to_end(&mut output).ok()?;
    Some(output)
}

fn encode_deflate(input: &[u8]) -> Option<Vec<u8>> {
    let mut encoder = ZlibEncoder::new(Vec::new(), FlateCompression::default());
    encoder.write_all(input).ok()?;
    encoder.finish().ok()
}

struct CcittBitReader<'a> {
    bytes: &'a [u8],
    bit_pos: usize,
}

impl<'a> CcittBitReader<'a> {
    fn new(bytes: &'a [u8], bit_pos: usize) -> Self {
        Self { bytes, bit_pos }
    }
}

impl FaxBitReader for CcittBitReader<'_> {
    type Error = ();

    fn peek(&self, bits: u8) -> Option<u16> {
        if bits > 16 || self.bit_pos.checked_add(bits as usize)? > self.bytes.len() * 8 {
            return None;
        }
        let mut value = 0u16;
        for bit_index in 0..usize::from(bits) {
            let absolute = self.bit_pos + bit_index;
            let byte = self.bytes[absolute / 8];
            let shift = 7 - (absolute % 8);
            value = (value << 1) | u16::from((byte >> shift) & 1);
        }
        Some(value)
    }

    fn consume(&mut self, bits: u8) -> Result<(), Self::Error> {
        self.bit_pos = self.bit_pos.checked_add(bits as usize).ok_or(())?;
        if self.bit_pos > self.bytes.len() * 8 {
            Err(())
        } else {
            Ok(())
        }
    }

    fn bits_to_byte_boundary(&self) -> u8 {
        ((8 - (self.bit_pos % 8)) % 8) as u8
    }
}

fn decode_fax_run(reader: &mut CcittBitReader<'_>, color: FaxColor) -> Option<u16> {
    let mut total = 0u16;
    loop {
        let part = match color {
            FaxColor::White => maps::white::decode(reader)?,
            FaxColor::Black => maps::black::decode(reader)?,
        };
        total = total.checked_add(part)?;
        if part < 64 {
            return Some(total);
        }
    }
}

fn decode_fax_1d_line(reader: &mut CcittBitReader<'_>, width: u16) -> Option<Vec<u16>> {
    let mut transitions = Vec::new();
    let mut current = FaxColor::White;
    let mut x = 0u16;
    let mut runs = 0usize;
    while x < width {
        let Some(run) = decode_fax_run(reader, current) else {
            return Some(transitions);
        };
        runs += 1;
        if runs > usize::from(width).saturating_mul(4).saturating_add(64) {
            return None;
        }
        x = x.checked_add(run)?;
        if x > width {
            return None;
        }
        if x < width {
            transitions.push(x);
            current = !current;
        }
    }
    Some(transitions)
}

fn fill_to_boundary(bits: &mut TrackingWriter, boundary: u8) {
    while boundary != 0 && (bits.bits_written % usize::from(boundary)) != 0 {
        let _ = bits.write(fax::Bits { data: 0, len: 1 });
    }
}

fn row_pixel_color(row: &[u8], x: u32, photometric: u16, lsb_first: bool) -> FaxColor {
    let byte = row[(x / 8) as usize];
    let shift = if lsb_first { x % 8 } else { 7 - (x % 8) };
    let bit = ((byte >> shift) & 1) != 0;
    match photometric {
        PHOTOMETRIC_MINISBLACK => {
            if bit {
                FaxColor::White
            } else {
                FaxColor::Black
            }
        }
        _ => {
            if bit {
                FaxColor::Black
            } else {
                FaxColor::White
            }
        }
    }
}

fn build_fax_transitions(row: &[u8], width: u32, photometric: u16, lsb_first: bool) -> Vec<u16> {
    let mut transitions = Vec::new();
    let mut current = FaxColor::White;
    for x in 0..width {
        let color = row_pixel_color(row, x, photometric, lsb_first);
        if color != current {
            transitions.push(x as u16);
            current = color;
        }
    }
    transitions
}

fn encode_run_length(bits: &mut TrackingWriter, color: FaxColor, mut run: u16) -> bool {
    let table = match color {
        FaxColor::White => &maps::white::ENTRIES,
        FaxColor::Black => &maps::black::ENTRIES,
    };
    let mut write_entry = |n: u16| {
        let index = if n >= 64 { 63 + n / 64 } else { n } as usize;
        let Some(&(value, code)) = table.get(index) else {
            return false;
        };
        value == n && bits.write(code).is_ok()
    };
    while run >= 2560 {
        if !write_entry(2560) {
            return false;
        }
        run -= 2560;
    }
    if run >= 64 {
        let makeup = run & !63;
        if !write_entry(makeup) {
            return false;
        }
        run -= makeup;
    }
    write_entry(run)
}

fn encode_fax_1d_row(bits: &mut TrackingWriter, row: &[u8], width: u32, photometric: u16, lsb_first: bool) -> bool {
    let transitions = build_fax_transitions(row, width, photometric, lsb_first);
    let mut current = FaxColor::White;
    let mut start = 0u16;
    for &stop in &transitions {
        if !encode_run_length(bits, current, stop.saturating_sub(start)) {
            return false;
        }
        start = stop;
        current = !current;
    }
    encode_run_length(bits, current, width.saturating_sub(u32::from(start)) as u16)
}

fn pack_fax_row(
    row: &mut [u8],
    width: u32,
    transitions: &[u16],
    photometric: u16,
    lsb_first: bool,
) {
    let white_bit = photometric == PHOTOMETRIC_MINISBLACK;
    row.fill(if white_bit { 0xff } else { 0x00 });
    let mut current = FaxColor::White;
    let mut transition_index = 0usize;
    for x in 0..width {
        while transition_index < transitions.len() && u32::from(transitions[transition_index]) == x {
            current = !current;
            transition_index += 1;
        }
        let black = current == FaxColor::Black;
        let bit = match photometric {
            PHOTOMETRIC_MINISBLACK => !black,
            _ => black,
        };
        let byte_index = (x / 8) as usize;
        let shift = if lsb_first { x % 8 } else { 7 - (x % 8) };
        if bit {
            row[byte_index] |= 1 << shift;
        } else {
            row[byte_index] &= !(1 << shift);
        }
    }
}

fn group3_fillbits(tif: *mut TIFF) -> bool {
    unsafe { (tag_u32(tif, TAG_GROUP3OPTIONS, true, 0) & GROUP3OPT_FILLBITS) != 0 }
}

fn prepare_fax_input(tif: *mut TIFF, input: &[u8]) -> Vec<u8> {
    let mut bytes = input.to_vec();
    if unsafe { fill_order(tif) } == FILLORDER_LSB2MSB {
        reverse_bits_in_place(&mut bytes);
    }
    bytes
}

fn finalize_fax_output(tif: *mut TIFF, output: &mut Vec<u8>) {
    if unsafe { fill_order(tif) } == FILLORDER_LSB2MSB {
        reverse_bits_in_place(output);
    }
}

unsafe fn decode_group3_1d(
    tif: *mut TIFF,
    input: &[u8],
    geometry: CodecGeometry,
) -> Option<Vec<u8>> {
    let row_size = geometry.row_size.checked_mul(geometry.rows)?;
    if let Some(rows) = decode_group3_rows(tif, input, geometry.width) {
        if rows.len() < geometry.rows {
            return None;
        }
        let mut output = vec![0u8; row_size];
        for (row_index, row) in rows.into_iter().take(geometry.rows).enumerate() {
            if row.len() != geometry.row_size {
                return None;
            }
            let start = row_index.checked_mul(geometry.row_size)?;
            output[start..start + geometry.row_size].copy_from_slice(&row);
        }
        return Some(output);
    }
    let bytes = prepare_fax_input(tif, input);
    let mut reader = CcittBitReader::new(&bytes, 0);
    let mut output = vec![0u8; row_size];
    for row_index in 0..geometry.rows {
        if !sync_to_eol(&mut reader, 16) {
            return None;
        }
        let transitions = decode_fax_1d_line(&mut reader, geometry.width as u16)?;
        let start = row_index.checked_mul(geometry.row_size)?;
        pack_fax_row(
            &mut output[start..start + geometry.row_size],
            geometry.width,
            &transitions,
            photometric(tif),
            memory_fillorder_lsb(tif),
        );
    }
    Some(output)
}

unsafe fn decode_group3_rows(tif: *mut TIFF, input: &[u8], width: u32) -> Option<Vec<Vec<u8>>> {
    let row_size = usize::try_from((width + 7) / 8).ok()?;
    let bytes = prepare_fax_input(tif, input);
    let mut reader = CcittBitReader::new(&bytes, 0);
    let mut rows = Vec::new();
    loop {
        if !sync_to_eol(&mut reader, 16) {
            return Some(rows);
        }
        let Some(transitions) = decode_fax_1d_line(&mut reader, width as u16) else {
            return Some(rows);
        };
        let mut row = vec![0u8; row_size];
        pack_fax_row(
            &mut row,
            width,
            &transitions,
            photometric(tif),
            memory_fillorder_lsb(tif),
        );
        rows.push(row);
    }
}

unsafe fn decode_group4(
    tif: *mut TIFF,
    input: &[u8],
    geometry: CodecGeometry,
) -> Option<Vec<u8>> {
    let bytes = prepare_fax_input(tif, input);
    let mut output = vec![0u8; geometry.row_size.checked_mul(geometry.rows)?];
    let mut row_index = 0usize;
    fax::decoder::decode_g4(bytes.iter().copied(), geometry.width as u16, Some(geometry.rows as u16), |transitions| {
        if row_index >= geometry.rows {
            return;
        }
        let start = row_index * geometry.row_size;
        pack_fax_row(
            &mut output[start..start + geometry.row_size],
            geometry.width,
            transitions,
            unsafe { photometric(tif) },
            unsafe { memory_fillorder_lsb(tif) },
        );
        row_index += 1;
    })?;
    (row_index == geometry.rows).then_some(output)
}

unsafe fn encode_group3_1d(
    tif: *mut TIFF,
    input: &[u8],
    geometry: CodecGeometry,
) -> Option<Vec<u8>> {
    let mut bits = TrackingWriter::with_capacity(input.len().checked_mul(12)?);
    for row_index in 0..geometry.rows {
        let start = row_index.checked_mul(geometry.row_size)?;
        if group3_fillbits(tif) {
            fill_to_boundary(&mut bits, 8);
        }
        bits.write(maps::EOL).ok()?;
        if !encode_fax_1d_row(
            &mut bits,
            &input[start..start + geometry.row_size],
            geometry.width,
            photometric(tif),
            memory_fillorder_lsb(tif),
        ) {
            return None;
        }
    }
    if (*(*tif).inner).codec_state.fax_mode & FAXMODE_NORTC == 0 {
        for _ in 0..6 {
            if group3_fillbits(tif) {
                fill_to_boundary(&mut bits, 8);
            }
            bits.write(maps::EOL).ok()?;
        }
    }
    let mut output = bits.finish();
    finalize_fax_output(tif, &mut output);
    Some(output)
}

unsafe fn encode_group4(
    tif: *mut TIFF,
    input: &[u8],
    geometry: CodecGeometry,
) -> Option<Vec<u8>> {
    let mut encoder =
        fax::encoder::Encoder::new(TrackingWriter::with_capacity(input.len().checked_mul(8)?));
    for row_index in 0..geometry.rows {
        let start = row_index.checked_mul(geometry.row_size)?;
        let row = &input[start..start + geometry.row_size];
        encoder
            .encode_line(
                (0..geometry.width).map(|x| {
                    row_pixel_color(row, x, photometric(tif), memory_fillorder_lsb(tif))
                }),
                geometry.width as u16,
            )
            .ok()?;
    }
    let writer = encoder.finish().ok()?;
    let mut output = writer.finish();
    finalize_fax_output(tif, &mut output);
    Some(output)
}

pub(crate) unsafe fn safe_tiff_codec_decode_bytes(
    tif: *mut TIFF,
    input: &[u8],
    geometry: CodecGeometry,
    expected_size: usize,
) -> Option<Vec<u8>> {
    let mut decoded = match active_scheme(tif) {
        COMPRESSION_NONE => input.get(..expected_size)?.to_vec(),
        COMPRESSION_PACKBITS => decode_packbits(input, expected_size)?,
        COMPRESSION_LZW => decode_lzw(input, expected_size)?,
        COMPRESSION_DEFLATE | COMPRESSION_ADOBE_DEFLATE => decode_deflate(input)?,
        COMPRESSION_CCITTRLE | COMPRESSION_CCITTRLEW | COMPRESSION_CCITTFAX3 => {
            decode_group3_1d(tif, input, geometry)?
        }
        COMPRESSION_CCITTFAX4 => decode_group4(tif, input, geometry)?,
        _ => return None,
    };
    if active_scheme(tif) != COMPRESSION_CCITTRLE
        && active_scheme(tif) != COMPRESSION_CCITTRLEW
        && active_scheme(tif) != COMPRESSION_CCITTFAX3
        && active_scheme(tif) != COMPRESSION_CCITTFAX4
    {
        if !decode_predictor_bytes(tif, geometry, &mut decoded) {
            return None;
        }
    }
    if decoded.len() < expected_size {
        return None;
    }
    decoded.truncate(expected_size);
    Some(decoded)
}

pub(crate) unsafe fn safe_tiff_codec_encode_bytes(
    tif: *mut TIFF,
    input: &[u8],
    geometry: CodecGeometry,
) -> Option<Vec<u8>> {
    match active_scheme(tif) {
        COMPRESSION_NONE => encode_predictor_bytes(tif, geometry, input),
        COMPRESSION_PACKBITS => {
            encode_predictor_bytes(tif, geometry, input).map(|bytes| encode_packbits(&bytes, geometry.row_size))
        }
        COMPRESSION_LZW => encode_lzw(&encode_predictor_bytes(tif, geometry, input)?),
        COMPRESSION_DEFLATE | COMPRESSION_ADOBE_DEFLATE => {
            encode_deflate(&encode_predictor_bytes(tif, geometry, input)?)
        }
        COMPRESSION_CCITTRLE | COMPRESSION_CCITTRLEW | COMPRESSION_CCITTFAX3 => {
            encode_group3_1d(tif, input, geometry)
        }
        COMPRESSION_CCITTFAX4 => encode_group4(tif, input, geometry),
        _ => None,
    }
}

fn group3_eol_is_available(bytes: &[u8], bit_pos: usize, allow_fillbits: bool) -> bool {
    let mut reader = CcittBitReader::new(bytes, bit_pos);
    let max_skip = if allow_fillbits { 16 } else { 12 };
    sync_to_eol(&mut reader, max_skip)
}

fn group3_rtc_is_available(bytes: &[u8], bit_pos: usize, allow_fillbits: bool) -> bool {
    let mut reader = CcittBitReader::new(bytes, bit_pos);
    for _ in 0..6 {
        if consume_expected(&mut reader, maps::EOL) {
            continue;
        }
        if !allow_fillbits {
            return false;
        }
        while reader.bits_to_byte_boundary() != 0 {
            if reader.peek(1) != Some(0) || reader.consume(1).is_err() {
                return false;
            }
        }
        if !consume_expected(&mut reader, maps::EOL) {
            return false;
        }
    }
    true
}

unsafe fn raw_group3_predecode(tif: *mut TIFF) -> c_int {
    if (*tif).tif_rawdata.is_null() || (*tif).tif_rawcc <= 0 {
        return 0;
    }
    let raw = slice::from_raw_parts((*tif).tif_rawdata, (*tif).tif_rawcc as usize);
    let width = tag_u32(tif, TAG_IMAGEWIDTH, true, 0);
    let rows = decode_group3_rows(tif, raw, width).unwrap_or_default();
    (*(*tif).inner).codec_state.raw_fax_decoder = Some(RawFaxDecoderState {
        rows,
        next_row: 0,
        bytes: prepare_fax_input(tif, raw),
        bit_pos: 0,
        width,
        photometric: photometric(tif),
        memory_lsb: memory_fillorder_lsb(tif),
        ended: false,
    });
    1
}

unsafe extern "C" fn fax3_setupdecode(_: *mut TIFF) -> c_int {
    1
}

unsafe extern "C" fn fax3_predecode(tif: *mut TIFF, _: u16) -> c_int {
    raw_group3_predecode(tif)
}

unsafe extern "C" fn fax3_decoderow(
    tif: *mut TIFF,
    buf: *mut u8,
    cc: crate::Tmsize,
    _: u16,
) -> c_int {
    if tif.is_null() || buf.is_null() || cc <= 0 {
        return 0;
    }
    let state = &mut (*(*tif).inner).codec_state;
    let Some(raw) = state.raw_fax_decoder.as_mut() else {
        return 0;
    };
    if raw.next_row < raw.rows.len() {
        let row = &raw.rows[raw.next_row];
        if row.len() > cc as usize {
            (*tif).tif_rawcc = 0;
            raw.ended = true;
            return 0;
        }
        slice::from_raw_parts_mut(buf, row.len()).copy_from_slice(row);
        raw.next_row += 1;
        raw.ended = raw.next_row >= raw.rows.len();
        (*tif).tif_rawcp = (*tif).tif_rawdata;
        (*tif).tif_rawcc = if raw.ended { 0 } else { 1 };
        return 1;
    }
    if raw.ended || raw.width == 0 {
        (*tif).tif_rawcc = 0;
        return 0;
    }
    let row_size = ((raw.width + 7) / 8) as usize;
    if row_size > cc as usize {
        raw.ended = true;
        (*tif).tif_rawcc = 0;
        return 0;
    }
    let mut reader = CcittBitReader::new(&raw.bytes, raw.bit_pos);
    if !sync_to_eol(&mut reader, 16) {
        raw.ended = true;
        (*tif).tif_rawcc = 0;
        return 0;
    }
    let Some(transitions) = decode_fax_1d_line(&mut reader, raw.width as u16) else {
        raw.ended = true;
        (*tif).tif_rawcc = 0;
        return 0;
    };
    let out = slice::from_raw_parts_mut(buf, row_size);
    pack_fax_row(out, raw.width, &transitions, raw.photometric, raw.memory_lsb);
    raw.bit_pos = reader.bit_pos;
    raw.ended = group3_rtc_is_available(&raw.bytes, raw.bit_pos, group3_fillbits(tif))
        || !group3_eol_is_available(&raw.bytes, raw.bit_pos, group3_fillbits(tif));
    let consumed_bytes = raw.bit_pos / 8;
    (*tif).tif_rawcp = (*tif).tif_rawdata.add(consumed_bytes.min((*tif).tif_rawcc.max(0) as usize));
    (*tif).tif_rawcc = if raw.ended {
        0
    } else {
        raw.bytes.len().saturating_sub(consumed_bytes) as crate::Tmsize
    };
    1
}

unsafe extern "C" fn init_dump_mode(_: *mut TIFF, _: c_int) -> c_int {
    1
}

unsafe extern "C" fn init_simple_codec(tif: *mut TIFF, scheme: c_int) -> c_int {
    if !tif.is_null() && scheme as u16 == COMPRESSION_CCITTFAX3 {
        (*tif).tif_setupdecode = Some(fax3_setupdecode);
        (*tif).tif_predecode = Some(fax3_predecode);
        (*tif).tif_decoderow = Some(fax3_decoderow);
    }
    1
}

unsafe extern "C" fn init_not_configured(_: *mut TIFF, _: c_int) -> c_int {
    1
}

fn is_configured_init(init: TIFFInitMethod) -> bool {
    init.is_some()
}

fn builtin_codec_configured(scheme: u16) -> bool {
    matches!(
        scheme,
        COMPRESSION_NONE
            | COMPRESSION_LZW
            | COMPRESSION_PACKBITS
            | COMPRESSION_CCITTRLE
            | COMPRESSION_CCITTRLEW
            | COMPRESSION_CCITTFAX3
            | COMPRESSION_CCITTFAX4
            | COMPRESSION_DEFLATE
            | COMPRESSION_ADOBE_DEFLATE
    )
}

static BUILTIN_CODECS: [TIFFCodec; 14] = [
    TIFFCodec {
        name: NAME_NONE.as_ptr() as *mut c_char,
        scheme: COMPRESSION_NONE,
        init: Some(init_dump_mode),
    },
    TIFFCodec {
        name: NAME_LZW.as_ptr() as *mut c_char,
        scheme: COMPRESSION_LZW,
        init: Some(init_simple_codec),
    },
    TIFFCodec {
        name: NAME_PACKBITS.as_ptr() as *mut c_char,
        scheme: COMPRESSION_PACKBITS,
        init: Some(init_simple_codec),
    },
    TIFFCodec {
        name: NAME_THUNDER.as_ptr() as *mut c_char,
        scheme: COMPRESSION_THUNDERSCAN,
        init: Some(init_not_configured),
    },
    TIFFCodec {
        name: NAME_NEXT.as_ptr() as *mut c_char,
        scheme: COMPRESSION_NEXT,
        init: Some(init_not_configured),
    },
    TIFFCodec {
        name: NAME_JPEG.as_ptr() as *mut c_char,
        scheme: 7,
        init: Some(init_not_configured),
    },
    TIFFCodec {
        name: NAME_OJPEG.as_ptr() as *mut c_char,
        scheme: 6,
        init: Some(init_not_configured),
    },
    TIFFCodec {
        name: NAME_CCITT_RLE.as_ptr() as *mut c_char,
        scheme: COMPRESSION_CCITTRLE,
        init: Some(init_simple_codec),
    },
    TIFFCodec {
        name: NAME_CCITT_RLEW.as_ptr() as *mut c_char,
        scheme: COMPRESSION_CCITTRLEW,
        init: Some(init_simple_codec),
    },
    TIFFCodec {
        name: NAME_CCITT_G3.as_ptr() as *mut c_char,
        scheme: COMPRESSION_CCITTFAX3,
        init: Some(init_simple_codec),
    },
    TIFFCodec {
        name: NAME_CCITT_G4.as_ptr() as *mut c_char,
        scheme: COMPRESSION_CCITTFAX4,
        init: Some(init_simple_codec),
    },
    TIFFCodec {
        name: NAME_DEFLATE.as_ptr() as *mut c_char,
        scheme: COMPRESSION_DEFLATE,
        init: Some(init_simple_codec),
    },
    TIFFCodec {
        name: NAME_ADOBE_DEFLATE.as_ptr() as *mut c_char,
        scheme: COMPRESSION_ADOBE_DEFLATE,
        init: Some(init_simple_codec),
    },
    TIFFCodec {
        name: ptr::null_mut(),
        scheme: 0,
        init: None,
    },
];

#[no_mangle]
pub unsafe extern "C" fn TIFFFindCODEC(scheme: u16) -> *const TIFFCodec {
    let registry = registry().lock().expect("codec registry lock");
    for entry in registry.codecs.iter().rev() {
        let codec = &(**entry).codec;
        if codec.scheme == scheme {
            return codec as *const TIFFCodec;
        }
    }
    for codec in &BUILTIN_CODECS {
        if codec.name.is_null() {
            break;
        }
        if codec.scheme == scheme {
            return codec as *const TIFFCodec;
        }
    }
    ptr::null()
}

#[no_mangle]
pub unsafe extern "C" fn TIFFRegisterCODEC(
    scheme: u16,
    name: *const c_char,
    init: TIFFInitMethod,
) -> *mut TIFFCodec {
    if name.is_null() {
        return ptr::null_mut();
    }
    let Ok(name) = CString::new(std::ffi::CStr::from_ptr(name).to_bytes()) else {
        return ptr::null_mut();
    };
    let mut registration = Box::new(RegisteredCodec {
        codec: TIFFCodec {
            name: ptr::null_mut(),
            scheme,
            init,
        },
        name,
    });
    registration.codec.name = registration.name.as_ptr() as *mut c_char;
    let raw = Box::into_raw(registration);
    let codec = ptr::addr_of_mut!((*raw).codec);
    let mut registry = registry().lock().expect("codec registry lock");
    registry.codecs.push(raw);
    codec
}

#[no_mangle]
pub unsafe extern "C" fn TIFFUnRegisterCODEC(codec: *mut TIFFCodec) {
    if codec.is_null() {
        return;
    }
    let mut registry = registry().lock().expect("codec registry lock");
    if let Some(index) = registry.codecs.iter().position(|entry| {
        ptr::addr_of!((**entry).codec).cast_mut() == codec
    }) {
        let raw = registry.codecs.remove(index);
        drop(Box::from_raw(raw));
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFIsCODECConfigured(scheme: u16) -> c_int {
    let codec = TIFFFindCODEC(scheme);
    if codec.is_null() {
        return 0;
    }
    if BUILTIN_CODECS
        .iter()
        .take_while(|entry| !entry.name.is_null())
        .any(|entry| std::ptr::eq(entry, codec))
    {
        builtin_codec_configured((*codec).scheme) as c_int
    } else {
        is_configured_init((*codec).init) as c_int
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFGetConfiguredCODECs() -> *mut TIFFCodec {
    let registry = registry().lock().expect("codec registry lock");
    let mut codecs = Vec::with_capacity(registry.codecs.len() + BUILTIN_CODECS.len());
    for entry in &registry.codecs {
        codecs.push((**entry).codec);
    }
    for codec in &BUILTIN_CODECS {
        if codec.name.is_null() {
            break;
        }
        if builtin_codec_configured(codec.scheme) {
            codecs.push(*codec);
        }
    }
    codecs.push(TIFFCodec {
        name: ptr::null_mut(),
        scheme: 0,
        init: None,
    });
    let bytes = codecs.len() * std::mem::size_of::<TIFFCodec>();
    let ptr = crate::_TIFFmalloc(bytes as crate::Tmsize).cast::<TIFFCodec>();
    if ptr.is_null() {
        return ptr::null_mut();
    }
    ptr::copy_nonoverlapping(codecs.as_ptr(), ptr, codecs.len());
    ptr
}
