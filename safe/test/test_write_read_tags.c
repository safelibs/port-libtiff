/*
 * Copyright (c) 2023, LibTIFF Contributors
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
 * Public-API write/read regression for representative TIFF tags.
 */

#include "tif_config.h"

#include <inttypes.h>
#include <math.h>
#include <stdio.h>
#include <string.h>

#ifdef HAVE_UNISTD_H
#include <unistd.h>
#endif

#include "tiffio.h"

#define FLOAT_EPSILON 1e-4

static int nearly_equal(float lhs, float rhs)
{
    return fabs((double)lhs - (double)rhs) <= FLOAT_EPSILON;
}

static int check_field_metadata(TIFF *tif, uint32_t tag, TIFFDataType type,
                                int read_count, int write_count,
                                int passcount)
{
    const TIFFField *field = TIFFFindField(tif, tag, TIFF_ANY);
    if (!field)
    {
        fprintf(stderr, "TIFFFindField failed for tag %" PRIu32 ".\n", tag);
        return 1;
    }

    if (TIFFFieldTag(field) != tag || TIFFFieldDataType(field) != type ||
        TIFFFieldReadCount(field) != read_count ||
        TIFFFieldWriteCount(field) != write_count ||
        TIFFFieldPassCount(field) != passcount)
    {
        fprintf(stderr, "Unexpected metadata for tag %" PRIu32 ".\n", tag);
        return 1;
    }

    return 0;
}

static int run_test(const char *filename, const char *mode)
{
    TIFF *tif = NULL;
    unsigned char buf[6] = {0, 64, 127, 191, 223, 255};
    const char *document_name = "Public tag write/read test";
    const char *artist = "libtiff";
    const char *software = "juvenal";
    const char *datetime = "2026:04:03 12:34:56";
    const uint16_t orientation = ORIENTATION_TOPLEFT;
    const uint16_t page_number = 2;
    const uint16_t page_total = 5;
    const uint16_t halftone_low = 4;
    const uint16_t halftone_high = 251;
    const float x_resolution = 72.5f;
    const float y_resolution = 144.25f;
    const float x_position = 1.25f;
    const float y_position = 2.5f;
    float whitepoint[2] = {0.3127f, 0.3290f};
    float primary_chromaticities[6] = {0.6400f, 0.3300f, 0.3000f,
                                       0.6000f, 0.1500f, 0.0600f};
    char *ascii_value = NULL;
    uint16_t value_u16 = 0;
    uint16_t value_u16_b = 0;
    float value_f32 = 0.0f;
    float *value_f32_array = NULL;

    unlink(filename);

    tif = TIFFOpen(filename, mode);
    if (!tif)
    {
        fprintf(stderr, "Can't create %s.\n", filename);
        return 1;
    }

    if (!TIFFSetField(tif, TIFFTAG_IMAGEWIDTH, 2U) ||
        !TIFFSetField(tif, TIFFTAG_IMAGELENGTH, 2U) ||
        !TIFFSetField(tif, TIFFTAG_BITSPERSAMPLE, 8) ||
        !TIFFSetField(tif, TIFFTAG_SAMPLESPERPIXEL, 3) ||
        !TIFFSetField(tif, TIFFTAG_ROWSPERSTRIP, 1U) ||
        !TIFFSetField(tif, TIFFTAG_PLANARCONFIG, PLANARCONFIG_CONTIG) ||
        !TIFFSetField(tif, TIFFTAG_PHOTOMETRIC, PHOTOMETRIC_RGB) ||
        !TIFFSetField(tif, TIFFTAG_DOCUMENTNAME, document_name) ||
        !TIFFSetField(tif, TIFFTAG_ARTIST, artist) ||
        !TIFFSetField(tif, TIFFTAG_SOFTWARE, software) ||
        !TIFFSetField(tif, TIFFTAG_DATETIME, datetime) ||
        !TIFFSetField(tif, TIFFTAG_ORIENTATION, orientation) ||
        !TIFFSetField(tif, TIFFTAG_PAGENUMBER, page_number, page_total) ||
        !TIFFSetField(tif, TIFFTAG_HALFTONEHINTS, halftone_low,
                      halftone_high) ||
        !TIFFSetField(tif, TIFFTAG_XRESOLUTION, x_resolution) ||
        !TIFFSetField(tif, TIFFTAG_YRESOLUTION, y_resolution) ||
        !TIFFSetField(tif, TIFFTAG_XPOSITION, x_position) ||
        !TIFFSetField(tif, TIFFTAG_YPOSITION, y_position) ||
        !TIFFSetField(tif, TIFFTAG_WHITEPOINT, whitepoint) ||
        !TIFFSetField(tif, TIFFTAG_PRIMARYCHROMATICITIES,
                      primary_chromaticities))
    {
        fprintf(stderr, "Failed to populate %s.\n", filename);
        goto failure;
    }

    if (check_field_metadata(tif, TIFFTAG_PAGENUMBER, TIFF_SHORT, 2, 2, 0) ||
        check_field_metadata(tif, TIFFTAG_WHITEPOINT, TIFF_RATIONAL, 2, 2, 0) ||
        check_field_metadata(tif, TIFFTAG_PRIMARYCHROMATICITIES, TIFF_RATIONAL,
                             6, 6, 0))
    {
        goto failure;
    }

    if (TIFFWriteScanline(tif, buf, 0, 0) == -1 ||
        TIFFWriteScanline(tif, buf, 1, 0) == -1)
    {
        fprintf(stderr, "Failed to write image data.\n");
        goto failure;
    }

    TIFFClose(tif);
    tif = TIFFOpen(filename, "r");
    if (!tif)
    {
        fprintf(stderr, "Can't reopen %s.\n", filename);
        return 1;
    }

    if (!TIFFGetField(tif, TIFFTAG_DOCUMENTNAME, &ascii_value) ||
        strcmp(ascii_value, document_name) != 0 ||
        !TIFFGetField(tif, TIFFTAG_ARTIST, &ascii_value) ||
        strcmp(ascii_value, artist) != 0 ||
        !TIFFGetField(tif, TIFFTAG_SOFTWARE, &ascii_value) ||
        strcmp(ascii_value, software) != 0 ||
        !TIFFGetField(tif, TIFFTAG_DATETIME, &ascii_value) ||
        strcmp(ascii_value, datetime) != 0)
    {
        fprintf(stderr, "ASCII tag verification failed.\n");
        goto failure;
    }

    if (!TIFFGetField(tif, TIFFTAG_ORIENTATION, &value_u16) ||
        value_u16 != orientation ||
        !TIFFGetField(tif, TIFFTAG_PAGENUMBER, &value_u16, &value_u16_b) ||
        value_u16 != page_number || value_u16_b != page_total ||
        !TIFFGetField(tif, TIFFTAG_HALFTONEHINTS, &value_u16, &value_u16_b) ||
        value_u16 != halftone_low || value_u16_b != halftone_high)
    {
        fprintf(stderr, "Integer tag verification failed.\n");
        goto failure;
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
        fprintf(stderr, "Scalar floating-point tag verification failed.\n");
        goto failure;
    }

    if (!TIFFGetField(tif, TIFFTAG_WHITEPOINT, &value_f32_array) ||
        !nearly_equal(value_f32_array[0], whitepoint[0]) ||
        !nearly_equal(value_f32_array[1], whitepoint[1]) ||
        !TIFFGetField(tif, TIFFTAG_PRIMARYCHROMATICITIES, &value_f32_array))
    {
        fprintf(stderr, "Floating-point array tag verification failed.\n");
        goto failure;
    }

    for (int i = 0; i < 6; i++)
    {
        if (!nearly_equal(value_f32_array[i], primary_chromaticities[i]))
        {
            fprintf(stderr, "PrimaryChromaticities verification failed.\n");
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

    ret |= run_test("test_write_read_tags.tif", "w");
    ret |= run_test("test_write_read_tags_bigtiff.tif", "w8");

    return ret;
}
