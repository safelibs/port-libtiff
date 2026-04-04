use super::field_registry::{
    _TIFFCreateAnonField, _TIFFGetExifFields, _TIFFGetGpsFields, _TIFFMergeFields,
    safe_tiff_record_custom_tag, safe_tiff_remove_custom_tag, TIFFFindField,
};
use super::{initialize_field_registry, reset_default_directory, reset_field_registry_with_array};
use crate::abi::{TIFFDataType, TIFFFieldArray};
use crate::{
    emit_error_message, emit_warning_message, parse_u16, parse_u32, parse_u64, read_from_proc,
    seek_in_proc, tif_inner, write_to_proc, TIFF, TIFF_BIGENDIAN, TIFF_ISTILED,
    TIFF_NON_EXISTENT_DIR_NUMBER, TIFF_VERSION_BIG, TIFF_VERSION_CLASSIC,
};
use libc::{c_int, c_void, ssize_t};
use std::collections::HashSet;
use std::mem::size_of;
use std::ptr;
use std::slice;

const FILLORDER_MSB2LSB_U16: u16 = 1;
const FILLORDER_LSB2MSB_U16: u16 = 2;
const RESUNIT_INCH: u16 = 2;
const RESUNIT_NONE: u16 = 1;
const RESUNIT_CENTIMETER: u16 = 3;
const ORIENTATION_TOPLEFT: u16 = 1;
const PLANARCONFIG_CONTIG: u16 = 1;
const PLANARCONFIG_SEPARATE: u16 = 2;
const THRESHHOLD_BILEVEL: u16 = 1;
const SAMPLEFORMAT_UINT: u16 = 1;
const SAMPLEFORMAT_INT: u16 = 2;
const SAMPLEFORMAT_IEEEFP: u16 = 3;
const SAMPLEFORMAT_VOID: u16 = 4;
const SAMPLEFORMAT_COMPLEXIEEEFP: u16 = 6;
const COMPRESSION_NONE: u16 = 1;
const EXTRASAMPLE_ASSOCALPHA: u16 = 1;
const EXTRASAMPLE_UNASSALPHA: u16 = 2;
const INKSET_CMYK: u16 = 1;
const YCBCRPOSITION_CENTERED: u16 = 1;

const TAG_SUBFILETYPE: u32 = 254;
const TAG_BITSPERSAMPLE: u32 = 258;
const TAG_COMPRESSION: u32 = 259;
const TAG_THRESHHOLDING: u32 = 263;
const TAG_FILLORDER: u32 = 266;
const TAG_XRESOLUTION: u32 = 282;
const TAG_YRESOLUTION: u32 = 283;
const TAG_ORIENTATION: u32 = 274;
const TAG_SAMPLESPERPIXEL: u32 = 277;
const TAG_ROWSPERSTRIP: u32 = 278;
const TAG_STRIPBYTECOUNTS: u32 = 279;
const TAG_MINSAMPLEVALUE: u32 = 280;
const TAG_MAXSAMPLEVALUE: u32 = 281;
const TAG_PLANARCONFIG: u32 = 284;
const TAG_RESOLUTIONUNIT: u32 = 296;
const TAG_TRANSFERFUNCTION: u32 = 301;
const TAG_WHITEPOINT: u32 = 318;
const TAG_TILEWIDTH: u32 = 322;
const TAG_TILELENGTH: u32 = 323;
const TAG_TILEOFFSETS: u32 = 324;
const TAG_TILEBYTECOUNTS: u32 = 325;
const TAG_SUBIFD: u32 = 330;
const TAG_INKSET: u32 = 332;
const TAG_INKNAMES: u32 = 333;
const TAG_NUMBEROFINKS: u32 = 334;
const TAG_DOTRANGE: u32 = 336;
const TAG_EXTRASAMPLES: u32 = 338;
const TAG_SAMPLEFORMAT: u32 = 339;
const TAG_MATTEING: u32 = 32995;
const TAG_DATATYPE: u32 = 32996;
const TAG_IMAGEDEPTH: u32 = 32997;
const TAG_TILEDEPTH: u32 = 32998;
const TAG_STRIPOFFSETS: u32 = 273;
const TAG_YCBCRSUBSAMPLING: u32 = 530;
const TAG_YCBCRPOSITIONING: u32 = 531;

const TIFF_FILLORDER: u32 = 0x00003;
const TIFF_DIRTYDIRECT: u32 = 0x00008;
const TIFF_BEENWRITING: u32 = 0x00040;
const TIFF_DIRTYSTRIP: u32 = 0x200000;

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
    pending_subifd: Option<PendingSubifdWrite>,
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

#[derive(Clone)]
struct CurrentDirectory {
    kind: DirectoryKind,
    offset: u64,
    next_offset: u64,
    tags: Vec<ParsedTag>,
}

struct PendingSubifdWrite {
    parent_offset: u64,
    offsets: Vec<u64>,
    next_index: usize,
    last_offset: u64,
}

#[derive(Clone)]
struct ParsedTag {
    tag: u32,
    canonical_type: TIFFDataType,
    count: u64,
    values: StoredValues,
}

#[derive(Clone)]
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

    fn len(&self) -> usize {
        match self {
            StoredValues::U8(values) => values.len(),
            StoredValues::I8(values) => values.len(),
            StoredValues::U16(values) => values.len(),
            StoredValues::I16(values) => values.len(),
            StoredValues::U32(values) => values.len(),
            StoredValues::I32(values) => values.len(),
            StoredValues::U64(values) => values.len(),
            StoredValues::I64(values) => values.len(),
            StoredValues::F32(values) => values.len(),
            StoredValues::F64(values) => values.len(),
        }
    }
}

impl CurrentDirectory {
    fn find_tag(&self, tag: u32) -> Option<&ParsedTag> {
        self.tags.iter().find(|entry| entry.tag == tag)
    }

    fn find_tag_mut(&mut self, tag: u32) -> Option<&mut ParsedTag> {
        self.tags.iter_mut().find(|entry| entry.tag == tag)
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

unsafe fn write_exact_at(tif: *mut TIFF, offset: u64, bytes: &[u8]) -> bool {
    if seek_in_proc(tif, offset, libc::SEEK_SET) != offset {
        return false;
    }
    if bytes.is_empty() {
        return true;
    }
    write_to_proc(tif, bytes.as_ptr().cast_mut().cast(), bytes.len() as isize)
}

fn align_up(value: u64, alignment: u64) -> Option<u64> {
    if alignment <= 1 {
        Some(value)
    } else {
        let remainder = value % alignment;
        if remainder == 0 {
            Some(value)
        } else {
            value.checked_add(alignment - remainder)
        }
    }
}

unsafe fn max_single_allocation_limit(tif: *mut TIFF) -> Option<usize> {
    let limit = (*tif_inner(tif)).tif_max_single_mem_alloc;
    if limit > 0 {
        usize::try_from(limit).ok()
    } else {
        None
    }
}

unsafe fn checked_allocation_len(
    tif: *mut TIFF,
    module_name: &str,
    what: &str,
    len: usize,
) -> Option<usize> {
    if let Some(limit) = max_single_allocation_limit(tif) {
        if len > limit {
            emit_error_message(
                tif,
                module_name,
                format!(
                    "Failed to allocate memory for {} ({} bytes exceeds the configured limit)",
                    what, len
                ),
            );
            return None;
        }
    }
    Some(len)
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
    (*tif_inner(tif)).strile_state.defer_array_writing = false;
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
            let denominator = i32::from_ne_bytes(parse_u32(&bytes[4..8], big_endian).to_ne_bytes());
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
            warn_bad_value(
                tif,
                module_name,
                tag,
                "value is negative or out of range for uint64",
            );
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
        x if x == TIFFDataType::TIFF_RATIONAL.0 || x == TIFFDataType::TIFF_SRATIONAL.0 => {
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
        x if x == TIFFDataType::TIFF_FLOAT.0 => StoredValues::F32(convert_to_f32_array(
            tif,
            module_name,
            tag,
            actual_type,
            payload,
            count,
            big_endian,
        )?),
        x if x == TIFFDataType::TIFF_DOUBLE.0 => StoredValues::F64(convert_to_f64_array(
            tif,
            module_name,
            tag,
            actual_type,
            payload,
            count,
            big_endian,
        )?),
        _ => {
            warn_bad_value(tif, module_name, tag, "data type is unsupported");
            return None;
        }
    };

    Some(ParsedTag {
        tag,
        canonical_type: if canonical_type.0 == TIFFDataType::TIFF_RATIONAL.0
            || canonical_type.0 == TIFFDataType::TIFF_SRATIONAL.0
        {
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
    let (count_size, entry_size, next_size, inline_size) =
        if (*inner).header_version == TIFF_VERSION_CLASSIC {
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
                format!(
                    "Unknown field with tag {} (0x{:x}) encountered",
                    tag_u32, tag_u32
                ),
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

    if !tags.iter().any(|entry| entry.tag == TAG_COMPRESSION) {
        tags.push(ParsedTag {
            tag: TAG_COMPRESSION,
            canonical_type: TIFFDataType::TIFF_SHORT,
            count: 1,
            values: StoredValues::U16(Box::<[u16]>::from([COMPRESSION_NONE])),
        });
    }

    tags.sort_by_key(|entry| entry.tag);
    Some(CurrentDirectory {
        kind,
        offset,
        next_offset,
        tags,
    })
}

unsafe fn read_directory_next_offset(
    tif: *mut TIFF,
    offset: u64,
    module_name: &str,
) -> Option<u64> {
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
        let current_next = if let Some(next) = state
            .main_next_offsets
            .get(state.main_offsets.len().saturating_sub(1))
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
        ActiveChain::Main {
            index: target_index,
        },
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
        ActiveChain::Custom {
            kind,
            visited,
            index,
        },
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
            if index + 1 < state.main_offsets.len() && state.main_offsets[index + 1] == next_offset
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
        (
            state.subifd_seed_offsets[..=position].to_vec(),
            position as u32,
        )
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
    let sample_format = current
        .and_then(|dir| dir.find_tag(TAG_SAMPLEFORMAT))
        .and_then(|entry| match &entry.values {
            StoredValues::U16(values) => values.first().copied(),
            _ => None,
        })
        .unwrap_or(SAMPLEFORMAT_UINT);
    let extrasample_values = current
        .and_then(|dir| dir.find_tag(TAG_EXTRASAMPLES))
        .and_then(|entry| match &entry.values {
            StoredValues::U16(values) => Some(values),
            _ => None,
        });
    let extrasamples = extrasample_values.map(|values| values.len()).unwrap_or(0);
    let matteing = (extrasamples == 1)
        && extrasample_values
            .and_then(|values| values.first().copied())
            .map(|value| value == EXTRASAMPLE_ASSOCALPHA)
            .unwrap_or(false);

    let (type_, count, data) = match tag {
        TAG_SUBFILETYPE => {
            let (data, count) = cache_u32_values(tif, vec![0]);
            (TIFFDataType::TIFF_LONG, count, data)
        }
        TAG_BITSPERSAMPLE => {
            let (data, count) = cache_u16_values(tif, vec![1]);
            (TIFFDataType::TIFF_SHORT, count, data)
        }
        TAG_COMPRESSION => {
            let (data, count) = cache_u16_values(tif, vec![COMPRESSION_NONE]);
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
        TAG_DOTRANGE => {
            let max = if bits_per_sample == 0 {
                0
            } else if bits_per_sample <= 16 {
                (1u32 << bits_per_sample) as u16 - 1
            } else {
                u16::MAX
            };
            let (data, count) = cache_u16_values(tif, vec![0, max]);
            (TIFFDataType::TIFF_SHORT, count, data)
        }
        TAG_EXTRASAMPLES => (TIFFDataType::TIFF_SHORT, 0, ptr::null()),
        TAG_MATTEING => {
            let (data, count) = cache_u16_values(tif, vec![matteing as u16]);
            (TIFFDataType::TIFF_SHORT, count, data)
        }
        TAG_DATATYPE => {
            let (data, count) = cache_u16_values(tif, vec![sample_format.saturating_sub(1)]);
            (TIFFDataType::TIFF_SHORT, count, data)
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
        TAG_INKSET => {
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

const FIELD_CUSTOM: u16 = 65;

fn default_main_directory_tags() -> Vec<ParsedTag> {
    vec![ParsedTag {
        tag: TAG_COMPRESSION,
        canonical_type: TIFFDataType::TIFF_SHORT,
        count: 1,
        values: StoredValues::U16(Box::<[u16]>::from([COMPRESSION_NONE])),
    }]
}

unsafe fn header_link_offset(tif: *mut TIFF) -> u64 {
    if (*tif_inner(tif)).header_version == TIFF_VERSION_BIG {
        8
    } else {
        4
    }
}

unsafe fn sync_fillorder_flags(tif: *mut TIFF) {
    if ((*tif).tif_flags & TIFF_FILLORDER) == 0 {
        (*tif).tif_flags |= FILLORDER_MSB2LSB_U16 as u32;
    }
}

unsafe fn clear_main_chain_cache(tif: *mut TIFF) {
    let state = directory_state_mut(tif);
    state.main_offsets.clear();
    state.main_next_offsets.clear();
    state.main_complete = false;
    state.subifd_seed_offsets.clear();
}

unsafe fn initialize_writable_directory(tif: *mut TIFF, kind: DirectoryKind) -> bool {
    let ok = match kind {
        DirectoryKind::Main | DirectoryKind::SubIfd => reset_default_directory(tif),
        DirectoryKind::Custom(info) => reset_field_registry_with_array(tif, info),
    };
    if !ok {
        emit_error_message(
            tif,
            "TIFFCreateDirectory",
            "Failed to initialize field registry",
        );
        return false;
    }

    let tags = if matches!(kind, DirectoryKind::Main | DirectoryKind::SubIfd) {
        default_main_directory_tags()
    } else {
        Vec::new()
    };
    set_current_directory(
        tif,
        CurrentDirectory {
            kind,
            offset: 0,
            next_offset: 0,
            tags,
        },
        ActiveChain::None,
        TIFF_NON_EXISTENT_DIR_NUMBER,
    );
    (*tif_inner(tif)).current_diroff = 0;
    (*tif_inner(tif)).next_diroff = 0;
    clear_main_chain_cache(tif);
    if matches!(kind, DirectoryKind::Main) {
        directory_state_mut(tif).first_ifd_offset = 0;
    }
    if !matches!(kind, DirectoryKind::SubIfd) {
        directory_state_mut(tif).pending_subifd = None;
    }
    (*tif).tif_flags &= !(TIFF_DIRTYDIRECT | TIFF_DIRTYSTRIP | TIFF_BEENWRITING);
    sync_fillorder_flags(tif);
    true
}

unsafe fn subifd_count_from_directory(current: &CurrentDirectory) -> usize {
    current
        .find_tag(TAG_SUBIFD)
        .map(|entry| entry.count as usize)
        .unwrap_or(0)
}

unsafe fn ensure_writable_directory(tif: *mut TIFF) -> bool {
    if directory_state(tif).current.is_some() {
        true
    } else {
        if !initialize_field_registry(tif) {
            emit_error_message(tif, "TIFFSetField", "Failed to initialize field registry");
            return false;
        }
        set_current_directory(
            tif,
            CurrentDirectory {
                kind: DirectoryKind::Main,
                offset: 0,
                next_offset: 0,
                tags: default_main_directory_tags(),
            },
            ActiveChain::None,
            TIFF_NON_EXISTENT_DIR_NUMBER,
        );
        (*tif_inner(tif)).current_diroff = 0;
        (*tif_inner(tif)).next_diroff = 0;
        clear_main_chain_cache(tif);
        sync_fillorder_flags(tif);
        true
    }
}

unsafe fn current_directory_mut(tif: *mut TIFF) -> Option<&'static mut CurrentDirectory> {
    directory_state_mut(tif).current.as_mut()
}

unsafe fn upsert_current_tag_with_flags(
    tif: *mut TIFF,
    parsed: ParsedTag,
    record_custom: bool,
    mark_directory_dirty: bool,
) -> c_int {
    let tag = parsed.tag;
    let Some(current) = current_directory_mut(tif) else {
        return 0;
    };
    if let Some(existing) = current.find_tag_mut(tag) {
        *existing = parsed;
    } else {
        current.tags.push(parsed);
        current.tags.sort_by_key(|entry| entry.tag);
    }
    if record_custom {
        safe_tiff_record_custom_tag(tif, tag);
    }
    if mark_directory_dirty {
        (*tif).tif_flags |= TIFF_DIRTYDIRECT;
    }
    configure_current_directory_flags(tif, current);
    sync_fillorder_flags(tif);
    1
}

unsafe fn remove_current_tag(tif: *mut TIFF, tag: u32, remove_custom: bool) -> c_int {
    let Some(current) = current_directory_mut(tif) else {
        return 0;
    };
    current.tags.retain(|entry| entry.tag != tag);
    if remove_custom {
        safe_tiff_remove_custom_tag(tif, tag);
    }
    (*tif).tif_flags |= TIFF_DIRTYDIRECT;
    configure_current_directory_flags(tif, current);
    sync_fillorder_flags(tif);
    1
}

unsafe fn current_u16_value_or_default(tif: *mut TIFF, tag: u32, default: u16) -> u16 {
    directory_state(tif)
        .current
        .as_ref()
        .and_then(|current| current.find_tag(tag))
        .and_then(|entry| match &entry.values {
            StoredValues::U16(values) => values.first().copied(),
            _ => None,
        })
        .unwrap_or(default)
}

unsafe fn current_real_value(entry: &ParsedTag) -> Option<f64> {
    match &entry.values {
        StoredValues::F32(values) => values.first().copied().map(|value| value as f64),
        StoredValues::F64(values) => values.first().copied(),
        _ => None,
    }
}

unsafe fn current_color_plane_count(tif: *mut TIFF) -> u16 {
    let samples = current_u16_value_or_default(tif, TAG_SAMPLESPERPIXEL, 1);
    let extras = directory_state(tif)
        .current
        .as_ref()
        .and_then(|current| current.find_tag(TAG_EXTRASAMPLES))
        .map(|entry| entry.count as u16)
        .unwrap_or(0);
    let color_planes = samples.saturating_sub(extras);
    if color_planes == 0 {
        1
    } else {
        color_planes
    }
}

unsafe fn transfer_function_sample_count(tif: *mut TIFF, module_name: &str) -> Option<u64> {
    let bits_per_sample = current_u16_value_or_default(tif, TAG_BITSPERSAMPLE, 1);
    if bits_per_sample >= 31 {
        emit_error_message(
            tif,
            module_name,
            format!(
                "BitsPerSample {} is too large for TransferFunction validation",
                bits_per_sample
            ),
        );
        return None;
    }
    Some(1u64 << bits_per_sample)
}

unsafe fn clear_transferfunction_for_layout_change(
    tif: *mut TIFF,
    module_name: &str,
    reason: &str,
) {
    if let Some(current) = current_directory_mut(tif) {
        let had_transfer_function = current.find_tag(TAG_TRANSFERFUNCTION).is_some();
        if had_transfer_function {
            current
                .tags
                .retain(|entry| entry.tag != TAG_TRANSFERFUNCTION);
            emit_warning_message(tif, module_name, reason);
        }
    }
}

unsafe fn stored_values_from_marshaled(
    tif: *mut TIFF,
    module_name: &str,
    tag: u32,
    storage_type: TIFFDataType,
    count: u64,
    data: *const c_void,
) -> Option<StoredValues> {
    let count_usize = usize::try_from(count).ok()?;
    match storage_type.0 {
        x if x == TIFFDataType::TIFF_ASCII.0 => {
            let mut bytes = if count == 0 {
                vec![0u8]
            } else {
                if data.is_null() {
                    emit_error_message(
                        tif,
                        module_name,
                        format!("Tag {} data pointer is NULL", tag),
                    );
                    return None;
                }
                checked_allocation_len(
                    tif,
                    module_name,
                    "ASCII tag data",
                    count_usize.saturating_add(1),
                )?;
                slice::from_raw_parts(data.cast::<u8>(), count_usize).to_vec()
            };
            if bytes.last().copied() != Some(0) {
                bytes.push(0);
            }
            Some(StoredValues::U8(bytes.into_boxed_slice()))
        }
        x if x == TIFFDataType::TIFF_BYTE.0 || x == TIFFDataType::TIFF_UNDEFINED.0 => {
            if count != 0 && data.is_null() {
                emit_error_message(
                    tif,
                    module_name,
                    format!("Tag {} data pointer is NULL", tag),
                );
                return None;
            }
            checked_allocation_len(tif, module_name, "byte tag data", count_usize)?;
            let values = if count == 0 {
                Vec::new()
            } else {
                slice::from_raw_parts(data.cast::<u8>(), count_usize).to_vec()
            };
            Some(StoredValues::U8(values.into_boxed_slice()))
        }
        x if x == TIFFDataType::TIFF_SBYTE.0 => {
            if count != 0 && data.is_null() {
                emit_error_message(
                    tif,
                    module_name,
                    format!("Tag {} data pointer is NULL", tag),
                );
                return None;
            }
            checked_allocation_len(tif, module_name, "signed byte tag data", count_usize)?;
            let values = if count == 0 {
                Vec::new()
            } else {
                slice::from_raw_parts(data.cast::<i8>(), count_usize).to_vec()
            };
            Some(StoredValues::I8(values.into_boxed_slice()))
        }
        x if x == TIFFDataType::TIFF_SHORT.0 => {
            if count != 0 && data.is_null() {
                emit_error_message(
                    tif,
                    module_name,
                    format!("Tag {} data pointer is NULL", tag),
                );
                return None;
            }
            checked_allocation_len(
                tif,
                module_name,
                "short tag data",
                count_usize.checked_mul(size_of::<u16>())?,
            )?;
            let values = if count == 0 {
                Vec::new()
            } else {
                slice::from_raw_parts(data.cast::<u16>(), count_usize).to_vec()
            };
            Some(StoredValues::U16(values.into_boxed_slice()))
        }
        x if x == TIFFDataType::TIFF_SSHORT.0 => {
            if count != 0 && data.is_null() {
                emit_error_message(
                    tif,
                    module_name,
                    format!("Tag {} data pointer is NULL", tag),
                );
                return None;
            }
            checked_allocation_len(
                tif,
                module_name,
                "signed short tag data",
                count_usize.checked_mul(size_of::<i16>())?,
            )?;
            let values = if count == 0 {
                Vec::new()
            } else {
                slice::from_raw_parts(data.cast::<i16>(), count_usize).to_vec()
            };
            Some(StoredValues::I16(values.into_boxed_slice()))
        }
        x if x == TIFFDataType::TIFF_LONG.0 || x == TIFFDataType::TIFF_IFD.0 => {
            if count != 0 && data.is_null() {
                emit_error_message(
                    tif,
                    module_name,
                    format!("Tag {} data pointer is NULL", tag),
                );
                return None;
            }
            checked_allocation_len(
                tif,
                module_name,
                "long tag data",
                count_usize.checked_mul(size_of::<u32>())?,
            )?;
            let values = if count == 0 {
                Vec::new()
            } else {
                slice::from_raw_parts(data.cast::<u32>(), count_usize).to_vec()
            };
            Some(StoredValues::U32(values.into_boxed_slice()))
        }
        x if x == TIFFDataType::TIFF_SLONG.0 => {
            if count != 0 && data.is_null() {
                emit_error_message(
                    tif,
                    module_name,
                    format!("Tag {} data pointer is NULL", tag),
                );
                return None;
            }
            checked_allocation_len(
                tif,
                module_name,
                "signed long tag data",
                count_usize.checked_mul(size_of::<i32>())?,
            )?;
            let values = if count == 0 {
                Vec::new()
            } else {
                slice::from_raw_parts(data.cast::<i32>(), count_usize).to_vec()
            };
            Some(StoredValues::I32(values.into_boxed_slice()))
        }
        x if x == TIFFDataType::TIFF_LONG8.0 || x == TIFFDataType::TIFF_IFD8.0 => {
            if count != 0 && data.is_null() {
                emit_error_message(
                    tif,
                    module_name,
                    format!("Tag {} data pointer is NULL", tag),
                );
                return None;
            }
            checked_allocation_len(
                tif,
                module_name,
                "long8 tag data",
                count_usize.checked_mul(size_of::<u64>())?,
            )?;
            let values = if count == 0 {
                Vec::new()
            } else {
                slice::from_raw_parts(data.cast::<u64>(), count_usize).to_vec()
            };
            Some(StoredValues::U64(values.into_boxed_slice()))
        }
        x if x == TIFFDataType::TIFF_SLONG8.0 => {
            if count != 0 && data.is_null() {
                emit_error_message(
                    tif,
                    module_name,
                    format!("Tag {} data pointer is NULL", tag),
                );
                return None;
            }
            checked_allocation_len(
                tif,
                module_name,
                "signed long8 tag data",
                count_usize.checked_mul(size_of::<i64>())?,
            )?;
            let values = if count == 0 {
                Vec::new()
            } else {
                slice::from_raw_parts(data.cast::<i64>(), count_usize).to_vec()
            };
            Some(StoredValues::I64(values.into_boxed_slice()))
        }
        x if x == TIFFDataType::TIFF_FLOAT.0 => {
            if count != 0 && data.is_null() {
                emit_error_message(
                    tif,
                    module_name,
                    format!("Tag {} data pointer is NULL", tag),
                );
                return None;
            }
            checked_allocation_len(
                tif,
                module_name,
                "float tag data",
                count_usize.checked_mul(size_of::<f32>())?,
            )?;
            let values = if count == 0 {
                Vec::new()
            } else {
                slice::from_raw_parts(data.cast::<f32>(), count_usize).to_vec()
            };
            Some(StoredValues::F32(values.into_boxed_slice()))
        }
        x if x == TIFFDataType::TIFF_DOUBLE.0
            || x == TIFFDataType::TIFF_RATIONAL.0
            || x == TIFFDataType::TIFF_SRATIONAL.0 =>
        {
            if count != 0 && data.is_null() {
                emit_error_message(
                    tif,
                    module_name,
                    format!("Tag {} data pointer is NULL", tag),
                );
                return None;
            }
            checked_allocation_len(
                tif,
                module_name,
                "double tag data",
                count_usize.checked_mul(size_of::<f64>())?,
            )?;
            let values = if count == 0 {
                Vec::new()
            } else {
                slice::from_raw_parts(data.cast::<f64>(), count_usize).to_vec()
            };
            Some(StoredValues::F64(values.into_boxed_slice()))
        }
        _ => {
            emit_error_message(
                tif,
                module_name,
                format!(
                    "Tag {} uses unsupported set-field storage type {}",
                    tag, storage_type.0
                ),
            );
            None
        }
    }
}

unsafe fn single_u16(entry: &ParsedTag) -> Option<u16> {
    match &entry.values {
        StoredValues::U16(values) if values.len() == 1 => Some(values[0]),
        _ => None,
    }
}

unsafe fn single_u32(entry: &ParsedTag) -> Option<u32> {
    match &entry.values {
        StoredValues::U32(values) if values.len() == 1 => Some(values[0]),
        _ => None,
    }
}

unsafe fn validate_and_normalize_tag(
    tif: *mut TIFF,
    field_bit: u16,
    tag: &mut u32,
    parsed: &mut ParsedTag,
) -> bool {
    let module_name = "_TIFFVSetField";
    match *tag {
        TAG_COMPRESSION => {
            let Some(value) = single_u16(parsed) else {
                emit_error_message(
                    tif,
                    module_name,
                    "Compression expects a single uint16 value",
                );
                return false;
            };
            if TIFFSetCompressionScheme(tif, value as c_int) == 0 {
                return false;
            }
        }
        TAG_FILLORDER => {
            let Some(value) = single_u16(parsed) else {
                emit_error_message(tif, module_name, "FillOrder expects a single uint16 value");
                return false;
            };
            if value != FILLORDER_MSB2LSB_U16 && value != FILLORDER_LSB2MSB_U16 {
                emit_error_message(
                    tif,
                    module_name,
                    format!("Bad value {} for FillOrder tag", value),
                );
                return false;
            }
        }
        TAG_ORIENTATION => {
            let Some(value) = single_u16(parsed) else {
                emit_error_message(
                    tif,
                    module_name,
                    "Orientation expects a single uint16 value",
                );
                return false;
            };
            if !(ORIENTATION_TOPLEFT..=8).contains(&value) {
                emit_error_message(
                    tif,
                    module_name,
                    format!("Bad value {} for Orientation tag", value),
                );
                return false;
            }
        }
        TAG_SAMPLESPERPIXEL => {
            let Some(value) = single_u16(parsed) else {
                emit_error_message(
                    tif,
                    module_name,
                    "SamplesPerPixel expects a single uint16 value",
                );
                return false;
            };
            if value == 0 {
                emit_error_message(tif, module_name, "Bad value 0 for SamplesPerPixel tag");
                return false;
            }
            clear_transferfunction_for_layout_change(
                tif,
                module_name,
                "TransferFunction was cleared after SamplesPerPixel changed",
            );
        }
        TAG_ROWSPERSTRIP | TAG_TILEDEPTH => {
            let Some(value) = single_u32(parsed) else {
                emit_error_message(
                    tif,
                    module_name,
                    format!("Tag {} expects a single uint32 value", tag),
                );
                return false;
            };
            if value == 0 {
                emit_error_message(tif, module_name, format!("Bad value 0 for tag {}", tag));
                return false;
            }
        }
        TAG_XRESOLUTION | TAG_YRESOLUTION => {
            let Some(value) = current_real_value(parsed) else {
                emit_error_message(
                    tif,
                    module_name,
                    format!("Tag {} expects a floating-point value", tag),
                );
                return false;
            };
            if !value.is_finite() || value < 0.0 {
                emit_error_message(
                    tif,
                    module_name,
                    format!("Bad value {} for tag {}", value, tag),
                );
                return false;
            }
        }
        TAG_PLANARCONFIG => {
            let Some(value) = single_u16(parsed) else {
                emit_error_message(
                    tif,
                    module_name,
                    "PlanarConfiguration expects a single uint16 value",
                );
                return false;
            };
            if value != PLANARCONFIG_CONTIG && value != PLANARCONFIG_SEPARATE {
                emit_error_message(
                    tif,
                    module_name,
                    format!("Bad value {} for PlanarConfiguration tag", value),
                );
                return false;
            }
        }
        TAG_RESOLUTIONUNIT => {
            let Some(value) = single_u16(parsed) else {
                emit_error_message(
                    tif,
                    module_name,
                    "ResolutionUnit expects a single uint16 value",
                );
                return false;
            };
            if !(RESUNIT_NONE..=RESUNIT_CENTIMETER).contains(&value) {
                emit_error_message(
                    tif,
                    module_name,
                    format!("Bad value {} for ResolutionUnit tag", value),
                );
                return false;
            }
        }
        TAG_DATATYPE => {
            let Some(value) = single_u16(parsed) else {
                emit_error_message(tif, module_name, "DataType expects a single uint16 value");
                return false;
            };
            let mapped = match value {
                0 => SAMPLEFORMAT_VOID,
                1 => SAMPLEFORMAT_INT,
                2 => SAMPLEFORMAT_UINT,
                3 => SAMPLEFORMAT_IEEEFP,
                _ => {
                    emit_error_message(
                        tif,
                        module_name,
                        format!("Bad value {} for DataType tag", value),
                    );
                    return false;
                }
            };
            *tag = TAG_SAMPLEFORMAT;
            parsed.tag = TAG_SAMPLEFORMAT;
            parsed.values = StoredValues::U16(Box::<[u16]>::from([mapped]));
        }
        TAG_SAMPLEFORMAT => {
            let Some(value) = single_u16(parsed) else {
                emit_error_message(
                    tif,
                    module_name,
                    "SampleFormat expects a single uint16 value",
                );
                return false;
            };
            if !(SAMPLEFORMAT_UINT..=SAMPLEFORMAT_COMPLEXIEEEFP).contains(&value) {
                emit_error_message(
                    tif,
                    module_name,
                    format!("Bad value {} for SampleFormat tag", value),
                );
                return false;
            }
        }
        TAG_SUBIFD => {
            if matches!(
                directory_state(tif)
                    .current
                    .as_ref()
                    .map(|current| current.kind),
                Some(DirectoryKind::SubIfd)
            ) {
                emit_error_message(tif, module_name, "Sorry, cannot nest SubIFDs");
                return false;
            }
        }
        TAG_EXTRASAMPLES => {
            let count = parsed.count as u16;
            let samples = current_u16_value_or_default(tif, TAG_SAMPLESPERPIXEL, 1);
            if count > samples {
                emit_error_message(
                    tif,
                    module_name,
                    format!(
                        "Bad ExtraSamples count {} for SamplesPerPixel {}",
                        count, samples
                    ),
                );
                return false;
            }
            let values = match &parsed.values {
                StoredValues::U16(values) => values,
                _ => {
                    emit_error_message(tif, module_name, "ExtraSamples expects uint16 array data");
                    return false;
                }
            };
            for value in values.iter().copied() {
                if value > EXTRASAMPLE_UNASSALPHA {
                    emit_error_message(
                        tif,
                        module_name,
                        format!("Bad ExtraSamples value {}", value),
                    );
                    return false;
                }
            }
            clear_transferfunction_for_layout_change(
                tif,
                module_name,
                "TransferFunction was cleared after ExtraSamples changed",
            );
        }
        TAG_TRANSFERFUNCTION => {
            let sample_count = match transfer_function_sample_count(tif, module_name) {
                Some(value) => value,
                None => return false,
            };
            let plane_count = if current_color_plane_count(tif) > 1 {
                3
            } else {
                1
            };
            let expected_count = sample_count.saturating_mul(plane_count);
            if parsed.count != expected_count {
                emit_error_message(
                    tif,
                    module_name,
                    format!(
                        "TransferFunction expects {} SHORT values, got {}",
                        expected_count, parsed.count
                    ),
                );
                return false;
            }
            if !matches!(parsed.values, StoredValues::U16(_)) {
                emit_error_message(
                    tif,
                    module_name,
                    "TransferFunction expects uint16 array data",
                );
                return false;
            }
        }
        _ => {
            if field_bit != FIELD_CUSTOM && parsed.count == 0 && *tag != TAG_EXTRASAMPLES {
                emit_error_message(tif, module_name, format!("Null count for tag {}", tag));
                return false;
            }
        }
    }

    if !matches!(*tag, TAG_TRANSFERFUNCTION | TAG_EXTRASAMPLES) {
        if let Some(current) = directory_state(tif).current.as_ref() {
            if let Some(entry) = current.find_tag(TAG_TRANSFERFUNCTION) {
                let sample_count = transfer_function_sample_count(tif, module_name).unwrap_or(0);
                let plane_count = if current_color_plane_count(tif) > 1 {
                    3
                } else {
                    1
                };
                if entry.count != sample_count.saturating_mul(plane_count) {
                    clear_transferfunction_for_layout_change(
                        tif,
                        module_name,
                        "TransferFunction was cleared after dependent directory metadata changed",
                    );
                }
            }
        }
    }

    true
}

#[no_mangle]
pub unsafe extern "C" fn safe_tiff_set_field_marshaled(
    tif: *mut TIFF,
    tag: u32,
    storage_type: TIFFDataType,
    count: u64,
    data: *const c_void,
) -> c_int {
    safe_tiff_set_field_marshaled_impl(tif, tag, storage_type, count, data, true)
}

pub(crate) unsafe fn safe_tiff_set_field_marshaled_nondirty(
    tif: *mut TIFF,
    tag: u32,
    storage_type: TIFFDataType,
    count: u64,
    data: *const c_void,
) -> c_int {
    safe_tiff_set_field_marshaled_impl(tif, tag, storage_type, count, data, false)
}

unsafe fn safe_tiff_set_field_marshaled_impl(
    tif: *mut TIFF,
    tag: u32,
    storage_type: TIFFDataType,
    count: u64,
    data: *const c_void,
    mark_directory_dirty: bool,
) -> c_int {
    if tif.is_null() {
        return 0;
    }
    if !ensure_writable_directory(tif) {
        return 0;
    }

    let field = TIFFFindField(tif, tag, TIFFDataType::TIFF_NOTYPE);
    if field.is_null() {
        emit_error_message(tif, "TIFFSetField", format!("Unknown tag {}", tag));
        return 0;
    }

    let Some(values) =
        stored_values_from_marshaled(tif, "_TIFFVSetField", tag, storage_type, count, data)
    else {
        return 0;
    };
    let mut parsed = ParsedTag {
        tag,
        canonical_type: storage_type,
        count,
        values,
    };
    let mut normalized_tag = tag;
    if !validate_and_normalize_tag(tif, (*field).field_bit, &mut normalized_tag, &mut parsed) {
        return 0;
    }

    upsert_current_tag_with_flags(
        tif,
        parsed,
        (*field).field_bit == FIELD_CUSTOM,
        mark_directory_dirty,
    )
}

#[no_mangle]
pub unsafe extern "C" fn safe_tiff_unset_field(tif: *mut TIFF, tag: u32) -> c_int {
    if tif.is_null() {
        return 0;
    }
    if !ensure_writable_directory(tif) {
        return 0;
    }
    let field = TIFFFindField(tif, tag, TIFFDataType::TIFF_NOTYPE);
    if field.is_null() {
        return 0;
    }
    let actual_tag = match tag {
        TAG_DATATYPE => TAG_SAMPLEFORMAT,
        _ => tag,
    };
    remove_current_tag(tif, actual_tag, (*field).field_bit == FIELD_CUSTOM)
}

#[derive(Clone, Copy)]
struct DirectoryEncoding {
    count_size: usize,
    entry_size: usize,
    next_size: usize,
    inline_size: usize,
    alignment: u64,
    big_endian: bool,
    classic: bool,
}

struct EncodedDirectoryEntry {
    tag: u16,
    field_type: TIFFDataType,
    count: u64,
    data: Vec<u8>,
    payload_offset: u64,
}

unsafe fn directory_encoding(tif: *mut TIFF, module_name: &str) -> Option<DirectoryEncoding> {
    let inner = tif_inner(tif);
    match (*inner).header_version {
        TIFF_VERSION_CLASSIC => Some(DirectoryEncoding {
            count_size: 2,
            entry_size: 12,
            next_size: 4,
            inline_size: 4,
            alignment: 2,
            big_endian: (*inner).header_magic == TIFF_BIGENDIAN,
            classic: true,
        }),
        TIFF_VERSION_BIG => Some(DirectoryEncoding {
            count_size: 8,
            entry_size: 20,
            next_size: 8,
            inline_size: 8,
            alignment: 8,
            big_endian: (*inner).header_magic == TIFF_BIGENDIAN,
            classic: false,
        }),
        _ => {
            emit_error_message(tif, module_name, "TIFF header is not initialized");
            None
        }
    }
}

fn encode_u16_bytes(value: u16, big_endian: bool) -> [u8; 2] {
    if big_endian {
        value.to_be_bytes()
    } else {
        value.to_le_bytes()
    }
}

fn encode_u32_bytes(value: u32, big_endian: bool) -> [u8; 4] {
    if big_endian {
        value.to_be_bytes()
    } else {
        value.to_le_bytes()
    }
}

fn encode_u64_bytes(value: u64, big_endian: bool) -> [u8; 8] {
    if big_endian {
        value.to_be_bytes()
    } else {
        value.to_le_bytes()
    }
}

fn encode_i16_bytes(value: i16, big_endian: bool) -> [u8; 2] {
    encode_u16_bytes(u16::from_ne_bytes(value.to_ne_bytes()), big_endian)
}

fn encode_i32_bytes(value: i32, big_endian: bool) -> [u8; 4] {
    encode_u32_bytes(u32::from_ne_bytes(value.to_ne_bytes()), big_endian)
}

fn encode_i64_bytes(value: i64, big_endian: bool) -> [u8; 8] {
    encode_u64_bytes(u64::from_ne_bytes(value.to_ne_bytes()), big_endian)
}

fn append_u16_bytes(buffer: &mut Vec<u8>, value: u16, big_endian: bool) {
    buffer.extend_from_slice(&encode_u16_bytes(value, big_endian));
}

fn append_u32_bytes(buffer: &mut Vec<u8>, value: u32, big_endian: bool) {
    buffer.extend_from_slice(&encode_u32_bytes(value, big_endian));
}

fn append_u64_bytes(buffer: &mut Vec<u8>, value: u64, big_endian: bool) {
    buffer.extend_from_slice(&encode_u64_bytes(value, big_endian));
}

fn append_i16_bytes(buffer: &mut Vec<u8>, value: i16, big_endian: bool) {
    buffer.extend_from_slice(&encode_i16_bytes(value, big_endian));
}

fn append_i32_bytes(buffer: &mut Vec<u8>, value: i32, big_endian: bool) {
    buffer.extend_from_slice(&encode_i32_bytes(value, big_endian));
}

fn append_i64_bytes(buffer: &mut Vec<u8>, value: i64, big_endian: bool) {
    buffer.extend_from_slice(&encode_i64_bytes(value, big_endian));
}

fn append_f32_bytes(buffer: &mut Vec<u8>, value: f32, big_endian: bool) {
    append_u32_bytes(buffer, value.to_bits(), big_endian);
}

fn append_f64_bytes(buffer: &mut Vec<u8>, value: f64, big_endian: bool) {
    append_u64_bytes(buffer, value.to_bits(), big_endian);
}

fn gcd_u64(mut lhs: u64, mut rhs: u64) -> u64 {
    while rhs != 0 {
        let remainder = lhs % rhs;
        lhs = rhs;
        rhs = remainder;
    }
    lhs
}

unsafe fn read_offset_value_at(
    tif: *mut TIFF,
    offset: u64,
    width: usize,
    module_name: &str,
) -> Option<u64> {
    let mut bytes = [0u8; 8];
    if !read_exact_at(tif, offset, &mut bytes[..width]) {
        emit_error_message(
            tif,
            module_name,
            format!("Cannot read directory offset at {}", offset),
        );
        return None;
    }
    let big_endian = (*tif_inner(tif)).header_magic == TIFF_BIGENDIAN;
    Some(if width == 4 {
        parse_u32(&bytes[..4], big_endian) as u64
    } else {
        parse_u64(&bytes[..8], big_endian)
    })
}

unsafe fn write_offset_value_at(
    tif: *mut TIFF,
    offset: u64,
    width: usize,
    value: u64,
    module_name: &str,
    what: &str,
) -> bool {
    let big_endian = (*tif_inner(tif)).header_magic == TIFF_BIGENDIAN;
    let bytes = if width == 4 {
        let Ok(value32) = u32::try_from(value) else {
            emit_error_message(
                tif,
                module_name,
                format!("{} offset {} does not fit in Classic TIFF", what, value),
            );
            return false;
        };
        encode_u32_bytes(value32, big_endian).to_vec()
    } else {
        encode_u64_bytes(value, big_endian).to_vec()
    };
    if write_exact_at(tif, offset, &bytes) {
        true
    } else {
        emit_error_message(tif, module_name, format!("Error writing {}", what));
        false
    }
}

unsafe fn directory_next_offset_location(
    tif: *mut TIFF,
    offset: u64,
    module_name: &str,
) -> Option<u64> {
    let fmt = directory_encoding(tif, module_name)?;
    let mut count_bytes = [0u8; 8];
    if !read_exact_at(tif, offset, &mut count_bytes[..fmt.count_size]) {
        emit_error_message(
            tif,
            module_name,
            format!("Cannot read TIFF directory at offset {}", offset),
        );
        return None;
    }
    let entry_count = if fmt.classic {
        parse_u16(&count_bytes[..2], fmt.big_endian) as u64
    } else {
        parse_u64(&count_bytes[..8], fmt.big_endian)
    };
    let entries_len = entry_count.checked_mul(fmt.entry_size as u64)?;
    offset
        .checked_add(fmt.count_size as u64)?
        .checked_add(entries_len)
}

unsafe fn find_tail_link_location(tif: *mut TIFF, module_name: &str) -> Option<u64> {
    let fmt = directory_encoding(tif, module_name)?;
    let mut link_location = header_link_offset(tif);
    let mut next_offset = read_offset_value_at(tif, link_location, fmt.next_size, module_name)?;
    let mut seen = HashSet::new();
    while next_offset != 0 {
        if !seen.insert(next_offset) {
            emit_error_message(
                tif,
                module_name,
                format!("Directory loop detected at offset 0x{:x}", next_offset),
            );
            return None;
        }
        link_location = directory_next_offset_location(tif, next_offset, module_name)?;
        next_offset = read_offset_value_at(tif, link_location, fmt.next_size, module_name)?;
    }
    Some(link_location)
}

unsafe fn find_predecessor_link_location(
    tif: *mut TIFF,
    target_offset: u64,
    module_name: &str,
) -> Option<u64> {
    let fmt = directory_encoding(tif, module_name)?;
    let mut link_location = header_link_offset(tif);
    let mut next_offset = read_offset_value_at(tif, link_location, fmt.next_size, module_name)?;
    let mut seen = HashSet::new();
    while next_offset != 0 {
        if next_offset == target_offset {
            return Some(link_location);
        }
        if !seen.insert(next_offset) {
            break;
        }
        link_location = directory_next_offset_location(tif, next_offset, module_name)?;
        next_offset = read_offset_value_at(tif, link_location, fmt.next_size, module_name)?;
    }
    emit_error_message(
        tif,
        module_name,
        format!(
            "Directory at offset 0x{:x} is not linked from the main chain",
            target_offset
        ),
    );
    None
}

unsafe fn find_directory_link_by_number(
    tif: *mut TIFF,
    dirnum: u32,
    module_name: &str,
) -> Option<(u64, u64, u64)> {
    let fmt = directory_encoding(tif, module_name)?;
    let mut link_location = header_link_offset(tif);
    let mut current_offset = read_offset_value_at(tif, link_location, fmt.next_size, module_name)?;
    let mut current_number = 1u32;
    let mut seen = HashSet::new();

    while current_offset != 0 {
        if current_number == dirnum {
            let next_offset = read_directory_next_offset(tif, current_offset, module_name)?;
            return Some((link_location, current_offset, next_offset));
        }
        if !seen.insert(current_offset) {
            break;
        }
        link_location = directory_next_offset_location(tif, current_offset, module_name)?;
        current_offset = read_offset_value_at(tif, link_location, fmt.next_size, module_name)?;
        current_number = current_number.saturating_add(1);
    }

    emit_error_message(
        tif,
        module_name,
        format!("Directory {} does not exist", dirnum),
    );
    None
}

fn inferred_field_type_from_values(values: &StoredValues) -> TIFFDataType {
    match values {
        StoredValues::U8(_) => TIFFDataType::TIFF_BYTE,
        StoredValues::I8(_) => TIFFDataType::TIFF_SBYTE,
        StoredValues::U16(_) => TIFFDataType::TIFF_SHORT,
        StoredValues::I16(_) => TIFFDataType::TIFF_SSHORT,
        StoredValues::U32(_) => TIFFDataType::TIFF_LONG,
        StoredValues::I32(_) => TIFFDataType::TIFF_SLONG,
        StoredValues::U64(_) => TIFFDataType::TIFF_LONG8,
        StoredValues::I64(_) => TIFFDataType::TIFF_SLONG8,
        StoredValues::F32(_) => TIFFDataType::TIFF_FLOAT,
        StoredValues::F64(_) => TIFFDataType::TIFF_DOUBLE,
    }
}

unsafe fn numeric_values_as_f64(
    tif: *mut TIFF,
    module_name: &str,
    tag: u32,
    values: &StoredValues,
) -> Option<Vec<f64>> {
    let converted = match values {
        StoredValues::U8(values) => values.iter().map(|value| *value as f64).collect(),
        StoredValues::I8(values) => values.iter().map(|value| *value as f64).collect(),
        StoredValues::U16(values) => values.iter().map(|value| *value as f64).collect(),
        StoredValues::I16(values) => values.iter().map(|value| *value as f64).collect(),
        StoredValues::U32(values) => values.iter().map(|value| *value as f64).collect(),
        StoredValues::I32(values) => values.iter().map(|value| *value as f64).collect(),
        StoredValues::U64(values) => values.iter().map(|value| *value as f64).collect(),
        StoredValues::I64(values) => values.iter().map(|value| *value as f64).collect(),
        StoredValues::F32(values) => values.iter().map(|value| *value as f64).collect(),
        StoredValues::F64(values) => values.to_vec(),
    };
    if converted.iter().all(|value| value.is_finite()) {
        Some(converted)
    } else {
        emit_error_message(
            tif,
            module_name,
            format!("Tag {} contains non-finite floating point values", tag),
        );
        None
    }
}

unsafe fn double_to_rational(
    tif: *mut TIFF,
    module_name: &str,
    tag: u32,
    value: f64,
) -> Option<(u32, u32)> {
    if !value.is_finite() || value < 0.0 {
        emit_error_message(
            tif,
            module_name,
            format!(
                "Tag {} value {} is invalid for unsigned rational encoding",
                tag, value
            ),
        );
        return None;
    }
    if value == 0.0 {
        return Some((0, 1));
    }
    if value > u32::MAX as f64 {
        emit_error_message(
            tif,
            module_name,
            format!("Tag {} value {} exceeds Classic rational range", tag, value),
        );
        return None;
    }

    let mut denom = 1_000_000u64;
    let scaled = (value * denom as f64).round();
    if !scaled.is_finite() || scaled < 0.0 || scaled > u64::MAX as f64 {
        emit_error_message(
            tif,
            module_name,
            format!("Tag {} value {} cannot be represented safely", tag, value),
        );
        return None;
    }
    let mut numer = scaled as u64;
    let gcd = gcd_u64(numer, denom);
    numer /= gcd;
    denom /= gcd;

    while numer > u32::MAX as u64 || denom > u32::MAX as u64 {
        if denom <= 1 {
            emit_error_message(
                tif,
                module_name,
                format!("Tag {} value {} exceeds TIFF rational bounds", tag, value),
            );
            return None;
        }
        numer = (numer + 1) / 2;
        denom = (denom + 1) / 2;
        let gcd = gcd_u64(numer, denom);
        numer /= gcd;
        denom /= gcd;
    }

    Some((numer as u32, denom as u32))
}

unsafe fn double_to_srational(
    tif: *mut TIFF,
    module_name: &str,
    tag: u32,
    value: f64,
) -> Option<(i32, i32)> {
    if !value.is_finite() {
        emit_error_message(
            tif,
            module_name,
            format!(
                "Tag {} value {} is invalid for signed rational encoding",
                tag, value
            ),
        );
        return None;
    }
    if value == 0.0 {
        return Some((0, 1));
    }
    if value.abs() > i32::MAX as f64 {
        emit_error_message(
            tif,
            module_name,
            format!("Tag {} value {} exceeds signed rational range", tag, value),
        );
        return None;
    }

    let negative = value.is_sign_negative();
    let (numer, denom) = double_to_rational(tif, module_name, tag, value.abs())?;
    let signed_numer = if negative {
        let Ok(value) = i32::try_from(numer) else {
            emit_error_message(
                tif,
                module_name,
                format!("Tag {} value {} exceeds signed rational range", tag, value),
            );
            return None;
        };
        -value
    } else {
        let Ok(value) = i32::try_from(numer) else {
            emit_error_message(
                tif,
                module_name,
                format!("Tag {} value {} exceeds signed rational range", tag, value),
            );
            return None;
        };
        value
    };
    let Ok(signed_denom) = i32::try_from(denom) else {
        emit_error_message(
            tif,
            module_name,
            format!("Tag {} value {} exceeds signed rational range", tag, value),
        );
        return None;
    };
    Some((signed_numer, signed_denom))
}

unsafe fn expand_values_for_write(
    tif: *mut TIFF,
    module_name: &str,
    parsed: &ParsedTag,
) -> Option<(StoredValues, u64)> {
    let samples = current_u16_value_or_default(tif, TAG_SAMPLESPERPIXEL, 1) as usize;
    if samples <= 1 || parsed.values.len() != 1 {
        return Some((parsed.values.clone(), parsed.values.len() as u64));
    }

    match parsed.tag {
        TAG_BITSPERSAMPLE | TAG_MINSAMPLEVALUE | TAG_MAXSAMPLEVALUE | TAG_SAMPLEFORMAT => {
            let checked_len = samples.checked_mul(size_of::<u16>())?;
            checked_allocation_len(tif, module_name, "per-sample tag data", checked_len)?;
            let base = match &parsed.values {
                StoredValues::U16(values) => values[0],
                _ => return Some((parsed.values.clone(), parsed.values.len() as u64)),
            };
            Some((
                StoredValues::U16(vec![base; samples].into_boxed_slice()),
                samples as u64,
            ))
        }
        _ => Some((parsed.values.clone(), parsed.values.len() as u64)),
    }
}

unsafe fn on_disk_field_type(
    tif: *mut TIFF,
    module_name: &str,
    tag: u32,
    values: &StoredValues,
) -> Option<TIFFDataType> {
    let field = TIFFFindField(tif, tag, TIFFDataType::TIFF_NOTYPE);
    if field.is_null() {
        emit_error_message(tif, module_name, format!("Unknown tag {}", tag));
        return None;
    }
    let field_type = if (*field).field_type.0 == TIFFDataType::TIFF_NOTYPE.0 {
        inferred_field_type_from_values(values)
    } else {
        (*field).field_type
    };
    Some(field_type)
}

unsafe fn normalize_classic_field_type(
    tif: *mut TIFF,
    module_name: &str,
    tag: u32,
    field_type: TIFFDataType,
    values: &StoredValues,
) -> Option<TIFFDataType> {
    if (*tif_inner(tif)).header_version != TIFF_VERSION_CLASSIC {
        return Some(field_type);
    }

    match field_type.0 {
        x if x == TIFFDataType::TIFF_LONG8.0 => {
            let fits = match values {
                StoredValues::U32(_) => true,
                StoredValues::U64(values) => values.iter().all(|value| *value <= u32::MAX as u64),
                _ => false,
            };
            if !fits {
                emit_error_message(
                    tif,
                    module_name,
                    format!("Tag {} requires BigTIFF LONG8 values", tag),
                );
                return None;
            }
            Some(TIFFDataType::TIFF_LONG)
        }
        x if x == TIFFDataType::TIFF_IFD8.0 => {
            let fits = match values {
                StoredValues::U32(_) => true,
                StoredValues::U64(values) => values.iter().all(|value| *value <= u32::MAX as u64),
                _ => false,
            };
            if !fits {
                emit_error_message(
                    tif,
                    module_name,
                    format!("Tag {} requires BigTIFF IFD8 values", tag),
                );
                return None;
            }
            Some(TIFFDataType::TIFF_IFD)
        }
        x if x == TIFFDataType::TIFF_SLONG8.0 => {
            let fits = match values {
                StoredValues::I32(_) => true,
                StoredValues::I64(values) => values
                    .iter()
                    .all(|value| *value >= i32::MIN as i64 && *value <= i32::MAX as i64),
                _ => false,
            };
            if !fits {
                emit_error_message(
                    tif,
                    module_name,
                    format!("Tag {} requires BigTIFF SLONG8 values", tag),
                );
                return None;
            }
            Some(TIFFDataType::TIFF_SLONG)
        }
        _ => Some(field_type),
    }
}

unsafe fn encode_stored_values_as_type(
    tif: *mut TIFF,
    module_name: &str,
    tag: u32,
    field_type: TIFFDataType,
    values: &StoredValues,
    count: u64,
    big_endian: bool,
) -> Option<Vec<u8>> {
    let byte_len = checked_total_bytes(tif, count, type_width(field_type)?, module_name, tag)?;
    checked_allocation_len(tif, module_name, "directory tag payload", byte_len)?;

    let mut buffer = Vec::with_capacity(byte_len);
    match field_type.0 {
        x if x == TIFFDataType::TIFF_ASCII.0 => {
            let StoredValues::U8(values) = values else {
                emit_error_message(tif, module_name, format!("Tag {} expects ASCII bytes", tag));
                return None;
            };
            buffer.extend_from_slice(values);
        }
        x if x == TIFFDataType::TIFF_BYTE.0 || x == TIFFDataType::TIFF_UNDEFINED.0 => {
            let StoredValues::U8(values) = values else {
                emit_error_message(tif, module_name, format!("Tag {} expects byte data", tag));
                return None;
            };
            buffer.extend_from_slice(values);
        }
        x if x == TIFFDataType::TIFF_SBYTE.0 => {
            let StoredValues::I8(values) = values else {
                emit_error_message(
                    tif,
                    module_name,
                    format!("Tag {} expects signed byte data", tag),
                );
                return None;
            };
            buffer.extend(values.iter().map(|value| *value as u8));
        }
        x if x == TIFFDataType::TIFF_SHORT.0 => match values {
            StoredValues::U16(values) => {
                for value in values.iter().copied() {
                    append_u16_bytes(&mut buffer, value, big_endian);
                }
            }
            StoredValues::U32(values) => {
                for value in values.iter().copied() {
                    let Ok(value16) = u16::try_from(value) else {
                        emit_error_message(
                            tif,
                            module_name,
                            format!("Tag {} value {} exceeds uint16 range", tag, value),
                        );
                        return None;
                    };
                    append_u16_bytes(&mut buffer, value16, big_endian);
                }
            }
            StoredValues::U64(values) => {
                for value in values.iter().copied() {
                    let Ok(value16) = u16::try_from(value) else {
                        emit_error_message(
                            tif,
                            module_name,
                            format!("Tag {} value {} exceeds uint16 range", tag, value),
                        );
                        return None;
                    };
                    append_u16_bytes(&mut buffer, value16, big_endian);
                }
            }
            _ => {
                emit_error_message(tif, module_name, format!("Tag {} expects uint16 data", tag));
                return None;
            }
        },
        x if x == TIFFDataType::TIFF_SSHORT.0 => match values {
            StoredValues::I16(values) => {
                for value in values.iter().copied() {
                    append_i16_bytes(&mut buffer, value, big_endian);
                }
            }
            StoredValues::I32(values) => {
                for value in values.iter().copied() {
                    let Ok(value16) = i16::try_from(value) else {
                        emit_error_message(
                            tif,
                            module_name,
                            format!("Tag {} value {} exceeds int16 range", tag, value),
                        );
                        return None;
                    };
                    append_i16_bytes(&mut buffer, value16, big_endian);
                }
            }
            StoredValues::I64(values) => {
                for value in values.iter().copied() {
                    let Ok(value16) = i16::try_from(value) else {
                        emit_error_message(
                            tif,
                            module_name,
                            format!("Tag {} value {} exceeds int16 range", tag, value),
                        );
                        return None;
                    };
                    append_i16_bytes(&mut buffer, value16, big_endian);
                }
            }
            _ => {
                emit_error_message(tif, module_name, format!("Tag {} expects int16 data", tag));
                return None;
            }
        },
        x if x == TIFFDataType::TIFF_LONG.0 || x == TIFFDataType::TIFF_IFD.0 => match values {
            StoredValues::U32(values) => {
                for value in values.iter().copied() {
                    append_u32_bytes(&mut buffer, value, big_endian);
                }
            }
            StoredValues::U64(values) => {
                for value in values.iter().copied() {
                    let Ok(value32) = u32::try_from(value) else {
                        emit_error_message(
                            tif,
                            module_name,
                            format!("Tag {} value {} exceeds uint32 range", tag, value),
                        );
                        return None;
                    };
                    append_u32_bytes(&mut buffer, value32, big_endian);
                }
            }
            _ => {
                emit_error_message(tif, module_name, format!("Tag {} expects uint32 data", tag));
                return None;
            }
        },
        x if x == TIFFDataType::TIFF_SLONG.0 => match values {
            StoredValues::I32(values) => {
                for value in values.iter().copied() {
                    append_i32_bytes(&mut buffer, value, big_endian);
                }
            }
            StoredValues::I64(values) => {
                for value in values.iter().copied() {
                    let Ok(value32) = i32::try_from(value) else {
                        emit_error_message(
                            tif,
                            module_name,
                            format!("Tag {} value {} exceeds int32 range", tag, value),
                        );
                        return None;
                    };
                    append_i32_bytes(&mut buffer, value32, big_endian);
                }
            }
            _ => {
                emit_error_message(tif, module_name, format!("Tag {} expects int32 data", tag));
                return None;
            }
        },
        x if x == TIFFDataType::TIFF_LONG8.0 || x == TIFFDataType::TIFF_IFD8.0 => match values {
            StoredValues::U32(values) => {
                for value in values.iter().copied() {
                    append_u64_bytes(&mut buffer, value as u64, big_endian);
                }
            }
            StoredValues::U64(values) => {
                for value in values.iter().copied() {
                    append_u64_bytes(&mut buffer, value, big_endian);
                }
            }
            _ => {
                emit_error_message(tif, module_name, format!("Tag {} expects uint64 data", tag));
                return None;
            }
        },
        x if x == TIFFDataType::TIFF_SLONG8.0 => match values {
            StoredValues::I32(values) => {
                for value in values.iter().copied() {
                    append_i64_bytes(&mut buffer, value as i64, big_endian);
                }
            }
            StoredValues::I64(values) => {
                for value in values.iter().copied() {
                    append_i64_bytes(&mut buffer, value, big_endian);
                }
            }
            _ => {
                emit_error_message(tif, module_name, format!("Tag {} expects int64 data", tag));
                return None;
            }
        },
        x if x == TIFFDataType::TIFF_FLOAT.0 => match values {
            StoredValues::F32(values) => {
                for value in values.iter().copied() {
                    append_f32_bytes(&mut buffer, value, big_endian);
                }
            }
            StoredValues::F64(values) => {
                for value in values.iter().copied() {
                    if !value.is_finite() || value < f32::MIN as f64 || value > f32::MAX as f64 {
                        emit_error_message(
                            tif,
                            module_name,
                            format!("Tag {} value {} exceeds float range", tag, value),
                        );
                        return None;
                    }
                    append_f32_bytes(&mut buffer, value as f32, big_endian);
                }
            }
            _ => {
                emit_error_message(tif, module_name, format!("Tag {} expects float data", tag));
                return None;
            }
        },
        x if x == TIFFDataType::TIFF_DOUBLE.0 => match values {
            StoredValues::F32(values) => {
                for value in values.iter().copied() {
                    append_f64_bytes(&mut buffer, value as f64, big_endian);
                }
            }
            StoredValues::F64(values) => {
                for value in values.iter().copied() {
                    append_f64_bytes(&mut buffer, value, big_endian);
                }
            }
            _ => {
                emit_error_message(tif, module_name, format!("Tag {} expects double data", tag));
                return None;
            }
        },
        x if x == TIFFDataType::TIFF_RATIONAL.0 => {
            for value in numeric_values_as_f64(tif, module_name, tag, values)? {
                let (numer, denom) = double_to_rational(tif, module_name, tag, value)?;
                append_u32_bytes(&mut buffer, numer, big_endian);
                append_u32_bytes(&mut buffer, denom, big_endian);
            }
        }
        x if x == TIFFDataType::TIFF_SRATIONAL.0 => {
            for value in numeric_values_as_f64(tif, module_name, tag, values)? {
                let (numer, denom) = double_to_srational(tif, module_name, tag, value)?;
                append_i32_bytes(&mut buffer, numer, big_endian);
                append_i32_bytes(&mut buffer, denom, big_endian);
            }
        }
        _ => {
            emit_error_message(
                tif,
                module_name,
                format!("Tag {} uses unsupported on-disk type {}", tag, field_type.0),
            );
            return None;
        }
    }
    Some(buffer)
}

unsafe fn encode_directory_entry(
    tif: *mut TIFF,
    module_name: &str,
    parsed: &ParsedTag,
) -> Option<EncodedDirectoryEntry> {
    let Ok(tag) = u16::try_from(parsed.tag) else {
        emit_error_message(
            tif,
            module_name,
            format!("Tag {} is outside the TIFF on-disk range", parsed.tag),
        );
        return None;
    };
    let (values, count) = expand_values_for_write(tif, module_name, parsed)?;
    let field_type = on_disk_field_type(tif, module_name, parsed.tag, &values)?;
    let field_type =
        normalize_classic_field_type(tif, module_name, parsed.tag, field_type, &values)?;
    let data = encode_stored_values_as_type(
        tif,
        module_name,
        parsed.tag,
        field_type,
        &values,
        count,
        (*tif_inner(tif)).header_magic == TIFF_BIGENDIAN,
    )?;
    Some(EncodedDirectoryEntry {
        tag,
        field_type,
        count,
        data,
        payload_offset: 0,
    })
}

unsafe fn assign_directory_payload_offsets(
    tif: *mut TIFF,
    module_name: &str,
    dir_offset: u64,
    fmt: DirectoryEncoding,
    entries: &mut [EncodedDirectoryEntry],
) -> Option<()> {
    let base_size = (fmt.count_size as u64)
        .checked_add((entries.len() as u64).checked_mul(fmt.entry_size as u64)?)?
        .checked_add(fmt.next_size as u64)?;
    let mut cursor = align_up(dir_offset.checked_add(base_size)?, fmt.alignment)?;
    for entry in entries.iter_mut() {
        if entry.data.len() > fmt.inline_size {
            entry.payload_offset = cursor;
            cursor = align_up(cursor.checked_add(entry.data.len() as u64)?, fmt.alignment)?;
        } else {
            entry.payload_offset = 0;
        }
    }
    let _ = tif;
    let _ = module_name;
    Some(())
}

unsafe fn write_encoded_directory(
    tif: *mut TIFF,
    module_name: &str,
    dir_offset: u64,
    next_offset: u64,
    entries: &mut [EncodedDirectoryEntry],
) -> bool {
    let fmt = match directory_encoding(tif, module_name) {
        Some(fmt) => fmt,
        None => return false,
    };
    if assign_directory_payload_offsets(tif, module_name, dir_offset, fmt, entries).is_none() {
        emit_error_message(
            tif,
            module_name,
            "Directory layout overflowed file addressing",
        );
        return false;
    }

    let count_bytes = if fmt.classic {
        let Ok(count) = u16::try_from(entries.len()) else {
            emit_error_message(tif, module_name, "Too many tags for Classic TIFF");
            return false;
        };
        encode_u16_bytes(count, fmt.big_endian).to_vec()
    } else {
        encode_u64_bytes(entries.len() as u64, fmt.big_endian).to_vec()
    };

    let entries_buffer_len = match (entries.len() as u64)
        .checked_mul(fmt.entry_size as u64)
        .and_then(|value| value.checked_add(fmt.next_size as u64))
        .and_then(|value| usize::try_from(value).ok())
    {
        Some(value) => value,
        None => {
            emit_error_message(
                tif,
                module_name,
                "Directory is too large to serialize safely",
            );
            return false;
        }
    };
    if checked_allocation_len(
        tif,
        module_name,
        "directory entry table",
        entries_buffer_len,
    )
    .is_none()
    {
        return false;
    }

    let mut entries_buffer = vec![0u8; entries_buffer_len];
    for (index, entry) in entries.iter().enumerate() {
        let base = index * fmt.entry_size;
        entries_buffer[base..base + 2]
            .copy_from_slice(&encode_u16_bytes(entry.tag, fmt.big_endian));
        entries_buffer[base + 2..base + 4]
            .copy_from_slice(&encode_u16_bytes(entry.field_type.0 as u16, fmt.big_endian));
        if fmt.classic {
            let Ok(count32) = u32::try_from(entry.count) else {
                emit_error_message(
                    tif,
                    module_name,
                    format!(
                        "Tag {} count {} exceeds Classic TIFF range",
                        entry.tag, entry.count
                    ),
                );
                return false;
            };
            entries_buffer[base + 4..base + 8]
                .copy_from_slice(&encode_u32_bytes(count32, fmt.big_endian));
            if entry.data.len() <= fmt.inline_size {
                entries_buffer[base + 8..base + 8 + entry.data.len()].copy_from_slice(&entry.data);
            } else {
                let Ok(offset32) = u32::try_from(entry.payload_offset) else {
                    emit_error_message(
                        tif,
                        module_name,
                        format!(
                            "Tag {} payload offset {} exceeds Classic TIFF range",
                            entry.tag, entry.payload_offset
                        ),
                    );
                    return false;
                };
                entries_buffer[base + 8..base + 12]
                    .copy_from_slice(&encode_u32_bytes(offset32, fmt.big_endian));
            }
        } else {
            entries_buffer[base + 4..base + 12]
                .copy_from_slice(&encode_u64_bytes(entry.count, fmt.big_endian));
            if entry.data.len() <= fmt.inline_size {
                entries_buffer[base + 12..base + 12 + entry.data.len()]
                    .copy_from_slice(&entry.data);
            } else {
                entries_buffer[base + 12..base + 20]
                    .copy_from_slice(&encode_u64_bytes(entry.payload_offset, fmt.big_endian));
            }
        }
    }
    let next_offset_index = entries.len() * fmt.entry_size;
    if fmt.classic {
        let Ok(next32) = u32::try_from(next_offset) else {
            emit_error_message(
                tif,
                module_name,
                format!("Directory link {} exceeds Classic TIFF range", next_offset),
            );
            return false;
        };
        entries_buffer[next_offset_index..next_offset_index + 4]
            .copy_from_slice(&encode_u32_bytes(next32, fmt.big_endian));
    } else {
        entries_buffer[next_offset_index..next_offset_index + 8]
            .copy_from_slice(&encode_u64_bytes(next_offset, fmt.big_endian));
    }

    for entry in entries.iter() {
        if entry.data.len() > fmt.inline_size
            && !write_exact_at(tif, entry.payload_offset, &entry.data)
        {
            emit_error_message(
                tif,
                module_name,
                format!("Failed to write payload for tag {}", entry.tag),
            );
            return false;
        }
    }

    if !write_exact_at(tif, dir_offset, &count_bytes) {
        emit_error_message(tif, module_name, "Failed to write directory count");
        return false;
    }
    if !write_exact_at(tif, dir_offset + fmt.count_size as u64, &entries_buffer) {
        emit_error_message(tif, module_name, "Failed to write directory entry table");
        return false;
    }
    true
}

unsafe fn deferred_strile_tags(current: &CurrentDirectory) -> (u16, u16) {
    if current
        .find_tag(TAG_TILEWIDTH)
        .zip(current.find_tag(TAG_TILELENGTH))
        .is_some()
    {
        (TAG_TILEOFFSETS as u16, TAG_TILEBYTECOUNTS as u16)
    } else {
        (TAG_STRIPOFFSETS as u16, TAG_STRIPBYTECOUNTS as u16)
    }
}

unsafe fn should_defer_strile_array_writing(tif: *mut TIFF, current: &CurrentDirectory) -> bool {
    current.offset == 0
        && matches!(current.kind, DirectoryKind::Main | DirectoryKind::SubIfd)
        && (*tif_inner(tif)).strile_state.defer_array_writing
}

unsafe fn serialize_directory_at_offset(
    tif: *mut TIFF,
    module_name: &str,
    current: &CurrentDirectory,
    dir_offset: u64,
    next_offset: u64,
) -> bool {
    let deferred_tags =
        should_defer_strile_array_writing(tif, current).then(|| deferred_strile_tags(current));
    let mut entries =
        Vec::with_capacity(current.tags.len() + usize::from(deferred_tags.is_some()) * 2);
    for parsed in &current.tags {
        if let Some((offset_tag, bytecount_tag)) = deferred_tags {
            if parsed.tag == offset_tag as u32 || parsed.tag == bytecount_tag as u32 {
                continue;
            }
        }
        let Some(encoded) = encode_directory_entry(tif, module_name, parsed) else {
            return false;
        };
        entries.push(encoded);
    }
    if let Some((offset_tag, bytecount_tag)) = deferred_tags {
        entries.push(EncodedDirectoryEntry {
            tag: offset_tag,
            field_type: TIFFDataType::TIFF_NOTYPE,
            count: 0,
            data: Vec::new(),
            payload_offset: 0,
        });
        entries.push(EncodedDirectoryEntry {
            tag: bytecount_tag,
            field_type: TIFFDataType::TIFF_NOTYPE,
            count: 0,
            data: Vec::new(),
            payload_offset: 0,
        });
    }
    entries.sort_by_key(|entry| entry.tag);
    write_encoded_directory(tif, module_name, dir_offset, next_offset, &mut entries)
}

unsafe fn write_standalone_directory(
    tif: *mut TIFF,
    module_name: &str,
    current: &CurrentDirectory,
    next_offset: u64,
) -> Option<u64> {
    let fmt = directory_encoding(tif, module_name)?;
    let dir_offset = align_up(file_size(tif), fmt.alignment)?;
    if !serialize_directory_at_offset(tif, module_name, current, dir_offset, next_offset) {
        return None;
    }
    Some(dir_offset)
}

unsafe fn update_current_directory_location(tif: *mut TIFF, offset: u64, next_offset: u64) {
    if let Some(current) = current_directory_mut(tif) {
        current.offset = offset;
        current.next_offset = next_offset;
        configure_current_directory_flags(tif, current);
    }
    (*tif_inner(tif)).current_diroff = offset;
    (*tif_inner(tif)).next_diroff = next_offset;
}

unsafe fn refresh_main_chain_head(tif: *mut TIFF, module_name: &str) -> bool {
    let Some(fmt) = directory_encoding(tif, module_name) else {
        return false;
    };
    let Some(head_offset) =
        read_offset_value_at(tif, header_link_offset(tif), fmt.next_size, module_name)
    else {
        return false;
    };
    (*tif_inner(tif)).next_diroff = head_offset;
    clear_main_chain_cache(tif);
    directory_state_mut(tif).first_ifd_offset = head_offset;
    true
}

unsafe fn directory_next_link_location(
    tif: *mut TIFF,
    dir_offset: u64,
    module_name: &str,
) -> Option<u64> {
    let fmt = directory_encoding(tif, module_name)?;
    let mut count_bytes = [0u8; 8];
    if !read_exact_at(tif, dir_offset, &mut count_bytes[..fmt.count_size]) {
        emit_error_message(tif, module_name, "Error fetching directory count");
        return None;
    }
    let entry_count = if fmt.classic {
        parse_u16(&count_bytes[..2], fmt.big_endian) as u64
    } else {
        parse_u64(&count_bytes[..8], fmt.big_endian)
    };
    let entries_size = entry_count.checked_mul(fmt.entry_size as u64)?;
    dir_offset
        .checked_add(fmt.count_size as u64)?
        .checked_add(entries_size)
}

unsafe fn rewrite_directory_next_offset(
    tif: *mut TIFF,
    module_name: &str,
    dir_offset: u64,
    next_offset: u64,
) -> bool {
    let Some(fmt) = directory_encoding(tif, module_name) else {
        return false;
    };
    let Some(link_location) = directory_next_link_location(tif, dir_offset, module_name) else {
        emit_error_message(tif, module_name, "Directory next-link location overflowed");
        return false;
    };
    write_offset_value_at(
        tif,
        link_location,
        fmt.next_size,
        next_offset,
        module_name,
        "directory next-link",
    )
}

unsafe fn find_directory_entry_metadata(
    tif: *mut TIFF,
    module_name: &str,
    dir_offset: u64,
    tag: u16,
) -> Option<(u64, u16, u64, u64)> {
    let fmt = directory_encoding(tif, module_name)?;
    let mut count_bytes = [0u8; 8];
    if !read_exact_at(tif, dir_offset, &mut count_bytes[..fmt.count_size]) {
        emit_error_message(tif, module_name, "Error fetching directory count");
        return None;
    }
    let entry_count = if fmt.classic {
        parse_u16(&count_bytes[..2], fmt.big_endian) as usize
    } else {
        parse_u64(&count_bytes[..8], fmt.big_endian) as usize
    };

    let entries_offset = dir_offset + fmt.count_size as u64;
    for index in 0..entry_count {
        let entry_offset = entries_offset + index as u64 * fmt.entry_size as u64;
        let mut raw = [0u8; 20];
        if !read_exact_at(tif, entry_offset, &mut raw[..fmt.entry_size]) {
            emit_error_message(tif, module_name, "Error reading directory entry");
            return None;
        }
        let entry_tag = parse_u16(&raw[..2], fmt.big_endian);
        if entry_tag == tag {
            let raw_type = parse_u16(&raw[2..4], fmt.big_endian);
            let raw_count = if fmt.classic {
                parse_u32(&raw[4..8], fmt.big_endian) as u64
            } else {
                parse_u64(&raw[4..12], fmt.big_endian)
            };
            let raw_offset = if fmt.classic {
                parse_u32(&raw[8..12], fmt.big_endian) as u64
            } else {
                parse_u64(&raw[12..20], fmt.big_endian)
            };
            return Some((entry_offset, raw_type, raw_count, raw_offset));
        }
    }

    emit_error_message(
        tif,
        module_name,
        format!(
            "Tag {} does not exist in the current on-disk directory",
            tag
        ),
    );
    None
}

pub(crate) unsafe fn safe_tiff_directory_entry_is_dummy(
    tif: *mut TIFF,
    dir_offset: u64,
    tag: u16,
) -> bool {
    match find_directory_entry_metadata(tif, "TIFFForceStrileArrayWriting", dir_offset, tag) {
        Some((_entry_offset, raw_type, raw_count, raw_offset)) => {
            raw_type == 0 && raw_count == 0 && raw_offset == 0
        }
        None => false,
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFSetCompressionScheme(tif: *mut TIFF, _scheme: c_int) -> c_int {
    if tif.is_null() {
        0
    } else {
        1
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFCreateDirectory(tif: *mut TIFF) -> c_int {
    if tif.is_null() || !initialize_writable_directory(tif, DirectoryKind::Main) {
        1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFCreateCustomDirectory(
    tif: *mut TIFF,
    infoarray: *const TIFFFieldArray,
) -> c_int {
    if tif.is_null()
        || infoarray.is_null()
        || !initialize_writable_directory(tif, DirectoryKind::Custom(infoarray))
    {
        1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFCreateEXIFDirectory(tif: *mut TIFF) -> c_int {
    TIFFCreateCustomDirectory(tif, _TIFFGetExifFields())
}

#[no_mangle]
pub unsafe extern "C" fn TIFFCreateGPSDirectory(tif: *mut TIFF) -> c_int {
    TIFFCreateCustomDirectory(tif, _TIFFGetGpsFields())
}

#[no_mangle]
pub unsafe extern "C" fn TIFFWriteCustomDirectory(tif: *mut TIFF, pdiroff: *mut u64) -> c_int {
    let module_name = "TIFFWriteCustomDirectory";
    if tif.is_null() || (*tif_inner(tif)).tif_mode == libc::O_RDONLY {
        if !tif.is_null() {
            emit_error_message(tif, module_name, "File opened in read-only mode");
        }
        return 0;
    }
    let Some(current) = directory_state(tif).current.clone() else {
        emit_error_message(tif, module_name, "No directory is loaded for writing");
        return 0;
    };
    let Some(dir_offset) =
        write_standalone_directory(tif, module_name, &current, current.next_offset)
    else {
        return 0;
    };
    update_current_directory_location(tif, dir_offset, current.next_offset);
    if !pdiroff.is_null() {
        *pdiroff = dir_offset;
    }
    (*tif).tif_flags &= !(TIFF_DIRTYDIRECT | TIFF_DIRTYSTRIP | TIFF_BEENWRITING);
    1
}

#[no_mangle]
pub unsafe extern "C" fn TIFFWriteDirectory(tif: *mut TIFF) -> c_int {
    let module_name = "TIFFWriteDirectory";
    if tif.is_null() || (*tif_inner(tif)).tif_mode == libc::O_RDONLY {
        if !tif.is_null() {
            emit_error_message(tif, module_name, "File opened in read-only mode");
        }
        return 0;
    }
    if !ensure_writable_directory(tif) {
        return 0;
    }
    let Some(current) = directory_state(tif).current.clone() else {
        emit_error_message(tif, module_name, "No directory is loaded for writing");
        return 0;
    };
    if matches!(current.kind, DirectoryKind::SubIfd) {
        let Some(dir_offset) = write_standalone_directory(tif, module_name, &current, 0) else {
            return 0;
        };
        let (parent_offset, previous_offset, done, offsets) = {
            let state = directory_state_mut(tif);
            let Some(pending) = state.pending_subifd.as_mut() else {
                emit_error_message(
                    tif,
                    module_name,
                    "No pending SubIFD write sequence is active",
                );
                return 0;
            };
            if pending.next_index >= pending.offsets.len() {
                emit_error_message(
                    tif,
                    module_name,
                    "SubIFD write sequence is already complete",
                );
                return 0;
            }
            let previous_offset = pending.last_offset;
            pending.offsets[pending.next_index] = dir_offset;
            pending.next_index += 1;
            pending.last_offset = dir_offset;
            (
                pending.parent_offset,
                previous_offset,
                pending.next_index == pending.offsets.len(),
                pending.offsets.clone(),
            )
        };
        if previous_offset != 0
            && !rewrite_directory_next_offset(tif, module_name, previous_offset, dir_offset)
        {
            return 0;
        }
        (*tif).tif_flags &= !(TIFF_DIRTYDIRECT | TIFF_DIRTYSTRIP | TIFF_BEENWRITING);
        if done {
            if rewrite_field_in_directory_at_offset(
                tif,
                module_name,
                parent_offset,
                TAG_SUBIFD as u16,
                TIFFDataType::TIFF_IFD8,
                offsets.len() as u64,
                offsets.as_ptr().cast(),
            ) == 0
            {
                return 0;
            }
            directory_state_mut(tif).pending_subifd = None;
            if !initialize_writable_directory(tif, DirectoryKind::Main) {
                return 0;
            }
            return refresh_main_chain_head(tif, module_name) as c_int;
        }
        return initialize_writable_directory(tif, DirectoryKind::SubIfd) as c_int;
    }
    if !matches!(current.kind, DirectoryKind::Main) {
        emit_error_message(
            tif,
            module_name,
            "Use TIFFWriteCustomDirectory() for non-image directories",
        );
        return 0;
    }

    let Some(link_location) = find_tail_link_location(tif, module_name) else {
        return 0;
    };
    let Some(dir_offset) = write_standalone_directory(tif, module_name, &current, 0) else {
        return 0;
    };
    let Some(fmt) = directory_encoding(tif, module_name) else {
        return 0;
    };
    if !write_offset_value_at(
        tif,
        link_location,
        fmt.next_size,
        dir_offset,
        module_name,
        "directory link",
    ) {
        return 0;
    }
    if !refresh_main_chain_head(tif, module_name) {
        return 0;
    }
    (*tif).tif_flags &= !(TIFF_DIRTYDIRECT | TIFF_DIRTYSTRIP | TIFF_BEENWRITING);
    let subifd_count = subifd_count_from_directory(&current);
    {
        let state = directory_state_mut(tif);
        state.pending_subifd = if subifd_count == 0 {
            None
        } else {
            Some(PendingSubifdWrite {
                parent_offset: dir_offset,
                offsets: vec![0; subifd_count],
                next_index: 0,
                last_offset: 0,
            })
        };
    }
    let next_kind = if subifd_count == 0 {
        DirectoryKind::Main
    } else {
        DirectoryKind::SubIfd
    };
    if !initialize_writable_directory(tif, next_kind) {
        return 0;
    }
    if matches!(next_kind, DirectoryKind::Main) {
        refresh_main_chain_head(tif, module_name) as c_int
    } else {
        1
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFCheckpointDirectory(tif: *mut TIFF) -> c_int {
    let module_name = "TIFFCheckpointDirectory";
    if tif.is_null() || (*tif_inner(tif)).tif_mode == libc::O_RDONLY {
        if !tif.is_null() {
            emit_error_message(tif, module_name, "File opened in read-only mode");
        }
        return 0;
    }
    if !ensure_writable_directory(tif) {
        return 0;
    }
    let Some(current) = directory_state(tif).current.clone() else {
        emit_error_message(tif, module_name, "No directory is loaded for writing");
        return 0;
    };
    if !matches!(current.kind, DirectoryKind::Main) {
        emit_error_message(
            tif,
            module_name,
            "Use TIFFWriteCustomDirectory() for non-image directories",
        );
        return 0;
    }

    if current.offset != 0 {
        return TIFFRewriteDirectory(tif);
    }

    let Some(link_location) = find_tail_link_location(tif, module_name) else {
        return 0;
    };
    let Some(dir_offset) = write_standalone_directory(tif, module_name, &current, 0) else {
        return 0;
    };
    let Some(fmt) = directory_encoding(tif, module_name) else {
        return 0;
    };
    if !write_offset_value_at(
        tif,
        link_location,
        fmt.next_size,
        dir_offset,
        module_name,
        "directory link",
    ) {
        return 0;
    }
    update_current_directory_location(tif, dir_offset, 0);
    (*tif).tif_flags &= !(TIFF_DIRTYDIRECT | TIFF_DIRTYSTRIP | TIFF_BEENWRITING);
    refresh_main_chain_head(tif, module_name) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn TIFFRewriteDirectory(tif: *mut TIFF) -> c_int {
    let module_name = "TIFFRewriteDirectory";
    if tif.is_null() || (*tif_inner(tif)).tif_mode == libc::O_RDONLY {
        if !tif.is_null() {
            emit_error_message(tif, module_name, "File opened in read-only mode");
        }
        return 0;
    }
    if !ensure_writable_directory(tif) {
        return 0;
    }
    let Some(current) = directory_state(tif).current.clone() else {
        emit_error_message(tif, module_name, "No directory is loaded for writing");
        return 0;
    };
    if current.offset == 0 {
        return TIFFWriteDirectory(tif);
    }
    if !matches!(current.kind, DirectoryKind::Main) {
        emit_error_message(
            tif,
            module_name,
            "Use TIFFWriteCustomDirectory() for non-image directories",
        );
        return 0;
    }

    let Some(link_location) = find_predecessor_link_location(tif, current.offset, module_name)
    else {
        return 0;
    };
    let Some(dir_offset) =
        write_standalone_directory(tif, module_name, &current, current.next_offset)
    else {
        return 0;
    };
    let Some(fmt) = directory_encoding(tif, module_name) else {
        return 0;
    };
    if !write_offset_value_at(
        tif,
        link_location,
        fmt.next_size,
        dir_offset,
        module_name,
        "directory link",
    ) {
        return 0;
    }
    update_current_directory_location(tif, dir_offset, current.next_offset);
    (*tif).tif_flags &= !(TIFF_DIRTYDIRECT | TIFF_DIRTYSTRIP | TIFF_BEENWRITING);
    refresh_main_chain_head(tif, module_name) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn TIFFUnlinkDirectory(tif: *mut TIFF, dirnum: u32) -> c_int {
    let module_name = "TIFFUnlinkDirectory";
    if tif.is_null() || (*tif_inner(tif)).tif_mode == libc::O_RDONLY {
        if !tif.is_null() {
            emit_error_message(
                tif,
                module_name,
                "Can not unlink directory in read-only file",
            );
        }
        return 0;
    }
    if dirnum == 0 {
        emit_error_message(
            tif,
            module_name,
            "For TIFFUnlinkDirectory() first directory starts with number 1 and not 0",
        );
        return 0;
    }
    let Some((link_location, _current_offset, next_offset)) =
        find_directory_link_by_number(tif, dirnum, module_name)
    else {
        return 0;
    };
    let Some(fmt) = directory_encoding(tif, module_name) else {
        return 0;
    };
    if !write_offset_value_at(
        tif,
        link_location,
        fmt.next_size,
        next_offset,
        module_name,
        "directory link",
    ) {
        return 0;
    }
    if !refresh_main_chain_head(tif, module_name) {
        return 0;
    }
    if !initialize_writable_directory(tif, DirectoryKind::Main) {
        return 0;
    }
    refresh_main_chain_head(tif, module_name) as c_int
}

unsafe fn rewrite_field_in_directory_at_offset(
    tif: *mut TIFF,
    module_name: &str,
    dir_offset: u64,
    tag: u16,
    in_datatype: TIFFDataType,
    count: u64,
    data: *const c_void,
) -> c_int {
    let Some(fmt) = directory_encoding(tif, module_name) else {
        return 0;
    };
    let Some((entry_offset, raw_type, raw_count, raw_offset)) =
        find_directory_entry_metadata(tif, module_name, dir_offset, tag)
    else {
        return 0;
    };

    let Some(values) =
        stored_values_from_marshaled(tif, module_name, tag as u32, in_datatype, count, data)
    else {
        return 0;
    };
    let actual_count = values.len() as u64;
    let target_type = if raw_type == 0 && raw_count == 0 && raw_offset == 0 {
        let Some(field_type) = on_disk_field_type(tif, module_name, tag as u32, &values) else {
            return 0;
        };
        let Some(field_type) =
            normalize_classic_field_type(tif, module_name, tag as u32, field_type, &values)
        else {
            return 0;
        };
        field_type
    } else {
        let Some(found_type) = type_from_raw(raw_type) else {
            emit_error_message(
                tif,
                module_name,
                format!("Tag {} has an unsupported on-disk type", tag),
            );
            return 0;
        };
        found_type
    };
    let Some(bytes) = encode_stored_values_as_type(
        tif,
        module_name,
        tag as u32,
        target_type,
        &values,
        actual_count,
        (*tif_inner(tif)).header_magic == TIFF_BIGENDIAN,
    ) else {
        return 0;
    };

    let payload_offset = if bytes.len() > fmt.inline_size {
        let Some(offset) = align_up(file_size(tif), fmt.alignment) else {
            emit_error_message(tif, module_name, "Field rewrite overflowed file addressing");
            return 0;
        };
        if !write_exact_at(tif, offset, &bytes) {
            emit_error_message(tif, module_name, "Failed to write rewritten field payload");
            return 0;
        }
        offset
    } else {
        0
    };

    let mut entry_bytes = vec![0u8; fmt.entry_size];
    entry_bytes[..2].copy_from_slice(&encode_u16_bytes(tag, fmt.big_endian));
    entry_bytes[2..4].copy_from_slice(&encode_u16_bytes(target_type.0 as u16, fmt.big_endian));
    if fmt.classic {
        let Ok(count32) = u32::try_from(actual_count) else {
            emit_error_message(
                tif,
                module_name,
                format!(
                    "Tag {} count {} exceeds Classic TIFF range",
                    tag, actual_count
                ),
            );
            return 0;
        };
        entry_bytes[4..8].copy_from_slice(&encode_u32_bytes(count32, fmt.big_endian));
        if bytes.len() <= fmt.inline_size {
            entry_bytes[8..8 + bytes.len()].copy_from_slice(&bytes);
        } else {
            let Ok(offset32) = u32::try_from(payload_offset) else {
                emit_error_message(
                    tif,
                    module_name,
                    format!(
                        "Tag {} payload offset {} exceeds Classic TIFF range",
                        tag, payload_offset
                    ),
                );
                return 0;
            };
            entry_bytes[8..12].copy_from_slice(&encode_u32_bytes(offset32, fmt.big_endian));
        }
    } else {
        entry_bytes[4..12].copy_from_slice(&encode_u64_bytes(actual_count, fmt.big_endian));
        if bytes.len() <= fmt.inline_size {
            entry_bytes[12..12 + bytes.len()].copy_from_slice(&bytes);
        } else {
            entry_bytes[12..20].copy_from_slice(&encode_u64_bytes(payload_offset, fmt.big_endian));
        }
    }

    if !write_exact_at(tif, entry_offset, &entry_bytes) {
        emit_error_message(tif, module_name, "Failed to rewrite directory entry");
        return 0;
    }
    1
}

#[no_mangle]
pub unsafe extern "C" fn _TIFFRewriteField(
    tif: *mut TIFF,
    tag: u16,
    in_datatype: TIFFDataType,
    count: ssize_t,
    data: *mut c_void,
) -> c_int {
    let module_name = "_TIFFRewriteField";
    if tif.is_null() {
        return 0;
    }
    if count < 0 {
        emit_error_message(tif, module_name, "Negative element count is invalid");
        return 0;
    }
    let dir_offset = (*tif_inner(tif)).current_diroff;
    if dir_offset == 0 {
        emit_error_message(
            tif,
            module_name,
            "Attempt to rewrite a field on a directory not already on disk",
        );
        return 0;
    }
    rewrite_field_in_directory_at_offset(
        tif,
        module_name,
        dir_offset,
        tag,
        in_datatype,
        count as u64,
        data.cast_const(),
    )
}
