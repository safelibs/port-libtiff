mod abi;
mod core;
mod rgba;
mod strile;

pub use abi::{
    TIFFCIELabToRGB, TIFFCodec, TIFFDataType, TIFFDisplay, TIFFExtendProc, TIFFField,
    TIFFFieldArray, TIFFFieldArrayType, TIFFFieldInfo, TIFFInitMethod, TIFFRGBAImage, TIFFRGBValue,
    TIFFSetGetFieldType, TIFFTagMethods, TIFFYCbCrToRGB,
};

use crate::core::{
    current_tag_at, current_tag_count, free_directory_state, get_tag_value, last_directory,
    number_of_directories, read_custom_directory, read_next_directory, set_default_codec_methods,
    set_directory, set_sub_directory, CodecState, DirectoryState, FieldRegistryState,
};
use crate::strile::StrileState;
use libc::{c_char, c_int, c_void, off_t, size_t, ssize_t};
use std::ffi::{CStr, CString};
use std::io;
use std::mem;
use std::ptr;

type Tmsize = ssize_t;
type Toff = u64;
type Thandle = *mut c_void;

type TIFFErrorHandlerExtRRaw = *mut c_void;
type TIFFReadWriteProc = Option<unsafe extern "C" fn(Thandle, *mut c_void, Tmsize) -> Tmsize>;
type TIFFSeekProc = Option<unsafe extern "C" fn(Thandle, Toff, c_int) -> Toff>;
type TIFFCloseProc = Option<unsafe extern "C" fn(Thandle) -> c_int>;
type TIFFSizeProc = Option<unsafe extern "C" fn(Thandle) -> Toff>;
type TIFFMapFileProc = Option<unsafe extern "C" fn(Thandle, *mut *mut c_void, *mut Toff) -> c_int>;
type TIFFUnmapFileProc = Option<unsafe extern "C" fn(Thandle, *mut c_void, Toff)>;

type TIFFVoidMethod = Option<unsafe extern "C" fn(*mut TIFF)>;
type TIFFBoolMethod = Option<unsafe extern "C" fn(*mut TIFF) -> c_int>;
type TIFFPreMethod = Option<unsafe extern "C" fn(*mut TIFF, u16) -> c_int>;
type TIFFCodeMethod = Option<unsafe extern "C" fn(*mut TIFF, *mut u8, Tmsize, u16) -> c_int>;

const FILLORDER_MSB2LSB: u32 = 1;
const FILLORDER_LSB2MSB: u32 = 2;

const TIFF_FILLORDER: u32 = 0x00003;
const TIFF_SWAB: u32 = 0x00080;
const TIFF_MYBUFFER: u32 = 0x00200;
const TIFF_ISTILED: u32 = 0x00400;
const TIFF_MAPPED: u32 = 0x00800;
const TIFF_UPSAMPLED: u32 = 0x04000;
const TIFF_STRIPCHOP: u32 = 0x08000;
const TIFF_HEADERONLY: u32 = 0x10000;
const TIFF_BIGTIFF: u32 = 0x80000;
const TIFF_DEFERSTRILELOAD: u32 = 0x1000000;
const TIFF_LAZYSTRILELOAD: u32 = 0x2000000;

const TIFF_VERSION_CLASSIC: u16 = 42;
const TIFF_VERSION_BIG: u16 = 43;
const TIFF_BIGENDIAN: u16 = 0x4d4d;
const TIFF_LITTLEENDIAN: u16 = 0x4949;
const TIFF_NON_EXISTENT_DIR_NUMBER: u32 = u32::MAX;
const TIFF_TMSIZE_T_MAX: Tmsize = (usize::MAX >> 1) as Tmsize;

const VERSION_STRING: &[u8] = b"LIBTIFF, Version 4.5.1\nCopyright (c) 1988-1996 Sam Leffler\nCopyright (c) 1991-1996 Silicon Graphics, Inc.\0";
const MODULE_TIFF_CLIENT_OPEN_EXT: &[u8] = b"TIFFClientOpenExt\0";
const MODULE_TIFF_OPEN: &[u8] = b"TIFFOpen\0";

#[repr(C)]
pub struct TIFF {
    // Build-only facade fields used by copied tools and C-side lifecycle shims.
    // The authoritative lifecycle/open state lives in `TiffHandleInner`.
    pub tif_name: *mut c_char,
    pub tif_flags: u32,
    pub tif_row: u32,
    pub tif_curdir: u32,
    pub tif_rawdata: *mut u8,
    pub tif_rawdatasize: Tmsize,
    pub tif_rawcp: *mut u8,
    pub tif_rawcc: Tmsize,
    pub tif_clientdata: Thandle,
    pub tif_readproc: TIFFReadWriteProc,
    pub tif_writeproc: TIFFReadWriteProc,
    pub tif_seekproc: TIFFSeekProc,
    pub tif_closeproc: TIFFCloseProc,
    pub tif_sizeproc: TIFFSizeProc,
    pub tif_mapproc: TIFFMapFileProc,
    pub tif_unmapproc: TIFFUnmapFileProc,
    pub tif_setupdecode: TIFFBoolMethod,
    pub tif_predecode: TIFFPreMethod,
    pub tif_decoderow: TIFFCodeMethod,
    pub tif_close: TIFFVoidMethod,
    pub tif_cleanup: TIFFVoidMethod,
    pub tif_errorhandler: TIFFErrorHandlerExtRRaw,
    pub tif_errorhandler_user_data: *mut c_void,
    pub tif_warnhandler: TIFFErrorHandlerExtRRaw,
    pub tif_warnhandler_user_data: *mut c_void,
    inner: *mut TiffHandleInner,
}

#[repr(C)]
struct TiffHandleInner {
    tif_fd: c_int,
    tif_mode: c_int,
    tif_curstrip: u32,
    tif_curtile: u32,
    tif_max_single_mem_alloc: Tmsize,
    mapped_base: *mut c_void,
    mapped_size: Toff,
    header_magic: u16,
    header_version: u16,
    header_size: u16,
    _reserved0: u16,
    current_diroff: Toff,
    next_diroff: Toff,
    owned_name: *mut c_char,
    directory_state: DirectoryState,
    field_registry: FieldRegistryState,
    codec_state: CodecState,
    strile_state: StrileState,
}

#[repr(C)]
#[derive(Default)]
pub struct TIFFOpenOptions {
    errorhandler: TIFFErrorHandlerExtRRaw,
    errorhandler_user_data: *mut c_void,
    warnhandler: TIFFErrorHandlerExtRRaw,
    warnhandler_user_data: *mut c_void,
    max_single_mem_alloc: Tmsize,
}

#[allow(improper_ctypes)]
unsafe extern "C" {
    fn safe_tiff_emit_error_message(tif: *mut TIFF, module: *const c_char, message: *const c_char);
    fn safe_tiff_emit_warning_message(
        tif: *mut TIFF,
        module: *const c_char,
        message: *const c_char,
    );
    fn safe_tiff_emit_early_error_message(
        opts: *mut TIFFOpenOptions,
        clientdata: Thandle,
        module: *const c_char,
        message: *const c_char,
    );
}

fn host_is_big_endian() -> bool {
    cfg!(target_endian = "big")
}

fn make_cstring(value: impl AsRef<str>) -> CString {
    CString::new(value.as_ref().replace('\0', "?")).expect("CString::new failed")
}

fn c_name(ptr: *const c_char) -> String {
    if ptr.is_null() {
        String::from("<null>")
    } else {
        unsafe { CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned()
    }
}

fn c_module(bytes: &'static [u8]) -> *const c_char {
    bytes.as_ptr().cast()
}

fn emit_error_message(tif: *mut TIFF, module: &str, message: impl AsRef<str>) {
    let module = make_cstring(module);
    let message = make_cstring(message);
    unsafe {
        safe_tiff_emit_error_message(tif, module.as_ptr(), message.as_ptr());
    }
}

fn emit_warning_message(tif: *mut TIFF, module: &str, message: impl AsRef<str>) {
    let module = make_cstring(module);
    let message = make_cstring(message);
    unsafe {
        safe_tiff_emit_warning_message(tif, module.as_ptr(), message.as_ptr());
    }
}

fn emit_early_error_message(
    opts: *mut TIFFOpenOptions,
    clientdata: Thandle,
    module: &'static [u8],
    message: impl AsRef<str>,
) {
    let message = make_cstring(message);
    unsafe {
        safe_tiff_emit_early_error_message(opts, clientdata, c_module(module), message.as_ptr());
    }
}

fn fd_to_handle(fd: c_int) -> Thandle {
    (fd as isize) as *mut c_void
}

fn handle_to_fd(handle: Thandle) -> c_int {
    handle as isize as c_int
}

unsafe extern "C" fn stub_void_method(_: *mut TIFF) {}

unsafe extern "C" fn stub_bool_method(_: *mut TIFF) -> c_int {
    1
}

unsafe extern "C" fn stub_predecode_method(_: *mut TIFF, _: u16) -> c_int {
    1
}

unsafe extern "C" fn stub_decoderow_method(_: *mut TIFF, _: *mut u8, _: Tmsize, _: u16) -> c_int {
    0
}

unsafe extern "C" fn dummy_map_proc(_: Thandle, _: *mut *mut c_void, _: *mut Toff) -> c_int {
    0
}

unsafe extern "C" fn dummy_unmap_proc(_: Thandle, _: *mut c_void, _: Toff) {}

unsafe extern "C" fn unix_read_proc(handle: Thandle, buf: *mut c_void, size: Tmsize) -> Tmsize {
    unsafe {
        if size < 0 {
            return -1;
        }
        let fd = handle_to_fd(handle);
        let size = size as usize;
        if size == 0 {
            return 0;
        }
        let mut total = 0usize;
        while total < size {
            let chunk = size - total;
            let rc = libc::read(
                fd,
                (buf.cast::<u8>().add(total)).cast::<c_void>(),
                chunk as size_t,
            );
            if rc <= 0 {
                return if rc < 0 { -1 } else { total as Tmsize };
            }
            total += rc as usize;
        }
        total as Tmsize
    }
}

unsafe extern "C" fn unix_write_proc(handle: Thandle, buf: *mut c_void, size: Tmsize) -> Tmsize {
    unsafe {
        if size < 0 {
            return -1;
        }
        let fd = handle_to_fd(handle);
        let size = size as usize;
        if size == 0 {
            return 0;
        }
        let mut total = 0usize;
        while total < size {
            let chunk = size - total;
            let rc = libc::write(
                fd,
                (buf.cast::<u8>().add(total)).cast::<c_void>(),
                chunk as size_t,
            );
            if rc <= 0 {
                return if rc < 0 { -1 } else { total as Tmsize };
            }
            total += rc as usize;
        }
        total as Tmsize
    }
}

unsafe extern "C" fn unix_seek_proc(handle: Thandle, off: Toff, whence: c_int) -> Toff {
    unsafe {
        let fd = handle_to_fd(handle);
        if off > i64::MAX as u64 {
            return u64::MAX;
        }
        let rc = libc::lseek(fd, off as off_t, whence);
        if rc < 0 {
            u64::MAX
        } else {
            rc as Toff
        }
    }
}

unsafe extern "C" fn unix_close_proc(handle: Thandle) -> c_int {
    unsafe { libc::close(handle_to_fd(handle)) }
}

unsafe extern "C" fn unix_size_proc(handle: Thandle) -> Toff {
    unsafe {
        let fd = handle_to_fd(handle);
        let mut stat: libc::stat = mem::zeroed();
        if libc::fstat(fd, &mut stat) != 0 {
            0
        } else {
            stat.st_size as Toff
        }
    }
}

unsafe extern "C" fn unix_map_proc(
    handle: Thandle,
    base: *mut *mut c_void,
    size: *mut Toff,
) -> c_int {
    unsafe {
        let total = unix_size_proc(handle);
        if total == 0 || total > isize::MAX as u64 {
            return 0;
        }
        let fd = handle_to_fd(handle);
        let mapped = libc::mmap(
            ptr::null_mut(),
            total as size_t,
            libc::PROT_READ,
            libc::MAP_SHARED,
            fd,
            0,
        );
        if mapped == libc::MAP_FAILED {
            return 0;
        }
        *base = mapped;
        *size = total;
        1
    }
}

unsafe extern "C" fn unix_unmap_proc(_: Thandle, base: *mut c_void, size: Toff) {
    unsafe {
        if !base.is_null() && size > 0 {
            let _ = libc::munmap(base, size as size_t);
        }
    }
}

fn file_big_endian_from_swab(swab: bool) -> bool {
    host_is_big_endian() ^ swab
}

fn set_default_methods(tif: *mut TIFF) {
    set_default_codec_methods(tif);
}

fn tif_inner(tif: *mut TIFF) -> *mut TiffHandleInner {
    unsafe { (*tif).inner }
}

fn destroy_handle_allocation(tif: *mut TIFF) {
    unsafe {
        if tif.is_null() {
            return;
        }
        let inner = tif_inner(tif);
        if !inner.is_null() {
            let owned_name = (*inner).owned_name;
            ptr::drop_in_place(inner);
            if !owned_name.is_null() {
                _TIFFfree(owned_name.cast::<c_void>());
            }
            _TIFFfree(inner.cast::<c_void>());
        }
        _TIFFfree(tif.cast::<c_void>());
    }
}

fn emit_early_tiff_structure_oom(
    opts: *mut TIFFOpenOptions,
    clientdata: Thandle,
    name_ptr: *const c_char,
) {
    unsafe {
        static OOM_DETAIL: &[u8] = b"Out of memory (TIFF structure)\0";
        static NAMELESS: &[u8] = b"<null>\0";
        static FORMAT: &[u8] = b"%s: %s\0";
        let mut message = [0 as c_char; 512];
        let file_name = if name_ptr.is_null() {
            NAMELESS.as_ptr().cast()
        } else {
            name_ptr
        };
        libc::snprintf(
            message.as_mut_ptr(),
            message.len(),
            FORMAT.as_ptr().cast(),
            file_name,
            OOM_DETAIL.as_ptr().cast::<c_char>(),
        );
        safe_tiff_emit_early_error_message(
            opts,
            clientdata,
            c_module(MODULE_TIFF_CLIENT_OPEN_EXT),
            message.as_ptr(),
        );
    }
}

fn read_from_proc(tif: *mut TIFF, buf: *mut c_void, size: Tmsize) -> bool {
    unsafe {
        match (*tif).tif_readproc {
            Some(proc_) => proc_((*tif).tif_clientdata, buf, size) == size,
            None => false,
        }
    }
}

fn write_to_proc(tif: *mut TIFF, buf: *mut c_void, size: Tmsize) -> bool {
    unsafe {
        match (*tif).tif_writeproc {
            Some(proc_) => proc_((*tif).tif_clientdata, buf, size) == size,
            None => false,
        }
    }
}

fn seek_in_proc(tif: *mut TIFF, off: Toff, whence: c_int) -> Toff {
    unsafe {
        match (*tif).tif_seekproc {
            Some(proc_) => proc_((*tif).tif_clientdata, off, whence),
            None => u64::MAX,
        }
    }
}

fn map_contents(tif: *mut TIFF) {
    unsafe {
        if (*tif).tif_flags & TIFF_MAPPED == 0 {
            return;
        }
        let inner = tif_inner(tif);
        let mut base: *mut c_void = ptr::null_mut();
        let mut size: Toff = 0;
        match (*tif).tif_mapproc {
            Some(proc_) if proc_((*tif).tif_clientdata, &mut base, &mut size) != 0 => {
                (*inner).mapped_base = base;
                (*inner).mapped_size = size;
            }
            _ => {
                (*tif).tif_flags &= !TIFF_MAPPED;
            }
        }
    }
}

fn default_directory(tif: *mut TIFF) -> bool {
    crate::core::reset_default_directory(tif)
}

fn parse_open_mode(
    opts: *mut TIFFOpenOptions,
    clientdata: Thandle,
    mode_bytes: &[u8],
    module: &'static [u8],
) -> Option<c_int> {
    if mode_bytes.is_empty() {
        emit_early_error_message(opts, clientdata, module, "\"\": Bad mode");
        return None;
    }
    match mode_bytes[0] {
        b'r' => {
            if mode_bytes.get(1) == Some(&b'+') {
                Some(libc::O_RDWR)
            } else {
                Some(libc::O_RDONLY)
            }
        }
        b'w' => Some(libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC),
        b'a' => Some(libc::O_RDWR | libc::O_CREAT),
        _ => {
            emit_early_error_message(
                opts,
                clientdata,
                module,
                format!("\"{}\": Bad mode", String::from_utf8_lossy(mode_bytes)),
            );
            None
        }
    }
}

fn apply_mode_modifiers(tif: *mut TIFF, mode_bytes: &[u8], module_name: &str, open_flags: c_int) {
    unsafe {
        for &byte in mode_bytes {
            match byte {
                b'b' => {
                    if open_flags & libc::O_CREAT != 0 && !host_is_big_endian() {
                        (*tif).tif_flags |= TIFF_SWAB;
                    }
                }
                b'l' => {
                    if open_flags & libc::O_CREAT != 0 && host_is_big_endian() {
                        (*tif).tif_flags |= TIFF_SWAB;
                    }
                }
                b'B' => {
                    (*tif).tif_flags = ((*tif).tif_flags & !TIFF_FILLORDER) | FILLORDER_MSB2LSB;
                }
                b'L' => {
                    (*tif).tif_flags = ((*tif).tif_flags & !TIFF_FILLORDER) | FILLORDER_LSB2MSB;
                }
                b'H' => {
                    emit_warning_message(
                    tif,
                    module_name,
                    "H(ost) mode is deprecated. Since libtiff 4.5.1, it is an alias of 'B' / FILLORDER_MSB2LSB.",
                );
                    (*tif).tif_flags = ((*tif).tif_flags & !TIFF_FILLORDER) | FILLORDER_MSB2LSB;
                }
                b'M' => {
                    if open_flags == libc::O_RDONLY {
                        (*tif).tif_flags |= TIFF_MAPPED;
                    }
                }
                b'm' => {
                    if open_flags == libc::O_RDONLY {
                        (*tif).tif_flags &= !TIFF_MAPPED;
                    }
                }
                b'C' => {
                    if open_flags == libc::O_RDONLY {
                        (*tif).tif_flags |= TIFF_STRIPCHOP;
                    }
                }
                b'c' => {
                    if open_flags == libc::O_RDONLY {
                        (*tif).tif_flags &= !TIFF_STRIPCHOP;
                    }
                }
                b'h' => {
                    (*tif).tif_flags |= TIFF_HEADERONLY;
                }
                b'8' => {
                    if open_flags & libc::O_CREAT != 0 {
                        (*tif).tif_flags |= TIFF_BIGTIFF;
                    }
                }
                b'4' => {
                    if open_flags & libc::O_CREAT != 0 {
                        (*tif).tif_flags &= !TIFF_BIGTIFF;
                    }
                }
                b'D' => {
                    (*tif).tif_flags |= TIFF_DEFERSTRILELOAD;
                }
                b'O' => {
                    if open_flags == libc::O_RDONLY {
                        (*tif).tif_flags |= TIFF_DEFERSTRILELOAD | TIFF_LAZYSTRILELOAD;
                    }
                }
                _ => {}
            }
        }
    }
}

fn initialize_created_header(tif: *mut TIFF) {
    unsafe {
        let inner = tif_inner(tif);
        let file_big_endian = file_big_endian_from_swab((*tif).tif_flags & TIFF_SWAB != 0);
        (*inner).header_magic = if file_big_endian {
            TIFF_BIGENDIAN
        } else {
            TIFF_LITTLEENDIAN
        };
        (*inner).header_version = if (*tif).tif_flags & TIFF_BIGTIFF != 0 {
            TIFF_VERSION_BIG
        } else {
            TIFF_VERSION_CLASSIC
        };
        (*inner).header_size = if (*inner).header_version == TIFF_VERSION_BIG {
            16
        } else {
            8
        };
        (*inner).current_diroff = 0;
        (*inner).next_diroff = 0;
    }
}

fn write_created_header(tif: *mut TIFF, module_name: &str) -> bool {
    unsafe {
        let inner = tif_inner(tif);
        if seek_in_proc(tif, 0, libc::SEEK_SET) != 0 {
            emit_error_message(tif, module_name, "Error writing TIFF header");
            return false;
        }

        if (*inner).header_version == TIFF_VERSION_CLASSIC {
            let mut header = [0u8; 8];
            let file_big_endian = (*inner).header_magic == TIFF_BIGENDIAN;
            header[0] = if file_big_endian { b'M' } else { b'I' };
            header[1] = header[0];
            let version = if file_big_endian {
                TIFF_VERSION_CLASSIC.to_be_bytes()
            } else {
                TIFF_VERSION_CLASSIC.to_le_bytes()
            };
            let diroff = if file_big_endian {
                0u32.to_be_bytes()
            } else {
                0u32.to_le_bytes()
            };
            header[2..4].copy_from_slice(&version);
            header[4..8].copy_from_slice(&diroff);
            write_to_proc(
                tif,
                header.as_mut_ptr().cast::<c_void>(),
                header.len() as Tmsize,
            )
        } else {
            let mut header = [0u8; 16];
            let file_big_endian = (*inner).header_magic == TIFF_BIGENDIAN;
            header[0] = if file_big_endian { b'M' } else { b'I' };
            header[1] = header[0];
            let version = if file_big_endian {
                TIFF_VERSION_BIG.to_be_bytes()
            } else {
                TIFF_VERSION_BIG.to_le_bytes()
            };
            let offsetsize = if file_big_endian {
                8u16.to_be_bytes()
            } else {
                8u16.to_le_bytes()
            };
            let unused = if file_big_endian {
                0u16.to_be_bytes()
            } else {
                0u16.to_le_bytes()
            };
            let diroff = if file_big_endian {
                0u64.to_be_bytes()
            } else {
                0u64.to_le_bytes()
            };
            header[2..4].copy_from_slice(&version);
            header[4..6].copy_from_slice(&offsetsize);
            header[6..8].copy_from_slice(&unused);
            header[8..16].copy_from_slice(&diroff);
            write_to_proc(
                tif,
                header.as_mut_ptr().cast::<c_void>(),
                header.len() as Tmsize,
            )
        }
    }
}

fn parse_u16(bytes: &[u8], big_endian: bool) -> u16 {
    if big_endian {
        u16::from_be_bytes([bytes[0], bytes[1]])
    } else {
        u16::from_le_bytes([bytes[0], bytes[1]])
    }
}

fn parse_u32(bytes: &[u8], big_endian: bool) -> u32 {
    if big_endian {
        u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
    } else {
        u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
    }
}

fn parse_u64(bytes: &[u8], big_endian: bool) -> u64 {
    if big_endian {
        u64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ])
    } else {
        u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ])
    }
}

enum HeaderReadResult {
    Valid,
    NeedCreate,
    Fatal,
}

fn read_existing_header(
    tif: *mut TIFF,
    module_name: &str,
    report_short_read_error: bool,
) -> HeaderReadResult {
    unsafe {
        let inner = tif_inner(tif);
        let mut header = [0u8; 8];
        if !read_from_proc(
            tif,
            header.as_mut_ptr().cast::<c_void>(),
            header.len() as Tmsize,
        ) {
            if report_short_read_error {
                emit_error_message(tif, module_name, "Cannot read TIFF header");
            }
            return HeaderReadResult::NeedCreate;
        }

        let file_big_endian = match (&header[0], &header[1]) {
            (b'M', b'M') => true,
            (b'I', b'I') => false,
            _ => {
                let bad_magic = u16::from_ne_bytes([header[0], header[1]]);
                emit_error_message(
                    tif,
                    module_name,
                    format!(
                        "Not a TIFF file, bad magic number {} (0x{:x})",
                        bad_magic, bad_magic
                    ),
                );
                return HeaderReadResult::Fatal;
            }
        };

        let version = parse_u16(&header[2..4], file_big_endian);
        (*inner).header_magic = if file_big_endian {
            TIFF_BIGENDIAN
        } else {
            TIFF_LITTLEENDIAN
        };
        (*tif).tif_flags &= !TIFF_SWAB;
        if host_is_big_endian() != file_big_endian {
            (*tif).tif_flags |= TIFF_SWAB;
        }

        match version {
            TIFF_VERSION_CLASSIC => {
                (*inner).header_version = TIFF_VERSION_CLASSIC;
                (*inner).header_size = 8;
                (*tif).tif_flags &= !TIFF_BIGTIFF;
                (*inner).next_diroff = parse_u32(&header[4..8], file_big_endian) as u64;
            }
            TIFF_VERSION_BIG => {
                let mut extra = [0u8; 8];
                if !read_from_proc(
                    tif,
                    extra.as_mut_ptr().cast::<c_void>(),
                    extra.len() as Tmsize,
                ) {
                    emit_error_message(tif, module_name, "Cannot read TIFF header");
                    return HeaderReadResult::Fatal;
                }
                let offsetsize = parse_u16(&header[4..6], file_big_endian);
                let unused = parse_u16(&header[6..8], file_big_endian);
                if offsetsize != 8 {
                    emit_error_message(
                        tif,
                        module_name,
                        format!(
                            "Not a TIFF file, bad BigTIFF offsetsize {} (0x{:x})",
                            offsetsize, offsetsize
                        ),
                    );
                    return HeaderReadResult::Fatal;
                }
                if unused != 0 {
                    emit_error_message(
                        tif,
                        module_name,
                        format!(
                            "Not a TIFF file, bad BigTIFF unused {} (0x{:x})",
                            unused, unused
                        ),
                    );
                    return HeaderReadResult::Fatal;
                }
                (*inner).header_version = TIFF_VERSION_BIG;
                (*inner).header_size = 16;
                (*tif).tif_flags |= TIFF_BIGTIFF;
                (*inner).next_diroff = parse_u64(&extra[0..8], file_big_endian);
            }
            _ => {
                emit_error_message(
                    tif,
                    module_name,
                    format!(
                        "Not a TIFF file, bad version number {} (0x{:x})",
                        version, version
                    ),
                );
                return HeaderReadResult::Fatal;
            }
        }

        (*tif).tif_flags |= TIFF_MYBUFFER;
        (*tif).tif_rawdata = ptr::null_mut();
        (*tif).tif_rawdatasize = 0;
        (*tif).tif_rawcp = ptr::null_mut();
        (*tif).tif_rawcc = 0;
        HeaderReadResult::Valid
    }
}

fn read_directory_internal(tif: *mut TIFF) -> bool {
    read_next_directory(tif)
}

fn finalize_open(tif: *mut TIFF, mode_bytes: &[u8], open_flags: c_int) -> bool {
    unsafe {
        let inner = tif_inner(tif);
        let module_name = c_name((*tif).tif_name);

        if (open_flags & libc::O_TRUNC) != 0 {
            initialize_created_header(tif);
            if !write_created_header(tif, &module_name) {
                return false;
            }
            return default_directory(tif);
        }

        let saved_position = seek_in_proc(tif, 0, libc::SEEK_SET);
        if saved_position != 0 {
            let _ = saved_position;
        }

        match read_existing_header(tif, &module_name, (*inner).tif_mode == libc::O_RDONLY) {
            HeaderReadResult::Valid => {}
            HeaderReadResult::NeedCreate => {
                if (*inner).tif_mode == libc::O_RDONLY {
                    return false;
                }
                initialize_created_header(tif);
                if !write_created_header(tif, &module_name) {
                    return false;
                }
                return default_directory(tif);
            }
            HeaderReadResult::Fatal => return false,
        }

        match mode_bytes[0] {
            b'r' => {
                map_contents(tif);
                if (*tif).tif_flags & TIFF_HEADERONLY != 0 {
                    true
                } else {
                    read_directory_internal(tif)
                }
            }
            b'a' => default_directory(tif),
            _ => true,
        }
    }
}

fn fail_open(tif: *mut TIFF) {
    unsafe {
        if tif.is_null() {
            return;
        }
        (*tif_inner(tif)).tif_mode = libc::O_RDONLY;
        TIFFCleanup(tif);
    }
}

fn make_handle(
    name_ptr: *const c_char,
    mode_ptr: *const c_char,
    clientdata: Thandle,
    readproc: TIFFReadWriteProc,
    writeproc: TIFFReadWriteProc,
    seekproc: TIFFSeekProc,
    closeproc: TIFFCloseProc,
    sizeproc: TIFFSizeProc,
    mapproc: TIFFMapFileProc,
    unmapproc: TIFFUnmapFileProc,
    opts: *mut TIFFOpenOptions,
) -> *mut TIFF {
    unsafe {
        let module = MODULE_TIFF_CLIENT_OPEN_EXT;
        let name = c_name(name_ptr);
        let name_bytes = if name_ptr.is_null() {
            b"<null>".as_slice()
        } else {
            CStr::from_ptr(name_ptr).to_bytes()
        };
        let mode = if mode_ptr.is_null() {
            Vec::new()
        } else {
            CStr::from_ptr(mode_ptr).to_bytes().to_vec()
        };

        let open_flags = match parse_open_mode(opts, clientdata, &mode, module) {
            Some(flags) => flags,
            None => return ptr::null_mut(),
        };

        let Some(alloc_size_usize) = mem::size_of::<TIFF>()
            .checked_add(mem::size_of::<TiffHandleInner>())
            .and_then(|value| value.checked_add(name_bytes.len() + 1))
        else {
            emit_early_tiff_structure_oom(opts, clientdata, name_ptr);
            return ptr::null_mut();
        };
        let alloc_size = alloc_size_usize as Tmsize;
        if !opts.is_null()
            && (*opts).max_single_mem_alloc > 0
            && alloc_size > (*opts).max_single_mem_alloc
        {
            emit_early_error_message(
            opts,
            clientdata,
            module,
            format!(
                "{}: Memory allocation of {} bytes is beyond the {} byte limit defined in open options",
                name, alloc_size, (*opts).max_single_mem_alloc
            ),
        );
            return ptr::null_mut();
        }

        let tif = _TIFFcalloc(1, mem::size_of::<TIFF>() as Tmsize).cast::<TIFF>();
        if tif.is_null() {
            emit_early_tiff_structure_oom(opts, clientdata, name_ptr);
            return ptr::null_mut();
        }
        let inner =
            _TIFFcalloc(1, mem::size_of::<TiffHandleInner>() as Tmsize).cast::<TiffHandleInner>();
        if inner.is_null() {
            _TIFFfree(tif.cast::<c_void>());
            emit_early_tiff_structure_oom(opts, clientdata, name_ptr);
            return ptr::null_mut();
        }
        let name_owner = _TIFFcalloc(1, (name_bytes.len() + 1) as Tmsize).cast::<c_char>();
        if name_owner.is_null() {
            _TIFFfree(inner.cast::<c_void>());
            _TIFFfree(tif.cast::<c_void>());
            emit_early_tiff_structure_oom(opts, clientdata, name_ptr);
            return ptr::null_mut();
        }
        ptr::copy_nonoverlapping(
            name_bytes.as_ptr(),
            name_owner.cast::<u8>(),
            name_bytes.len(),
        );
        *name_owner.add(name_bytes.len()) = 0;

        ptr::write(
            inner,
            TiffHandleInner {
                tif_fd: -1,
                tif_mode: open_flags & !(libc::O_CREAT | libc::O_TRUNC),
                tif_curstrip: u32::MAX,
                tif_curtile: u32::MAX,
                tif_max_single_mem_alloc: if opts.is_null() {
                    0
                } else {
                    (*opts).max_single_mem_alloc
                },
                mapped_base: ptr::null_mut(),
                mapped_size: 0,
                header_magic: 0,
                header_version: 0,
                header_size: 0,
                _reserved0: 0,
                current_diroff: 0,
                next_diroff: 0,
                owned_name: name_owner,
                directory_state: DirectoryState::default(),
                field_registry: FieldRegistryState::default(),
                codec_state: CodecState::default(),
                strile_state: StrileState::default(),
            },
        );

        ptr::write(
            tif,
            TIFF {
                tif_name: name_owner,
                tif_flags: FILLORDER_MSB2LSB,
                tif_row: u32::MAX,
                tif_curdir: TIFF_NON_EXISTENT_DIR_NUMBER,
                tif_rawdata: ptr::null_mut(),
                tif_rawdatasize: 0,
                tif_rawcp: ptr::null_mut(),
                tif_rawcc: 0,
                tif_clientdata: clientdata,
                tif_readproc: readproc,
                tif_writeproc: writeproc,
                tif_seekproc: seekproc,
                tif_closeproc: closeproc,
                tif_sizeproc: sizeproc,
                tif_mapproc: mapproc.or(Some(dummy_map_proc)),
                tif_unmapproc: unmapproc.or(Some(dummy_unmap_proc)),
                tif_setupdecode: Some(stub_bool_method),
                tif_predecode: Some(stub_predecode_method),
                tif_decoderow: Some(stub_decoderow_method),
                tif_close: Some(stub_void_method),
                tif_cleanup: Some(stub_void_method),
                tif_errorhandler: if opts.is_null() {
                    ptr::null_mut()
                } else {
                    (*opts).errorhandler
                },
                tif_errorhandler_user_data: if opts.is_null() {
                    ptr::null_mut()
                } else {
                    (*opts).errorhandler_user_data
                },
                tif_warnhandler: if opts.is_null() {
                    ptr::null_mut()
                } else {
                    (*opts).warnhandler
                },
                tif_warnhandler_user_data: if opts.is_null() {
                    ptr::null_mut()
                } else {
                    (*opts).warnhandler_user_data
                },
                inner,
            },
        );

        if readproc.is_none()
            || writeproc.is_none()
            || seekproc.is_none()
            || closeproc.is_none()
            || sizeproc.is_none()
        {
            emit_error_message(
                tif,
                "TIFFClientOpenExt",
                "One of the client procedures is NULL pointer.",
            );
            destroy_handle_allocation(tif);
            return ptr::null_mut();
        }

        set_default_methods(tif);
        if open_flags == libc::O_RDONLY {
            (*tif).tif_flags |= TIFF_MAPPED;
        }
        if open_flags == libc::O_RDONLY || open_flags == libc::O_RDWR {
            (*tif).tif_flags |= TIFF_STRIPCHOP;
        }

        apply_mode_modifiers(tif, &mode, &name, open_flags);
        if !crate::core::initialize_field_registry(tif) {
            destroy_handle_allocation(tif);
            return ptr::null_mut();
        }

        if finalize_open(tif, &mode, open_flags) {
            tif
        } else {
            fail_open(tif);
            ptr::null_mut()
        }
    }
}

fn limit_allocation_message(_function_name: &str, size: Tmsize, limit: Tmsize) -> String {
    format!(
        "Memory allocation of {} bytes is beyond the {} byte limit defined in open options",
        size, limit
    )
}

fn emit_limit_error(tif: *mut TIFF, function_name: &str, size: Tmsize) {
    unsafe {
        if tif.is_null() {
            return;
        }
        let limit = (*tif_inner(tif)).tif_max_single_mem_alloc;
        if limit > 0 {
            emit_error_message(
                tif,
                function_name,
                limit_allocation_message(function_name, size, limit),
            );
        }
    }
}

fn check_mul_tmsize(first: Tmsize, second: Tmsize) -> Option<Tmsize> {
    if first <= 0 || second <= 0 {
        None
    } else if first > TIFF_TMSIZE_T_MAX / second {
        None
    } else {
        Some(first * second)
    }
}

#[no_mangle]
pub extern "C" fn TIFFGetVersion() -> *const c_char {
    VERSION_STRING.as_ptr().cast()
}

#[no_mangle]
pub extern "C" fn TIFFOpenOptionsAlloc() -> *mut TIFFOpenOptions {
    unsafe { _TIFFcalloc(1, mem::size_of::<TIFFOpenOptions>() as Tmsize).cast::<TIFFOpenOptions>() }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFOpenOptionsFree(opts: *mut TIFFOpenOptions) {
    unsafe {
        if !opts.is_null() {
            _TIFFfree(opts.cast::<c_void>());
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFOpenOptionsSetMaxSingleMemAlloc(
    opts: *mut TIFFOpenOptions,
    max_single_mem_alloc: Tmsize,
) {
    unsafe {
        if !opts.is_null() {
            (*opts).max_single_mem_alloc = max_single_mem_alloc;
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFOpenOptionsSetErrorHandlerExtR(
    opts: *mut TIFFOpenOptions,
    handler: TIFFErrorHandlerExtRRaw,
    user_data: *mut c_void,
) {
    unsafe {
        if !opts.is_null() {
            (*opts).errorhandler = handler;
            (*opts).errorhandler_user_data = user_data;
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFOpenOptionsSetWarningHandlerExtR(
    opts: *mut TIFFOpenOptions,
    handler: TIFFErrorHandlerExtRRaw,
    user_data: *mut c_void,
) {
    unsafe {
        if !opts.is_null() {
            (*opts).warnhandler = handler;
            (*opts).warnhandler_user_data = user_data;
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn _TIFFmalloc(s: Tmsize) -> *mut c_void {
    unsafe {
        if s == 0 {
            ptr::null_mut()
        } else {
            libc::malloc(s as size_t)
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn _TIFFcalloc(nmemb: Tmsize, siz: Tmsize) -> *mut c_void {
    unsafe {
        if nmemb == 0 || siz == 0 {
            return ptr::null_mut();
        }
        if nmemb > 0 && siz > 0 && nmemb > TIFF_TMSIZE_T_MAX / siz {
            return ptr::null_mut();
        }
        libc::calloc(nmemb as size_t, siz as size_t)
    }
}

#[no_mangle]
pub unsafe extern "C" fn _TIFFrealloc(p: *mut c_void, s: Tmsize) -> *mut c_void {
    unsafe { libc::realloc(p, s as size_t) }
}

#[no_mangle]
pub unsafe extern "C" fn _TIFFmemset(p: *mut c_void, v: c_int, c: Tmsize) {
    unsafe {
        libc::memset(p, v, c as size_t);
    }
}

#[no_mangle]
pub unsafe extern "C" fn _TIFFmemcpy(d: *mut c_void, s: *const c_void, c: Tmsize) {
    unsafe {
        libc::memcpy(d, s, c as size_t);
    }
}

#[no_mangle]
pub unsafe extern "C" fn _TIFFmemcmp(p1: *const c_void, p2: *const c_void, c: Tmsize) -> c_int {
    unsafe { libc::memcmp(p1, p2, c as size_t) }
}

#[no_mangle]
pub unsafe extern "C" fn _TIFFfree(p: *mut c_void) {
    unsafe {
        libc::free(p);
    }
}

#[no_mangle]
pub unsafe extern "C" fn _TIFFmallocExt(tif: *mut TIFF, s: Tmsize) -> *mut c_void {
    unsafe {
        if !tif.is_null()
            && (*tif_inner(tif)).tif_max_single_mem_alloc > 0
            && s > (*tif_inner(tif)).tif_max_single_mem_alloc
        {
            emit_limit_error(tif, "_TIFFmallocExt", s);
            ptr::null_mut()
        } else {
            _TIFFmalloc(s)
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn _TIFFcallocExt(tif: *mut TIFF, nmemb: Tmsize, siz: Tmsize) -> *mut c_void {
    unsafe {
        if !tif.is_null() && (*tif_inner(tif)).tif_max_single_mem_alloc > 0 {
            let Some(total) = check_mul_tmsize(nmemb, siz) else {
                return ptr::null_mut();
            };
            if total > (*tif_inner(tif)).tif_max_single_mem_alloc {
                emit_limit_error(tif, "_TIFFcallocExt", total);
                return ptr::null_mut();
            }
        }
        _TIFFcalloc(nmemb, siz)
    }
}

#[no_mangle]
pub unsafe extern "C" fn _TIFFreallocExt(tif: *mut TIFF, p: *mut c_void, s: Tmsize) -> *mut c_void {
    unsafe {
        if !tif.is_null()
            && (*tif_inner(tif)).tif_max_single_mem_alloc > 0
            && s > (*tif_inner(tif)).tif_max_single_mem_alloc
        {
            emit_limit_error(tif, "_TIFFreallocExt", s);
            ptr::null_mut()
        } else {
            _TIFFrealloc(p, s)
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn _TIFFfreeExt(_: *mut TIFF, p: *mut c_void) {
    unsafe {
        _TIFFfree(p);
    }
}

#[no_mangle]
pub unsafe extern "C" fn _TIFFMultiply32(
    tif: *mut TIFF,
    first: u32,
    second: u32,
    where_ptr: *const c_char,
) -> u32 {
    if second != 0 && first > u32::MAX / second {
        let where_name = c_name(where_ptr);
        emit_error_message(
            tif,
            &where_name,
            format!("Integer overflow in {}", where_name),
        );
        0
    } else {
        first.wrapping_mul(second)
    }
}

#[no_mangle]
pub unsafe extern "C" fn _TIFFMultiply64(
    tif: *mut TIFF,
    first: u64,
    second: u64,
    where_ptr: *const c_char,
) -> u64 {
    if second != 0 && first > u64::MAX / second {
        let where_name = c_name(where_ptr);
        emit_error_message(
            tif,
            &where_name,
            format!("Integer overflow in {}", where_name),
        );
        0
    } else {
        first.wrapping_mul(second)
    }
}

#[no_mangle]
pub unsafe extern "C" fn _TIFFCheckRealloc(
    tif: *mut TIFF,
    buffer: *mut c_void,
    nmemb: Tmsize,
    elem_size: Tmsize,
    what: *const c_char,
) -> *mut c_void {
    unsafe {
        let what_name = c_name(what);
        let Some(total) = check_mul_tmsize(nmemb, elem_size) else {
            let module_name = if tif.is_null() {
                "TIFFCheckRealloc"
            } else {
                &c_name((*tif).tif_name)
            };
            emit_error_message(
                tif,
                module_name,
                format!(
                    "Failed to allocate memory for {} ({} elements of {} bytes each)",
                    what_name, nmemb, elem_size
                ),
            );
            return ptr::null_mut();
        };

        let result = _TIFFreallocExt(tif, buffer, total);
        if result.is_null() {
            let module_name = if tif.is_null() {
                "TIFFCheckRealloc"
            } else {
                &c_name((*tif).tif_name)
            };
            emit_error_message(
                tif,
                module_name,
                format!(
                    "Failed to allocate memory for {} ({} elements of {} bytes each)",
                    what_name, nmemb, elem_size
                ),
            );
        }
        result
    }
}

#[no_mangle]
pub unsafe extern "C" fn _TIFFCheckMalloc(
    tif: *mut TIFF,
    nmemb: Tmsize,
    elem_size: Tmsize,
    what: *const c_char,
) -> *mut c_void {
    unsafe { _TIFFCheckRealloc(tif, ptr::null_mut(), nmemb, elem_size, what) }
}

#[no_mangle]
pub extern "C" fn _TIFFClampDoubleToUInt32(val: f64) -> u32 {
    if val < 0.0 {
        0
    } else if val.is_nan() || val > u32::MAX as f64 {
        u32::MAX
    } else {
        val as u32
    }
}

#[no_mangle]
pub unsafe extern "C" fn _TIFFSeekOK(tif: *mut TIFF, off: Toff) -> c_int {
    if tif.is_null() {
        return 0;
    }
    if off > u64::MAX / 2 {
        return 0;
    }
    (seek_in_proc(tif, off, libc::SEEK_SET) == off) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn TIFFClientOpen(
    name: *const c_char,
    mode: *const c_char,
    clientdata: Thandle,
    readproc: TIFFReadWriteProc,
    writeproc: TIFFReadWriteProc,
    seekproc: TIFFSeekProc,
    closeproc: TIFFCloseProc,
    sizeproc: TIFFSizeProc,
    mapproc: TIFFMapFileProc,
    unmapproc: TIFFUnmapFileProc,
) -> *mut TIFF {
    unsafe {
        TIFFClientOpenExt(
            name,
            mode,
            clientdata,
            readproc,
            writeproc,
            seekproc,
            closeproc,
            sizeproc,
            mapproc,
            unmapproc,
            ptr::null_mut(),
        )
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFClientOpenExt(
    name: *const c_char,
    mode: *const c_char,
    clientdata: Thandle,
    readproc: TIFFReadWriteProc,
    writeproc: TIFFReadWriteProc,
    seekproc: TIFFSeekProc,
    closeproc: TIFFCloseProc,
    sizeproc: TIFFSizeProc,
    mapproc: TIFFMapFileProc,
    unmapproc: TIFFUnmapFileProc,
    opts: *mut TIFFOpenOptions,
) -> *mut TIFF {
    make_handle(
        name, mode, clientdata, readproc, writeproc, seekproc, closeproc, sizeproc, mapproc,
        unmapproc, opts,
    )
}

#[no_mangle]
pub unsafe extern "C" fn TIFFFdOpen(
    fd: c_int,
    name: *const c_char,
    mode: *const c_char,
) -> *mut TIFF {
    unsafe { TIFFFdOpenExt(fd, name, mode, ptr::null_mut()) }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFFdOpenExt(
    fd: c_int,
    name: *const c_char,
    mode: *const c_char,
    opts: *mut TIFFOpenOptions,
) -> *mut TIFF {
    unsafe {
        let tif = TIFFClientOpenExt(
            name,
            mode,
            fd_to_handle(fd),
            Some(unix_read_proc),
            Some(unix_write_proc),
            Some(unix_seek_proc),
            Some(unix_close_proc),
            Some(unix_size_proc),
            Some(unix_map_proc),
            Some(unix_unmap_proc),
            opts,
        );
        if !tif.is_null() {
            (*tif_inner(tif)).tif_fd = fd;
        }
        tif
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFOpen(name: *const c_char, mode: *const c_char) -> *mut TIFF {
    unsafe { TIFFOpenExt(name, mode, ptr::null_mut()) }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFOpenExt(
    name: *const c_char,
    mode: *const c_char,
    opts: *mut TIFFOpenOptions,
) -> *mut TIFF {
    unsafe {
        if name.is_null() || mode.is_null() {
            return ptr::null_mut();
        }
        let module = MODULE_TIFF_OPEN;
        let mode_bytes = CStr::from_ptr(mode).to_bytes();
        let open_flags = match parse_open_mode(opts, ptr::null_mut(), mode_bytes, module) {
            Some(flags) => flags,
            None => return ptr::null_mut(),
        };

        let fd = libc::open(CStr::from_ptr(name).as_ptr(), open_flags, 0o666);
        if fd < 0 {
            let message = match io::Error::last_os_error().raw_os_error() {
                Some(_) => format!("{}: {}", c_name(name), io::Error::last_os_error()),
                None => format!("{}: Cannot open", c_name(name)),
            };
            emit_early_error_message(opts, ptr::null_mut(), module, message);
            return ptr::null_mut();
        }

        let tif = TIFFFdOpenExt(fd, name, mode, opts);
        if tif.is_null() {
            let _ = libc::close(fd);
        }
        tif
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFCleanup(tif: *mut TIFF) {
    unsafe {
        if tif.is_null() {
            return;
        }
        let inner = tif_inner(tif);

        if (*inner).tif_mode != libc::O_RDONLY {
            let _ = crate::strile::TIFFFlush(tif);
        }

        if let Some(cleanup) = (*tif).tif_cleanup {
            cleanup(tif);
        }

        if !(*tif).tif_rawdata.is_null() && ((*tif).tif_flags & TIFF_MYBUFFER) != 0 {
            _TIFFfree((*tif).tif_rawdata.cast::<c_void>());
            (*tif).tif_rawdata = ptr::null_mut();
        }

        if ((*tif).tif_flags & TIFF_MAPPED) != 0 && !(*inner).mapped_base.is_null() {
            if let Some(unmap) = (*tif).tif_unmapproc {
                unmap(
                    (*tif).tif_clientdata,
                    (*inner).mapped_base,
                    (*inner).mapped_size,
                );
            }
            (*inner).mapped_base = ptr::null_mut();
            (*inner).mapped_size = 0;
        }

        destroy_handle_allocation(tif);
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFClose(tif: *mut TIFF) {
    unsafe {
        if tif.is_null() {
            return;
        }
        let closeproc = (*tif).tif_closeproc;
        let clientdata = (*tif).tif_clientdata;
        TIFFCleanup(tif);
        if let Some(closeproc) = closeproc {
            let _ = closeproc(clientdata);
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFReadDirectory(tif: *mut TIFF) -> c_int {
    if tif.is_null() {
        0
    } else {
        read_directory_internal(tif) as c_int
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFFileName(tif: *mut TIFF) -> *const c_char {
    unsafe { (*tif).tif_name }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFSetFileName(tif: *mut TIFF, name: *const c_char) -> *const c_char {
    unsafe {
        let old_name = (*tif).tif_name;
        (*tif).tif_name = name.cast_mut();
        old_name
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFFileno(tif: *mut TIFF) -> c_int {
    unsafe { (*tif_inner(tif)).tif_fd }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFSetFileno(tif: *mut TIFF, fd: c_int) -> c_int {
    unsafe {
        let inner = tif_inner(tif);
        let old = (*inner).tif_fd;
        (*inner).tif_fd = fd;
        old
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFClientdata(tif: *mut TIFF) -> Thandle {
    unsafe { (*tif).tif_clientdata }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFSetClientdata(tif: *mut TIFF, clientdata: Thandle) -> Thandle {
    unsafe {
        let old = (*tif).tif_clientdata;
        (*tif).tif_clientdata = clientdata;
        old
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFGetMode(tif: *mut TIFF) -> c_int {
    unsafe { (*tif_inner(tif)).tif_mode }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFSetMode(tif: *mut TIFF, mode: c_int) -> c_int {
    unsafe {
        let inner = tif_inner(tif);
        let old = (*inner).tif_mode;
        (*inner).tif_mode = mode;
        old
    }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFGetReadProc(tif: *mut TIFF) -> TIFFReadWriteProc {
    unsafe { (*tif).tif_readproc }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFGetWriteProc(tif: *mut TIFF) -> TIFFReadWriteProc {
    unsafe { (*tif).tif_writeproc }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFGetSeekProc(tif: *mut TIFF) -> TIFFSeekProc {
    unsafe { (*tif).tif_seekproc }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFGetCloseProc(tif: *mut TIFF) -> TIFFCloseProc {
    unsafe { (*tif).tif_closeproc }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFGetSizeProc(tif: *mut TIFF) -> TIFFSizeProc {
    unsafe { (*tif).tif_sizeproc }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFGetMapFileProc(tif: *mut TIFF) -> TIFFMapFileProc {
    unsafe { (*tif).tif_mapproc }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFGetUnmapFileProc(tif: *mut TIFF) -> TIFFUnmapFileProc {
    unsafe { (*tif).tif_unmapproc }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFCurrentRow(tif: *mut TIFF) -> u32 {
    unsafe { (*tif).tif_row }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFCurrentDirectory(tif: *mut TIFF) -> u32 {
    unsafe { (*tif).tif_curdir }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFCurrentDirOffset(tif: *mut TIFF) -> u64 {
    unsafe { (*tif_inner(tif)).current_diroff }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFCurrentStrip(tif: *mut TIFF) -> u32 {
    unsafe { (*tif_inner(tif)).tif_curstrip }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFCurrentTile(tif: *mut TIFF) -> u32 {
    unsafe { (*tif_inner(tif)).tif_curtile }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFIsTiled(tif: *mut TIFF) -> c_int {
    unsafe { (((*tif).tif_flags & TIFF_ISTILED) != 0) as c_int }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFIsByteSwapped(tif: *mut TIFF) -> c_int {
    unsafe { (((*tif).tif_flags & TIFF_SWAB) != 0) as c_int }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFIsUpSampled(tif: *mut TIFF) -> c_int {
    unsafe { (((*tif).tif_flags & TIFF_UPSAMPLED) != 0) as c_int }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFIsMSB2LSB(tif: *mut TIFF) -> c_int {
    unsafe { (((*tif).tif_flags & FILLORDER_MSB2LSB) != 0) as c_int }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFIsBigEndian(tif: *mut TIFF) -> c_int {
    unsafe { ((*tif_inner(tif)).header_magic == TIFF_BIGENDIAN) as c_int }
}

#[no_mangle]
pub unsafe extern "C" fn TIFFIsBigTIFF(tif: *mut TIFF) -> c_int {
    unsafe { ((*tif_inner(tif)).header_version == TIFF_VERSION_BIG) as c_int }
}

#[no_mangle]
pub unsafe extern "C" fn safe_tiff_read_custom_directory(
    tif: *mut TIFF,
    diroff: u64,
    infoarray: *const TIFFFieldArray,
) -> c_int {
    read_custom_directory(tif, diroff, infoarray) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn safe_tiff_set_directory(tif: *mut TIFF, dirnum: u32) -> c_int {
    set_directory(tif, dirnum) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn safe_tiff_set_sub_directory(tif: *mut TIFF, diroff: u64) -> c_int {
    set_sub_directory(tif, diroff) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn safe_tiff_number_of_directories(tif: *mut TIFF) -> u32 {
    number_of_directories(tif)
}

#[no_mangle]
pub unsafe extern "C" fn safe_tiff_last_directory(tif: *mut TIFF) -> c_int {
    last_directory(tif) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn safe_tiff_free_directory(tif: *mut TIFF) {
    free_directory_state(tif);
}

#[no_mangle]
pub unsafe extern "C" fn safe_tiff_current_tag_count(tif: *mut TIFF) -> u32 {
    current_tag_count(tif)
}

#[no_mangle]
pub unsafe extern "C" fn safe_tiff_current_tag_at(tif: *mut TIFF, index: u32) -> u32 {
    current_tag_at(tif, index)
}

#[no_mangle]
pub unsafe extern "C" fn safe_tiff_get_tag_value(
    tif: *mut TIFF,
    tag: u32,
    defaulted: c_int,
    out_type: *mut TIFFDataType,
    out_count: *mut u64,
    out_data: *mut *const c_void,
) -> c_int {
    get_tag_value(tif, tag, defaulted != 0, out_type, out_count, out_data)
}

#[no_mangle]
pub extern "C" fn tiff_safe_core_placeholder() -> i32 {
    0
}
