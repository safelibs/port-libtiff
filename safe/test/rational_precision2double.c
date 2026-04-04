/*
 * Copyright (c) 2012, Frank Warmerdam <warmerdam@pobox.com>
 *
 * Permission to use, copy, modify, distribute, and sell this software and
 * its documentation for any purpose is hereby granted without fee, provided
 * that (i) the above copyright notices and this permission notice appear in
 * all copies of the software and related documentation, and (ii) the names of
 * Sam Leffler and Silicon Graphics may not be used in any advertising or
 * publicity relating to the software without the specific, prior written
 * permission of Sam Leffler and Silicon Graphics.
 *
 * THE SOFTWARE IS PROVIDED "AS-IS" AND WITHOUT WARRANTY OF ANY KIND,
 * EXPRESS, IMPLIED OR OTHERWISE, INCLUDING WITHOUT LIMITATION, ANY
 * WARRANTY OF MERCHANTABILITY OR FITNESS FOR A PARTICULAR PURPOSE.
 *
 * IN NO EVENT SHALL SAM LEFFLER OR SILICON GRAPHICS BE LIABLE FOR
 * ANY SPECIAL, INCIDENTAL, INDIRECT OR CONSEQUENTIAL DAMAGES OF ANY KIND,
 * OR ANY DAMAGES WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS,
 * WHETHER OR NOT ADVISED OF THE POSSIBILITY OF DAMAGE, AND ON ANY THEORY OF
 * LIABILITY, ARISING OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE
 * OF THIS SOFTWARE.
 */

/*
 * TIFF Library
 *
 * Public-API regression for rational tag round-tripping and custom tag
 * registration through TIFFMergeFieldInfo().
 */

#include "tif_config.h"

#include <math.h>
#include <stdio.h>
#include <string.h>

#ifdef HAVE_UNISTD_H
#include <unistd.h>
#endif

#include "tiffio.h"

enum
{
    TIFFTAG_CUSTOM_RATIONAL = 60000,
    TIFFTAG_CUSTOM_SRATIONAL,
    TIFFTAG_CUSTOM_RATIONAL_ARRAY,
    TIFFTAG_CUSTOM_SRATIONAL_ARRAY,
};

static const TIFFFieldInfo custom_field_info[] = {
    {TIFFTAG_CUSTOM_RATIONAL, 1, 1, TIFF_RATIONAL, FIELD_CUSTOM, 0, 0,
     "CustomRational"},
    {TIFFTAG_CUSTOM_SRATIONAL, 1, 1, TIFF_SRATIONAL, FIELD_CUSTOM, 0, 0,
     "CustomSRational"},
    {TIFFTAG_CUSTOM_RATIONAL_ARRAY, 3, 3, TIFF_RATIONAL, FIELD_CUSTOM, 0, 0,
     "CustomRationalArray"},
    {TIFFTAG_CUSTOM_SRATIONAL_ARRAY, TIFF_VARIABLE, TIFF_VARIABLE,
     TIFF_SRATIONAL, FIELD_CUSTOM, 0, 1, "CustomSRationalArray"},
};

static TIFFExtendProc parent_extender = NULL;

static void rational_extender(TIFF *tif)
{
    TIFFMergeFieldInfo(tif, custom_field_info,
                       (uint32_t)(sizeof(custom_field_info) /
                                  sizeof(custom_field_info[0])));
    if (parent_extender)
        (*parent_extender)(tif);
}

static void initialize_extender(void)
{
    static int initialized = 0;
    if (!initialized)
    {
        parent_extender = TIFFSetTagExtender(rational_extender);
        initialized = 1;
    }
}

static int nearly_equal(float lhs, float rhs)
{
    return fabs((double)lhs - (double)rhs) <= 1e-4;
}

static int check_field_metadata(TIFF *tif, uint32_t tag, TIFFDataType type,
                                int read_count, int write_count,
                                int passcount)
{
    const TIFFField *field = TIFFFindField(tif, tag, TIFF_ANY);
    if (!field)
    {
        fprintf(stderr, "TIFFFindField failed for tag %u.\n", tag);
        return 1;
    }

    if (TIFFFieldTag(field) != tag || TIFFFieldDataType(field) != type ||
        TIFFFieldReadCount(field) != read_count ||
        TIFFFieldWriteCount(field) != write_count ||
        TIFFFieldPassCount(field) != passcount)
    {
        fprintf(stderr, "Unexpected metadata for tag %u.\n", tag);
        return 1;
    }

    return 0;
}

static int run_test(const char *filename, const char *mode)
{
    TIFF *tif = NULL;
    unsigned char pixel = 0;
    const float x_resolution = 150.25f;
    const float y_resolution = 72.5f;
    const float x_position = 0.125f;
    const float y_position = 1.75f;
    const float custom_rational = 1.0f / 7.0f;
    const float custom_srational = -2.5f;
    const float custom_rational_array[3] = {0.5f, 1.25f, 2.75f};
    const float custom_srational_array[4] = {-0.5f, -1.25f, 3.5f, 8.25f};
    uint16_t count16 = 0;
    float value_f32 = 0.0f;
    float *value_f32_array = NULL;

    unlink(filename);

    tif = TIFFOpen(filename, mode);
    if (!tif)
    {
        fprintf(stderr, "Can't create %s.\n", filename);
        return 1;
    }

    if (!TIFFSetField(tif, TIFFTAG_IMAGEWIDTH, 1U) ||
        !TIFFSetField(tif, TIFFTAG_IMAGELENGTH, 1U) ||
        !TIFFSetField(tif, TIFFTAG_BITSPERSAMPLE, 8) ||
        !TIFFSetField(tif, TIFFTAG_SAMPLESPERPIXEL, 1) ||
        !TIFFSetField(tif, TIFFTAG_ROWSPERSTRIP, 1U) ||
        !TIFFSetField(tif, TIFFTAG_PLANARCONFIG, PLANARCONFIG_CONTIG) ||
        !TIFFSetField(tif, TIFFTAG_PHOTOMETRIC, PHOTOMETRIC_MINISBLACK) ||
        !TIFFSetField(tif, TIFFTAG_XRESOLUTION, (double)x_resolution) ||
        !TIFFSetField(tif, TIFFTAG_YRESOLUTION, y_resolution) ||
        !TIFFSetField(tif, TIFFTAG_XPOSITION, (double)x_position) ||
        !TIFFSetField(tif, TIFFTAG_YPOSITION, y_position) ||
        !TIFFSetField(tif, TIFFTAG_CUSTOM_RATIONAL, custom_rational) ||
        !TIFFSetField(tif, TIFFTAG_CUSTOM_SRATIONAL, custom_srational) ||
        !TIFFSetField(tif, TIFFTAG_CUSTOM_RATIONAL_ARRAY,
                      custom_rational_array) ||
        !TIFFSetField(tif, TIFFTAG_CUSTOM_SRATIONAL_ARRAY,
                      (uint16_t)(sizeof(custom_srational_array) /
                                 sizeof(custom_srational_array[0])),
                      custom_srational_array))
    {
        fprintf(stderr, "Failed to populate %s.\n", filename);
        goto failure;
    }

    if (check_field_metadata(tif, TIFFTAG_CUSTOM_RATIONAL, TIFF_RATIONAL, 1, 1,
                             0) ||
        check_field_metadata(tif, TIFFTAG_CUSTOM_RATIONAL_ARRAY, TIFF_RATIONAL,
                             3, 3, 0) ||
        check_field_metadata(tif, TIFFTAG_CUSTOM_SRATIONAL_ARRAY,
                             TIFF_SRATIONAL, TIFF_VARIABLE, TIFF_VARIABLE, 1))
    {
        goto failure;
    }

    if (TIFFWriteScanline(tif, &pixel, 0, 0) == -1)
    {
        fprintf(stderr, "Failed to write pixel data.\n");
        goto failure;
    }

    TIFFClose(tif);
    tif = TIFFOpen(filename, "r");
    if (!tif)
    {
        fprintf(stderr, "Can't reopen %s.\n", filename);
        return 1;
    }

    if (!TIFFGetField(tif, TIFFTAG_XRESOLUTION, &value_f32) ||
        !nearly_equal(value_f32, x_resolution) ||
        !TIFFGetField(tif, TIFFTAG_YRESOLUTION, &value_f32) ||
        !nearly_equal(value_f32, y_resolution) ||
        !TIFFGetField(tif, TIFFTAG_XPOSITION, &value_f32) ||
        !nearly_equal(value_f32, x_position) ||
        !TIFFGetField(tif, TIFFTAG_YPOSITION, &value_f32) ||
        !nearly_equal(value_f32, y_position))
    {
        fprintf(stderr, "Built-in rational tag verification failed.\n");
        goto failure;
    }

    if (!TIFFGetField(tif, TIFFTAG_CUSTOM_RATIONAL, &value_f32) ||
        !nearly_equal(value_f32, custom_rational) ||
        !TIFFGetField(tif, TIFFTAG_CUSTOM_SRATIONAL, &value_f32) ||
        !nearly_equal(value_f32, custom_srational))
    {
        fprintf(stderr, "Single custom rational tag verification failed.\n");
        goto failure;
    }

    if (!TIFFGetField(tif, TIFFTAG_CUSTOM_RATIONAL_ARRAY, &value_f32_array))
    {
        fprintf(stderr, "Failed to read CustomRationalArray.\n");
        goto failure;
    }
    for (int i = 0; i < 3; i++)
    {
        if (!nearly_equal(value_f32_array[i], custom_rational_array[i]))
        {
            fprintf(stderr, "CustomRationalArray verification failed.\n");
            goto failure;
        }
    }

    if (!TIFFGetField(tif, TIFFTAG_CUSTOM_SRATIONAL_ARRAY, &count16,
                      &value_f32_array) ||
        count16 != (uint16_t)(sizeof(custom_srational_array) /
                              sizeof(custom_srational_array[0])))
    {
        fprintf(stderr, "Failed to read CustomSRationalArray.\n");
        goto failure;
    }
    for (int i = 0; i < count16; i++)
    {
        if (!nearly_equal(value_f32_array[i], custom_srational_array[i]))
        {
            fprintf(stderr, "CustomSRationalArray verification failed.\n");
            goto failure;
        }
    }

    TIFFClose(tif);
    unlink(filename);
    return 0;

failure:
    if (tif)
        TIFFClose(tif);
    return 1;
}

int main(void)
{
    int ret = 0;

    initialize_extender();

    ret |= run_test("rationalPrecision2Double.tif", "w");
    ret |= run_test("rationalPrecision2Double_Big.tif", "w8");

    return ret;
}
