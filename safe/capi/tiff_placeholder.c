#include "tiffiop.h"

#include <stdarg.h>

extern int tiff_safe_core_placeholder(void);
extern int safe_tiff_record_custom_tag(TIFF *tif, uint32_t tag);
extern int safe_tiff_remove_custom_tag(TIFF *tif, uint32_t tag);

static void unix_error_handler(const char *module, const char *fmt, va_list ap)
{
    if (module != NULL)
        fprintf(stderr, "%s: ", module);
    vfprintf(stderr, fmt, ap);
    fprintf(stderr, ".\n");
}

static void unix_warning_handler(const char *module, const char *fmt,
                                 va_list ap)
{
    if (module != NULL)
        fprintf(stderr, "%s: ", module);
    fprintf(stderr, "Warning, ");
    vfprintf(stderr, fmt, ap);
    fprintf(stderr, ".\n");
}

static TIFFErrorHandler g_error_handler = unix_error_handler;
static TIFFErrorHandlerExt g_error_handler_ext = NULL;
static TIFFErrorHandler g_warning_handler = unix_warning_handler;
static TIFFErrorHandlerExt g_warning_handler_ext = NULL;
static uint64_t g_zero_strile_counts[1] = {0};

static uint16_t safe_stub_fillorder(const TIFF *tif)
{
    if (tif != NULL && (tif->tif_flags & TIFF_FILLORDER) == FILLORDER_LSB2MSB)
        return FILLORDER_LSB2MSB;
    return FILLORDER_MSB2LSB;
}

static int safe_stub_vget_field(TIFF *tif, uint32_t tag, va_list ap,
                                int defaulted)
{
    (void)tif;

    switch (tag)
    {
        case TIFFTAG_IMAGEWIDTH:
        case TIFFTAG_IMAGELENGTH:
        case TIFFTAG_TILEWIDTH:
        case TIFFTAG_TILELENGTH:
        {
            uint32_t *value = va_arg(ap, uint32_t *);
            if (value)
                *value = 0;
            return 1;
        }
        case TIFFTAG_ROWSPERSTRIP:
        {
            uint32_t *value = va_arg(ap, uint32_t *);
            if (value)
                *value = defaulted ? (uint32_t)-1 : 1;
            return 1;
        }
        case TIFFTAG_SAMPLESPERPIXEL:
        {
            uint16_t *value = va_arg(ap, uint16_t *);
            if (value)
                *value = 1;
            return 1;
        }
        case TIFFTAG_PLANARCONFIG:
        {
            uint16_t *value = va_arg(ap, uint16_t *);
            if (value)
                *value = PLANARCONFIG_CONTIG;
            return 1;
        }
        case TIFFTAG_FILLORDER:
        {
            uint16_t *value = va_arg(ap, uint16_t *);
            if (value)
                *value = safe_stub_fillorder(tif);
            return 1;
        }
        case TIFFTAG_BITSPERSAMPLE:
        {
            uint16_t *value = va_arg(ap, uint16_t *);
            if (value)
                *value = 1;
            return 1;
        }
        case TIFFTAG_COMPRESSION:
        {
            uint16_t *value = va_arg(ap, uint16_t *);
            if (value)
                *value = COMPRESSION_NONE;
            return 1;
        }
        case TIFFTAG_ORIENTATION:
        {
            uint16_t *value = va_arg(ap, uint16_t *);
            if (value)
                *value = ORIENTATION_TOPLEFT;
            return 1;
        }
        case TIFFTAG_RESOLUTIONUNIT:
        {
            uint16_t *value = va_arg(ap, uint16_t *);
            if (value)
                *value = RESUNIT_INCH;
            return 1;
        }
        case TIFFTAG_STRIPBYTECOUNTS:
        case TIFFTAG_TILEBYTECOUNTS:
        {
            uint64_t **value = va_arg(ap, uint64_t **);
            if (value)
                *value = g_zero_strile_counts;
            return 1;
        }
        default:
            return 0;
    }
}

static uint64_t safe_stub_scanline_size64(TIFF *tif)
{
    (void)tif;
    return 1;
}

static uint64_t safe_stub_tile_row_size64(TIFF *tif)
{
    (void)tif;
    return 1;
}

static uint64_t safe_stub_tile_size64(TIFF *tif)
{
    (void)tif;
    return 1;
}

static uint64_t safe_stub_strip_size64(TIFF *tif)
{
    (void)tif;
    return 1;
}

static void safe_stub_zero_fill(void *buf, tmsize_t size)
{
    if (buf != NULL && size > 0)
        memset(buf, 0, (size_t)size);
}

static uint8_t safe_reverse_byte(uint8_t value)
{
    value = (uint8_t)(((value & 0xF0U) >> 4) | ((value & 0x0FU) << 4));
    value = (uint8_t)(((value & 0xCCU) >> 2) | ((value & 0x33U) << 2));
    value = (uint8_t)(((value & 0xAAU) >> 1) | ((value & 0x55U) << 1));
    return value;
}

static void call_error_handler_message(TIFFErrorHandler handler,
                                       const char *module, const char *fmt,
                                       ...)
{
    va_list ap;
    va_start(ap, fmt);
    handler(module, fmt, ap);
    va_end(ap);
}

static void call_error_handler_ext_message(TIFFErrorHandlerExt handler,
                                           thandle_t clientdata,
                                           const char *module,
                                           const char *fmt, ...)
{
    va_list ap;
    va_start(ap, fmt);
    handler(clientdata, module, fmt, ap);
    va_end(ap);
}

static int call_error_handler_extr_message(TIFFErrorHandlerExtR handler,
                                           TIFF *tif, void *user_data,
                                           const char *module,
                                           const char *fmt, ...)
{
    va_list ap;
    int stop;
    va_start(ap, fmt);
    stop = handler(tif, user_data, module, fmt, ap);
    va_end(ap);
    return stop;
}

TIFFErrorHandler TIFFSetErrorHandler(TIFFErrorHandler handler)
{
    TIFFErrorHandler previous = g_error_handler;
    g_error_handler = handler;
    return previous;
}

TIFFErrorHandlerExt TIFFSetErrorHandlerExt(TIFFErrorHandlerExt handler)
{
    TIFFErrorHandlerExt previous = g_error_handler_ext;
    g_error_handler_ext = handler;
    return previous;
}

TIFFErrorHandler TIFFSetWarningHandler(TIFFErrorHandler handler)
{
    TIFFErrorHandler previous = g_warning_handler;
    g_warning_handler = handler;
    return previous;
}

TIFFErrorHandlerExt TIFFSetWarningHandlerExt(TIFFErrorHandlerExt handler)
{
    TIFFErrorHandlerExt previous = g_warning_handler_ext;
    g_warning_handler_ext = handler;
    return previous;
}

void TIFFError(const char *module, const char *fmt, ...)
{
    va_list ap;
    if (g_error_handler)
    {
        va_start(ap, fmt);
        g_error_handler(module, fmt, ap);
        va_end(ap);
    }
    if (g_error_handler_ext)
    {
        va_start(ap, fmt);
        g_error_handler_ext(NULL, module, fmt, ap);
        va_end(ap);
    }
}

void TIFFErrorExt(thandle_t clientdata, const char *module, const char *fmt,
                  ...)
{
    va_list ap;
    if (g_error_handler)
    {
        va_start(ap, fmt);
        g_error_handler(module, fmt, ap);
        va_end(ap);
    }
    if (g_error_handler_ext)
    {
        va_start(ap, fmt);
        g_error_handler_ext(clientdata, module, fmt, ap);
        va_end(ap);
    }
}

void TIFFWarning(const char *module, const char *fmt, ...)
{
    va_list ap;
    if (g_warning_handler)
    {
        va_start(ap, fmt);
        g_warning_handler(module, fmt, ap);
        va_end(ap);
    }
    if (g_warning_handler_ext)
    {
        va_start(ap, fmt);
        g_warning_handler_ext(NULL, module, fmt, ap);
        va_end(ap);
    }
}

void TIFFWarningExt(thandle_t clientdata, const char *module, const char *fmt,
                    ...)
{
    va_list ap;
    if (g_warning_handler)
    {
        va_start(ap, fmt);
        g_warning_handler(module, fmt, ap);
        va_end(ap);
    }
    if (g_warning_handler_ext)
    {
        va_start(ap, fmt);
        g_warning_handler_ext(clientdata, module, fmt, ap);
        va_end(ap);
    }
}

void TIFFErrorExtR(TIFF *tif, const char *module, const char *fmt, ...)
{
    va_list ap;
    if (tif && tif->tif_errorhandler)
    {
        va_start(ap, fmt);
        if (tif->tif_errorhandler(tif, tif->tif_errorhandler_user_data, module,
                                  fmt, ap))
        {
            va_end(ap);
            return;
        }
        va_end(ap);
    }
    if (g_error_handler)
    {
        va_start(ap, fmt);
        g_error_handler(module, fmt, ap);
        va_end(ap);
    }
    if (g_error_handler_ext)
    {
        va_start(ap, fmt);
        g_error_handler_ext(tif ? tif->tif_clientdata : NULL, module, fmt, ap);
        va_end(ap);
    }
}

void TIFFWarningExtR(TIFF *tif, const char *module, const char *fmt, ...)
{
    va_list ap;
    if (tif && tif->tif_warnhandler)
    {
        va_start(ap, fmt);
        if (tif->tif_warnhandler(tif, tif->tif_warnhandler_user_data, module,
                                 fmt, ap))
        {
            va_end(ap);
            return;
        }
        va_end(ap);
    }
    if (g_warning_handler)
    {
        va_start(ap, fmt);
        g_warning_handler(module, fmt, ap);
        va_end(ap);
    }
    if (g_warning_handler_ext)
    {
        va_start(ap, fmt);
        g_warning_handler_ext(tif ? tif->tif_clientdata : NULL, module, fmt,
                              ap);
        va_end(ap);
    }
}

void safe_tiff_emit_error_message(TIFF *tif, const char *module,
                                  const char *message)
{
    if (tif && tif->tif_errorhandler)
    {
        if (call_error_handler_extr_message(tif->tif_errorhandler, tif,
                                            tif->tif_errorhandler_user_data,
                                            module, "%s", message))
        {
            return;
        }
    }
    if (g_error_handler)
        call_error_handler_message(g_error_handler, module, "%s", message);
    if (g_error_handler_ext)
        call_error_handler_ext_message(g_error_handler_ext,
                                       tif ? tif->tif_clientdata : NULL,
                                       module, "%s", message);
}

void safe_tiff_emit_warning_message(TIFF *tif, const char *module,
                                    const char *message)
{
    if (tif && tif->tif_warnhandler)
    {
        if (call_error_handler_extr_message(tif->tif_warnhandler, tif,
                                            tif->tif_warnhandler_user_data,
                                            module, "%s", message))
        {
            return;
        }
    }
    if (g_warning_handler)
        call_error_handler_message(g_warning_handler, module, "%s", message);
    if (g_warning_handler_ext)
        call_error_handler_ext_message(g_warning_handler_ext,
                                       tif ? tif->tif_clientdata : NULL,
                                       module, "%s", message);
}

void safe_tiff_emit_early_error_message(TIFFOpenOptions *opts,
                                        thandle_t clientdata,
                                        const char *module,
                                        const char *message)
{
    if (opts && opts->errorhandler)
    {
        if (call_error_handler_extr_message(opts->errorhandler, NULL,
                                            opts->errorhandler_user_data,
                                            module, "%s", message))
        {
            return;
        }
    }
    if (g_error_handler)
        call_error_handler_message(g_error_handler, module, "%s", message);
    if (g_error_handler_ext)
        call_error_handler_ext_message(g_error_handler_ext, clientdata, module,
                                       "%s", message);
}

/*
 * These compatibility helpers are intentionally narrow. They let copied
 * tools such as tiffinfo link against the lifecycle-phase library surface
 * without claiming the later tag/directory implementation phases are done.
 */
int TIFFGetField(TIFF *tif, uint32_t tag, ...)
{
    va_list ap;
    int ret;
    va_start(ap, tag);
    ret = safe_stub_vget_field(tif, tag, ap, 0);
    va_end(ap);
    return ret;
}

int TIFFVGetField(TIFF *tif, uint32_t tag, va_list ap)
{
    return safe_stub_vget_field(tif, tag, ap, 0);
}

int TIFFGetFieldDefaulted(TIFF *tif, uint32_t tag, ...)
{
    va_list ap;
    int ret;
    va_start(ap, tag);
    ret = safe_stub_vget_field(tif, tag, ap, 1);
    va_end(ap);
    return ret;
}

int TIFFVGetFieldDefaulted(TIFF *tif, uint32_t tag, va_list ap)
{
    return safe_stub_vget_field(tif, tag, ap, 1);
}

int TIFFSetField(TIFF *tif, uint32_t tag, ...)
{
    va_list ap;
    int ret;
    va_start(ap, tag);
    ret = TIFFVSetField(tif, tag, ap);
    va_end(ap);
    return ret;
}

int TIFFVSetField(TIFF *tif, uint32_t tag, va_list ap)
{
    if (tif == NULL)
        return 0;

    if (tag == TIFFTAG_FILLORDER)
    {
        int order = va_arg(ap, int);
        tif->tif_flags &= ~TIFF_FILLORDER;
        tif->tif_flags |=
            (order == FILLORDER_LSB2MSB) ? FILLORDER_LSB2MSB : FILLORDER_MSB2LSB;
    }
    safe_tiff_record_custom_tag(tif, tag);
    return 1;
}

int TIFFUnsetField(TIFF *tif, uint32_t tag)
{
    if (tif == NULL)
        return 0;

    if (tag == TIFFTAG_FILLORDER)
    {
        tif->tif_flags &= ~TIFF_FILLORDER;
        tif->tif_flags |= FILLORDER_MSB2LSB;
    }

    return safe_tiff_remove_custom_tag(tif, tag);
}

int TIFFReadEXIFDirectory(TIFF *tif, toff_t diroff)
{
    return TIFFSetSubDirectory(tif, diroff);
}

int TIFFReadGPSDirectory(TIFF *tif, toff_t diroff)
{
    return TIFFSetSubDirectory(tif, diroff);
}

uint64_t TIFFScanlineSize64(TIFF *tif) { return safe_stub_scanline_size64(tif); }

tmsize_t TIFFScanlineSize(TIFF *tif)
{
    return (tmsize_t)TIFFScanlineSize64(tif);
}

uint64_t TIFFRasterScanlineSize64(TIFF *tif)
{
    return TIFFScanlineSize64(tif);
}

tmsize_t TIFFRasterScanlineSize(TIFF *tif)
{
    return TIFFScanlineSize(tif);
}

uint64_t TIFFStripSize64(TIFF *tif) { return safe_stub_strip_size64(tif); }

tmsize_t TIFFStripSize(TIFF *tif)
{
    return (tmsize_t)TIFFStripSize64(tif);
}

uint64_t TIFFRawStripSize64(TIFF *tif, uint32_t strip)
{
    (void)strip;
    return TIFFStripSize64(tif);
}

tmsize_t TIFFRawStripSize(TIFF *tif, uint32_t strip)
{
    return (tmsize_t)TIFFRawStripSize64(tif, strip);
}

uint64_t TIFFVStripSize64(TIFF *tif, uint32_t nrows)
{
    (void)nrows;
    return TIFFStripSize64(tif);
}

tmsize_t TIFFVStripSize(TIFF *tif, uint32_t nrows)
{
    return (tmsize_t)TIFFVStripSize64(tif, nrows);
}

uint64_t TIFFTileRowSize64(TIFF *tif)
{
    return safe_stub_tile_row_size64(tif);
}

tmsize_t TIFFTileRowSize(TIFF *tif)
{
    return (tmsize_t)TIFFTileRowSize64(tif);
}

uint64_t TIFFTileSize64(TIFF *tif) { return safe_stub_tile_size64(tif); }

tmsize_t TIFFTileSize(TIFF *tif)
{
    return (tmsize_t)TIFFTileSize64(tif);
}

uint64_t TIFFVTileSize64(TIFF *tif, uint32_t nrows)
{
    (void)nrows;
    return TIFFTileSize64(tif);
}

tmsize_t TIFFVTileSize(TIFF *tif, uint32_t nrows)
{
    return (tmsize_t)TIFFVTileSize64(tif, nrows);
}

uint32_t TIFFDefaultStripSize(TIFF *tif, uint32_t request)
{
    (void)tif;
    return request != 0 ? request : (uint32_t)-1;
}

void TIFFDefaultTileSize(TIFF *tif, uint32_t *tw, uint32_t *th)
{
    (void)tif;
    if (tw && *tw == 0)
        *tw = 1;
    if (th && *th == 0)
        *th = 1;
}

uint32_t TIFFComputeTile(TIFF *tif, uint32_t x, uint32_t y, uint32_t z,
                         uint16_t sample)
{
    (void)tif;
    (void)x;
    (void)y;
    (void)z;
    (void)sample;
    return 0;
}

int TIFFCheckTile(TIFF *tif, uint32_t x, uint32_t y, uint32_t z,
                  uint16_t sample)
{
    (void)tif;
    (void)x;
    (void)y;
    (void)z;
    (void)sample;
    return 1;
}

uint32_t TIFFNumberOfTiles(TIFF *tif)
{
    return TIFFIsTiled(tif) ? 1U : 0U;
}

tmsize_t TIFFReadTile(TIFF *tif, void *buf, uint32_t x, uint32_t y, uint32_t z,
                      uint16_t sample)
{
    tmsize_t size = TIFFTileSize(tif);
    (void)x;
    (void)y;
    (void)z;
    (void)sample;
    safe_stub_zero_fill(buf, size);
    return size;
}

uint32_t TIFFComputeStrip(TIFF *tif, uint32_t row, uint16_t sample)
{
    (void)tif;
    (void)row;
    return sample;
}

uint32_t TIFFNumberOfStrips(TIFF *tif)
{
    return TIFFIsTiled(tif) ? 0U : 1U;
}

tmsize_t TIFFReadEncodedStrip(TIFF *tif, uint32_t strip, void *buf,
                              tmsize_t size)
{
    (void)tif;
    (void)strip;
    if (size < 0)
        size = TIFFStripSize(tif);
    safe_stub_zero_fill(buf, size);
    return size < 0 ? 0 : size;
}

tmsize_t TIFFReadRawStrip(TIFF *tif, uint32_t strip, void *buf, tmsize_t size)
{
    (void)tif;
    (void)strip;
    safe_stub_zero_fill(buf, size);
    return size < 0 ? 0 : size;
}

tmsize_t TIFFReadRawTile(TIFF *tif, uint32_t tile, void *buf, tmsize_t size)
{
    (void)tif;
    (void)tile;
    safe_stub_zero_fill(buf, size);
    return size < 0 ? 0 : size;
}

void TIFFPrintDirectory(TIFF *tif, FILE *fd, long flags)
{
    if (fd == NULL)
        return;
    (void)flags;
    fprintf(fd,
            "TIFF directory information is not available in the safe "
            "lifecycle phase for %s.\n",
            tif != NULL && tif->tif_name != NULL ? tif->tif_name : "<unnamed>");
}

int TIFFSetDirectory(TIFF *tif, tdir_t dirnum)
{
    if (tif == NULL)
        return 0;
    if (dirnum == tif->tif_curdir)
        return 1;
    if (dirnum == 0 && tif->tif_curdir == TIFF_NON_EXISTENT_DIR_NUMBER)
        return TIFFReadDirectory(tif);
    return 0;
}

int TIFFSetSubDirectory(TIFF *tif, uint64_t diroff)
{
    if (tif == NULL)
        return 0;
    return diroff != 0 && diroff == TIFFCurrentDirOffset(tif);
}

tdir_t TIFFNumberOfDirectories(TIFF *tif)
{
    if (tif == NULL)
        return 0;
    if (tif->tif_curdir == TIFF_NON_EXISTENT_DIR_NUMBER)
        return 0;
    return (tdir_t)(tif->tif_curdir + 1);
}

int TIFFLastDirectory(TIFF *tif)
{
    (void)tif;
    return 1;
}

void TIFFReverseBits(uint8_t *cp, tmsize_t n)
{
    tmsize_t i;
    if (cp == NULL || n <= 0)
        return;
    for (i = 0; i < n; ++i)
        cp[i] = safe_reverse_byte(cp[i]);
}

int tiff_safe_capi_placeholder(void) { return tiff_safe_core_placeholder(); }
