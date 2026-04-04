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

pub type TIFFRGBValue = u8;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TIFFDisplay {
    pub d_mat: [[f32; 3]; 3],
    pub d_YCR: f32,
    pub d_YCG: f32,
    pub d_YCB: f32,
    pub d_Vrwr: u32,
    pub d_Vrwg: u32,
    pub d_Vrwb: u32,
    pub d_Y0R: f32,
    pub d_Y0G: f32,
    pub d_Y0B: f32,
    pub d_gammaR: f32,
    pub d_gammaG: f32,
    pub d_gammaB: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TIFFYCbCrToRGB {
    pub clamptab: *mut TIFFRGBValue,
    pub Cr_r_tab: *mut c_int,
    pub Cb_b_tab: *mut c_int,
    pub Cr_g_tab: *mut i32,
    pub Cb_g_tab: *mut i32,
    pub Y_tab: *mut i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TIFFCIELabToRGB {
    pub range: c_int,
    pub rstep: f32,
    pub gstep: f32,
    pub bstep: f32,
    pub X0: f32,
    pub Y0: f32,
    pub Z0: f32,
    pub display: TIFFDisplay,
    pub Yr2r: [f32; 1501],
    pub Yg2g: [f32; 1501],
    pub Yb2b: [f32; 1501],
}

pub type TileContigRoutine = Option<
    unsafe extern "C" fn(
        *mut TIFFRGBAImage,
        *mut u32,
        u32,
        u32,
        u32,
        u32,
        i32,
        i32,
        *mut u8,
    ),
>;
pub type TileSeparateRoutine = Option<
    unsafe extern "C" fn(
        *mut TIFFRGBAImage,
        *mut u32,
        u32,
        u32,
        u32,
        u32,
        i32,
        i32,
        *mut u8,
        *mut u8,
        *mut u8,
        *mut u8,
    ),
>;
pub type TIFFRGBAImageGetRoutine =
    Option<unsafe extern "C" fn(*mut TIFFRGBAImage, *mut u32, u32, u32) -> c_int>;

#[repr(C)]
#[derive(Clone, Copy)]
pub union TIFFRGBAImagePut {
    pub any: Option<unsafe extern "C" fn(*mut TIFFRGBAImage)>,
    pub contig: TileContigRoutine,
    pub separate: TileSeparateRoutine,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TIFFRGBAImage {
    pub tif: *mut TIFF,
    pub stoponerr: c_int,
    pub isContig: c_int,
    pub alpha: c_int,
    pub width: u32,
    pub height: u32,
    pub bitspersample: u16,
    pub samplesperpixel: u16,
    pub orientation: u16,
    pub req_orientation: u16,
    pub photometric: u16,
    pub redcmap: *mut u16,
    pub greencmap: *mut u16,
    pub bluecmap: *mut u16,
    pub get: TIFFRGBAImageGetRoutine,
    pub put: TIFFRGBAImagePut,
    pub Map: *mut TIFFRGBValue,
    pub BWmap: *mut *mut u32,
    pub PALmap: *mut *mut u32,
    pub ycbcr: *mut TIFFYCbCrToRGB,
    pub cielab: *mut TIFFCIELabToRGB,
    pub UaToAa: *mut u8,
    pub Bitdepth16To8: *mut u8,
    pub row_offset: c_int,
    pub col_offset: c_int,
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
    pub tiff_display_size: usize,
    pub tiff_display_d_mat_offset: usize,
    pub tiff_display_d_ycr_offset: usize,
    pub tiff_display_d_ycg_offset: usize,
    pub tiff_display_d_ycb_offset: usize,
    pub tiff_display_d_vrwr_offset: usize,
    pub tiff_display_d_vrwg_offset: usize,
    pub tiff_display_d_vrwb_offset: usize,
    pub tiff_display_d_y0r_offset: usize,
    pub tiff_display_d_y0g_offset: usize,
    pub tiff_display_d_y0b_offset: usize,
    pub tiff_display_d_gammar_offset: usize,
    pub tiff_display_d_gammag_offset: usize,
    pub tiff_display_d_gammab_offset: usize,
    pub tiff_ycbcr_to_rgb_size: usize,
    pub tiff_ycbcr_to_rgb_clamptab_offset: usize,
    pub tiff_ycbcr_to_rgb_cr_r_tab_offset: usize,
    pub tiff_ycbcr_to_rgb_cb_b_tab_offset: usize,
    pub tiff_ycbcr_to_rgb_cr_g_tab_offset: usize,
    pub tiff_ycbcr_to_rgb_cb_g_tab_offset: usize,
    pub tiff_ycbcr_to_rgb_y_tab_offset: usize,
    pub tiff_cielab_to_rgb_size: usize,
    pub tiff_cielab_to_rgb_range_offset: usize,
    pub tiff_cielab_to_rgb_rstep_offset: usize,
    pub tiff_cielab_to_rgb_gstep_offset: usize,
    pub tiff_cielab_to_rgb_bstep_offset: usize,
    pub tiff_cielab_to_rgb_x0_offset: usize,
    pub tiff_cielab_to_rgb_y0_offset: usize,
    pub tiff_cielab_to_rgb_z0_offset: usize,
    pub tiff_cielab_to_rgb_display_offset: usize,
    pub tiff_cielab_to_rgb_yr2r_offset: usize,
    pub tiff_cielab_to_rgb_yg2g_offset: usize,
    pub tiff_cielab_to_rgb_yb2b_offset: usize,
    pub tiff_rgba_image_size: usize,
    pub tiff_rgba_image_tif_offset: usize,
    pub tiff_rgba_image_stoponerr_offset: usize,
    pub tiff_rgba_image_is_contig_offset: usize,
    pub tiff_rgba_image_alpha_offset: usize,
    pub tiff_rgba_image_width_offset: usize,
    pub tiff_rgba_image_height_offset: usize,
    pub tiff_rgba_image_bitspersample_offset: usize,
    pub tiff_rgba_image_samplesperpixel_offset: usize,
    pub tiff_rgba_image_orientation_offset: usize,
    pub tiff_rgba_image_req_orientation_offset: usize,
    pub tiff_rgba_image_photometric_offset: usize,
    pub tiff_rgba_image_redcmap_offset: usize,
    pub tiff_rgba_image_greencmap_offset: usize,
    pub tiff_rgba_image_bluecmap_offset: usize,
    pub tiff_rgba_image_get_offset: usize,
    pub tiff_rgba_image_put_offset: usize,
    pub tiff_rgba_image_map_offset: usize,
    pub tiff_rgba_image_bwmap_offset: usize,
    pub tiff_rgba_image_palmap_offset: usize,
    pub tiff_rgba_image_ycbcr_offset: usize,
    pub tiff_rgba_image_cielab_offset: usize,
    pub tiff_rgba_image_uatoaa_offset: usize,
    pub tiff_rgba_image_bitdepth16to8_offset: usize,
    pub tiff_rgba_image_row_offset_offset: usize,
    pub tiff_rgba_image_col_offset_offset: usize,
}

pub(crate) static SAFE_TIFF_ABI_LAYOUT_PROBE: SafeTiffAbiLayoutProbe = SafeTiffAbiLayoutProbe {
    version: 3,
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
    tiff_display_size: size_of::<TIFFDisplay>(),
    tiff_display_d_mat_offset: offset_of!(TIFFDisplay, d_mat),
    tiff_display_d_ycr_offset: offset_of!(TIFFDisplay, d_YCR),
    tiff_display_d_ycg_offset: offset_of!(TIFFDisplay, d_YCG),
    tiff_display_d_ycb_offset: offset_of!(TIFFDisplay, d_YCB),
    tiff_display_d_vrwr_offset: offset_of!(TIFFDisplay, d_Vrwr),
    tiff_display_d_vrwg_offset: offset_of!(TIFFDisplay, d_Vrwg),
    tiff_display_d_vrwb_offset: offset_of!(TIFFDisplay, d_Vrwb),
    tiff_display_d_y0r_offset: offset_of!(TIFFDisplay, d_Y0R),
    tiff_display_d_y0g_offset: offset_of!(TIFFDisplay, d_Y0G),
    tiff_display_d_y0b_offset: offset_of!(TIFFDisplay, d_Y0B),
    tiff_display_d_gammar_offset: offset_of!(TIFFDisplay, d_gammaR),
    tiff_display_d_gammag_offset: offset_of!(TIFFDisplay, d_gammaG),
    tiff_display_d_gammab_offset: offset_of!(TIFFDisplay, d_gammaB),
    tiff_ycbcr_to_rgb_size: size_of::<TIFFYCbCrToRGB>(),
    tiff_ycbcr_to_rgb_clamptab_offset: offset_of!(TIFFYCbCrToRGB, clamptab),
    tiff_ycbcr_to_rgb_cr_r_tab_offset: offset_of!(TIFFYCbCrToRGB, Cr_r_tab),
    tiff_ycbcr_to_rgb_cb_b_tab_offset: offset_of!(TIFFYCbCrToRGB, Cb_b_tab),
    tiff_ycbcr_to_rgb_cr_g_tab_offset: offset_of!(TIFFYCbCrToRGB, Cr_g_tab),
    tiff_ycbcr_to_rgb_cb_g_tab_offset: offset_of!(TIFFYCbCrToRGB, Cb_g_tab),
    tiff_ycbcr_to_rgb_y_tab_offset: offset_of!(TIFFYCbCrToRGB, Y_tab),
    tiff_cielab_to_rgb_size: size_of::<TIFFCIELabToRGB>(),
    tiff_cielab_to_rgb_range_offset: offset_of!(TIFFCIELabToRGB, range),
    tiff_cielab_to_rgb_rstep_offset: offset_of!(TIFFCIELabToRGB, rstep),
    tiff_cielab_to_rgb_gstep_offset: offset_of!(TIFFCIELabToRGB, gstep),
    tiff_cielab_to_rgb_bstep_offset: offset_of!(TIFFCIELabToRGB, bstep),
    tiff_cielab_to_rgb_x0_offset: offset_of!(TIFFCIELabToRGB, X0),
    tiff_cielab_to_rgb_y0_offset: offset_of!(TIFFCIELabToRGB, Y0),
    tiff_cielab_to_rgb_z0_offset: offset_of!(TIFFCIELabToRGB, Z0),
    tiff_cielab_to_rgb_display_offset: offset_of!(TIFFCIELabToRGB, display),
    tiff_cielab_to_rgb_yr2r_offset: offset_of!(TIFFCIELabToRGB, Yr2r),
    tiff_cielab_to_rgb_yg2g_offset: offset_of!(TIFFCIELabToRGB, Yg2g),
    tiff_cielab_to_rgb_yb2b_offset: offset_of!(TIFFCIELabToRGB, Yb2b),
    tiff_rgba_image_size: size_of::<TIFFRGBAImage>(),
    tiff_rgba_image_tif_offset: offset_of!(TIFFRGBAImage, tif),
    tiff_rgba_image_stoponerr_offset: offset_of!(TIFFRGBAImage, stoponerr),
    tiff_rgba_image_is_contig_offset: offset_of!(TIFFRGBAImage, isContig),
    tiff_rgba_image_alpha_offset: offset_of!(TIFFRGBAImage, alpha),
    tiff_rgba_image_width_offset: offset_of!(TIFFRGBAImage, width),
    tiff_rgba_image_height_offset: offset_of!(TIFFRGBAImage, height),
    tiff_rgba_image_bitspersample_offset: offset_of!(TIFFRGBAImage, bitspersample),
    tiff_rgba_image_samplesperpixel_offset: offset_of!(TIFFRGBAImage, samplesperpixel),
    tiff_rgba_image_orientation_offset: offset_of!(TIFFRGBAImage, orientation),
    tiff_rgba_image_req_orientation_offset: offset_of!(TIFFRGBAImage, req_orientation),
    tiff_rgba_image_photometric_offset: offset_of!(TIFFRGBAImage, photometric),
    tiff_rgba_image_redcmap_offset: offset_of!(TIFFRGBAImage, redcmap),
    tiff_rgba_image_greencmap_offset: offset_of!(TIFFRGBAImage, greencmap),
    tiff_rgba_image_bluecmap_offset: offset_of!(TIFFRGBAImage, bluecmap),
    tiff_rgba_image_get_offset: offset_of!(TIFFRGBAImage, get),
    tiff_rgba_image_put_offset: offset_of!(TIFFRGBAImage, put),
    tiff_rgba_image_map_offset: offset_of!(TIFFRGBAImage, Map),
    tiff_rgba_image_bwmap_offset: offset_of!(TIFFRGBAImage, BWmap),
    tiff_rgba_image_palmap_offset: offset_of!(TIFFRGBAImage, PALmap),
    tiff_rgba_image_ycbcr_offset: offset_of!(TIFFRGBAImage, ycbcr),
    tiff_rgba_image_cielab_offset: offset_of!(TIFFRGBAImage, cielab),
    tiff_rgba_image_uatoaa_offset: offset_of!(TIFFRGBAImage, UaToAa),
    tiff_rgba_image_bitdepth16to8_offset: offset_of!(TIFFRGBAImage, Bitdepth16To8),
    tiff_rgba_image_row_offset_offset: offset_of!(TIFFRGBAImage, row_offset),
    tiff_rgba_image_col_offset_offset: offset_of!(TIFFRGBAImage, col_offset),
};

unsafe impl Sync for TIFFCodec {}
unsafe impl Sync for TIFFYCbCrToRGB {}
unsafe impl Sync for TIFFCIELabToRGB {}
unsafe impl Sync for TIFFRGBAImage {}

#[no_mangle]
pub extern "C" fn safe_tiff_abi_layout_probe() -> *const SafeTiffAbiLayoutProbe {
    &SAFE_TIFF_ABI_LAYOUT_PROBE
}
