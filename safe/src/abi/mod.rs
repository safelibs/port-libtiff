use crate::TIFF;
use libc::{c_char, c_int, c_void};
use std::mem::{offset_of, size_of};

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TIFFDataType(pub c_int);

#[allow(non_upper_case_globals)]
impl TIFFDataType {
    pub const TIFF_NOTYPE: Self = Self(0);
    pub const TIFF_BYTE: Self = Self(1);
    pub const TIFF_ASCII: Self = Self(2);
    pub const TIFF_SHORT: Self = Self(3);
    pub const TIFF_LONG: Self = Self(4);
    pub const TIFF_RATIONAL: Self = Self(5);
    pub const TIFF_SBYTE: Self = Self(6);
    pub const TIFF_UNDEFINED: Self = Self(7);
    pub const TIFF_SSHORT: Self = Self(8);
    pub const TIFF_SLONG: Self = Self(9);
    pub const TIFF_SRATIONAL: Self = Self(10);
    pub const TIFF_FLOAT: Self = Self(11);
    pub const TIFF_DOUBLE: Self = Self(12);
    pub const TIFF_IFD: Self = Self(13);
    pub const TIFF_LONG8: Self = Self(16);
    pub const TIFF_SLONG8: Self = Self(17);
    pub const TIFF_IFD8: Self = Self(18);
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TIFFFieldArrayType(pub c_int);

#[allow(non_upper_case_globals)]
impl TIFFFieldArrayType {
    pub const tfiatImage: Self = Self(0);
    pub const tfiatExif: Self = Self(1);
    pub const tfiatGps: Self = Self(2);
    pub const tfiatOther: Self = Self(3);
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TIFFSetGetFieldType(pub c_int);

#[allow(non_upper_case_globals)]
impl TIFFSetGetFieldType {
    pub const TIFF_SETGET_UNDEFINED: Self = Self(0);
    pub const TIFF_SETGET_ASCII: Self = Self(1);
    pub const TIFF_SETGET_UINT8: Self = Self(2);
    pub const TIFF_SETGET_SINT8: Self = Self(3);
    pub const TIFF_SETGET_UINT16: Self = Self(4);
    pub const TIFF_SETGET_SINT16: Self = Self(5);
    pub const TIFF_SETGET_UINT32: Self = Self(6);
    pub const TIFF_SETGET_SINT32: Self = Self(7);
    pub const TIFF_SETGET_UINT64: Self = Self(8);
    pub const TIFF_SETGET_SINT64: Self = Self(9);
    pub const TIFF_SETGET_FLOAT: Self = Self(10);
    pub const TIFF_SETGET_DOUBLE: Self = Self(11);
    pub const TIFF_SETGET_IFD8: Self = Self(12);
    pub const TIFF_SETGET_INT: Self = Self(13);
    pub const TIFF_SETGET_UINT16_PAIR: Self = Self(14);
    pub const TIFF_SETGET_C0_ASCII: Self = Self(15);
    pub const TIFF_SETGET_C0_UINT8: Self = Self(16);
    pub const TIFF_SETGET_C0_SINT8: Self = Self(17);
    pub const TIFF_SETGET_C0_UINT16: Self = Self(18);
    pub const TIFF_SETGET_C0_SINT16: Self = Self(19);
    pub const TIFF_SETGET_C0_UINT32: Self = Self(20);
    pub const TIFF_SETGET_C0_SINT32: Self = Self(21);
    pub const TIFF_SETGET_C0_UINT64: Self = Self(22);
    pub const TIFF_SETGET_C0_SINT64: Self = Self(23);
    pub const TIFF_SETGET_C0_FLOAT: Self = Self(24);
    pub const TIFF_SETGET_C0_DOUBLE: Self = Self(25);
    pub const TIFF_SETGET_C0_IFD8: Self = Self(26);
    pub const TIFF_SETGET_C16_ASCII: Self = Self(27);
    pub const TIFF_SETGET_C16_UINT8: Self = Self(28);
    pub const TIFF_SETGET_C16_SINT8: Self = Self(29);
    pub const TIFF_SETGET_C16_UINT16: Self = Self(30);
    pub const TIFF_SETGET_C16_SINT16: Self = Self(31);
    pub const TIFF_SETGET_C16_UINT32: Self = Self(32);
    pub const TIFF_SETGET_C16_SINT32: Self = Self(33);
    pub const TIFF_SETGET_C16_UINT64: Self = Self(34);
    pub const TIFF_SETGET_C16_SINT64: Self = Self(35);
    pub const TIFF_SETGET_C16_FLOAT: Self = Self(36);
    pub const TIFF_SETGET_C16_DOUBLE: Self = Self(37);
    pub const TIFF_SETGET_C16_IFD8: Self = Self(38);
    pub const TIFF_SETGET_C32_ASCII: Self = Self(39);
    pub const TIFF_SETGET_C32_UINT8: Self = Self(40);
    pub const TIFF_SETGET_C32_SINT8: Self = Self(41);
    pub const TIFF_SETGET_C32_UINT16: Self = Self(42);
    pub const TIFF_SETGET_C32_SINT16: Self = Self(43);
    pub const TIFF_SETGET_C32_UINT32: Self = Self(44);
    pub const TIFF_SETGET_C32_SINT32: Self = Self(45);
    pub const TIFF_SETGET_C32_UINT64: Self = Self(46);
    pub const TIFF_SETGET_C32_SINT64: Self = Self(47);
    pub const TIFF_SETGET_C32_FLOAT: Self = Self(48);
    pub const TIFF_SETGET_C32_DOUBLE: Self = Self(49);
    pub const TIFF_SETGET_C32_IFD8: Self = Self(50);
    pub const TIFF_SETGET_OTHER: Self = Self(51);
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TIFFFieldArray {
    pub r#type: TIFFFieldArrayType,
    pub allocated_size: u32,
    pub count: u32,
    pub fields: *mut TIFFField,
}

unsafe impl Sync for TIFFFieldArray {}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TIFFField {
    pub field_tag: u32,
    pub field_readcount: i16,
    pub field_writecount: i16,
    pub field_type: TIFFDataType,
    pub field_anonymous: u32,
    pub set_field_type: TIFFSetGetFieldType,
    pub get_field_type: TIFFSetGetFieldType,
    pub field_bit: u16,
    pub field_oktochange: u8,
    pub field_passcount: u8,
    pub field_name: *mut c_char,
    pub field_subfields: *mut TIFFFieldArray,
}

unsafe impl Sync for TIFFField {}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TIFFFieldInfo {
    pub field_tag: u32,
    pub field_readcount: i16,
    pub field_writecount: i16,
    pub field_type: TIFFDataType,
    pub field_bit: u16,
    pub field_oktochange: u8,
    pub field_passcount: u8,
    pub field_name: *mut c_char,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct TIFFTagMethods {
    pub vsetfield: *mut c_void,
    pub vgetfield: *mut c_void,
    pub printdir: *mut c_void,
}

pub type TIFFExtendProc = Option<unsafe extern "C" fn(*mut TIFF)>;
pub type TIFFInitMethod = Option<unsafe extern "C" fn(*mut TIFF, c_int) -> c_int>;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TIFFCodec {
    pub name: *mut c_char,
    pub scheme: u16,
    pub init: TIFFInitMethod,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SafeTiffAbiLayoutProbe {
    pub version: u32,
    pub struct_size: usize,
    pub tiff_field_info_size: usize,
    pub tiff_field_info_field_tag_offset: usize,
    pub tiff_field_info_field_readcount_offset: usize,
    pub tiff_field_info_field_writecount_offset: usize,
    pub tiff_field_info_field_type_offset: usize,
    pub tiff_field_info_field_bit_offset: usize,
    pub tiff_field_info_field_oktochange_offset: usize,
    pub tiff_field_info_field_passcount_offset: usize,
    pub tiff_field_info_field_name_offset: usize,
    pub tiff_tag_methods_size: usize,
    pub tiff_tag_methods_vsetfield_offset: usize,
    pub tiff_tag_methods_vgetfield_offset: usize,
    pub tiff_tag_methods_printdir_offset: usize,
    pub tiff_codec_size: usize,
    pub tiff_codec_name_offset: usize,
    pub tiff_codec_scheme_offset: usize,
    pub tiff_codec_init_offset: usize,
}

pub(crate) static SAFE_TIFF_ABI_LAYOUT_PROBE: SafeTiffAbiLayoutProbe = SafeTiffAbiLayoutProbe {
    version: 2,
    struct_size: size_of::<SafeTiffAbiLayoutProbe>(),
    tiff_field_info_size: size_of::<TIFFFieldInfo>(),
    tiff_field_info_field_tag_offset: offset_of!(TIFFFieldInfo, field_tag),
    tiff_field_info_field_readcount_offset: offset_of!(TIFFFieldInfo, field_readcount),
    tiff_field_info_field_writecount_offset: offset_of!(TIFFFieldInfo, field_writecount),
    tiff_field_info_field_type_offset: offset_of!(TIFFFieldInfo, field_type),
    tiff_field_info_field_bit_offset: offset_of!(TIFFFieldInfo, field_bit),
    tiff_field_info_field_oktochange_offset: offset_of!(TIFFFieldInfo, field_oktochange),
    tiff_field_info_field_passcount_offset: offset_of!(TIFFFieldInfo, field_passcount),
    tiff_field_info_field_name_offset: offset_of!(TIFFFieldInfo, field_name),
    tiff_tag_methods_size: size_of::<TIFFTagMethods>(),
    tiff_tag_methods_vsetfield_offset: offset_of!(TIFFTagMethods, vsetfield),
    tiff_tag_methods_vgetfield_offset: offset_of!(TIFFTagMethods, vgetfield),
    tiff_tag_methods_printdir_offset: offset_of!(TIFFTagMethods, printdir),
    tiff_codec_size: size_of::<TIFFCodec>(),
    tiff_codec_name_offset: offset_of!(TIFFCodec, name),
    tiff_codec_scheme_offset: offset_of!(TIFFCodec, scheme),
    tiff_codec_init_offset: offset_of!(TIFFCodec, init),
};

unsafe impl Sync for TIFFCodec {}

#[no_mangle]
pub extern "C" fn safe_tiff_abi_layout_probe() -> *const SafeTiffAbiLayoutProbe {
    &SAFE_TIFF_ABI_LAYOUT_PROBE
}
