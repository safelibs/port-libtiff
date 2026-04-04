#include "tiffiop.h"
#include "tif_dir.h"

#include <ctype.h>
#include <stdarg.h>

extern int tiff_safe_core_placeholder(void);
extern int safe_tiff_record_custom_tag(TIFF *tif, uint32_t tag);
extern int safe_tiff_remove_custom_tag(TIFF *tif, uint32_t tag);
extern int safe_tiff_read_custom_directory(TIFF *tif, uint64_t diroff,
                                           const TIFFFieldArray *infoarray);
extern int safe_tiff_set_directory(TIFF *tif, uint32_t dirnum);
extern int safe_tiff_set_sub_directory(TIFF *tif, uint64_t diroff);
extern uint32_t safe_tiff_number_of_directories(TIFF *tif);
extern int safe_tiff_last_directory(TIFF *tif);
extern void safe_tiff_free_directory(TIFF *tif);
extern uint32_t safe_tiff_current_tag_count(TIFF *tif);
extern uint32_t safe_tiff_current_tag_at(TIFF *tif, uint32_t index);
extern int safe_tiff_get_tag_value(TIFF *tif, uint32_t tag, int defaulted,
                                   TIFFDataType *out_type, uint64_t *out_count,
                                   const void **out_data);

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

static int safe_query_tag(TIFF *tif, uint32_t tag, int defaulted,
                          TIFFDataType *type, uint64_t *count,
                          const void **data)
{
    if (tif == NULL || type == NULL || count == NULL || data == NULL)
        return 0;
    *data = NULL;
    *count = 0;
    *type = TIFF_NOTYPE;
    return safe_tiff_get_tag_value(tif, tag, defaulted, type, count, data);
}

static int safe_copy_u16(va_list ap, const void *data)
{
    uint16_t *value = va_arg(ap, uint16_t *);
    if (value != NULL)
        *value = ((const uint16_t *)data)[0];
    return 1;
}

static int safe_copy_u32(va_list ap, const void *data)
{
    uint32_t *value = va_arg(ap, uint32_t *);
    if (value != NULL)
        *value = ((const uint32_t *)data)[0];
    return 1;
}

static int safe_copy_u64(va_list ap, const void *data)
{
    uint64_t *value = va_arg(ap, uint64_t *);
    if (value != NULL)
        *value = ((const uint64_t *)data)[0];
    return 1;
}

static int safe_copy_float(va_list ap, TIFFDataType type, const void *data)
{
    float *value = va_arg(ap, float *);
    if (value == NULL)
        return 1;
    if (type == TIFF_DOUBLE)
        *value = (float)((const double *)data)[0];
    else
        *value = ((const float *)data)[0];
    return 1;
}

static int safe_copy_double(va_list ap, TIFFDataType type, const void *data)
{
    double *value = va_arg(ap, double *);
    if (value == NULL)
        return 1;
    if (type == TIFF_FLOAT)
        *value = ((const float *)data)[0];
    else
        *value = ((const double *)data)[0];
    return 1;
}

static int safe_marshal_custom_field(const TIFFField *fip, TIFFDataType type,
                                     uint64_t count, const void *data,
                                     va_list ap)
{
    if (fip == NULL || data == NULL)
        return 0;

    if (fip->field_passcount)
    {
        if (fip->field_readcount == TIFF_VARIABLE2)
        {
            uint32_t *value_count = va_arg(ap, uint32_t *);
            if (value_count != NULL)
                *value_count = (count > UINT32_MAX) ? UINT32_MAX : (uint32_t)count;
        }
        else
        {
            uint16_t *value_count = va_arg(ap, uint16_t *);
            if (value_count != NULL)
                *value_count = (count > UINT16_MAX) ? UINT16_MAX : (uint16_t)count;
        }
        {
            const void **value = va_arg(ap, const void **);
            if (value != NULL)
                *value = data;
        }
        return 1;
    }

    if (fip->field_tag == TIFFTAG_DOTRANGE &&
        strcmp(fip->field_name, "DotRange") == 0)
    {
        if (count < 2 || type != TIFF_SHORT)
            return 0;
        {
            uint16_t *first = va_arg(ap, uint16_t *);
            uint16_t *second = va_arg(ap, uint16_t *);
            if (first != NULL)
                *first = ((const uint16_t *)data)[0];
            if (second != NULL)
                *second = ((const uint16_t *)data)[1];
        }
        return 1;
    }

    if (type == TIFF_ASCII || fip->field_readcount == TIFF_VARIABLE ||
        fip->field_readcount == TIFF_VARIABLE2 || count > 1)
    {
        void **value = va_arg(ap, void **);
        if (value != NULL)
            *value = (void *)data;
        return 1;
    }

    switch (type)
    {
        case TIFF_BYTE:
        case TIFF_UNDEFINED:
        {
            uint8_t *value = va_arg(ap, uint8_t *);
            if (value != NULL)
                *value = ((const uint8_t *)data)[0];
            return 1;
        }
        case TIFF_SBYTE:
        {
            int8_t *value = va_arg(ap, int8_t *);
            if (value != NULL)
                *value = ((const int8_t *)data)[0];
            return 1;
        }
        case TIFF_SHORT:
            return safe_copy_u16(ap, data);
        case TIFF_SSHORT:
        {
            int16_t *value = va_arg(ap, int16_t *);
            if (value != NULL)
                *value = ((const int16_t *)data)[0];
            return 1;
        }
        case TIFF_LONG:
        case TIFF_IFD:
            return safe_copy_u32(ap, data);
        case TIFF_SLONG:
        {
            int32_t *value = va_arg(ap, int32_t *);
            if (value != NULL)
                *value = ((const int32_t *)data)[0];
            return 1;
        }
        case TIFF_LONG8:
        case TIFF_IFD8:
            return safe_copy_u64(ap, data);
        case TIFF_SLONG8:
        {
            int64_t *value = va_arg(ap, int64_t *);
            if (value != NULL)
                *value = ((const int64_t *)data)[0];
            return 1;
        }
        case TIFF_RATIONAL:
        case TIFF_FLOAT:
            return safe_copy_float(ap, type, data);
        case TIFF_SRATIONAL:
        case TIFF_DOUBLE:
            return safe_copy_double(ap, type, data);
        default:
            return 0;
    }
}

static int safe_vget_field_impl(TIFF *tif, uint32_t tag, va_list ap,
                                int defaulted)
{
    TIFFDataType type;
    uint64_t count;
    const void *data;
    const TIFFField *fip;

    if (!safe_query_tag(tif, tag, defaulted, &type, &count, &data))
        return 0;

    fip = TIFFFindField(tif, tag, TIFF_ANY);
    if (fip == NULL)
        return 0;

    if (fip->field_bit == FIELD_CUSTOM)
        return safe_marshal_custom_field(fip, type, count, data, ap);

    switch (tag)
    {
        case TIFFTAG_SUBFILETYPE:
        case TIFFTAG_IMAGEWIDTH:
        case TIFFTAG_IMAGELENGTH:
        case TIFFTAG_ROWSPERSTRIP:
        case TIFFTAG_TILEWIDTH:
        case TIFFTAG_TILELENGTH:
        case TIFFTAG_TILEDEPTH:
        case TIFFTAG_IMAGEDEPTH:
            return safe_copy_u32(ap, data);

        case TIFFTAG_BITSPERSAMPLE:
        case TIFFTAG_COMPRESSION:
        case TIFFTAG_PHOTOMETRIC:
        case TIFFTAG_THRESHHOLDING:
        case TIFFTAG_FILLORDER:
        case TIFFTAG_ORIENTATION:
        case TIFFTAG_SAMPLESPERPIXEL:
        case TIFFTAG_MINSAMPLEVALUE:
        case TIFFTAG_MAXSAMPLEVALUE:
        case TIFFTAG_MATTEING:
        case TIFFTAG_PLANARCONFIG:
        case TIFFTAG_RESOLUTIONUNIT:
        case TIFFTAG_NUMBEROFINKS:
        case TIFFTAG_DATATYPE:
        case TIFFTAG_SAMPLEFORMAT:
        case TIFFTAG_YCBCRPOSITIONING:
            return safe_copy_u16(ap, data);

        case TIFFTAG_EXIFIFD:
        case TIFFTAG_GPSIFD:
            return safe_copy_u64(ap, data);

        case TIFFTAG_XRESOLUTION:
        case TIFFTAG_YRESOLUTION:
        case TIFFTAG_XPOSITION:
        case TIFFTAG_YPOSITION:
            return safe_copy_float(ap, type, data);

        case TIFFTAG_STONITS:
            return safe_copy_double(ap, type, data);

        case TIFFTAG_PAGENUMBER:
        case TIFFTAG_HALFTONEHINTS:
        case TIFFTAG_DOTRANGE:
        case TIFFTAG_YCBCRSUBSAMPLING:
        {
            uint16_t *first = va_arg(ap, uint16_t *);
            uint16_t *second = va_arg(ap, uint16_t *);
            if (count < 2 || type != TIFF_SHORT)
                return 0;
            if (first != NULL)
                *first = ((const uint16_t *)data)[0];
            if (second != NULL)
                *second = ((const uint16_t *)data)[1];
            return 1;
        }

        case TIFFTAG_COLORMAP:
        {
            const uint16_t **red = va_arg(ap, const uint16_t **);
            const uint16_t **green = va_arg(ap, const uint16_t **);
            const uint16_t **blue = va_arg(ap, const uint16_t **);
            const uint16_t *values = (const uint16_t *)data;
            uint64_t plane_count;
            if (count == 0 || type != TIFF_SHORT || count % 3 != 0)
                return 0;
            plane_count = count / 3;
            if (red != NULL)
                *red = values;
            if (green != NULL)
                *green = values + plane_count;
            if (blue != NULL)
                *blue = values + plane_count * 2;
            return 1;
        }

        case TIFFTAG_STRIPOFFSETS:
        case TIFFTAG_TILEOFFSETS:
        case TIFFTAG_STRIPBYTECOUNTS:
        case TIFFTAG_TILEBYTECOUNTS:
        {
            const uint64_t **value = va_arg(ap, const uint64_t **);
            if (value != NULL)
                *value = (count == 0) ? g_zero_strile_counts : (const uint64_t *)data;
            return 1;
        }

        case TIFFTAG_EXTRASAMPLES:
        {
            uint16_t *value_count = va_arg(ap, uint16_t *);
            const uint16_t **value = va_arg(ap, const uint16_t **);
            if (type != TIFF_SHORT)
                return 0;
            if (value_count != NULL)
                *value_count = (count > UINT16_MAX) ? UINT16_MAX : (uint16_t)count;
            if (value != NULL)
                *value = (count == 0) ? NULL : (const uint16_t *)data;
            return 1;
        }

        case TIFFTAG_SUBIFD:
        {
            uint16_t *value_count = va_arg(ap, uint16_t *);
            const void **value = va_arg(ap, const void **);
            if (type != TIFF_LONG8 && type != TIFF_IFD8)
                return 0;
            if (value_count != NULL)
                *value_count = (count > UINT16_MAX) ? UINT16_MAX : (uint16_t)count;
            if (value != NULL)
                *value = data;
            return 1;
        }

        case TIFFTAG_TRANSFERFUNCTION:
        {
            const uint16_t **red = va_arg(ap, const uint16_t **);
            const uint16_t **green = NULL;
            const uint16_t **blue = NULL;
            const uint16_t *values = (const uint16_t *)data;
            uint16_t samplesperpixel = 1;
            uint16_t extrasamples = 0;
            const uint16_t *sampleinfo = NULL;
            uint16_t color_planes;
            uint64_t plane_count;
            if (type != TIFF_SHORT || count == 0 || red == NULL)
                return 0;
            TIFFGetFieldDefaulted(tif, TIFFTAG_SAMPLESPERPIXEL, &samplesperpixel);
            TIFFGetFieldDefaulted(tif, TIFFTAG_EXTRASAMPLES, &extrasamples,
                                  &sampleinfo);
            color_planes = (samplesperpixel > extrasamples)
                               ? (uint16_t)(samplesperpixel - extrasamples)
                               : samplesperpixel;
            if (color_planes == 0)
                color_planes = 1;
            plane_count = (color_planes > 1) ? 3 : 1;
            if (count % plane_count != 0)
                return 0;
            *red = values;
            if (color_planes > 1)
            {
                green = va_arg(ap, const uint16_t **);
                blue = va_arg(ap, const uint16_t **);
                if (green != NULL)
                    *green = values + (count / 3);
                if (blue != NULL)
                    *blue = values + ((count / 3) * 2);
            }
            return 1;
        }

        default:
            return safe_marshal_custom_field(fip, type, count, data, ap);
    }
}

static int safe_default_vget_field(TIFF *tif, uint32_t tag, va_list ap)
{
    return safe_vget_field_impl(tif, tag, ap, 0);
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

static int safe_default_vset_field(TIFF *tif, uint32_t tag, va_list ap)
{
    if (tif == NULL)
        return 0;

    if (tag == TIFFTAG_FILLORDER)
    {
        int order = va_arg(ap, int);
        tif->tif_flags &= ~TIFF_FILLORDER;
        tif->tif_flags |=
            (order == FILLORDER_LSB2MSB) ? FILLORDER_LSB2MSB
                                         : FILLORDER_MSB2LSB;
    }
    safe_tiff_record_custom_tag(tif, tag);
    return 1;
}

void safe_tiff_initialize_tag_methods(TIFFTagMethods *methods)
{
    if (methods == NULL)
        return;

    methods->vsetfield = safe_default_vset_field;
    methods->vgetfield = safe_default_vget_field;
    methods->printdir = NULL;
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
    ret = TIFFVGetField(tif, tag, ap);
    va_end(ap);
    return ret;
}

int TIFFVGetField(TIFF *tif, uint32_t tag, va_list ap)
{
    TIFFTagMethods *tag_methods;

    if (tif == NULL)
        return 0;

    tag_methods = TIFFAccessTagMethods(tif);
    if (tag_methods != NULL && tag_methods->vgetfield != NULL)
        return tag_methods->vgetfield(tif, tag, ap);

    return safe_vget_field_impl(tif, tag, ap, 0);
}

int TIFFGetFieldDefaulted(TIFF *tif, uint32_t tag, ...)
{
    va_list ap;
    int ret;
    va_start(ap, tag);
    ret = TIFFVGetFieldDefaulted(tif, tag, ap);
    va_end(ap);
    return ret;
}

int TIFFVGetFieldDefaulted(TIFF *tif, uint32_t tag, va_list ap)
{
    va_list copy;
    int ret;

    va_copy(copy, ap);
    ret = safe_vget_field_impl(tif, tag, copy, 0);
    va_end(copy);
    if (ret)
        return ret;

    return safe_vget_field_impl(tif, tag, ap, 1);
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
    TIFFTagMethods *tag_methods;

    if (tif == NULL)
        return 0;

    tag_methods = TIFFAccessTagMethods(tif);
    if (tag_methods != NULL && tag_methods->vsetfield != NULL)
        return tag_methods->vsetfield(tif, tag, ap);

    return safe_default_vset_field(tif, tag, ap);
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
    return safe_tiff_read_custom_directory(tif, diroff, _TIFFGetExifFields());
}

int TIFFReadGPSDirectory(TIFF *tif, toff_t diroff)
{
    return safe_tiff_read_custom_directory(tif, diroff, _TIFFGetGpsFields());
}

int TIFFReadCustomDirectory(TIFF *tif, toff_t diroff,
                            const TIFFFieldArray *infoarray)
{
    return safe_tiff_read_custom_directory(tif, diroff, infoarray);
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

static int safe_is_printed_in_summary(uint32_t tag)
{
    switch (tag)
    {
        case TIFFTAG_SUBFILETYPE:
        case TIFFTAG_IMAGEWIDTH:
        case TIFFTAG_IMAGELENGTH:
        case TIFFTAG_TILEWIDTH:
        case TIFFTAG_TILELENGTH:
        case TIFFTAG_XRESOLUTION:
        case TIFFTAG_YRESOLUTION:
        case TIFFTAG_BITSPERSAMPLE:
        case TIFFTAG_COMPRESSION:
        case TIFFTAG_PHOTOMETRIC:
        case TIFFTAG_ORIENTATION:
        case TIFFTAG_SAMPLESPERPIXEL:
        case TIFFTAG_ROWSPERSTRIP:
        case TIFFTAG_PLANARCONFIG:
        case TIFFTAG_RESOLUTIONUNIT:
            return 1;
        default:
            return 0;
    }
}

static void safe_print_ascii(FILE *fd, const uint8_t *data, uint64_t count)
{
    uint64_t i;
    for (i = 0; i < count && data[i] != '\0'; ++i)
        fputc(isprint(data[i]) ? data[i] : '?', fd);
}

static void safe_print_value_list(FILE *fd, uint32_t tag, TIFFDataType type,
                                  uint64_t count, const void *data, long flags)
{
    uint64_t i;

    if (data == NULL)
    {
        fprintf(fd, "<absent>");
        return;
    }

    if (type == TIFF_ASCII)
    {
        safe_print_ascii(fd, (const uint8_t *)data, count);
        return;
    }

    if ((tag == TIFFTAG_STRIPOFFSETS || tag == TIFFTAG_STRIPBYTECOUNTS ||
         tag == TIFFTAG_TILEOFFSETS || tag == TIFFTAG_TILEBYTECOUNTS) &&
        !(flags & TIFFPRINT_STRIPS) && count > 8)
    {
        fprintf(fd, "<%" PRIu64 " values>", count);
        return;
    }

    if ((tag == TIFFTAG_COLORMAP || tag == TIFFTAG_TRANSFERFUNCTION) &&
        !(flags & (TIFFPRINT_COLORMAP | TIFFPRINT_CURVES)) && count > 16)
    {
        fprintf(fd, "<%" PRIu64 " values>", count);
        return;
    }

    if (count > 32)
    {
        fprintf(fd, "<%" PRIu64 " values>", count);
        return;
    }

    for (i = 0; i < count; ++i)
    {
        if (i != 0)
            fputs(", ", fd);
        switch (type)
        {
            case TIFF_BYTE:
                fprintf(fd, "%" PRIu8, ((const uint8_t *)data)[i]);
                break;
            case TIFF_UNDEFINED:
                fprintf(fd, "0x%02" PRIx8, ((const uint8_t *)data)[i]);
                break;
            case TIFF_SBYTE:
                fprintf(fd, "%" PRId8, ((const int8_t *)data)[i]);
                break;
            case TIFF_SHORT:
                fprintf(fd, "%" PRIu16, ((const uint16_t *)data)[i]);
                break;
            case TIFF_SSHORT:
                fprintf(fd, "%" PRId16, ((const int16_t *)data)[i]);
                break;
            case TIFF_LONG:
            case TIFF_IFD:
                fprintf(fd, "%" PRIu32, ((const uint32_t *)data)[i]);
                break;
            case TIFF_SLONG:
                fprintf(fd, "%" PRId32, ((const int32_t *)data)[i]);
                break;
            case TIFF_LONG8:
            case TIFF_IFD8:
                fprintf(fd, "%" PRIu64, ((const uint64_t *)data)[i]);
                break;
            case TIFF_SLONG8:
                fprintf(fd, "%" PRId64, ((const int64_t *)data)[i]);
                break;
            case TIFF_FLOAT:
                fprintf(fd, "%g", ((const float *)data)[i]);
                break;
            case TIFF_DOUBLE:
                fprintf(fd, "%g", ((const double *)data)[i]);
                break;
            default:
                fprintf(fd, "<unsupported>");
                return;
        }
    }
}

void TIFFPrintDirectory(TIFF *tif, FILE *fd, long flags)
{
    uint32_t tag_count;
    uint32_t i;

    if (tif == NULL || fd == NULL)
        return;

    fprintf(fd, "TIFF Directory at offset 0x%" PRIx64 " (%" PRIu64 ")\n",
            TIFFCurrentDirOffset(tif), TIFFCurrentDirOffset(tif));

    {
        uint32_t width = 0, length = 0, tilewidth = 0, tilelength = 0;
        float xres = 0.0f, yres = 0.0f;
        uint16_t bitspersample = 0, compression = 0, photometric = 0,
                 orientation = 0, samplesperpixel = 0, planar = 0,
                 resolutionunit = 0;
        uint32_t rowsperstrip = 0;
        uint32_t subfiletype = 0;

        if (TIFFGetField(tif, TIFFTAG_SUBFILETYPE, &subfiletype))
            fprintf(fd, "  Subfile Type: %" PRIu32 "\n", subfiletype);
        if (TIFFGetField(tif, TIFFTAG_IMAGEWIDTH, &width) &&
            TIFFGetField(tif, TIFFTAG_IMAGELENGTH, &length))
            fprintf(fd, "  Image Width: %" PRIu32 " Image Length: %" PRIu32 "\n",
                    width, length);
        if (TIFFGetField(tif, TIFFTAG_TILEWIDTH, &tilewidth) &&
            TIFFGetField(tif, TIFFTAG_TILELENGTH, &tilelength))
            fprintf(fd, "  Tile Width: %" PRIu32 " Tile Length: %" PRIu32 "\n",
                    tilewidth, tilelength);
        if (TIFFGetField(tif, TIFFTAG_XRESOLUTION, &xres) &&
            TIFFGetField(tif, TIFFTAG_YRESOLUTION, &yres))
            fprintf(fd, "  Resolution: %g, %g\n", xres, yres);
        if (TIFFGetField(tif, TIFFTAG_BITSPERSAMPLE, &bitspersample))
            fprintf(fd, "  Bits/Sample: %" PRIu16 "\n", bitspersample);
        if (TIFFGetField(tif, TIFFTAG_COMPRESSION, &compression))
            fprintf(fd, "  Compression: %" PRIu16 "\n", compression);
        if (TIFFGetField(tif, TIFFTAG_PHOTOMETRIC, &photometric))
            fprintf(fd, "  Photometric Interpretation: %" PRIu16 "\n",
                    photometric);
        if (TIFFGetField(tif, TIFFTAG_ORIENTATION, &orientation))
            fprintf(fd, "  Orientation: %" PRIu16 "\n", orientation);
        if (TIFFGetField(tif, TIFFTAG_SAMPLESPERPIXEL, &samplesperpixel))
            fprintf(fd, "  Samples/Pixel: %" PRIu16 "\n", samplesperpixel);
        if (TIFFGetField(tif, TIFFTAG_ROWSPERSTRIP, &rowsperstrip))
            fprintf(fd, "  Rows/Strip: %" PRIu32 "\n", rowsperstrip);
        if (TIFFGetField(tif, TIFFTAG_PLANARCONFIG, &planar))
            fprintf(fd, "  Planar Configuration: %" PRIu16 "\n", planar);
        if (TIFFGetField(tif, TIFFTAG_RESOLUTIONUNIT, &resolutionunit))
            fprintf(fd, "  Resolution Unit: %" PRIu16 "\n", resolutionunit);
    }

    tag_count = safe_tiff_current_tag_count(tif);
    for (i = 0; i < tag_count; ++i)
    {
        uint32_t tag = safe_tiff_current_tag_at(tif, i);
        TIFFDataType type;
        uint64_t count;
        const void *data;
        const TIFFField *field;

        if (tag == UINT32_MAX || safe_is_printed_in_summary(tag))
            continue;
        field = TIFFFindField(tif, tag, TIFF_ANY);
        if (field == NULL || !safe_query_tag(tif, tag, 0, &type, &count, &data))
            continue;

        fprintf(fd, "  %s: ", TIFFFieldName(field));
        safe_print_value_list(fd, tag, type, count, data, flags);
        fputc('\n', fd);
    }
}

void TIFFFreeDirectory(TIFF *tif)
{
    safe_tiff_free_directory(tif);
}

int TIFFSetDirectory(TIFF *tif, tdir_t dirnum)
{
    return safe_tiff_set_directory(tif, dirnum);
}

int TIFFSetSubDirectory(TIFF *tif, uint64_t diroff)
{
    return safe_tiff_set_sub_directory(tif, diroff);
}

tdir_t TIFFNumberOfDirectories(TIFF *tif)
{
    return safe_tiff_number_of_directories(tif);
}

int TIFFLastDirectory(TIFF *tif)
{
    return safe_tiff_last_directory(tif);
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
