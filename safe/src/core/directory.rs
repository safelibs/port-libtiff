use super::field_registry::{_TIFFCreateAnonField, _TIFFMergeFields, TIFFFindField};
use super::{reset_default_directory, reset_field_registry_with_array};
use crate::abi::{TIFFDataType, TIFFFieldArray};
use crate::{
    emit_error_message, emit_warning_message, parse_u16, parse_u32, parse_u64, read_from_proc,
    seek_in_proc, tif_inner, TIFF, TIFF_BIGENDIAN, TIFF_ISTILED, TIFF_VERSION_BIG,
    TIFF_VERSION_CLASSIC,
};
use libc::{c_int, c_void};
use std::collections::HashSet;
use std::ptr;

const RESUNIT_INCH: u16 = 2;
const ORIENTATION_TOPLEFT: u16 = 1;
const PLANARCONFIG_CONTIG: u16 = 1;
const THRESHHOLD_BILEVEL: u16 = 1;
const SAMPLEFORMAT_UINT: u16 = 1;
const INKSET_CMYK: u16 = 1;
const YCBCRPOSITION_CENTERED: u16 = 1;

const TAG_SUBFILETYPE: u32 = 254;
const TAG_BITSPERSAMPLE: u32 = 258;
const TAG_THRESHHOLDING: u32 = 263;
const TAG_FILLORDER: u32 = 266;
const TAG_ORIENTATION: u32 = 274;
const TAG_SAMPLESPERPIXEL: u32 = 277;
const TAG_ROWSPERSTRIP: u32 = 278;
const TAG_MINSAMPLEVALUE: u32 = 280;
const TAG_MAXSAMPLEVALUE: u32 = 281;
const TAG_PLANARCONFIG: u32 = 284;
const TAG_RESOLUTIONUNIT: u32 = 296;
const TAG_WHITEPOINT: u32 = 318;
const TAG_TILEWIDTH: u32 = 322;
const TAG_TILELENGTH: u32 = 323;
const TAG_SUBIFD: u32 = 330;
const TAG_INKNAMES: u32 = 333;
const TAG_NUMBEROFINKS: u32 = 334;
const TAG_EXTRASAMPLES: u32 = 338;
const TAG_SAMPLEFORMAT: u32 = 339;
const TAG_IMAGEDEPTH: u32 = 32997;
const TAG_TILEDEPTH: u32 = 32998;
const TAG_YCBCRSUBSAMPLING: u32 = 530;
const TAG_YCBCRPOSITIONING: u32 = 531;

static DEFAULT_WHITEPOINT: [f32; 2] = [0.34570292, 0.3585386];
static DEFAULT_YCBCR_COEFFICIENTS: [f32; 3] = [0.299, 0.587, 0.114];
static DEFAULT_REFERENCE_BLACK_WHITE: [f32; 6] = [0.0, 255.0, 128.0, 255.0, 128.0, 255.0];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DirectoryKind {
    Main,
    SubIfd,
    Custom(*const TIFFFieldArray),
}

#[derive(Default)]
pub(crate) struct DirectoryState {
    first_ifd_offset: u64,
    main_offsets: Vec<u64>,
    main_next_offsets: Vec<u64>,
    main_complete: bool,
    subifd_seed_offsets: Vec<u64>,
    current: Option<CurrentDirectory>,
    active_chain: ActiveChain,
    default_cache: DefaultCache,
}

#[derive(Default)]
enum ActiveChain {
    #[default]
    None,
    Main {
        index: usize,
    },
    Custom {
        kind: DirectoryKind,
        visited: Vec<u64>,
        index: u32,
    },
}

struct CurrentDirectory {
    kind: DirectoryKind,
    offset: u64,
    next_offset: u64,
    tags: Vec<ParsedTag>,
}

struct ParsedTag {
    tag: u32,
    canonical_type: TIFFDataType,
    count: u64,
    values: StoredValues,
}

enum StoredValues {
    U8(Box<[u8]>),
    I8(Box<[i8]>),
    U16(Box<[u16]>),
    I16(Box<[i16]>),
    U32(Box<[u32]>),
    I32(Box<[i32]>),
    U64(Box<[u64]>),
    I64(Box<[i64]>),
    F32(Box<[f32]>),
    F64(Box<[f64]>),
}

#[derive(Default)]
struct DefaultCache {
    u8_values: Option<Box<[u8]>>,
    u16_values: Option<Box<[u16]>>,
    u32_values: Option<Box<[u32]>>,
    f32_values: Option<Box<[f32]>>,
}

impl StoredValues {
    fn as_ptr(&self) -> *const c_void {
        match self {
            StoredValues::U8(values) => values.as_ptr().cast(),
            StoredValues::I8(values) => values.as_ptr().cast(),
            StoredValues::U16(values) => values.as_ptr().cast(),
            StoredValues::I16(values) => values.as_ptr().cast(),
            StoredValues::U32(values) => values.as_ptr().cast(),
            StoredValues::I32(values) => values.as_ptr().cast(),
            StoredValues::U64(values) => values.as_ptr().cast(),
            StoredValues::I64(values) => values.as_ptr().cast(),
            StoredValues::F32(values) => values.as_ptr().cast(),
            StoredValues::F64(values) => values.as_ptr().cast(),
        }
    }
}

impl CurrentDirectory {
    fn find_tag(&self, tag: u32) -> Option<&ParsedTag> {
        self.tags.iter().find(|entry| entry.tag == tag)
    }
}

unsafe fn directory_state_mut(tif: *mut TIFF) -> &'static mut DirectoryState {
    &mut (*tif_inner(tif)).directory_state
}

unsafe fn directory_state(tif: *mut TIFF) -> &'static DirectoryState {
    &(*tif_inner(tif)).directory_state
}

unsafe fn file_size(tif: *mut TIFF) -> u64 {
    let inner = tif_inner(tif);
    if !(*inner).mapped_base.is_null() && (*inner).mapped_size != 0 {
        (*inner).mapped_size
    } else if let Some(proc_) = (*tif).tif_sizeproc {
        proc_((*tif).tif_clientdata)
    } else {
        0
    }
}

unsafe fn read_exact_at(tif: *mut TIFF, offset: u64, bytes: &mut [u8]) -> bool {
    let size = file_size(tif);
    let Some(end) = offset.checked_add(bytes.len() as u64) else {
        return false;
    };
    if end > size {
        return false;
    }

    let inner = tif_inner(tif);
    if !(*inner).mapped_base.is_null() && end <= (*inner).mapped_size {
        ptr::copy_nonoverlapping(
            ((*inner).mapped_base.cast::<u8>()).add(offset as usize),
            bytes.as_mut_ptr(),
            bytes.len(),
        );
        true
    } else if seek_in_proc(tif, offset, libc::SEEK_SET) == offset {
        read_from_proc(tif, bytes.as_mut_ptr().cast(), bytes.len() as isize)
    } else {
        false
    }
}

fn type_width(type_: TIFFDataType) -> Option<u64> {
    match type_.0 {
        1 | 2 | 6 | 7 => Some(1),
        3 | 8 => Some(2),
        4 | 9 | 11 | 13 => Some(4),
        5 | 10 | 12 | 16 | 17 | 18 => Some(8),
        _ => None,
    }
}

fn type_from_raw(raw: u16) -> Option<TIFFDataType> {
    match raw as c_int {
        1 => Some(TIFFDataType::TIFF_BYTE),
        2 => Some(TIFFDataType::TIFF_ASCII),
        3 => Some(TIFFDataType::TIFF_SHORT),
        4 => Some(TIFFDataType::TIFF_LONG),
        5 => Some(TIFFDataType::TIFF_RATIONAL),
        6 => Some(TIFFDataType::TIFF_SBYTE),
        7 => Some(TIFFDataType::TIFF_UNDEFINED),
        8 => Some(TIFFDataType::TIFF_SSHORT),
        9 => Some(TIFFDataType::TIFF_SLONG),
        10 => Some(TIFFDataType::TIFF_SRATIONAL),
        11 => Some(TIFFDataType::TIFF_FLOAT),
        12 => Some(TIFFDataType::TIFF_DOUBLE),
        13 => Some(TIFFDataType::TIFF_IFD),
        16 => Some(TIFFDataType::TIFF_LONG8),
        17 => Some(TIFFDataType::TIFF_SLONG8),
        18 => Some(TIFFDataType::TIFF_IFD8),
        _ => None,
    }
}

fn canonical_rational_type(tif: *mut TIFF, field_tag: u32, fallback: TIFFDataType) -> TIFFDataType {
    unsafe {
        let field = TIFFFindField(tif, field_tag, TIFFDataType::TIFF_NOTYPE);
        if field.is_null() {
            return fallback;
        }
        match (*field).set_field_type.0 {
            11 | 25 | 37 | 49 => TIFFDataType::TIFF_DOUBLE,
            _ => TIFFDataType::TIFF_FLOAT,
        }
    }
}

fn field_array_for_kind(kind: DirectoryKind) -> *const TIFFFieldArray {
    match kind {
        DirectoryKind::Main | DirectoryKind::SubIfd => ptr::null(),
        DirectoryKind::Custom(info) => info,
    }
}

unsafe fn reset_fields_for_kind(tif: *mut TIFF, kind: DirectoryKind) -> bool {
    match kind {
        DirectoryKind::Main | DirectoryKind::SubIfd => reset_default_directory(tif),
        _ => {
            let info = field_array_for_kind(kind);
            reset_field_registry_with_array(tif, info)
        }
    }
}

unsafe fn configure_current_directory_flags(tif: *mut TIFF, current: &CurrentDirectory) {
    let is_tiled = current
        .find_tag(TAG_TILEWIDTH)
        .zip(current.find_tag(TAG_TILELENGTH))
        .is_some();
    if is_tiled {
        (*tif).tif_flags |= TIFF_ISTILED;
    } else {
        (*tif).tif_flags &= !TIFF_ISTILED;
    }
    (*tif).tif_row = u32::MAX;
    (*tif_inner(tif)).tif_curstrip = u32::MAX;
    (*tif_inner(tif)).tif_curtile = u32::MAX;
}

unsafe fn set_current_directory(
    tif: *mut TIFF,
    current: CurrentDirectory,
    active_chain: ActiveChain,
    curdir_value: u32,
) {
    let offset = current.offset;
    let next_offset = current.next_offset;
    configure_current_directory_flags(tif, &current);

    let state = directory_state_mut(tif);
    state.current = Some(current);
    state.active_chain = active_chain;
    state.default_cache = DefaultCache::default();
    if matches!(
        state.current.as_ref().map(|dir| dir.kind),
        Some(DirectoryKind::Main)
    ) {
        state.subifd_seed_offsets = state
            .current
            .as_ref()
            .and_then(|dir| dir.find_tag(TAG_SUBIFD))
            .and_then(|entry| match &entry.values {
                StoredValues::U64(values) => Some(values.to_vec()),
                _ => None,
            })
            .unwrap_or_default();
    }
    (*tif_inner(tif)).current_diroff = offset;
    (*tif_inner(tif)).next_diroff = next_offset;
    (*tif).tif_curdir = curdir_value;
}

fn checked_total_bytes(
    tif: *mut TIFF,
    count: u64,
    type_width: u64,
    module_name: &str,
    tag: u32,
) -> Option<usize> {
    let total = count.checked_mul(type_width)?;
    if total > isize::MAX as u64 {
        emit_error_message(
            tif,
            module_name,
            format!("Tag {} is too large to process safely", tag),
        );
        return None;
    }
    Some(total as usize)
}

unsafe fn read_entry_payload(
    tif: *mut TIFF,
    module_name: &str,
    tag: u32,
    actual_type: TIFFDataType,
    count: u64,
    inline_size: usize,
    inline_bytes: &[u8],
    value_offset: u64,
) -> Option<Vec<u8>> {
    let type_width = type_width(actual_type)?;
    let total_bytes = checked_total_bytes(tif, count, type_width, module_name, tag)?;
    if total_bytes == 0 {
        return Some(Vec::new());
    }

    if total_bytes <= inline_size {
        return Some(inline_bytes[..total_bytes].to_vec());
    }

    let mut payload = vec![0u8; total_bytes];
    if read_exact_at(tif, value_offset, &mut payload) {
        Some(payload)
    } else {
        emit_error_message(
            tif,
            module_name,
            format!("Cannot read tag {} data at offset {}", tag, value_offset),
        );
        None
    }
}

fn parse_integer_value(bytes: &[u8], actual_type: TIFFDataType, big_endian: bool) -> Option<i128> {
    match actual_type.0 {
        x if x == TIFFDataType::TIFF_BYTE.0 || x == TIFFDataType::TIFF_UNDEFINED.0 => {
            Some(bytes[0] as i128)
        }
        x if x == TIFFDataType::TIFF_SBYTE.0 => Some((bytes[0] as i8) as i128),
        x if x == TIFFDataType::TIFF_SHORT.0 => Some(parse_u16(bytes, big_endian) as i128),
        x if x == TIFFDataType::TIFF_SSHORT.0 => {
            Some(i16::from_ne_bytes(parse_u16(bytes, big_endian).to_ne_bytes()) as i128)
        }
        x if x == TIFFDataType::TIFF_LONG.0 || x == TIFFDataType::TIFF_IFD.0 => {
            Some(parse_u32(bytes, big_endian) as i128)
        }
        x if x == TIFFDataType::TIFF_SLONG.0 => {
            Some(i32::from_ne_bytes(parse_u32(bytes, big_endian).to_ne_bytes()) as i128)
        }
        x if x == TIFFDataType::TIFF_LONG8.0 || x == TIFFDataType::TIFF_IFD8.0 => {
            Some(parse_u64(bytes, big_endian) as i128)
        }
        x if x == TIFFDataType::TIFF_SLONG8.0 => {
            Some(i64::from_ne_bytes(parse_u64(bytes, big_endian).to_ne_bytes()) as i128)
        }
        _ => None,
    }
}

fn parse_real_value(bytes: &[u8], actual_type: TIFFDataType, big_endian: bool) -> Option<f64> {
    match actual_type.0 {
        x if x == TIFFDataType::TIFF_RATIONAL.0 => {
            let numerator = parse_u32(&bytes[..4], big_endian);
            let denominator = parse_u32(&bytes[4..8], big_endian);
            if denominator == 0 {
                None
            } else {
                Some(numerator as f64 / denominator as f64)
            }
        }
        x if x == TIFFDataType::TIFF_SRATIONAL.0 => {
            let numerator = i32::from_ne_bytes(parse_u32(&bytes[..4], big_endian).to_ne_bytes());
            let denominator =
                i32::from_ne_bytes(parse_u32(&bytes[4..8], big_endian).to_ne_bytes());
            if denominator == 0 {
                None
            } else {
                Some(numerator as f64 / denominator as f64)
            }
        }
        x if x == TIFFDataType::TIFF_FLOAT.0 => {
            Some(f32::from_bits(parse_u32(bytes, big_endian)) as f64)
        }
        x if x == TIFFDataType::TIFF_DOUBLE.0 => Some(f64::from_bits(parse_u64(bytes, big_endian))),
        _ => parse_integer_value(bytes, actual_type, big_endian).map(|value| value as f64),
    }
}

fn warn_bad_value(tif: *mut TIFF, module_name: &str, tag: u32, message: &str) {
    emit_warning_message(
        tif,
        module_name,
        format!("Tag {} ignored: {}", tag, message),
    );
}

fn convert_to_u16_array(
    tif: *mut TIFF,
    module_name: &str,
    tag: u32,
    actual_type: TIFFDataType,
    payload: &[u8],
    count: u64,
    big_endian: bool,
) -> Option<Box<[u16]>> {
    let width = type_width(actual_type)? as usize;
    let mut values = Vec::with_capacity(count as usize);
    for chunk in payload.chunks_exact(width) {
        let value = parse_integer_value(chunk, actual_type, big_endian)?;
        let Ok(value) = u16::try_from(value) else {
            warn_bad_value(tif, module_name, tag, "value is out of range for uint16");
            return None;
        };
        values.push(value);
    }
    Some(values.into_boxed_slice())
}

fn convert_to_u32_array(
    tif: *mut TIFF,
    module_name: &str,
    tag: u32,
    actual_type: TIFFDataType,
    payload: &[u8],
    count: u64,
    big_endian: bool,
) -> Option<Box<[u32]>> {
    let width = type_width(actual_type)? as usize;
    let mut values = Vec::with_capacity(count as usize);
    for chunk in payload.chunks_exact(width) {
        let value = parse_integer_value(chunk, actual_type, big_endian)?;
        let Ok(value) = u32::try_from(value) else {
            warn_bad_value(tif, module_name, tag, "value is out of range for uint32");
            return None;
        };
        values.push(value);
    }
    Some(values.into_boxed_slice())
}

fn convert_to_u64_array(
    tif: *mut TIFF,
    module_name: &str,
    tag: u32,
    actual_type: TIFFDataType,
    payload: &[u8],
    count: u64,
    big_endian: bool,
) -> Option<Box<[u64]>> {
    let width = type_width(actual_type)? as usize;
    let mut values = Vec::with_capacity(count as usize);
    for chunk in payload.chunks_exact(width) {
        let value = parse_integer_value(chunk, actual_type, big_endian)?;
        let Ok(value) = u64::try_from(value) else {
            warn_bad_value(tif, module_name, tag, "value is negative or out of range for uint64");
            return None;
        };
        values.push(value);
    }
    Some(values.into_boxed_slice())
}

fn convert_to_i16_array(
    tif: *mut TIFF,
    module_name: &str,
    tag: u32,
    actual_type: TIFFDataType,
    payload: &[u8],
    count: u64,
    big_endian: bool,
) -> Option<Box<[i16]>> {
    let width = type_width(actual_type)? as usize;
    let mut values = Vec::with_capacity(count as usize);
    for chunk in payload.chunks_exact(width) {
        let value = parse_integer_value(chunk, actual_type, big_endian)?;
        let Ok(value) = i16::try_from(value) else {
            warn_bad_value(tif, module_name, tag, "value is out of range for int16");
            return None;
        };
        values.push(value);
    }
    Some(values.into_boxed_slice())
}

fn convert_to_i32_array(
    tif: *mut TIFF,
    module_name: &str,
    tag: u32,
    actual_type: TIFFDataType,
    payload: &[u8],
    count: u64,
    big_endian: bool,
) -> Option<Box<[i32]>> {
    let width = type_width(actual_type)? as usize;
    let mut values = Vec::with_capacity(count as usize);
    for chunk in payload.chunks_exact(width) {
        let value = parse_integer_value(chunk, actual_type, big_endian)?;
        let Ok(value) = i32::try_from(value) else {
            warn_bad_value(tif, module_name, tag, "value is out of range for int32");
            return None;
        };
        values.push(value);
    }
    Some(values.into_boxed_slice())
}

fn convert_to_i64_array(
    tif: *mut TIFF,
    module_name: &str,
    tag: u32,
    actual_type: TIFFDataType,
    payload: &[u8],
    count: u64,
    big_endian: bool,
) -> Option<Box<[i64]>> {
    let width = type_width(actual_type)? as usize;
    let mut values = Vec::with_capacity(count as usize);
    for chunk in payload.chunks_exact(width) {
        let value = parse_integer_value(chunk, actual_type, big_endian)?;
        let Ok(value) = i64::try_from(value) else {
            warn_bad_value(tif, module_name, tag, "value is out of range for int64");
            return None;
        };
        values.push(value);
    }
    Some(values.into_boxed_slice())
}

fn convert_to_f32_array(
    tif: *mut TIFF,
    module_name: &str,
    tag: u32,
    actual_type: TIFFDataType,
    payload: &[u8],
    count: u64,
    big_endian: bool,
) -> Option<Box<[f32]>> {
    let width = type_width(actual_type)? as usize;
    let mut values = Vec::with_capacity(count as usize);
    for chunk in payload.chunks_exact(width) {
        let value = parse_real_value(chunk, actual_type, big_endian)?;
        if !value.is_finite() || value > f32::MAX as f64 || value < f32::MIN as f64 {
            warn_bad_value(tif, module_name, tag, "value is out of range for float");
            return None;
        }
        values.push(value as f32);
    }
    Some(values.into_boxed_slice())
}

fn convert_to_f64_array(
    tif: *mut TIFF,
    module_name: &str,
    tag: u32,
    actual_type: TIFFDataType,
    payload: &[u8],
    count: u64,
    big_endian: bool,
) -> Option<Box<[f64]>> {
    let width = type_width(actual_type)? as usize;
    let mut values = Vec::with_capacity(count as usize);
    for chunk in payload.chunks_exact(width) {
        let value = parse_real_value(chunk, actual_type, big_endian)?;
        if !value.is_finite() {
            warn_bad_value(tif, module_name, tag, "value is not finite");
            return None;
        }
        values.push(value);
    }
    Some(values.into_boxed_slice())
}

fn convert_ascii(payload: &[u8]) -> Box<[u8]> {
    if payload.is_empty() {
        return Box::<[u8]>::from([0u8]);
    }
    if payload.last() == Some(&0) {
        payload.to_vec().into_boxed_slice()
    } else {
        let mut data = payload.to_vec();
        data.push(0);
        data.into_boxed_slice()
    }
}

fn actual_storage_type(field_type: TIFFDataType, actual_type: TIFFDataType) -> TIFFDataType {
    if field_type.0 == TIFFDataType::TIFF_NOTYPE.0 {
        actual_type
    } else {
        field_type
    }
}

unsafe fn parse_tag_value(
    tif: *mut TIFF,
    module_name: &str,
    tag: u32,
    field_type: TIFFDataType,
    actual_type: TIFFDataType,
    count: u64,
    payload: &[u8],
    big_endian: bool,
) -> Option<ParsedTag> {
    let canonical_type = actual_storage_type(field_type, actual_type);

    let values = match canonical_type.0 {
        x if x == TIFFDataType::TIFF_ASCII.0 => StoredValues::U8(convert_ascii(payload)),
        x if x == TIFFDataType::TIFF_BYTE.0 || x == TIFFDataType::TIFF_UNDEFINED.0 => {
            StoredValues::U8(payload.to_vec().into_boxed_slice())
        }
        x if x == TIFFDataType::TIFF_SBYTE.0 => StoredValues::I8(
            payload
                .iter()
                .copied()
                .map(|byte| byte as i8)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ),
        x if x == TIFFDataType::TIFF_SHORT.0 => StoredValues::U16(convert_to_u16_array(
            tif,
            module_name,
            tag,
            actual_type,
            payload,
            count,
            big_endian,
        )?),
        x if x == TIFFDataType::TIFF_SSHORT.0 => StoredValues::I16(convert_to_i16_array(
            tif,
            module_name,
            tag,
            actual_type,
            payload,
            count,
            big_endian,
        )?),
        x if x == TIFFDataType::TIFF_LONG.0 || x == TIFFDataType::TIFF_IFD.0 => {
            StoredValues::U32(convert_to_u32_array(
                tif,
                module_name,
                tag,
                actual_type,
                payload,
                count,
                big_endian,
            )?)
        }
        x if x == TIFFDataType::TIFF_SLONG.0 => StoredValues::I32(convert_to_i32_array(
            tif,
            module_name,
            tag,
            actual_type,
            payload,
            count,
            big_endian,
        )?),
        x if x == TIFFDataType::TIFF_LONG8.0 || x == TIFFDataType::TIFF_IFD8.0 => {
            StoredValues::U64(convert_to_u64_array(
                tif,
                module_name,
                tag,
                actual_type,
                payload,
                count,
                big_endian,
            )?)
        }
        x if x == TIFFDataType::TIFF_SLONG8.0 => StoredValues::I64(convert_to_i64_array(
            tif,
            module_name,
            tag,
            actual_type,
            payload,
            count,
            big_endian,
        )?),
        x if x == TIFFDataType::TIFF_RATIONAL.0 || x == TIFFDataType::TIFF_FLOAT.0 => {
            let rational_type = canonical_rational_type(tif, tag, canonical_type);
            if rational_type.0 == TIFFDataType::TIFF_DOUBLE.0 {
                StoredValues::F64(convert_to_f64_array(
                    tif,
                    module_name,
                    tag,
                    actual_type,
                    payload,
                    count,
                    big_endian,
                )?)
            } else {
                StoredValues::F32(convert_to_f32_array(
                    tif,
                    module_name,
                    tag,
                    actual_type,
                    payload,
                    count,
                    big_endian,
                )?)
            }
        }
        x if x == TIFFDataType::TIFF_SRATIONAL.0 || x == TIFFDataType::TIFF_DOUBLE.0 => {
            StoredValues::F64(convert_to_f64_array(
                tif,
                module_name,
                tag,
                actual_type,
                payload,
                count,
                big_endian,
            )?)
        }
        _ => {
            warn_bad_value(tif, module_name, tag, "data type is unsupported");
            return None;
        }
    };

    Some(ParsedTag {
        tag,
        canonical_type: if canonical_type.0 == TIFFDataType::TIFF_RATIONAL.0 {
            canonical_rational_type(tif, tag, canonical_type)
        } else {
            canonical_type
        },
        count,
        values,
    })
}

unsafe fn load_directory(
    tif: *mut TIFF,
    offset: u64,
    kind: DirectoryKind,
    module_name: &str,
) -> Option<CurrentDirectory> {
    if offset == 0 {
        return None;
    }
    if !reset_fields_for_kind(tif, kind) {
        emit_error_message(tif, module_name, "Failed to initialize field registry");
        return None;
    }

    let inner = tif_inner(tif);
    let big_endian = (*inner).header_magic == TIFF_BIGENDIAN;
    let (count_size, entry_size, next_size, inline_size) = if (*inner).header_version
        == TIFF_VERSION_CLASSIC
    {
        (2usize, 12usize, 4usize, 4usize)
    } else if (*inner).header_version == TIFF_VERSION_BIG {
        (8usize, 20usize, 8usize, 8usize)
    } else {
        emit_error_message(tif, module_name, "TIFF header is not initialized");
        return None;
    };

    let mut count_bytes = [0u8; 8];
    if !read_exact_at(tif, offset, &mut count_bytes[..count_size]) {
        emit_error_message(
            tif,
            module_name,
            format!("Cannot read TIFF directory at offset {}", offset),
        );
        return None;
    }
    let entry_count = if count_size == 2 {
        parse_u16(&count_bytes[..2], big_endian) as u64
    } else {
        parse_u64(&count_bytes[..8], big_endian)
    };
    let Some(entries_len_u64) = entry_count.checked_mul(entry_size as u64) else {
        emit_error_message(
            tif,
            module_name,
            format!("Directory at offset {} is too large", offset),
        );
        return None;
    };
    if entries_len_u64 > usize::MAX as u64 {
        emit_error_message(
            tif,
            module_name,
            format!("Directory at offset {} is too large", offset),
        );
        return None;
    }

    let entries_offset = offset + count_size as u64;
    let next_offset_offset = match entries_offset.checked_add(entries_len_u64) {
        Some(value) => value,
        None => {
            emit_error_message(
                tif,
                module_name,
                format!("Directory at offset {} overflows file addressing", offset),
            );
            return None;
        }
    };

    let mut next_bytes = [0u8; 8];
    if !read_exact_at(tif, next_offset_offset, &mut next_bytes[..next_size]) {
        emit_error_message(
            tif,
            module_name,
            format!("Cannot read next IFD offset for directory {}", offset),
        );
        return None;
    }
    let next_offset = if next_size == 4 {
        parse_u32(&next_bytes[..4], big_endian) as u64
    } else {
        parse_u64(&next_bytes[..8], big_endian)
    };

    let mut entries_buf = vec![0u8; entries_len_u64 as usize];
    if !entries_buf.is_empty() && !read_exact_at(tif, entries_offset, &mut entries_buf) {
        emit_error_message(
            tif,
            module_name,
            format!("Cannot read TIFF directory entries at offset {}", offset),
        );
        return None;
    }

    let mut tags = Vec::new();
    let mut seen_tags = HashSet::new();
    let mut warned_order = false;
    let mut previous_tag = 0u16;

    for index in 0..entry_count as usize {
        let entry = &entries_buf[index * entry_size..(index + 1) * entry_size];
        let tag = parse_u16(&entry[..2], big_endian);
        let raw_type = parse_u16(&entry[2..4], big_endian);
        let Some(actual_type) = type_from_raw(raw_type) else {
            emit_warning_message(
                tif,
                module_name,
                format!("Unknown data type {} for tag {} ignored", raw_type, tag),
            );
            continue;
        };
        let count = if count_size == 2 {
            parse_u32(&entry[4..8], big_endian) as u64
        } else {
            parse_u64(&entry[4..12], big_endian)
        };
        let value_offset = if inline_size == 4 {
            parse_u32(&entry[8..12], big_endian) as u64
        } else {
            parse_u64(&entry[12..20], big_endian)
        };
        let inline_slice = if inline_size == 4 {
            &entry[8..12]
        } else {
            &entry[12..20]
        };

        if !warned_order && !tags.is_empty() && tag < previous_tag {
            emit_warning_message(
                tif,
                module_name,
                format!("Directory at offset {} has unsorted tag order", offset),
            );
            warned_order = true;
        }
        previous_tag = tag;

        if !seen_tags.insert(tag) {
            emit_warning_message(
                tif,
                module_name,
                format!("Duplicate tag {} encountered; later instance ignored", tag),
            );
            continue;
        }

        let tag_u32 = tag as u32;
        let mut field = TIFFFindField(tif, tag_u32, TIFFDataType::TIFF_NOTYPE);
        if field.is_null() {
            emit_warning_message(
                tif,
                module_name,
                format!("Unknown field with tag {} (0x{:x}) encountered", tag_u32, tag_u32),
            );
            let created = _TIFFCreateAnonField(tif, tag_u32, actual_type);
            if created.is_null() || _TIFFMergeFields(tif, created, 1) == 0 {
                emit_warning_message(
                    tif,
                    module_name,
                    format!(
                        "Registering anonymous field with tag {} (0x{:x}) failed",
                        tag_u32, tag_u32
                    ),
                );
                continue;
            }
            field = TIFFFindField(tif, tag_u32, TIFFDataType::TIFF_NOTYPE);
            if field.is_null() {
                continue;
            }
        }

        let field_type = (*field).field_type;
        let payload = match read_entry_payload(
            tif,
            module_name,
            tag_u32,
            actual_type,
            count,
            inline_size,
            inline_slice,
            value_offset,
        ) {
            Some(payload) => payload,
            None => continue,
        };
        let Some(parsed_tag) = parse_tag_value(
            tif,
            module_name,
            tag_u32,
            field_type,
            actual_type,
            count,
            &payload,
            big_endian,
        ) else {
            continue;
        };
        tags.push(parsed_tag);
    }

    tags.sort_by_key(|entry| entry.tag);
    Some(CurrentDirectory {
        kind,
        offset,
        next_offset,
        tags,
    })
}

unsafe fn read_directory_next_offset(tif: *mut TIFF, offset: u64, module_name: &str) -> Option<u64> {
    if offset == 0 {
        return Some(0);
    }

    let inner = tif_inner(tif);
    let big_endian = (*inner).header_magic == TIFF_BIGENDIAN;
    let (count_size, entry_size, next_size) = if (*inner).header_version == TIFF_VERSION_CLASSIC {
        (2usize, 12usize, 4usize)
    } else if (*inner).header_version == TIFF_VERSION_BIG {
        (8usize, 20usize, 8usize)
    } else {
        emit_error_message(tif, module_name, "TIFF header is not initialized");
        return None;
    };

    let mut count_bytes = [0u8; 8];
    if !read_exact_at(tif, offset, &mut count_bytes[..count_size]) {
        emit_error_message(
            tif,
            module_name,
            format!("Cannot read TIFF directory at offset {}", offset),
        );
        return None;
    }
    let entry_count = if count_size == 2 {
        parse_u16(&count_bytes[..2], big_endian) as u64
    } else {
        parse_u64(&count_bytes[..8], big_endian)
    };
    let Some(entries_len_u64) = entry_count.checked_mul(entry_size as u64) else {
        emit_error_message(
            tif,
            module_name,
            format!("Directory at offset {} is too large", offset),
        );
        return None;
    };

    let entries_offset = offset + count_size as u64;
    let Some(next_offset_offset) = entries_offset.checked_add(entries_len_u64) else {
        emit_error_message(
            tif,
            module_name,
            format!("Directory at offset {} overflows file addressing", offset),
        );
        return None;
    };

    let mut next_bytes = [0u8; 8];
    if !read_exact_at(tif, next_offset_offset, &mut next_bytes[..next_size]) {
        emit_error_message(
            tif,
            module_name,
            format!("Cannot read next IFD offset for directory {}", offset),
        );
        return None;
    }

    Some(if next_size == 4 {
        parse_u32(&next_bytes[..4], big_endian) as u64
    } else {
        parse_u64(&next_bytes[..8], big_endian)
    })
}

unsafe fn ensure_main_chain_initialized(tif: *mut TIFF) {
    let state = directory_state_mut(tif);
    if state.first_ifd_offset == 0 {
        state.first_ifd_offset = (*tif_inner(tif)).next_diroff;
    }
}

unsafe fn load_main_directory_at_index(
    tif: *mut TIFF,
    target_index: usize,
    module_name: &str,
) -> bool {
    ensure_main_chain_initialized(tif);
    let state = directory_state_mut(tif);
    if state.first_ifd_offset == 0 {
        return false;
    }
    if state.main_offsets.is_empty() {
        state.main_offsets.push(state.first_ifd_offset);
    }

    let mut seen: HashSet<u64> = state.main_offsets.iter().copied().collect();
    while state.main_offsets.len() <= target_index && !state.main_complete {
        let current_offset = *state.main_offsets.last().expect("main offsets");
        let current_next = if let Some(next) =
            state.main_next_offsets.get(state.main_offsets.len().saturating_sub(1))
        {
            *next
        } else {
            match read_directory_next_offset(tif, current_offset, module_name) {
                Some(next) => {
                    state.main_next_offsets.push(next);
                    next
                }
                None => {
                    state.main_complete = true;
                    0
                }
            }
        };

        if current_next == 0 || !seen.insert(current_next) {
            state.main_complete = true;
            break;
        }
        state.main_offsets.push(current_next);
    }

    if target_index >= state.main_offsets.len() {
        return false;
    }

    let offset = state.main_offsets[target_index];
    let Some(current) = load_directory(tif, offset, DirectoryKind::Main, module_name) else {
        return false;
    };
    if state.main_next_offsets.len() <= target_index {
        state.main_next_offsets.resize(target_index + 1, 0);
    }
    state.main_next_offsets[target_index] = current.next_offset;
    set_current_directory(
        tif,
        current,
        ActiveChain::Main { index: target_index },
        target_index as u32,
    );
    true
}

unsafe fn load_custom_chain_directory(
    tif: *mut TIFF,
    offset: u64,
    kind: DirectoryKind,
    module_name: &str,
    visited: Vec<u64>,
    index: u32,
) -> bool {
    let Some(current) = load_directory(tif, offset, kind, module_name) else {
        return false;
    };
    set_current_directory(
        tif,
        current,
        ActiveChain::Custom { kind, visited, index },
        index,
    );
    true
}

pub(crate) unsafe fn read_next_directory(tif: *mut TIFF) -> bool {
    if tif.is_null() {
        return false;
    }
    ensure_main_chain_initialized(tif);

    let module_name = "TIFFReadDirectory";
    let Some(current) = directory_state(tif).current.as_ref() else {
        return load_main_directory_at_index(tif, 0, module_name);
    };

    if current.next_offset == 0 {
        (*tif_inner(tif)).current_diroff = 0;
        return false;
    }

    match &directory_state(tif).active_chain {
        ActiveChain::Main { index } => {
            let next_offset = current.next_offset;
            let state = directory_state_mut(tif);
            if index + 1 < state.main_offsets.len()
                && state.main_offsets[index + 1] == next_offset
            {
                return load_main_directory_at_index(tif, index + 1, module_name);
            }
            if state.main_offsets[..=(*index)]
                .iter()
                .any(|offset| *offset == next_offset)
            {
                state.main_complete = true;
                emit_warning_message(
                    tif,
                    "_TIFFCheckDirNumberAndOffset",
                    format!(
                        "TIFF directory {} has IFD looping to offset 0x{:x}; stopping traversal",
                        index + 1,
                        next_offset
                    ),
                );
                return false;
            }
            load_main_directory_at_index(tif, index + 1, module_name)
        }
        ActiveChain::Custom {
            kind,
            visited,
            index,
        } => {
            if visited.contains(&current.next_offset) {
                emit_warning_message(
                    tif,
                    "_TIFFCheckDirNumberAndOffset",
                    format!(
                        "TIFF directory {} has IFD looping to offset 0x{:x}; stopping traversal",
                        index + 1,
                        current.next_offset
                    ),
                );
                return false;
            }
            let mut next_visited = visited.clone();
            next_visited.push(current.next_offset);
            load_custom_chain_directory(
                tif,
                current.next_offset,
                *kind,
                module_name,
                next_visited,
                index + 1,
            )
        }
        ActiveChain::None => false,
    }
}

pub(crate) unsafe fn read_custom_directory(
    tif: *mut TIFF,
    diroff: u64,
    infoarray: *const TIFFFieldArray,
) -> bool {
    if tif.is_null() || infoarray.is_null() || diroff == 0 {
        return false;
    }
    load_custom_chain_directory(
        tif,
        diroff,
        DirectoryKind::Custom(infoarray),
        "TIFFReadCustomDirectory",
        vec![diroff],
        0,
    )
}

pub(crate) unsafe fn set_sub_directory(tif: *mut TIFF, diroff: u64) -> bool {
    if tif.is_null() || diroff == 0 {
        return false;
    }
    let state = directory_state(tif);
    let (visited, index) = if let Some(position) = state
        .subifd_seed_offsets
        .iter()
        .position(|offset| *offset == diroff)
    {
        (state.subifd_seed_offsets[..=position].to_vec(), position as u32)
    } else {
        (vec![diroff], 0)
    };
    load_custom_chain_directory(
        tif,
        diroff,
        DirectoryKind::SubIfd,
        "TIFFSetSubDirectory",
        visited,
        index,
    )
}

pub(crate) unsafe fn set_directory(tif: *mut TIFF, dirnum: u32) -> bool {
    if tif.is_null() {
        return false;
    }
    load_main_directory_at_index(tif, dirnum as usize, "TIFFSetDirectory")
}

pub(crate) unsafe fn number_of_directories(tif: *mut TIFF) -> u32 {
    if tif.is_null() {
        return 0;
    }
    ensure_main_chain_initialized(tif);
    let state = directory_state_mut(tif);
    if state.first_ifd_offset == 0 {
        return 0;
    }
    if state.main_offsets.is_empty() {
        state.main_offsets.push(state.first_ifd_offset);
    }

    let mut seen: HashSet<u64> = state.main_offsets.iter().copied().collect();
    while !state.main_complete {
        let current_index = state.main_offsets.len().saturating_sub(1);
        let current_offset = state.main_offsets[current_index];
        let current_next = if let Some(next) = state.main_next_offsets.get(current_index) {
            *next
        } else {
            match read_directory_next_offset(tif, current_offset, "TIFFNumberOfDirectories") {
                Some(next) => {
                    state.main_next_offsets.push(next);
                    next
                }
                None => {
                    state.main_complete = true;
                    0
                }
            }
        };

        if current_next == 0 || !seen.insert(current_next) {
            state.main_complete = true;
            break;
        }
        state.main_offsets.push(current_next);
    }

    state.main_offsets.len() as u32
}

pub(crate) unsafe fn last_directory(tif: *mut TIFF) -> bool {
    if tif.is_null() {
        return true;
    }
    let Some(current) = directory_state(tif).current.as_ref() else {
        return true;
    };
    if current.next_offset == 0 {
        return true;
    }
    match &directory_state(tif).active_chain {
        ActiveChain::Main { index } => {
            let state = directory_state(tif);
            if index + 1 < state.main_offsets.len()
                && state.main_offsets[index + 1] == current.next_offset
            {
                false
            } else {
                state.main_offsets[..=(*index)]
                    .iter()
                    .any(|offset| *offset == current.next_offset)
            }
        }
        ActiveChain::Custom { visited, .. } => visited.contains(&current.next_offset),
        ActiveChain::None => true,
    }
}

pub(crate) unsafe fn free_directory_state(tif: *mut TIFF) {
    if tif.is_null() {
        return;
    }
    let state = directory_state_mut(tif);
    state.default_cache = DefaultCache::default();
    if let Some(current) = state.current.as_mut() {
        current.tags.clear();
        if matches!(current.kind, DirectoryKind::Main) {
            state.subifd_seed_offsets.clear();
        }
    }
}

pub(crate) unsafe fn current_tag_count(tif: *mut TIFF) -> u32 {
    directory_state(tif)
        .current
        .as_ref()
        .map(|current| current.tags.len() as u32)
        .unwrap_or(0)
}

pub(crate) unsafe fn current_tag_at(tif: *mut TIFF, index: u32) -> u32 {
    directory_state(tif)
        .current
        .as_ref()
        .and_then(|current| current.tags.get(index as usize))
        .map(|entry| entry.tag)
        .unwrap_or(u32::MAX)
}

unsafe fn cache_u16_values(tif: *mut TIFF, values: Vec<u16>) -> (*const c_void, u64) {
    let state = directory_state_mut(tif);
    state.default_cache.u16_values = Some(values.into_boxed_slice());
    let values = state.default_cache.u16_values.as_ref().expect("u16 cache");
    (values.as_ptr().cast(), values.len() as u64)
}

unsafe fn cache_u32_values(tif: *mut TIFF, values: Vec<u32>) -> (*const c_void, u64) {
    let state = directory_state_mut(tif);
    state.default_cache.u32_values = Some(values.into_boxed_slice());
    let values = state.default_cache.u32_values.as_ref().expect("u32 cache");
    (values.as_ptr().cast(), values.len() as u64)
}

unsafe fn cache_f32_values(tif: *mut TIFF, values: Vec<f32>) -> (*const c_void, u64) {
    let state = directory_state_mut(tif);
    state.default_cache.f32_values = Some(values.into_boxed_slice());
    let values = state.default_cache.f32_values.as_ref().expect("f32 cache");
    (values.as_ptr().cast(), values.len() as u64)
}

unsafe fn cache_u8_values(tif: *mut TIFF, values: Vec<u8>) -> (*const c_void, u64) {
    let state = directory_state_mut(tif);
    state.default_cache.u8_values = Some(values.into_boxed_slice());
    let values = state.default_cache.u8_values.as_ref().expect("u8 cache");
    (values.as_ptr().cast(), values.len() as u64)
}

unsafe fn default_tag_value(
    tif: *mut TIFF,
    tag: u32,
    out_type: *mut TIFFDataType,
    out_count: *mut u64,
    out_data: *mut *const c_void,
) -> c_int {
    let current = directory_state(tif).current.as_ref();
    let bits_per_sample = current
        .and_then(|dir| dir.find_tag(TAG_BITSPERSAMPLE))
        .and_then(|entry| match &entry.values {
            StoredValues::U16(values) => values.first().copied(),
            _ => None,
        })
        .unwrap_or(1);

    let (type_, count, data) = match tag {
        TAG_SUBFILETYPE => {
            let (data, count) = cache_u32_values(tif, vec![0]);
            (TIFFDataType::TIFF_LONG, count, data)
        }
        TAG_BITSPERSAMPLE => {
            let (data, count) = cache_u16_values(tif, vec![1]);
            (TIFFDataType::TIFF_SHORT, count, data)
        }
        TAG_THRESHHOLDING => {
            let (data, count) = cache_u16_values(tif, vec![THRESHHOLD_BILEVEL]);
            (TIFFDataType::TIFF_SHORT, count, data)
        }
        TAG_FILLORDER => {
            let (data, count) = cache_u16_values(tif, vec![1]);
            (TIFFDataType::TIFF_SHORT, count, data)
        }
        TAG_ORIENTATION => {
            let (data, count) = cache_u16_values(tif, vec![ORIENTATION_TOPLEFT]);
            (TIFFDataType::TIFF_SHORT, count, data)
        }
        TAG_SAMPLESPERPIXEL => {
            let (data, count) = cache_u16_values(tif, vec![1]);
            (TIFFDataType::TIFF_SHORT, count, data)
        }
        TAG_ROWSPERSTRIP => {
            let (data, count) = cache_u32_values(tif, vec![u32::MAX]);
            (TIFFDataType::TIFF_LONG, count, data)
        }
        TAG_MINSAMPLEVALUE => {
            let (data, count) = cache_u16_values(tif, vec![0]);
            (TIFFDataType::TIFF_SHORT, count, data)
        }
        TAG_MAXSAMPLEVALUE => {
            let max = if bits_per_sample == 0 {
                0
            } else if bits_per_sample <= 16 {
                (1u32 << bits_per_sample) as u16 - 1
            } else {
                u16::MAX
            };
            let (data, count) = cache_u16_values(tif, vec![max]);
            (TIFFDataType::TIFF_SHORT, count, data)
        }
        TAG_PLANARCONFIG => {
            let (data, count) = cache_u16_values(tif, vec![PLANARCONFIG_CONTIG]);
            (TIFFDataType::TIFF_SHORT, count, data)
        }
        TAG_RESOLUTIONUNIT => {
            let (data, count) = cache_u16_values(tif, vec![RESUNIT_INCH]);
            (TIFFDataType::TIFF_SHORT, count, data)
        }
        TAG_NUMBEROFINKS => {
            let (data, count) = cache_u16_values(tif, vec![4]);
            (TIFFDataType::TIFF_SHORT, count, data)
        }
        TAG_EXTRASAMPLES => {
            (TIFFDataType::TIFF_SHORT, 0, ptr::null())
        }
        TAG_TILEDEPTH => {
            let (data, count) = cache_u32_values(tif, vec![1]);
            (TIFFDataType::TIFF_LONG, count, data)
        }
        TAG_SAMPLEFORMAT => {
            let (data, count) = cache_u16_values(tif, vec![SAMPLEFORMAT_UINT]);
            (TIFFDataType::TIFF_SHORT, count, data)
        }
        TAG_IMAGEDEPTH => {
            let (data, count) = cache_u32_values(tif, vec![1]);
            (TIFFDataType::TIFF_LONG, count, data)
        }
        TAG_YCBCRSUBSAMPLING => {
            let (data, count) = cache_u16_values(tif, vec![2, 2]);
            (TIFFDataType::TIFF_SHORT, count, data)
        }
        TAG_YCBCRPOSITIONING => {
            let (data, count) = cache_u16_values(tif, vec![YCBCRPOSITION_CENTERED]);
            (TIFFDataType::TIFF_SHORT, count, data)
        }
        TAG_WHITEPOINT => {
            let (data, count) = cache_f32_values(tif, DEFAULT_WHITEPOINT.to_vec());
            (TIFFDataType::TIFF_FLOAT, count, data)
        }
        529 => {
            let (data, count) = cache_f32_values(tif, DEFAULT_YCBCR_COEFFICIENTS.to_vec());
            (TIFFDataType::TIFF_FLOAT, count, data)
        }
        532 => {
            let (data, count) = cache_f32_values(tif, DEFAULT_REFERENCE_BLACK_WHITE.to_vec());
            (TIFFDataType::TIFF_FLOAT, count, data)
        }
        TAG_INKNAMES => {
            let (data, count) = cache_u8_values(tif, vec![0]);
            (TIFFDataType::TIFF_ASCII, count, data)
        }
        332 => {
            let (data, count) = cache_u16_values(tif, vec![INKSET_CMYK]);
            (TIFFDataType::TIFF_SHORT, count, data)
        }
        _ => return 0,
    };

    *out_type = type_;
    *out_count = count;
    *out_data = data;
    1
}

pub(crate) unsafe fn get_tag_value(
    tif: *mut TIFF,
    tag: u32,
    defaulted: bool,
    out_type: *mut TIFFDataType,
    out_count: *mut u64,
    out_data: *mut *const c_void,
) -> c_int {
    if tif.is_null() || out_type.is_null() || out_count.is_null() || out_data.is_null() {
        return 0;
    }

    if let Some(current) = directory_state(tif).current.as_ref() {
        if let Some(entry) = current.find_tag(tag) {
            *out_type = entry.canonical_type;
            *out_count = entry.count;
            *out_data = entry.values.as_ptr();
            return 1;
        }
    }

    if defaulted {
        default_tag_value(tif, tag, out_type, out_count, out_data)
    } else {
        0
    }
}
