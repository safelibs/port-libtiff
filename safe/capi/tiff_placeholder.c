#include "tiffiop.h"

#include <stdarg.h>

extern int tiff_safe_core_placeholder(void);

static TIFFErrorHandler g_error_handler = NULL;
static TIFFErrorHandlerExt g_error_handler_ext = NULL;
static TIFFErrorHandler g_warning_handler = NULL;
static TIFFErrorHandlerExt g_warning_handler_ext = NULL;

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

int tiff_safe_capi_placeholder(void) { return tiff_safe_core_placeholder(); }
