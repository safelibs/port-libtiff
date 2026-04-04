use super::field_tables::{
    TIFF_FIELD_ARRAY_EXIF, TIFF_FIELD_ARRAY_GPS, TIFF_FIELD_ARRAY_IMAGE,
};
use crate::abi::{
    TIFFDataType, TIFFExtendProc, TIFFField, TIFFFieldArray, TIFFFieldInfo,
    TIFFSetGetFieldType, TIFFTagMethods,
};
use crate::{emit_error_message, emit_warning_message, tif_inner, TIFF};
use libc::{c_char, c_int, c_void};
use std::cmp::Ordering;
use std::ffi::{CStr, CString};
use std::ptr;
use std::sync::Mutex;

const TIFF_ANY: TIFFDataType = TIFFDataType::TIFF_NOTYPE;
const TIFF_VARIABLE: i16 = -1;
const TIFF_VARIABLE2: i16 = -3;
const FIELD_CUSTOM: u16 = 65;

static TAG_EXTENDER: Mutex<TIFFExtendProc> = Mutex::new(None);

unsafe extern "C" {
    fn safe_tiff_initialize_tag_methods(methods: *mut TIFFTagMethods);
}

struct AnonymousField {
    field: Box<TIFFField>,
    _name: CString,
}

struct ClientInfoEntry {
    data: *mut c_void,
    name: CString,
}

pub(crate) struct FieldRegistryState {
    fields: Vec<*mut TIFFField>,
    foundfield: *const TIFFField,
    compat_arrays: Vec<Box<[TIFFField]>>,
    anonymous_fields: Vec<AnonymousField>,
    client_info: Vec<ClientInfoEntry>,
    tagmethods: TIFFTagMethods,
    tag_list: Vec<u32>,
}

impl Default for FieldRegistryState {
    fn default() -> Self {
        let mut tagmethods = TIFFTagMethods::default();
        unsafe {
            safe_tiff_initialize_tag_methods(&mut tagmethods);
        }
        Self {
            fields: Vec::new(),
            foundfield: ptr::null(),
            compat_arrays: Vec::new(),
            anonymous_fields: Vec::new(),
            client_info: Vec::new(),
            tagmethods,
            tag_list: Vec::new(),
        }
    }
}

unsafe fn registry_state(tif: *mut TIFF) -> &'static FieldRegistryState {
    &(*tif_inner(tif)).field_registry
}

unsafe fn registry_state_mut(tif: *mut TIFF) -> &'static mut FieldRegistryState {
    &mut (*tif_inner(tif)).field_registry
}

unsafe fn sort_fields(state: &mut FieldRegistryState) {
    state.fields.sort_unstable_by(|lhs, rhs| tag_compare(*lhs, *rhs));
}

unsafe fn tag_compare(lhs: *const TIFFField, rhs: *const TIFFField) -> Ordering {
    let lhs = &*lhs;
    let rhs = &*rhs;
    if lhs.field_tag != rhs.field_tag {
        lhs.field_tag.cmp(&rhs.field_tag)
    } else if lhs.field_type.0 == TIFF_ANY.0 {
        Ordering::Equal
    } else {
        rhs.field_type.0.cmp(&lhs.field_type.0)
    }
}

unsafe fn compare_key_to_field(tag: u32, dt: TIFFDataType, field: *const TIFFField) -> Ordering {
    let field = &*field;
    if tag != field.field_tag {
        tag.cmp(&field.field_tag)
    } else if dt.0 == TIFF_ANY.0 {
        Ordering::Equal
    } else {
        field.field_type.0.cmp(&dt.0)
    }
}

unsafe fn find_field_impl(
    state: &mut FieldRegistryState,
    tag: u32,
    dt: TIFFDataType,
) -> *const TIFFField {
    if !state.foundfield.is_null() {
        let cached = &*state.foundfield;
        if cached.field_tag == tag && (dt.0 == TIFF_ANY.0 || cached.field_type.0 == dt.0) {
            return state.foundfield;
        }
    }

    let mut left = 0usize;
    let mut right = state.fields.len();
    while left < right {
        let mid = left + (right - left) / 2;
        match compare_key_to_field(tag, dt, state.fields[mid]) {
            Ordering::Less => {
                right = mid;
            }
            Ordering::Greater => {
                left = mid + 1;
            }
            Ordering::Equal => {
                state.foundfield = state.fields[mid];
                return state.foundfield;
            }
        }
    }

    state.foundfield = ptr::null();
    ptr::null()
}

unsafe fn find_field_by_name_impl(
    state: &mut FieldRegistryState,
    field_name: *const c_char,
    dt: TIFFDataType,
) -> *const TIFFField {
    if field_name.is_null() {
        state.foundfield = ptr::null();
        return ptr::null();
    }

    if !state.foundfield.is_null() {
        let cached = &*state.foundfield;
        if !cached.field_name.is_null()
            && libc::strcmp(cached.field_name, field_name) == 0
            && (dt.0 == TIFF_ANY.0 || cached.field_type.0 == dt.0)
        {
            return state.foundfield;
        }
    }

    for field in &state.fields {
        let candidate = &**field;
        if !candidate.field_name.is_null()
            && libc::strcmp(candidate.field_name, field_name) == 0
            && (dt.0 == TIFF_ANY.0 || candidate.field_type.0 == dt.0)
        {
            state.foundfield = *field;
            return state.foundfield;
        }
    }

    state.foundfield = ptr::null();
    ptr::null()
}

unsafe fn setup_fields_impl(tif: *mut TIFF, infoarray: *const TIFFFieldArray) -> bool {
    if tif.is_null() || infoarray.is_null() {
        return false;
    }

    let state = registry_state_mut(tif);
    state.fields.clear();
    state.anonymous_fields.clear();
    state.foundfield = ptr::null();

    let fields = std::slice::from_raw_parts((*infoarray).fields, (*infoarray).count as usize);
    for field in fields {
        state.fields.push(field as *const TIFFField as *mut TIFFField);
    }
    sort_fields(state);
    true
}

pub(crate) unsafe fn reset_field_registry_with_array(
    tif: *mut TIFF,
    infoarray: *const TIFFFieldArray,
) -> bool {
    if tif.is_null() || infoarray.is_null() {
        return false;
    }
    {
        let state = registry_state_mut(tif);
        state.compat_arrays.clear();
        state.anonymous_fields.clear();
        state.tagmethods = TIFFTagMethods::default();
        safe_tiff_initialize_tag_methods(&mut state.tagmethods);
        state.foundfield = ptr::null();
        state.tag_list.clear();
    }
    setup_fields_impl(tif, infoarray)
}

unsafe fn record_custom_tag_impl(tif: *mut TIFF, tag: u32) -> c_int {
    if tif.is_null() {
        return 0;
    }
    let _ = initialize_field_registry(tif);
    let state = registry_state_mut(tif);
    let field = find_field_impl(state, tag, TIFF_ANY);
    if field.is_null() || (*field).field_bit != FIELD_CUSTOM {
        return 1;
    }
    if !state.tag_list.contains(&tag) {
        state.tag_list.push(tag);
    }
    1
}

unsafe fn remove_custom_tag_impl(tif: *mut TIFF, tag: u32) -> c_int {
    if tif.is_null() {
        return 0;
    }
    let state = registry_state_mut(tif);
    state.tag_list.retain(|entry| *entry != tag);
    1
}

unsafe fn merge_fields_impl(tif: *mut TIFF, info: *const TIFFField, n: u32) -> c_int {
    if tif.is_null() {
        return 0;
    }
    if info.is_null() && n != 0 {
        emit_error_message(tif, "_TIFFMergeFields", "Failed to allocate fields array");
        return 0;
    }

    let state = registry_state_mut(tif);
    state.foundfield = ptr::null();
    let fields = std::slice::from_raw_parts(info, n as usize);
    for field in fields {
        if find_field_impl(state, field.field_tag, TIFF_ANY).is_null() {
            state.fields.push(field as *const TIFFField as *mut TIFFField);
        }
    }
    sort_fields(state);
    n as c_int
}

fn set_get_type(type_: TIFFDataType, count: i16, passcount: u8) -> TIFFSetGetFieldType {
    if type_.0 == TIFFDataType::TIFF_ASCII.0 && count == TIFF_VARIABLE && passcount == 0 {
        return TIFFSetGetFieldType::TIFF_SETGET_ASCII;
    }

    if count == 1 && passcount == 0 {
        return match type_.0 {
            x if x == TIFFDataType::TIFF_BYTE.0 || x == TIFFDataType::TIFF_UNDEFINED.0 => {
                TIFFSetGetFieldType::TIFF_SETGET_UINT8
            }
            x if x == TIFFDataType::TIFF_ASCII.0 => TIFFSetGetFieldType::TIFF_SETGET_ASCII,
            x if x == TIFFDataType::TIFF_SHORT.0 => TIFFSetGetFieldType::TIFF_SETGET_UINT16,
            x if x == TIFFDataType::TIFF_LONG.0 => TIFFSetGetFieldType::TIFF_SETGET_UINT32,
            x if x == TIFFDataType::TIFF_RATIONAL.0
                || x == TIFFDataType::TIFF_SRATIONAL.0
                || x == TIFFDataType::TIFF_FLOAT.0 =>
            {
                TIFFSetGetFieldType::TIFF_SETGET_FLOAT
            }
            x if x == TIFFDataType::TIFF_SBYTE.0 => TIFFSetGetFieldType::TIFF_SETGET_SINT8,
            x if x == TIFFDataType::TIFF_SSHORT.0 => TIFFSetGetFieldType::TIFF_SETGET_SINT16,
            x if x == TIFFDataType::TIFF_SLONG.0 => TIFFSetGetFieldType::TIFF_SETGET_SINT32,
            x if x == TIFFDataType::TIFF_DOUBLE.0 => TIFFSetGetFieldType::TIFF_SETGET_DOUBLE,
            x if x == TIFFDataType::TIFF_IFD.0 || x == TIFFDataType::TIFF_IFD8.0 => {
                TIFFSetGetFieldType::TIFF_SETGET_IFD8
            }
            x if x == TIFFDataType::TIFF_LONG8.0 => TIFFSetGetFieldType::TIFF_SETGET_UINT64,
            x if x == TIFFDataType::TIFF_SLONG8.0 => TIFFSetGetFieldType::TIFF_SETGET_SINT64,
            _ => TIFFSetGetFieldType::TIFF_SETGET_UNDEFINED,
        };
    }

    if count >= 1 && passcount == 0 {
        return match type_.0 {
            x if x == TIFFDataType::TIFF_BYTE.0 || x == TIFFDataType::TIFF_UNDEFINED.0 => {
                TIFFSetGetFieldType::TIFF_SETGET_C0_UINT8
            }
            x if x == TIFFDataType::TIFF_ASCII.0 => TIFFSetGetFieldType::TIFF_SETGET_C0_ASCII,
            x if x == TIFFDataType::TIFF_SHORT.0 => TIFFSetGetFieldType::TIFF_SETGET_C0_UINT16,
            x if x == TIFFDataType::TIFF_LONG.0 => TIFFSetGetFieldType::TIFF_SETGET_C0_UINT32,
            x if x == TIFFDataType::TIFF_RATIONAL.0
                || x == TIFFDataType::TIFF_SRATIONAL.0
                || x == TIFFDataType::TIFF_FLOAT.0 =>
            {
                TIFFSetGetFieldType::TIFF_SETGET_C0_FLOAT
            }
            x if x == TIFFDataType::TIFF_SBYTE.0 => TIFFSetGetFieldType::TIFF_SETGET_C0_SINT8,
            x if x == TIFFDataType::TIFF_SSHORT.0 => TIFFSetGetFieldType::TIFF_SETGET_C0_SINT16,
            x if x == TIFFDataType::TIFF_SLONG.0 => TIFFSetGetFieldType::TIFF_SETGET_C0_SINT32,
            x if x == TIFFDataType::TIFF_DOUBLE.0 => TIFFSetGetFieldType::TIFF_SETGET_C0_DOUBLE,
            x if x == TIFFDataType::TIFF_IFD.0 || x == TIFFDataType::TIFF_IFD8.0 => {
                TIFFSetGetFieldType::TIFF_SETGET_C0_IFD8
            }
            x if x == TIFFDataType::TIFF_LONG8.0 => TIFFSetGetFieldType::TIFF_SETGET_C0_UINT64,
            x if x == TIFFDataType::TIFF_SLONG8.0 => TIFFSetGetFieldType::TIFF_SETGET_C0_SINT64,
            _ => TIFFSetGetFieldType::TIFF_SETGET_UNDEFINED,
        };
    }

    if count == TIFF_VARIABLE && passcount == 1 {
        return match type_.0 {
            x if x == TIFFDataType::TIFF_BYTE.0 || x == TIFFDataType::TIFF_UNDEFINED.0 => {
                TIFFSetGetFieldType::TIFF_SETGET_C16_UINT8
            }
            x if x == TIFFDataType::TIFF_ASCII.0 => TIFFSetGetFieldType::TIFF_SETGET_C16_ASCII,
            x if x == TIFFDataType::TIFF_SHORT.0 => TIFFSetGetFieldType::TIFF_SETGET_C16_UINT16,
            x if x == TIFFDataType::TIFF_LONG.0 => TIFFSetGetFieldType::TIFF_SETGET_C16_UINT32,
            x if x == TIFFDataType::TIFF_RATIONAL.0
                || x == TIFFDataType::TIFF_SRATIONAL.0
                || x == TIFFDataType::TIFF_FLOAT.0 =>
            {
                TIFFSetGetFieldType::TIFF_SETGET_C16_FLOAT
            }
            x if x == TIFFDataType::TIFF_SBYTE.0 => TIFFSetGetFieldType::TIFF_SETGET_C16_SINT8,
            x if x == TIFFDataType::TIFF_SSHORT.0 => TIFFSetGetFieldType::TIFF_SETGET_C16_SINT16,
            x if x == TIFFDataType::TIFF_SLONG.0 => TIFFSetGetFieldType::TIFF_SETGET_C16_SINT32,
            x if x == TIFFDataType::TIFF_DOUBLE.0 => TIFFSetGetFieldType::TIFF_SETGET_C16_DOUBLE,
            x if x == TIFFDataType::TIFF_IFD.0 || x == TIFFDataType::TIFF_IFD8.0 => {
                TIFFSetGetFieldType::TIFF_SETGET_C16_IFD8
            }
            x if x == TIFFDataType::TIFF_LONG8.0 => TIFFSetGetFieldType::TIFF_SETGET_C16_UINT64,
            x if x == TIFFDataType::TIFF_SLONG8.0 => TIFFSetGetFieldType::TIFF_SETGET_C16_SINT64,
            _ => TIFFSetGetFieldType::TIFF_SETGET_UNDEFINED,
        };
    }

    if count == TIFF_VARIABLE2 && passcount == 1 {
        return match type_.0 {
            x if x == TIFFDataType::TIFF_BYTE.0 || x == TIFFDataType::TIFF_UNDEFINED.0 => {
                TIFFSetGetFieldType::TIFF_SETGET_C32_UINT8
            }
            x if x == TIFFDataType::TIFF_ASCII.0 => TIFFSetGetFieldType::TIFF_SETGET_C32_ASCII,
            x if x == TIFFDataType::TIFF_SHORT.0 => TIFFSetGetFieldType::TIFF_SETGET_C32_UINT16,
            x if x == TIFFDataType::TIFF_LONG.0 => TIFFSetGetFieldType::TIFF_SETGET_C32_UINT32,
            x if x == TIFFDataType::TIFF_RATIONAL.0
                || x == TIFFDataType::TIFF_SRATIONAL.0
                || x == TIFFDataType::TIFF_FLOAT.0 =>
            {
                TIFFSetGetFieldType::TIFF_SETGET_C32_FLOAT
            }
            x if x == TIFFDataType::TIFF_SBYTE.0 => TIFFSetGetFieldType::TIFF_SETGET_C32_SINT8,
            x if x == TIFFDataType::TIFF_SSHORT.0 => TIFFSetGetFieldType::TIFF_SETGET_C32_SINT16,
            x if x == TIFFDataType::TIFF_SLONG.0 => TIFFSetGetFieldType::TIFF_SETGET_C32_SINT32,
            x if x == TIFFDataType::TIFF_DOUBLE.0 => TIFFSetGetFieldType::TIFF_SETGET_C32_DOUBLE,
            x if x == TIFFDataType::TIFF_IFD.0 || x == TIFFDataType::TIFF_IFD8.0 => {
                TIFFSetGetFieldType::TIFF_SETGET_C32_IFD8
            }
            x if x == TIFFDataType::TIFF_LONG8.0 => TIFFSetGetFieldType::TIFF_SETGET_C32_UINT64,
            x if x == TIFFDataType::TIFF_SLONG8.0 => TIFFSetGetFieldType::TIFF_SETGET_C32_SINT64,
            _ => TIFFSetGetFieldType::TIFF_SETGET_UNDEFINED,
        };
    }

    TIFFSetGetFieldType::TIFF_SETGET_UNDEFINED
}

fn create_anonymous_field(tag: u32, field_type: TIFFDataType) -> AnonymousField {
    let name = CString::new(format!("Tag {}", tag)).expect("anonymous field name");
    let (set_field_type, get_field_type) = match field_type.0 {
        x if x == TIFFDataType::TIFF_BYTE.0 || x == TIFFDataType::TIFF_UNDEFINED.0 => (
            TIFFSetGetFieldType::TIFF_SETGET_C32_UINT8,
            TIFFSetGetFieldType::TIFF_SETGET_C32_UINT8,
        ),
        x if x == TIFFDataType::TIFF_ASCII.0 => (
            TIFFSetGetFieldType::TIFF_SETGET_C32_ASCII,
            TIFFSetGetFieldType::TIFF_SETGET_C32_ASCII,
        ),
        x if x == TIFFDataType::TIFF_SHORT.0 => (
            TIFFSetGetFieldType::TIFF_SETGET_C32_UINT16,
            TIFFSetGetFieldType::TIFF_SETGET_C32_UINT16,
        ),
        x if x == TIFFDataType::TIFF_LONG.0 => (
            TIFFSetGetFieldType::TIFF_SETGET_C32_UINT32,
            TIFFSetGetFieldType::TIFF_SETGET_C32_UINT32,
        ),
        x if x == TIFFDataType::TIFF_RATIONAL.0
            || x == TIFFDataType::TIFF_SRATIONAL.0
            || x == TIFFDataType::TIFF_FLOAT.0 =>
        {
            (
                TIFFSetGetFieldType::TIFF_SETGET_C32_FLOAT,
                TIFFSetGetFieldType::TIFF_SETGET_C32_FLOAT,
            )
        }
        x if x == TIFFDataType::TIFF_SBYTE.0 => (
            TIFFSetGetFieldType::TIFF_SETGET_C32_SINT8,
            TIFFSetGetFieldType::TIFF_SETGET_C32_SINT8,
        ),
        x if x == TIFFDataType::TIFF_SSHORT.0 => (
            TIFFSetGetFieldType::TIFF_SETGET_C32_SINT16,
            TIFFSetGetFieldType::TIFF_SETGET_C32_SINT16,
        ),
        x if x == TIFFDataType::TIFF_SLONG.0 => (
            TIFFSetGetFieldType::TIFF_SETGET_C32_SINT32,
            TIFFSetGetFieldType::TIFF_SETGET_C32_SINT32,
        ),
        x if x == TIFFDataType::TIFF_DOUBLE.0 => (
            TIFFSetGetFieldType::TIFF_SETGET_C32_DOUBLE,
            TIFFSetGetFieldType::TIFF_SETGET_C32_DOUBLE,
        ),
        x if x == TIFFDataType::TIFF_IFD.0 || x == TIFFDataType::TIFF_IFD8.0 => (
            TIFFSetGetFieldType::TIFF_SETGET_C32_IFD8,
            TIFFSetGetFieldType::TIFF_SETGET_C32_IFD8,
        ),
        x if x == TIFFDataType::TIFF_LONG8.0 => (
            TIFFSetGetFieldType::TIFF_SETGET_C32_UINT64,
            TIFFSetGetFieldType::TIFF_SETGET_C32_UINT64,
        ),
        x if x == TIFFDataType::TIFF_SLONG8.0 => (
            TIFFSetGetFieldType::TIFF_SETGET_C32_SINT64,
            TIFFSetGetFieldType::TIFF_SETGET_C32_SINT64,
        ),
        _ => (
            TIFFSetGetFieldType::TIFF_SETGET_UNDEFINED,
            TIFFSetGetFieldType::TIFF_SETGET_UNDEFINED,
        ),
    };
    let field = Box::new(TIFFField {
        field_tag: tag,
        field_readcount: TIFF_VARIABLE2,
        field_writecount: TIFF_VARIABLE2,
        field_type,
        field_anonymous: 1,
        set_field_type,
        get_field_type,
        field_bit: FIELD_CUSTOM,
        field_oktochange: 1,
        field_passcount: 1,
        field_name: name.as_ptr() as *mut c_char,
        field_subfields: ptr::null_mut(),
    });
    AnonymousField { field, _name: name }
}

pub(crate) unsafe fn initialize_field_registry(tif: *mut TIFF) -> bool {
    if tif.is_null() {
        return false;
    }
    if registry_state(tif).fields.is_empty() {
        setup_fields_impl(tif, &TIFF_FIELD_ARRAY_IMAGE)
    } else {
        true
    }
}

pub(crate) unsafe fn reset_default_directory(tif: *mut TIFF) -> bool {
    if tif.is_null() {
        return false;
    }
    if !reset_field_registry_with_array(tif, &TIFF_FIELD_ARRAY_IMAGE) {
        return false;
    }
    if let Some(extender) = *TAG_EXTENDER.lock().expect("tag extender mutex") {
        extender(tif);
    }
    true
}

#[no_mangle]
pub unsafe extern "C" fn safe_tiff_record_custom_tag(tif: *mut TIFF, tag: u32) -> c_int {
    record_custom_tag_impl(tif, tag)
}

#[no_mangle]
pub unsafe extern "C" fn safe_tiff_remove_custom_tag(tif: *mut TIFF, tag: u32) -> c_int {
    remove_custom_tag_impl(tif, tag)
}

#[no_mangle]
pub extern "C" fn TIFFDataWidth(type_: TIFFDataType) -> c_int {
    match type_.0 {
        0 | 1 | 2 | 6 | 7 => 1,
        3 | 8 => 2,
        4 | 9 | 11 | 13 => 4,
        5 | 10 | 12 | 16 | 17 | 18 => 8,
        _ => 0,
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFFieldSetGetSize(fip: *const TIFFField) -> c_int {
    if fip.is_null() {
        return 0;
    }
    match (*fip).set_field_type.0 {
        0 | 1 | 15 | 27 | 39 | 51 => 1,
        2 | 3 | 16 | 17 | 28 | 29 | 40 | 41 => 1,
        4 | 5 | 18 | 19 | 30 | 31 | 42 | 43 => 2,
        13 | 6 | 7 | 10 | 14 | 20 | 21 | 24 | 32 | 33 | 36 | 44 | 45 | 48 => 4,
        8 | 9 | 11 | 12 | 22 | 23 | 25 | 26 | 34 | 35 | 37 | 38 | 46 | 47 | 49 | 50 => 8,
        _ => 0,
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFFieldSetGetCountSize(fip: *const TIFFField) -> c_int {
    if fip.is_null() {
        return 0;
    }
    match (*fip).set_field_type.0 {
        27..=38 => 2,
        39..=50 => 4,
        _ => 0,
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFFindField(
    tif: *mut TIFF,
    tag: u32,
    dt: TIFFDataType,
) -> *const TIFFField {
    if tif.is_null() {
        return ptr::null();
    }
    let _ = initialize_field_registry(tif);
    find_field_impl(registry_state_mut(tif), tag, dt)
}

#[no_mangle]
pub unsafe extern "C" fn TIFFFieldWithTag(tif: *mut TIFF, tag: u32) -> *const TIFFField {
    let fip = TIFFFindField(tif, tag, TIFF_ANY);
    if fip.is_null() && !tif.is_null() {
        emit_warning_message(
            tif,
            "TIFFFieldWithTag",
            format!("Warning, unknown tag 0x{:x}", tag),
        );
    }
    fip
}

#[no_mangle]
pub unsafe extern "C" fn TIFFFieldWithName(
    tif: *mut TIFF,
    field_name: *const c_char,
) -> *const TIFFField {
    if tif.is_null() {
        return ptr::null();
    }
    let _ = initialize_field_registry(tif);
    let fip = find_field_by_name_impl(registry_state_mut(tif), field_name, TIFF_ANY);
    if fip.is_null() {
        emit_warning_message(
            tif,
            "TIFFFieldWithName",
            format!("Warning, unknown tag {}", crate::c_name(field_name)),
        );
    }
    fip
}

#[no_mangle]
pub unsafe extern "C" fn TIFFFieldTag(fip: *const TIFFField) -> u32 {
    if fip.is_null() {
        0
    } else {
        (*fip).field_tag
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFFieldName(fip: *const TIFFField) -> *const c_char {
    if fip.is_null() {
        ptr::null()
    } else {
        (*fip).field_name
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFFieldDataType(fip: *const TIFFField) -> TIFFDataType {
    if fip.is_null() {
        TIFF_ANY
    } else {
        (*fip).field_type
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFFieldPassCount(fip: *const TIFFField) -> c_int {
    if fip.is_null() {
        0
    } else {
        (*fip).field_passcount as c_int
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFFieldReadCount(fip: *const TIFFField) -> c_int {
    if fip.is_null() {
        0
    } else {
        (*fip).field_readcount as c_int
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFFieldWriteCount(fip: *const TIFFField) -> c_int {
    if fip.is_null() {
        0
    } else {
        (*fip).field_writecount as c_int
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFFieldIsAnonymous(fip: *const TIFFField) -> c_int {
    if fip.is_null() {
        0
    } else {
        (*fip).field_anonymous as c_int
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFGetTagListCount(tif: *mut TIFF) -> c_int {
    if tif.is_null() {
        0
    } else {
        registry_state(tif).tag_list.len() as c_int
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFGetTagListEntry(tif: *mut TIFF, tag_index: c_int) -> u32 {
    if tif.is_null() || tag_index < 0 {
        return u32::MAX;
    }
    registry_state(tif)
        .tag_list
        .get(tag_index as usize)
        .copied()
        .unwrap_or(u32::MAX)
}

#[no_mangle]
pub unsafe extern "C" fn TIFFAccessTagMethods(tif: *mut TIFF) -> *mut TIFFTagMethods {
    if tif.is_null() {
        ptr::null_mut()
    } else {
        &mut registry_state_mut(tif).tagmethods
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFGetClientInfo(
    tif: *mut TIFF,
    name: *const c_char,
) -> *mut c_void {
    if tif.is_null() || name.is_null() {
        return ptr::null_mut();
    }
    for entry in &registry_state(tif).client_info {
        if libc::strcmp(entry.name.as_ptr(), name) == 0 {
            return entry.data;
        }
    }
    ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn TIFFSetClientInfo(
    tif: *mut TIFF,
    data: *mut c_void,
    name: *const c_char,
) {
    if tif.is_null() || name.is_null() {
        return;
    }

    let state = registry_state_mut(tif);
    for entry in &mut state.client_info {
        if libc::strcmp(entry.name.as_ptr(), name) == 0 {
            entry.data = data;
            return;
        }
    }

    let owned_name = CString::new(CStr::from_ptr(name).to_bytes()).expect("client info name");
    state
        .client_info
        .insert(0, ClientInfoEntry { data, name: owned_name });
}

#[no_mangle]
pub unsafe extern "C" fn TIFFMergeFieldInfo(
    tif: *mut TIFF,
    info: *const TIFFFieldInfo,
    n: u32,
) -> c_int {
    if tif.is_null() || (info.is_null() && n != 0) {
        return -1;
    }
    let _ = initialize_field_registry(tif);

    let mut merged = Vec::with_capacity(n as usize);
    for index in 0..n as usize {
        let source = &*info.add(index);
        if source.field_name.is_null() {
            emit_error_message(
                tif,
                "TIFFMergeFieldInfo",
                format!("Field_name of {}.th allocation tag {} is NULL", index, source.field_tag),
            );
            return -1;
        }
        merged.push(TIFFField {
            field_tag: source.field_tag,
            field_readcount: source.field_readcount,
            field_writecount: source.field_writecount,
            field_type: source.field_type,
            field_anonymous: 0,
            set_field_type: set_get_type(
                source.field_type,
                source.field_readcount,
                source.field_passcount,
            ),
            get_field_type: set_get_type(
                source.field_type,
                source.field_readcount,
                source.field_passcount,
            ),
            field_bit: source.field_bit,
            field_oktochange: source.field_oktochange,
            field_passcount: source.field_passcount,
            field_name: source.field_name,
            field_subfields: ptr::null_mut(),
        });
    }

    let boxed = merged.into_boxed_slice();
    let merged_ptr = boxed.as_ptr() as *mut TIFFField;
    registry_state_mut(tif).compat_arrays.push(boxed);
    if merge_fields_impl(tif, merged_ptr, n) == 0 && n != 0 {
        emit_error_message(tif, "TIFFMergeFieldInfo", "Setting up field info failed");
        return -1;
    }
    0
}

#[no_mangle]
pub extern "C" fn TIFFSetTagExtender(extender: TIFFExtendProc) -> TIFFExtendProc {
    let mut state = TAG_EXTENDER.lock().expect("tag extender mutex");
    let previous = *state;
    *state = extender;
    previous
}

#[no_mangle]
pub extern "C" fn _TIFFGetFields() -> *const TIFFFieldArray {
    &TIFF_FIELD_ARRAY_IMAGE
}

#[no_mangle]
pub extern "C" fn _TIFFGetExifFields() -> *const TIFFFieldArray {
    &TIFF_FIELD_ARRAY_EXIF
}

#[no_mangle]
pub extern "C" fn _TIFFGetGpsFields() -> *const TIFFFieldArray {
    &TIFF_FIELD_ARRAY_GPS
}

#[no_mangle]
pub unsafe extern "C" fn _TIFFSetupFields(tif: *mut TIFF, infoarray: *const TIFFFieldArray) {
    if !setup_fields_impl(tif, infoarray) && !tif.is_null() {
        emit_error_message(tif, "_TIFFSetupFields", "Setting up field info failed");
    }
}

#[no_mangle]
pub unsafe extern "C" fn _TIFFMergeFields(
    tif: *mut TIFF,
    info: *const TIFFField,
    n: u32,
) -> c_int {
    merge_fields_impl(tif, info, n)
}

#[no_mangle]
pub unsafe extern "C" fn _TIFFFindOrRegisterField(
    tif: *mut TIFF,
    tag: u32,
    dt: TIFFDataType,
) -> *const TIFFField {
    if tif.is_null() {
        return ptr::null();
    }
    let _ = initialize_field_registry(tif);
    let field = TIFFFindField(tif, tag, dt);
    if !field.is_null() {
        return field;
    }
    let field = _TIFFCreateAnonField(tif, tag, dt);
    if field.is_null() || _TIFFMergeFields(tif, field, 1) == 0 {
        return ptr::null();
    }
    field
}

#[no_mangle]
pub unsafe extern "C" fn _TIFFCreateAnonField(
    tif: *mut TIFF,
    tag: u32,
    field_type: TIFFDataType,
) -> *mut TIFFField {
    if tif.is_null() {
        return ptr::null_mut();
    }
    let mut anonymous = create_anonymous_field(tag, field_type);
    let field_ptr = anonymous.field.as_mut() as *mut TIFFField;
    registry_state_mut(tif).anonymous_fields.push(anonymous);
    field_ptr
}
