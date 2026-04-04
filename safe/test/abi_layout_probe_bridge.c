#include "tiffio.h"

#include <stddef.h>

void safe_tiff_emit_error_message(TIFF *tif, const char *module,
                                  const char *message)
{
    (void)tif;
    (void)module;
    (void)message;
}

void safe_tiff_emit_warning_message(TIFF *tif, const char *module,
                                    const char *message)
{
    (void)tif;
    (void)module;
    (void)message;
}

void safe_tiff_emit_early_error_message(TIFFOpenOptions *opts,
                                        thandle_t clientdata,
                                        const char *module,
                                        const char *message)
{
    (void)opts;
    (void)clientdata;
    (void)module;
    (void)message;
}

void safe_tiff_initialize_tag_methods(TIFFTagMethods *methods)
{
    if (methods == NULL)
        return;

    methods->vsetfield = NULL;
    methods->vgetfield = NULL;
    methods->printdir = NULL;
}
