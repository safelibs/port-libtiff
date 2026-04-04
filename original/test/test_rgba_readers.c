/*
 * Copyright (c) 2026, LibTIFF Contributors
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
 * Regression coverage for public strip and RGBA reader APIs.
 */

#include "tif_config.h"

#include <stdio.h>
#include <string.h>

#ifdef HAVE_UNISTD_H
#include <unistd.h>
#endif

#include "tiffio.h"

static const char filename[] = "test_rgba_readers.tif";
static const uint32_t width = 2;
static const uint32_t height = 2;
static const uint32_t rows_per_strip = 1;
static const unsigned char row0[] = {255, 0, 0, 0, 255, 0};
static const unsigned char row1[] = {0, 0, 255, 255, 255, 255};

static int check_rgba_pixel(uint32_t pixel, unsigned char expected_r,
                            unsigned char expected_g,
                            unsigned char expected_b, const char *label)
{
    if (TIFFGetR(pixel) != (uint32_t)expected_r ||
        TIFFGetG(pixel) != (uint32_t)expected_g ||
        TIFFGetB(pixel) != (uint32_t)expected_b || TIFFGetA(pixel) != 255U)
    {
        fprintf(stderr,
                "%s: got RGBA=(%u,%u,%u,%u), expected (%u,%u,%u,255)\n", label,
                TIFFGetR(pixel), TIFFGetG(pixel), TIFFGetB(pixel),
                TIFFGetA(pixel), (unsigned int)expected_r,
                (unsigned int)expected_g, (unsigned int)expected_b);
        return 1;
    }

    return 0;
}

static int write_test_image(void)
{
    TIFF *tif = TIFFOpen(filename, "w");

    if (!tif)
    {
        fprintf(stderr, "Can't create %s.\n", filename);
        return 1;
    }

    if (!TIFFSetField(tif, TIFFTAG_IMAGEWIDTH, width) ||
        !TIFFSetField(tif, TIFFTAG_IMAGELENGTH, height) ||
        !TIFFSetField(tif, TIFFTAG_BITSPERSAMPLE, 8) ||
        !TIFFSetField(tif, TIFFTAG_SAMPLESPERPIXEL, 3) ||
        !TIFFSetField(tif, TIFFTAG_ROWSPERSTRIP, rows_per_strip) ||
        !TIFFSetField(tif, TIFFTAG_PLANARCONFIG, PLANARCONFIG_CONTIG) ||
        !TIFFSetField(tif, TIFFTAG_PHOTOMETRIC, PHOTOMETRIC_RGB))
    {
        fprintf(stderr, "Failed to initialize %s.\n", filename);
        TIFFClose(tif);
        return 1;
    }

    if (TIFFWriteScanline(tif, (void *)row0, 0, 0) == -1 ||
        TIFFWriteScanline(tif, (void *)row1, 1, 0) == -1)
    {
        fprintf(stderr, "Failed to write image data.\n");
        TIFFClose(tif);
        return 1;
    }

    TIFFClose(tif);
    return 0;
}

int main(void)
{
    TIFF *tif = NULL;
    uint32_t raster[4] = {0, 0, 0, 0};
    uint32_t oriented_raster[4] = {0, 0, 0, 0};
    uint32_t strip_raster[2] = {0, 0};
    unsigned char raw_strip[sizeof(row0)] = {0};
    int ret = 1;

    unlink(filename);

    if (write_test_image() != 0)
        goto failure;

    tif = TIFFOpen(filename, "r");
    if (!tif)
    {
        fprintf(stderr, "Can't reopen %s.\n", filename);
        goto failure;
    }

    if (!TIFFLastDirectory(tif))
    {
        fprintf(stderr, "TIFFLastDirectory() should be true for a single-IFD file.\n");
        goto failure;
    }

    if (TIFFNumberOfStrips(tif) != 2)
    {
        fprintf(stderr, "TIFFNumberOfStrips() returned an unexpected value.\n");
        goto failure;
    }

    if (TIFFComputeStrip(tif, 0, 0) != 0 || TIFFComputeStrip(tif, 1, 0) != 1)
    {
        fprintf(stderr, "TIFFComputeStrip() returned unexpected strip numbers.\n");
        goto failure;
    }

    if (TIFFRawStripSize(tif, 0) != (tmsize_t)sizeof(row0) ||
        TIFFRawStripSize(tif, 1) != (tmsize_t)sizeof(row1))
    {
        fprintf(stderr, "TIFFRawStripSize() returned an unexpected size.\n");
        goto failure;
    }

    if (TIFFReadRawStrip(tif, 0, raw_strip, sizeof(raw_strip)) !=
            (tmsize_t)sizeof(raw_strip) ||
        memcmp(raw_strip, row0, sizeof(row0)) != 0)
    {
        fprintf(stderr, "TIFFReadRawStrip() failed for strip 0.\n");
        goto failure;
    }

    if (TIFFReadRawStrip(tif, 1, raw_strip, sizeof(raw_strip)) !=
            (tmsize_t)sizeof(raw_strip) ||
        memcmp(raw_strip, row1, sizeof(row1)) != 0)
    {
        fprintf(stderr, "TIFFReadRawStrip() failed for strip 1.\n");
        goto failure;
    }

    if (!TIFFReadRGBAImage(tif, width, height, raster, 0))
    {
        fprintf(stderr, "TIFFReadRGBAImage() failed.\n");
        goto failure;
    }

    if (check_rgba_pixel(raster[0], row1[0], row1[1], row1[2],
                         "TIFFReadRGBAImage raster[0]") ||
        check_rgba_pixel(raster[1], row1[3], row1[4], row1[5],
                         "TIFFReadRGBAImage raster[1]") ||
        check_rgba_pixel(raster[2], row0[0], row0[1], row0[2],
                         "TIFFReadRGBAImage raster[2]") ||
        check_rgba_pixel(raster[3], row0[3], row0[4], row0[5],
                         "TIFFReadRGBAImage raster[3]"))
    {
        goto failure;
    }

    if (!TIFFReadRGBAImageOriented(tif, width, height, oriented_raster,
                                   ORIENTATION_TOPLEFT, 0))
    {
        fprintf(stderr, "TIFFReadRGBAImageOriented() failed.\n");
        goto failure;
    }

    if (check_rgba_pixel(oriented_raster[0], row0[0], row0[1], row0[2],
                         "TIFFReadRGBAImageOriented raster[0]") ||
        check_rgba_pixel(oriented_raster[1], row0[3], row0[4], row0[5],
                         "TIFFReadRGBAImageOriented raster[1]") ||
        check_rgba_pixel(oriented_raster[2], row1[0], row1[1], row1[2],
                         "TIFFReadRGBAImageOriented raster[2]") ||
        check_rgba_pixel(oriented_raster[3], row1[3], row1[4], row1[5],
                         "TIFFReadRGBAImageOriented raster[3]"))
    {
        goto failure;
    }

    if (!TIFFReadRGBAStrip(tif, 0, strip_raster) ||
        check_rgba_pixel(strip_raster[0], row0[0], row0[1], row0[2],
                         "TIFFReadRGBAStrip row 0 pixel 0") ||
        check_rgba_pixel(strip_raster[1], row0[3], row0[4], row0[5],
                         "TIFFReadRGBAStrip row 0 pixel 1"))
    {
        fprintf(stderr, "TIFFReadRGBAStrip() failed for row 0.\n");
        goto failure;
    }

    if (!TIFFReadRGBAStrip(tif, 1, strip_raster) ||
        check_rgba_pixel(strip_raster[0], row1[0], row1[1], row1[2],
                         "TIFFReadRGBAStrip row 1 pixel 0") ||
        check_rgba_pixel(strip_raster[1], row1[3], row1[4], row1[5],
                         "TIFFReadRGBAStrip row 1 pixel 1"))
    {
        fprintf(stderr, "TIFFReadRGBAStrip() failed for row 1.\n");
        goto failure;
    }

    ret = 0;

failure:
    if (tif)
        TIFFClose(tif);
    if (ret == 0)
        unlink(filename);
    return ret;
}
