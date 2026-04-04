/*
 * Build-only libtiff internal surface for copied tools and smoke tests.
 * Keep this header intentionally narrow: the installed public headers remain
 * opaque and only expose the public ABI.
 */

#ifndef _TIFFIOP_
#define _TIFFIOP_

#include "tif_config.h"

#ifdef HAVE_FCNTL_H
#include <fcntl.h>
#endif

#ifdef HAVE_SYS_TYPES_H
#include <sys/types.h>
#endif
#ifdef HAVE_SYS_STAT_H
#include <sys/stat.h>
#endif
#ifdef HAVE_UNISTD_H
#include <unistd.h>
#endif

#include <string.h>
#include <limits.h>

#include "tiffio.h"

#ifndef TRUE
#define TRUE 1
#define FALSE 0
#endif

#ifndef TIFF_MAX_DIR_COUNT
#define TIFF_MAX_DIR_COUNT 1048576
#endif

#define TIFF_NON_EXISTENT_DIR_NUMBER UINT_MAX
#define streq(a, b) (strcmp((a), (b)) == 0)
#define strneq(a, b, n) (strncmp((a), (b), (n)) == 0)

typedef unsigned char tidataval_t;
typedef tidataval_t *tidata_t;

typedef void (*TIFFVoidMethod)(TIFF *);
typedef int (*TIFFBoolMethod)(TIFF *);
typedef int (*TIFFPreMethod)(TIFF *, uint16_t);
typedef int (*TIFFCodeMethod)(TIFF *tif, uint8_t *buf, tmsize_t size,
                              uint16_t sample);

struct tiff
{
    /* Build-only facade fields. The Rust core owns the full lifecycle state
     * behind an internal pointer, so copied tools must not allocate/copy this.
     */
    char *tif_name;
    uint32_t tif_flags;
#define TIFF_FILLORDER 0x00003U
#define TIFF_DIRTYHEADER 0x00004U
#define TIFF_DIRTYDIRECT 0x00008U
#define TIFF_BUFFERSETUP 0x00010U
#define TIFF_CODERSETUP 0x00020U
#define TIFF_BEENWRITING 0x00040U
#define TIFF_SWAB 0x00080U
#define TIFF_NOBITREV 0x00100U
#define TIFF_MYBUFFER 0x00200U
#define TIFF_ISTILED 0x00400U
#define TIFF_MAPPED 0x00800U
#define TIFF_POSTENCODE 0x01000U
#define TIFF_INSUBIFD 0x02000U
#define TIFF_UPSAMPLED 0x04000U
#define TIFF_STRIPCHOP 0x08000U
#define TIFF_HEADERONLY 0x10000U
#define TIFF_NOREADRAW 0x20000U
#define TIFF_INCUSTOMIFD 0x40000U
#define TIFF_BIGTIFF 0x80000U
#define TIFF_BUF4WRITE 0x100000U
#define TIFF_DIRTYSTRIP 0x200000U
#define TIFF_PERSAMPLE 0x400000U
#define TIFF_BUFFERMMAP 0x800000U
#define TIFF_DEFERSTRILELOAD 0x1000000U
#define TIFF_LAZYSTRILELOAD 0x2000000U
#define TIFF_CHOPPEDUPARRAYS 0x4000000U
    uint32_t tif_row;
    tdir_t tif_curdir;
    uint8_t *tif_rawdata;
    tmsize_t tif_rawdatasize;
    uint8_t *tif_rawcp;
    tmsize_t tif_rawcc;
    thandle_t tif_clientdata;
    TIFFReadWriteProc tif_readproc;
    TIFFReadWriteProc tif_writeproc;
    TIFFSeekProc tif_seekproc;
    TIFFCloseProc tif_closeproc;
    TIFFSizeProc tif_sizeproc;
    TIFFMapFileProc tif_mapproc;
    TIFFUnmapFileProc tif_unmapproc;
    TIFFBoolMethod tif_setupdecode;
    TIFFPreMethod tif_predecode;
    TIFFCodeMethod tif_decoderow;
    TIFFVoidMethod tif_close;
    TIFFVoidMethod tif_cleanup;
    TIFFErrorHandlerExtR tif_errorhandler;
    void *tif_errorhandler_user_data;
    TIFFErrorHandlerExtR tif_warnhandler;
    void *tif_warnhandler_user_data;
};

struct TIFFOpenOptions
{
    TIFFErrorHandlerExtR errorhandler;
    void *errorhandler_user_data;
    TIFFErrorHandlerExtR warnhandler;
    void *warnhandler_user_data;
    tmsize_t max_single_mem_alloc;
};

#define isTiled(tif) (((tif)->tif_flags & TIFF_ISTILED) != 0)
#define isMapped(tif) (((tif)->tif_flags & TIFF_MAPPED) != 0)
#define isFillOrder(tif, o) (((tif)->tif_flags & (o)) != 0)
#define isUpSampled(tif) (((tif)->tif_flags & TIFF_UPSAMPLED) != 0)

#define TIFFReadFile(tif, buf, size)                                           \
    ((*(tif)->tif_readproc)((tif)->tif_clientdata, (buf), (size)))
#define TIFFWriteFile(tif, buf, size)                                          \
    ((*(tif)->tif_writeproc)((tif)->tif_clientdata, (buf), (size)))
#define TIFFSeekFile(tif, off, whence)                                         \
    ((*(tif)->tif_seekproc)((tif)->tif_clientdata, (off), (whence)))
#define TIFFCloseFile(tif) ((*(tif)->tif_closeproc)((tif)->tif_clientdata))
#define TIFFGetFileSize(tif) ((*(tif)->tif_sizeproc)((tif)->tif_clientdata))
#define TIFFMapFileContents(tif, paddr, psize)                                 \
    ((*(tif)->tif_mapproc)((tif)->tif_clientdata, (paddr), (psize)))
#define TIFFUnmapFileContents(tif, addr, size)                                 \
    ((*(tif)->tif_unmapproc)((tif)->tif_clientdata, (addr), (size)))

#ifndef ReadOK
#define ReadOK(tif, buf, size) (TIFFReadFile((tif), (buf), (size)) == (size))
#endif
#ifndef SeekOK
#define SeekOK(tif, off) _TIFFSeekOK((tif), (off))
#endif
#ifndef WriteOK
#define WriteOK(tif, buf, size) (TIFFWriteFile((tif), (buf), (size)) == (size))
#endif

#define TIFFhowmany_32(x, y)                                                   \
    (((uint32_t)x < (0xffffffffU - (uint32_t)(y - 1)))                         \
         ? ((((uint32_t)(x)) + (((uint32_t)(y)) - 1)) / ((uint32_t)(y)))       \
         : 0U)
#define TIFFhowmany_32_maxuint_compat(x, y)                                    \
    (((uint32_t)(x) / (uint32_t)(y)) +                                         \
     ((((uint32_t)(x) % (uint32_t)(y)) != 0) ? 1 : 0))
#define TIFFhowmany8_32(x)                                                     \
    (((x)&0x07) ? ((uint32_t)(x) >> 3) + 1 : (uint32_t)(x) >> 3)
#define TIFFroundup_32(x, y) (TIFFhowmany_32((x), (y)) * (y))
#define TIFFhowmany_64(x, y)                                                   \
    ((((uint64_t)(x)) + (((uint64_t)(y)) - 1)) / ((uint64_t)(y)))
#define TIFFhowmany8_64(x)                                                     \
    (((x)&0x07) ? ((uint64_t)(x) >> 3) + 1 : (uint64_t)(x) >> 3)
#define TIFFroundup_64(x, y) (TIFFhowmany_64((x), (y)) * (y))

#define TIFFSafeMultiply(t, v, m)                                              \
    ((((t)(m) != (t)0) && (((t)(((v) * (m)) / (m))) == (t)(v)))                \
         ? (t)((v) * (m))                                                      \
         : (t)0)

#define TIFFmax(A, B) ((A) > (B) ? (A) : (B))
#define TIFFmin(A, B) ((A) < (B) ? (A) : (B))
#define TIFFArrayCount(a) (sizeof(a) / sizeof((a)[0]))

typedef size_t TIFFIOSize_t;
#define _TIFF_lseek_f(fildes, offset, whence) lseek(fildes, offset, whence)
#define _TIFF_fseek_f(stream, offset, whence) fseek(stream, offset, whence)
#define _TIFF_fstat_f(fildes, stat_buff) fstat(fildes, stat_buff)
#define _TIFF_stat_s struct stat
#define _TIFF_off_t off_t

#if defined(__cplusplus)
extern "C"
{
#endif
    extern int _TIFFSeekOK(TIFF *tif, toff_t off);
    extern void *_TIFFmallocExt(TIFF *tif, tmsize_t s);
    extern void *_TIFFcallocExt(TIFF *tif, tmsize_t nmemb, tmsize_t siz);
    extern void *_TIFFreallocExt(TIFF *tif, void *p, tmsize_t s);
    extern void _TIFFfreeExt(TIFF *tif, void *p);
    extern void *_TIFFCheckMalloc(TIFF *tif, tmsize_t nmemb, tmsize_t elem_size,
                                  const char *what);
    extern void *_TIFFCheckRealloc(TIFF *tif, void *buffer, tmsize_t nmemb,
                                   tmsize_t elem_size, const char *what);
    extern uint32_t _TIFFMultiply32(TIFF *tif, uint32_t first, uint32_t second,
                                    const char *where);
    extern uint64_t _TIFFMultiply64(TIFF *tif, uint64_t first, uint64_t second,
                                    const char *where);
    extern uint32_t _TIFFClampDoubleToUInt32(double val);
#if defined(__cplusplus)
}
#endif

#endif
