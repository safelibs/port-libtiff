use crate::abi::{TIFFCIELabToRGB, TIFFDisplay, TIFFRGBValue, TIFFYCbCrToRGB};
use libc::c_int;
use std::ffi::c_void;
use std::mem::size_of;
use std::ptr;
use std::slice;

const CIELAB_TABLE_RANGE: usize = 1500;
const SHIFT: i32 = 16;
const ONE_HALF: i32 = 1 << (SHIFT - 1);
const SGILOGENCODE_NODITHER: c_int = 0;
const SGILOGENCODE_RANDITHER: c_int = 1;
const U_NEU: f64 = 0.210526316;
const V_NEU: f64 = 0.473684211;
const UVSCALE: f64 = 410.0;
const UV_SQSIZ: f64 = 0.0035;
const UV_NDIVS: i32 = 16289;
const UV_VSTART: f64 = 0.01694;
const UV_NVS: usize = 163;
const M_LN2: f64 = std::f64::consts::LN_2;

#[derive(Clone, Copy)]
struct UvRow {
    ustart: f64,
    nus: i16,
    ncum: i16,
}

static UV_ROWS: [UvRow; UV_NVS] = [
    UvRow { ustart: 0.247663, nus: 4, ncum: 0 },
    UvRow { ustart: 0.243779, nus: 6, ncum: 4 },
    UvRow { ustart: 0.241684, nus: 7, ncum: 10 },
    UvRow { ustart: 0.237874, nus: 9, ncum: 17 },
    UvRow { ustart: 0.235906, nus: 10, ncum: 26 },
    UvRow { ustart: 0.232153, nus: 12, ncum: 36 },
    UvRow { ustart: 0.228352, nus: 14, ncum: 48 },
    UvRow { ustart: 0.226259, nus: 15, ncum: 62 },
    UvRow { ustart: 0.222371, nus: 17, ncum: 77 },
    UvRow { ustart: 0.22041, nus: 18, ncum: 94 },
    UvRow { ustart: 0.21471, nus: 21, ncum: 112 },
    UvRow { ustart: 0.212714, nus: 22, ncum: 133 },
    UvRow { ustart: 0.210721, nus: 23, ncum: 155 },
    UvRow { ustart: 0.204976, nus: 26, ncum: 178 },
    UvRow { ustart: 0.202986, nus: 27, ncum: 204 },
    UvRow { ustart: 0.199245, nus: 29, ncum: 231 },
    UvRow { ustart: 0.195525, nus: 31, ncum: 260 },
    UvRow { ustart: 0.19356, nus: 32, ncum: 291 },
    UvRow { ustart: 0.189878, nus: 34, ncum: 323 },
    UvRow { ustart: 0.186216, nus: 36, ncum: 357 },
    UvRow { ustart: 0.186216, nus: 36, ncum: 393 },
    UvRow { ustart: 0.182592, nus: 38, ncum: 429 },
    UvRow { ustart: 0.179003, nus: 40, ncum: 467 },
    UvRow { ustart: 0.175466, nus: 42, ncum: 507 },
    UvRow { ustart: 0.172001, nus: 44, ncum: 549 },
    UvRow { ustart: 0.172001, nus: 44, ncum: 593 },
    UvRow { ustart: 0.168612, nus: 46, ncum: 637 },
    UvRow { ustart: 0.168612, nus: 46, ncum: 683 },
    UvRow { ustart: 0.163575, nus: 49, ncum: 729 },
    UvRow { ustart: 0.158642, nus: 52, ncum: 778 },
    UvRow { ustart: 0.158642, nus: 52, ncum: 830 },
    UvRow { ustart: 0.158642, nus: 52, ncum: 882 },
    UvRow { ustart: 0.153815, nus: 55, ncum: 934 },
    UvRow { ustart: 0.153815, nus: 55, ncum: 989 },
    UvRow { ustart: 0.149097, nus: 58, ncum: 1044 },
    UvRow { ustart: 0.149097, nus: 58, ncum: 1102 },
    UvRow { ustart: 0.142746, nus: 62, ncum: 1160 },
    UvRow { ustart: 0.142746, nus: 62, ncum: 1222 },
    UvRow { ustart: 0.142746, nus: 62, ncum: 1284 },
    UvRow { ustart: 0.13827, nus: 65, ncum: 1346 },
    UvRow { ustart: 0.13827, nus: 65, ncum: 1411 },
    UvRow { ustart: 0.13827, nus: 65, ncum: 1476 },
    UvRow { ustart: 0.132166, nus: 69, ncum: 1541 },
    UvRow { ustart: 0.132166, nus: 69, ncum: 1610 },
    UvRow { ustart: 0.126204, nus: 73, ncum: 1679 },
    UvRow { ustart: 0.126204, nus: 73, ncum: 1752 },
    UvRow { ustart: 0.126204, nus: 73, ncum: 1825 },
    UvRow { ustart: 0.120381, nus: 77, ncum: 1898 },
    UvRow { ustart: 0.120381, nus: 77, ncum: 1975 },
    UvRow { ustart: 0.120381, nus: 77, ncum: 2052 },
    UvRow { ustart: 0.120381, nus: 77, ncum: 2129 },
    UvRow { ustart: 0.112962, nus: 82, ncum: 2206 },
    UvRow { ustart: 0.112962, nus: 82, ncum: 2288 },
    UvRow { ustart: 0.112962, nus: 82, ncum: 2370 },
    UvRow { ustart: 0.10745, nus: 86, ncum: 2452 },
    UvRow { ustart: 0.10745, nus: 86, ncum: 2538 },
    UvRow { ustart: 0.10745, nus: 86, ncum: 2624 },
    UvRow { ustart: 0.10745, nus: 86, ncum: 2710 },
    UvRow { ustart: 0.100343, nus: 91, ncum: 2796 },
    UvRow { ustart: 0.100343, nus: 91, ncum: 2887 },
    UvRow { ustart: 0.100343, nus: 91, ncum: 2978 },
    UvRow { ustart: 0.095126, nus: 95, ncum: 3069 },
    UvRow { ustart: 0.095126, nus: 95, ncum: 3164 },
    UvRow { ustart: 0.095126, nus: 95, ncum: 3259 },
    UvRow { ustart: 0.095126, nus: 95, ncum: 3354 },
    UvRow { ustart: 0.088276, nus: 100, ncum: 3449 },
    UvRow { ustart: 0.088276, nus: 100, ncum: 3549 },
    UvRow { ustart: 0.088276, nus: 100, ncum: 3649 },
    UvRow { ustart: 0.088276, nus: 100, ncum: 3749 },
    UvRow { ustart: 0.081523, nus: 105, ncum: 3849 },
    UvRow { ustart: 0.081523, nus: 105, ncum: 3954 },
    UvRow { ustart: 0.081523, nus: 105, ncum: 4059 },
    UvRow { ustart: 0.081523, nus: 105, ncum: 4164 },
    UvRow { ustart: 0.074861, nus: 110, ncum: 4269 },
    UvRow { ustart: 0.074861, nus: 110, ncum: 4379 },
    UvRow { ustart: 0.074861, nus: 110, ncum: 4489 },
    UvRow { ustart: 0.074861, nus: 110, ncum: 4599 },
    UvRow { ustart: 0.06829, nus: 115, ncum: 4709 },
    UvRow { ustart: 0.06829, nus: 115, ncum: 4824 },
    UvRow { ustart: 0.06829, nus: 115, ncum: 4939 },
    UvRow { ustart: 0.06829, nus: 115, ncum: 5054 },
    UvRow { ustart: 0.063573, nus: 119, ncum: 5169 },
    UvRow { ustart: 0.063573, nus: 119, ncum: 5288 },
    UvRow { ustart: 0.063573, nus: 119, ncum: 5407 },
    UvRow { ustart: 0.063573, nus: 119, ncum: 5526 },
    UvRow { ustart: 0.057219, nus: 124, ncum: 5645 },
    UvRow { ustart: 0.057219, nus: 124, ncum: 5769 },
    UvRow { ustart: 0.057219, nus: 124, ncum: 5893 },
    UvRow { ustart: 0.057219, nus: 124, ncum: 6017 },
    UvRow { ustart: 0.050985, nus: 129, ncum: 6141 },
    UvRow { ustart: 0.050985, nus: 129, ncum: 6270 },
    UvRow { ustart: 0.050985, nus: 129, ncum: 6399 },
    UvRow { ustart: 0.050985, nus: 129, ncum: 6528 },
    UvRow { ustart: 0.050985, nus: 129, ncum: 6657 },
    UvRow { ustart: 0.044859, nus: 134, ncum: 6786 },
    UvRow { ustart: 0.044859, nus: 134, ncum: 6920 },
    UvRow { ustart: 0.044859, nus: 134, ncum: 7054 },
    UvRow { ustart: 0.044859, nus: 134, ncum: 7188 },
    UvRow { ustart: 0.040571, nus: 138, ncum: 7322 },
    UvRow { ustart: 0.040571, nus: 138, ncum: 7460 },
    UvRow { ustart: 0.040571, nus: 138, ncum: 7598 },
    UvRow { ustart: 0.040571, nus: 138, ncum: 7736 },
    UvRow { ustart: 0.036339, nus: 142, ncum: 7874 },
    UvRow { ustart: 0.036339, nus: 142, ncum: 8016 },
    UvRow { ustart: 0.036339, nus: 142, ncum: 8158 },
    UvRow { ustart: 0.036339, nus: 142, ncum: 8300 },
    UvRow { ustart: 0.032139, nus: 146, ncum: 8442 },
    UvRow { ustart: 0.032139, nus: 146, ncum: 8588 },
    UvRow { ustart: 0.032139, nus: 146, ncum: 8734 },
    UvRow { ustart: 0.032139, nus: 146, ncum: 8880 },
    UvRow { ustart: 0.027947, nus: 150, ncum: 9026 },
    UvRow { ustart: 0.027947, nus: 150, ncum: 9176 },
    UvRow { ustart: 0.027947, nus: 150, ncum: 9326 },
    UvRow { ustart: 0.023739, nus: 154, ncum: 9476 },
    UvRow { ustart: 0.023739, nus: 154, ncum: 9630 },
    UvRow { ustart: 0.023739, nus: 154, ncum: 9784 },
    UvRow { ustart: 0.023739, nus: 154, ncum: 9938 },
    UvRow { ustart: 0.019504, nus: 158, ncum: 10092 },
    UvRow { ustart: 0.019504, nus: 158, ncum: 10250 },
    UvRow { ustart: 0.019504, nus: 158, ncum: 10408 },
    UvRow { ustart: 0.016976, nus: 161, ncum: 10566 },
    UvRow { ustart: 0.016976, nus: 161, ncum: 10727 },
    UvRow { ustart: 0.016976, nus: 161, ncum: 10888 },
    UvRow { ustart: 0.016976, nus: 161, ncum: 11049 },
    UvRow { ustart: 0.012639, nus: 165, ncum: 11210 },
    UvRow { ustart: 0.012639, nus: 165, ncum: 11375 },
    UvRow { ustart: 0.012639, nus: 165, ncum: 11540 },
    UvRow { ustart: 0.009991, nus: 168, ncum: 11705 },
    UvRow { ustart: 0.009991, nus: 168, ncum: 11873 },
    UvRow { ustart: 0.009991, nus: 168, ncum: 12041 },
    UvRow { ustart: 0.009016, nus: 170, ncum: 12209 },
    UvRow { ustart: 0.009016, nus: 170, ncum: 12379 },
    UvRow { ustart: 0.009016, nus: 170, ncum: 12549 },
    UvRow { ustart: 0.006217, nus: 173, ncum: 12719 },
    UvRow { ustart: 0.006217, nus: 173, ncum: 12892 },
    UvRow { ustart: 0.005097, nus: 175, ncum: 13065 },
    UvRow { ustart: 0.005097, nus: 175, ncum: 13240 },
    UvRow { ustart: 0.005097, nus: 175, ncum: 13415 },
    UvRow { ustart: 0.003909, nus: 177, ncum: 13590 },
    UvRow { ustart: 0.003909, nus: 177, ncum: 13767 },
    UvRow { ustart: 0.00234, nus: 177, ncum: 13944 },
    UvRow { ustart: 0.002389, nus: 170, ncum: 14121 },
    UvRow { ustart: 0.001068, nus: 164, ncum: 14291 },
    UvRow { ustart: 0.001653, nus: 157, ncum: 14455 },
    UvRow { ustart: 0.000717, nus: 150, ncum: 14612 },
    UvRow { ustart: 0.001614, nus: 143, ncum: 14762 },
    UvRow { ustart: 0.00027, nus: 136, ncum: 14905 },
    UvRow { ustart: 0.000484, nus: 129, ncum: 15041 },
    UvRow { ustart: 0.001103, nus: 123, ncum: 15170 },
    UvRow { ustart: 0.001242, nus: 115, ncum: 15293 },
    UvRow { ustart: 0.001188, nus: 109, ncum: 15408 },
    UvRow { ustart: 0.001011, nus: 103, ncum: 15517 },
    UvRow { ustart: 0.000709, nus: 97, ncum: 15620 },
    UvRow { ustart: 0.000301, nus: 89, ncum: 15717 },
    UvRow { ustart: 0.002416, nus: 82, ncum: 15806 },
    UvRow { ustart: 0.003251, nus: 76, ncum: 15888 },
    UvRow { ustart: 0.003246, nus: 69, ncum: 15964 },
    UvRow { ustart: 0.004141, nus: 62, ncum: 16033 },
    UvRow { ustart: 0.005963, nus: 55, ncum: 16095 },
    UvRow { ustart: 0.008839, nus: 47, ncum: 16150 },
    UvRow { ustart: 0.01049, nus: 40, ncum: 16197 },
    UvRow { ustart: 0.016994, nus: 31, ncum: 16237 },
    UvRow { ustart: 0.023659, nus: 21, ncum: 16268 },
];

fn tiff_min<T: PartialOrd>(a: T, b: T) -> T {
    if a < b { a } else { b }
}

fn tiff_max<T: PartialOrd>(a: T, b: T) -> T {
    if a > b { a } else { b }
}

fn clamp_int(value: i32, min_value: i32, max_value: i32) -> u32 {
    value.clamp(min_value, max_value) as u32
}

fn clamp_float(value: f32, min_value: f32, max_value: f32) -> f32 {
    if !(value >= min_value) {
        min_value
    } else if value > max_value {
        max_value
    } else {
        value
    }
}

fn clampw(value: f32, min_value: f32, max_value: f32) -> f32 {
    if value < min_value {
        min_value
    } else if value > max_value {
        max_value
    } else {
        value
    }
}

fn fix(value: f32) -> i32 {
    (value * ((1_i64 << SHIFT) as f32) + 0.5) as i32
}

fn code_to_v(code: i32, ref_black: f32, ref_white: f32, code_range: i32) -> f32 {
    let denom = ref_white - ref_black;
    (((code as f32) - ref_black) * (code_range as f32)) / if denom != 0.0 { denom } else { 1.0 }
}

fn tiff_itrunc(value: f64, method: c_int) -> i32 {
    if method == SGILOGENCODE_NODITHER {
        value as i32
    } else {
        (value + (unsafe { libc::rand() as f64 } * (1.0 / libc::RAND_MAX as f64)) - 0.5) as i32
    }
}

fn xyz_to_rgb24_impl(xyz: &[f32; 3]) -> [u8; 3] {
    let r = 2.690 * xyz[0] as f64 + -1.276 * xyz[1] as f64 + -0.414 * xyz[2] as f64;
    let g = -1.022 * xyz[0] as f64 + 1.978 * xyz[1] as f64 + 0.044 * xyz[2] as f64;
    let b = 0.061 * xyz[0] as f64 + -0.224 * xyz[1] as f64 + 1.163 * xyz[2] as f64;
    [
        if r <= 0.0 {
            0
        } else if r >= 1.0 {
            255
        } else {
            (256.0 * r.sqrt()) as u8
        },
        if g <= 0.0 {
            0
        } else if g >= 1.0 {
            255
        } else {
            (256.0 * g.sqrt()) as u8
        },
        if b <= 0.0 {
            0
        } else if b >= 1.0 {
            255
        } else {
            (256.0 * b.sqrt()) as u8
        },
    ]
}

pub(crate) unsafe fn free_ycbcr_tables(state: *mut TIFFYCbCrToRGB) {
    if state.is_null() || (*state).clamptab.is_null() {
        return;
    }
    let base = (*state).clamptab.sub(256).cast::<c_void>();
    libc::free(base);
    (*state).clamptab = ptr::null_mut();
    (*state).Cr_r_tab = ptr::null_mut();
    (*state).Cb_b_tab = ptr::null_mut();
    (*state).Cr_g_tab = ptr::null_mut();
    (*state).Cb_g_tab = ptr::null_mut();
    (*state).Y_tab = ptr::null_mut();
}

#[no_mangle]
pub unsafe extern "C" fn safe_tiff_cielab_to_xyz(
    cielab: *mut TIFFCIELabToRGB,
    l: u32,
    a: i32,
    b: i32,
    x: *mut f32,
    y: *mut f32,
    z: *mut f32,
) {
    if cielab.is_null() || x.is_null() || y.is_null() || z.is_null() {
        return;
    }
    safe_tiff_cielab16_to_xyz(cielab, l.saturating_mul(257), a.saturating_mul(256), b.saturating_mul(256), x, y, z);
}

pub(crate) unsafe fn safe_tiff_cielab16_to_xyz(
    cielab: *mut TIFFCIELabToRGB,
    l: u32,
    a: i32,
    b: i32,
    x: *mut f32,
    y: *mut f32,
    z: *mut f32,
) {
    let l_value = l as f32 * 100.0 / 65535.0;
    let cby;
    if l_value < 8.856 {
        *y = (l_value * (*cielab).Y0) / 903.292;
        cby = 7.787 * (*y / (*cielab).Y0) + 16.0 / 116.0;
    } else {
        cby = (l_value + 16.0) / 116.0;
        *y = (*cielab).Y0 * cby * cby * cby;
    }
    let mut tmp = a as f32 / 256.0 / 500.0 + cby;
    if tmp < 0.2069 {
        *x = (*cielab).X0 * (tmp - 0.13793) / 7.787;
    } else {
        *x = (*cielab).X0 * tmp * tmp * tmp;
    }
    tmp = cby - b as f32 / 256.0 / 200.0;
    if tmp < 0.2069 {
        *z = (*cielab).Z0 * (tmp - 0.13793) / 7.787;
    } else {
        *z = (*cielab).Z0 * tmp * tmp * tmp;
    }
}

#[no_mangle]
pub unsafe extern "C" fn safe_tiff_xyz_to_rgb(
    cielab: *mut TIFFCIELabToRGB,
    x: f32,
    y: f32,
    z: f32,
    r: *mut u32,
    g: *mut u32,
    b: *mut u32,
) {
    if cielab.is_null() || r.is_null() || g.is_null() || b.is_null() {
        return;
    }
    let matrix = (*cielab).display.d_mat;
    let mut yr = matrix[0][0] * x + matrix[0][1] * y + matrix[0][2] * z;
    let mut yg = matrix[1][0] * x + matrix[1][1] * y + matrix[1][2] * z;
    let mut yb = matrix[2][0] * x + matrix[2][1] * y + matrix[2][2] * z;

    yr = tiff_max(yr, (*cielab).display.d_Y0R);
    yg = tiff_max(yg, (*cielab).display.d_Y0G);
    yb = tiff_max(yb, (*cielab).display.d_Y0B);
    yr = tiff_min(yr, (*cielab).display.d_YCR);
    yg = tiff_min(yg, (*cielab).display.d_YCG);
    yb = tiff_min(yb, (*cielab).display.d_YCB);

    let mut index = ((yr - (*cielab).display.d_Y0R) / (*cielab).rstep) as usize;
    index = tiff_min((*cielab).range as usize, index);
    *r = ((*cielab).Yr2r[index] + if (*cielab).Yr2r[index] >= 0.0 { 0.5 } else { -0.5 }) as u32;

    index = ((yg - (*cielab).display.d_Y0G) / (*cielab).gstep) as usize;
    index = tiff_min((*cielab).range as usize, index);
    *g = ((*cielab).Yg2g[index] + if (*cielab).Yg2g[index] >= 0.0 { 0.5 } else { -0.5 }) as u32;

    index = ((yb - (*cielab).display.d_Y0B) / (*cielab).bstep) as usize;
    index = tiff_min((*cielab).range as usize, index);
    *b = ((*cielab).Yb2b[index] + if (*cielab).Yb2b[index] >= 0.0 { 0.5 } else { -0.5 }) as u32;

    *r = tiff_min(*r, (*cielab).display.d_Vrwr);
    *g = tiff_min(*g, (*cielab).display.d_Vrwg);
    *b = tiff_min(*b, (*cielab).display.d_Vrwb);
}

#[no_mangle]
pub unsafe extern "C" fn safe_tiff_cielab_to_rgb_init(
    cielab: *mut TIFFCIELabToRGB,
    display: *const TIFFDisplay,
    ref_white: *mut f32,
) -> c_int {
    if cielab.is_null() || display.is_null() || ref_white.is_null() {
        return -1;
    }
    (*cielab).range = CIELAB_TABLE_RANGE as c_int;
    (*cielab).display = *display;

    let mut gamma = 1.0 / (*cielab).display.d_gammaR as f64;
    (*cielab).rstep = ((*cielab).display.d_YCR - (*cielab).display.d_Y0R) / (*cielab).range as f32;
    for i in 0..=CIELAB_TABLE_RANGE {
        (*cielab).Yr2r[i] = (*cielab).display.d_Vrwr as f32 * ((i as f64 / (*cielab).range as f64).powf(gamma) as f32);
    }

    gamma = 1.0 / (*cielab).display.d_gammaG as f64;
    (*cielab).gstep = ((*cielab).display.d_YCR - (*cielab).display.d_Y0R) / (*cielab).range as f32;
    for i in 0..=CIELAB_TABLE_RANGE {
        (*cielab).Yg2g[i] = (*cielab).display.d_Vrwg as f32 * ((i as f64 / (*cielab).range as f64).powf(gamma) as f32);
    }

    gamma = 1.0 / (*cielab).display.d_gammaB as f64;
    (*cielab).bstep = ((*cielab).display.d_YCR - (*cielab).display.d_Y0R) / (*cielab).range as f32;
    for i in 0..=CIELAB_TABLE_RANGE {
        (*cielab).Yb2b[i] = (*cielab).display.d_Vrwb as f32 * ((i as f64 / (*cielab).range as f64).powf(gamma) as f32);
    }

    let white = slice::from_raw_parts(ref_white, 3);
    (*cielab).X0 = white[0];
    (*cielab).Y0 = white[1];
    (*cielab).Z0 = white[2];
    0
}

#[no_mangle]
pub unsafe extern "C" fn safe_tiff_ycbcr_to_rgb(
    ycbcr: *mut TIFFYCbCrToRGB,
    y: u32,
    cb: i32,
    cr: i32,
    r: *mut u32,
    g: *mut u32,
    b: *mut u32,
) {
    if ycbcr.is_null() || r.is_null() || g.is_null() || b.is_null() {
        return;
    }
    let y = tiff_min(y, 255) as usize;
    let cb = cb.clamp(0, 255) as usize;
    let cr = cr.clamp(0, 255) as usize;
    let i = *(*ycbcr).Y_tab.add(y) + *(*ycbcr).Cr_r_tab.add(cr);
    *r = clamp_int(i, 0, 255);
    let i = *(*ycbcr).Y_tab.add(y) + ((*(*ycbcr).Cb_g_tab.add(cb) + *(*ycbcr).Cr_g_tab.add(cr)) >> SHIFT);
    *g = clamp_int(i, 0, 255);
    let i = *(*ycbcr).Y_tab.add(y) + *(*ycbcr).Cb_b_tab.add(cb);
    *b = clamp_int(i, 0, 255);
}

#[no_mangle]
pub unsafe extern "C" fn safe_tiff_ycbcr_to_rgb_init(
    ycbcr: *mut TIFFYCbCrToRGB,
    luma: *mut f32,
    ref_black_white: *mut f32,
) -> c_int {
    if ycbcr.is_null() || luma.is_null() || ref_black_white.is_null() {
        return -1;
    }

    free_ycbcr_tables(ycbcr);

    let block_bytes = 1024
        + 256 * size_of::<c_int>() * 2
        + 256 * size_of::<i32>() * 3;
    let base = libc::malloc(block_bytes).cast::<u8>();
    if base.is_null() {
        return -1;
    }
    ptr::write_bytes(base, 0, block_bytes);

    let clamptab_base = base.cast::<TIFFRGBValue>();
    ptr::write_bytes(clamptab_base, 0, 256);
    let clamptab = clamptab_base.add(256);
    for i in 0..256 {
        *clamptab.add(i) = i as TIFFRGBValue;
    }
    ptr::write_bytes(clamptab.add(256), 255, 512);

    (*ycbcr).clamptab = clamptab;
    (*ycbcr).Cr_r_tab = clamptab.add(3 * 256).cast::<c_int>();
    (*ycbcr).Cb_b_tab = (*ycbcr).Cr_r_tab.add(256);
    (*ycbcr).Cr_g_tab = (*ycbcr).Cb_b_tab.add(256).cast::<i32>();
    (*ycbcr).Cb_g_tab = (*ycbcr).Cr_g_tab.add(256);
    (*ycbcr).Y_tab = (*ycbcr).Cb_g_tab.add(256);

    let luma = slice::from_raw_parts(luma, 3);
    let ref_black_white = slice::from_raw_parts(ref_black_white, 6);
    let d1 = fix(clamp_float(2.0 - 2.0 * luma[0], 0.0, 2.0));
    let d2 = -fix(clamp_float(luma[0] * (2.0 - 2.0 * luma[0]) / luma[1], 0.0, 2.0));
    let d3 = fix(clamp_float(2.0 - 2.0 * luma[2], 0.0, 2.0));
    let d4 = -fix(clamp_float(luma[2] * (2.0 - 2.0 * luma[2]) / luma[1], 0.0, 2.0));

    for (i, x) in (-128..128).enumerate() {
        let cr = clampw(
            code_to_v(x, ref_black_white[4] - 128.0, ref_black_white[5] - 128.0, 127),
            -128.0 * 32.0,
            128.0 * 32.0,
        ) as i32;
        let cb = clampw(
            code_to_v(x, ref_black_white[2] - 128.0, ref_black_white[3] - 128.0, 127),
            -128.0 * 32.0,
            128.0 * 32.0,
        ) as i32;
        *(*ycbcr).Cr_r_tab.add(i) = (d1 * cr + ONE_HALF) >> SHIFT;
        *(*ycbcr).Cb_b_tab.add(i) = (d3 * cb + ONE_HALF) >> SHIFT;
        *(*ycbcr).Cr_g_tab.add(i) = d2 * cr;
        *(*ycbcr).Cb_g_tab.add(i) = d4 * cb + ONE_HALF;
        *(*ycbcr).Y_tab.add(i) = clampw(
            code_to_v(x + 128, ref_black_white[0], ref_black_white[1], 255),
            -128.0 * 32.0,
            128.0 * 32.0,
        ) as i32;
    }
    0
}

#[no_mangle]
pub extern "C" fn safe_tiff_logl16_to_y(p16: c_int) -> f64 {
    let le = p16 & 0x7fff;
    if le == 0 {
        return 0.0;
    }
    let y = (M_LN2 / 256.0 * (le as f64 + 0.5) - M_LN2 * 64.0).exp();
    if (p16 & 0x8000) == 0 { y } else { -y }
}

#[no_mangle]
pub extern "C" fn safe_tiff_logl16_from_y(y: f64, method: c_int) -> c_int {
    if y >= 1.8371976e19 {
        0x7fff
    } else if y <= -1.8371976e19 {
        0xffff_u16 as c_int
    } else if y > 5.4136769e-20 {
        tiff_itrunc(256.0 * (y.log2() + 64.0), method)
    } else if y < -5.4136769e-20 {
        (!0x7fff_i32) | tiff_itrunc(256.0 * ((-y).log2() + 64.0), method)
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn safe_tiff_logl10_to_y(p10: c_int) -> f64 {
    if p10 == 0 {
        0.0
    } else {
        (M_LN2 / 64.0 * (p10 as f64 + 0.5) - M_LN2 * 12.0).exp()
    }
}

#[no_mangle]
pub extern "C" fn safe_tiff_logl10_from_y(y: f64, method: c_int) -> c_int {
    if y >= 15.742 {
        0x3ff
    } else if y <= 0.00024283 {
        0
    } else {
        tiff_itrunc(64.0 * (y.log2() + 12.0), method)
    }
}

fn oog_encode(u: f64, v: f64) -> c_int {
    let mut best_index = 0usize;
    let mut best_error = f64::INFINITY;
    for (vi, row) in UV_ROWS.iter().enumerate() {
        let mut step = row.nus as i32 - 1;
        if vi == 0 || vi + 1 == UV_NVS || step <= 0 {
            step = 1;
        }
        let va = UV_VSTART + (vi as f64 + 0.5) * UV_SQSIZ;
        let mut ui = row.nus as i32 - 1;
        while ui >= 0 {
            let ua = row.ustart + (ui as f64 + 0.5) * UV_SQSIZ;
            let ang = (100.0 * 0.499999999 / std::f64::consts::PI)
                * (v - V_NEU).atan2(u - U_NEU)
                + 50.0;
            let candidate = row.ncum as i32 + ui;
            let current = (ang - (best_index as f64 + 0.5)).abs();
            let error = ((va - v) * (va - v) + (ua - u) * (ua - u)).sqrt() + current * 1e-6;
            if error < best_error {
                best_error = error;
                best_index = candidate as usize;
            }
            ui -= step;
        }
    }
    best_index as c_int
}

#[no_mangle]
pub extern "C" fn safe_tiff_uv_encode(u: f64, v: f64, method: c_int) -> c_int {
    let (u, v) = if u.is_nan() || v.is_nan() { (U_NEU, V_NEU) } else { (u, v) };
    if v < UV_VSTART {
        return oog_encode(u, v);
    }
    let vi = tiff_itrunc((v - UV_VSTART) * (1.0 / UV_SQSIZ), method);
    if vi < 0 || vi as usize >= UV_NVS {
        return oog_encode(u, v);
    }
    let row = UV_ROWS[vi as usize];
    if u < row.ustart {
        return oog_encode(u, v);
    }
    let ui = tiff_itrunc((u - row.ustart) * (1.0 / UV_SQSIZ), method);
    if ui < 0 || ui >= row.nus as i32 {
        return oog_encode(u, v);
    }
    row.ncum as c_int + ui
}

#[no_mangle]
pub unsafe extern "C" fn safe_tiff_uv_decode(up: *mut f64, vp: *mut f64, code: c_int) -> c_int {
    if up.is_null() || vp.is_null() {
        return -1;
    }
    if !(0..UV_NDIVS).contains(&code) {
        return -1;
    }
    let mut lower = 0usize;
    let mut upper = UV_NVS;
    while upper - lower > 1 {
        let vi = (lower + upper) >> 1;
        let ui = code - UV_ROWS[vi].ncum as c_int;
        if ui > 0 {
            lower = vi;
        } else if ui < 0 {
            upper = vi;
        } else {
            lower = vi;
            break;
        }
    }
    let vi = lower;
    let ui = code - UV_ROWS[vi].ncum as c_int;
    *up = UV_ROWS[vi].ustart + (ui as f64 + 0.5) * UV_SQSIZ;
    *vp = UV_VSTART + (vi as f64 + 0.5) * UV_SQSIZ;
    0
}

#[no_mangle]
pub unsafe extern "C" fn safe_tiff_logluv24_to_xyz(p: u32, xyz: *mut f32) {
    if xyz.is_null() {
        return;
    }
    let xyz_slice = slice::from_raw_parts_mut(xyz, 3);
    let l = safe_tiff_logl10_to_y(((p >> 14) & 0x3ff) as c_int);
    if l <= 0.0 {
        xyz_slice.fill(0.0);
        return;
    }
    let mut u = U_NEU;
    let mut v = V_NEU;
    if safe_tiff_uv_decode(&mut u, &mut v, (p & 0x3fff) as c_int) < 0 {
        u = U_NEU;
        v = V_NEU;
    }
    let s = 1.0 / (6.0 * u - 16.0 * v + 12.0);
    let x = 9.0 * u * s;
    let y = 4.0 * v * s;
    xyz_slice[0] = (x / y * l) as f32;
    xyz_slice[1] = l as f32;
    xyz_slice[2] = ((1.0 - x - y) / y * l) as f32;
}

#[no_mangle]
pub unsafe extern "C" fn safe_tiff_logluv24_from_xyz(xyz: *mut f32, method: c_int) -> u32 {
    if xyz.is_null() {
        return 0;
    }
    let xyz = slice::from_raw_parts(xyz, 3);
    let le = safe_tiff_logl10_from_y(xyz[1] as f64, method);
    let s = xyz[0] as f64 + 15.0 * xyz[1] as f64 + 3.0 * xyz[2] as f64;
    let (u, v) = if le == 0 || s <= 0.0 {
        (U_NEU, V_NEU)
    } else {
        (4.0 * xyz[0] as f64 / s, 9.0 * xyz[1] as f64 / s)
    };
    let mut ce = safe_tiff_uv_encode(u, v, method);
    if ce < 0 {
        ce = safe_tiff_uv_encode(U_NEU, V_NEU, SGILOGENCODE_NODITHER);
    }
    ((le as u32) << 14) | (ce as u32 & 0x3fff)
}

#[no_mangle]
pub unsafe extern "C" fn safe_tiff_logluv32_to_xyz(p: u32, xyz: *mut f32) {
    if xyz.is_null() {
        return;
    }
    let xyz_slice = slice::from_raw_parts_mut(xyz, 3);
    let l = safe_tiff_logl16_to_y((p >> 16) as c_int);
    if l <= 0.0 {
        xyz_slice.fill(0.0);
        return;
    }
    let u = ((p >> 8) & 0xff) as f64 / UVSCALE + 0.5 / UVSCALE;
    let v = (p & 0xff) as f64 / UVSCALE + 0.5 / UVSCALE;
    let s = 1.0 / (6.0 * u - 16.0 * v + 12.0);
    let x = 9.0 * u * s;
    let y = 4.0 * v * s;
    xyz_slice[0] = (x / y * l) as f32;
    xyz_slice[1] = l as f32;
    xyz_slice[2] = ((1.0 - x - y) / y * l) as f32;
}

#[no_mangle]
pub unsafe extern "C" fn safe_tiff_logluv32_from_xyz(xyz: *mut f32, method: c_int) -> u32 {
    if xyz.is_null() {
        return 0;
    }
    let xyz = slice::from_raw_parts(xyz, 3);
    let le = safe_tiff_logl16_from_y(xyz[1] as f64, method) as u32;
    let s = xyz[0] as f64 + 15.0 * xyz[1] as f64 + 3.0 * xyz[2] as f64;
    let (u, v) = if le == 0 || s <= 0.0 {
        (U_NEU, V_NEU)
    } else {
        (4.0 * xyz[0] as f64 / s, 9.0 * xyz[1] as f64 / s)
    };
    let mut ue = if u <= 0.0 { 0 } else { tiff_itrunc(UVSCALE * u, method) as u32 };
    let mut ve = if v <= 0.0 { 0 } else { tiff_itrunc(UVSCALE * v, method) as u32 };
    if ue > 255 {
        ue = 255;
    }
    if ve > 255 {
        ve = 255;
    }
    (le << 16) | (ue << 8) | ve
}

#[no_mangle]
pub unsafe extern "C" fn safe_tiff_xyz_to_rgb24(xyz: *mut f32, rgb: *mut u8) {
    if xyz.is_null() || rgb.is_null() {
        return;
    }
    let xyz_ref = &*(xyz.cast::<[f32; 3]>());
    let rgb_value = xyz_to_rgb24_impl(xyz_ref);
    ptr::copy_nonoverlapping(rgb_value.as_ptr(), rgb, 3);
}

pub(crate) fn sgilog24_decode_row(input: &[u8], pixels: usize) -> Option<Vec<u32>> {
    let needed = pixels.checked_mul(3)?;
    if input.len() < needed {
        return None;
    }
    let mut out = Vec::with_capacity(pixels);
    for chunk in input[..needed].chunks_exact(3) {
        out.push(((chunk[0] as u32) << 16) | ((chunk[1] as u32) << 8) | chunk[2] as u32);
    }
    Some(out)
}

pub(crate) fn sgilog32_decode_row(input: &[u8], pixels: usize) -> Option<Vec<u32>> {
    let mut out = vec![0u32; pixels];
    let mut offset = 0usize;
    for shift in [24u32, 16, 8, 0] {
        let mut written = 0usize;
        while written < pixels && offset < input.len() {
            let control = input[offset];
            offset += 1;
            if control >= 128 {
                if offset >= input.len() {
                    return None;
                }
                let run = control as usize + 2 - 128;
                let value = (input[offset] as u32) << shift;
                offset += 1;
                for _ in 0..run {
                    if written >= pixels {
                        break;
                    }
                    out[written] |= value;
                    written += 1;
                }
            } else {
                let run = control as usize;
                for _ in 0..run {
                    let value = (input.get(offset).copied()? as u32) << shift;
                    offset += 1;
                    if written >= pixels {
                        return None;
                    }
                    out[written] |= value;
                    written += 1;
                }
            }
        }
        if written != pixels {
            return None;
        }
    }
    Some(out)
}
