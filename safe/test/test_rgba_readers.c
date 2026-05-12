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
static const char info_filename[] = "test_rgba_readers_info.txt";
static const uint32_t width = 2;
static const uint32_t height = 2;
static const unsigned char row0[] = {255, 0, 0, 0, 255, 0};
static const unsigned char row1[] = {0, 0, 255, 255, 255, 255};
static const unsigned char rgba_row0[] = {255, 0, 0, 128, 0, 255, 0, 64};
static const unsigned char rgba_row1[] = {0, 0, 255, 32, 255, 255, 255, 255};

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

static int write_test_image(uint32_t rows_per_strip)
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

static int write_rgba_alpha_image(void)
{
    TIFF *tif = TIFFOpen(filename, "w");
    uint16_t extrasample = EXTRASAMPLE_UNASSALPHA;

    if (!tif)
    {
        fprintf(stderr, "Can't create %s for RGBA alpha checks.\n", filename);
        return 1;
    }

    if (!TIFFSetField(tif, TIFFTAG_IMAGEWIDTH, width) ||
        !TIFFSetField(tif, TIFFTAG_IMAGELENGTH, height) ||
        !TIFFSetField(tif, TIFFTAG_BITSPERSAMPLE, 8) ||
        !TIFFSetField(tif, TIFFTAG_SAMPLESPERPIXEL, 4) ||
        !TIFFSetField(tif, TIFFTAG_EXTRASAMPLES, 1, &extrasample) ||
        !TIFFSetField(tif, TIFFTAG_ROWSPERSTRIP, height) ||
        !TIFFSetField(tif, TIFFTAG_PLANARCONFIG, PLANARCONFIG_CONTIG) ||
        !TIFFSetField(tif, TIFFTAG_PHOTOMETRIC, PHOTOMETRIC_RGB))
    {
        fprintf(stderr, "Failed to initialize %s for RGBA alpha checks.\n",
                filename);
        TIFFClose(tif);
        return 1;
    }

    if (TIFFWriteScanline(tif, (void *)rgba_row0, 0, 0) == -1 ||
        TIFFWriteScanline(tif, (void *)rgba_row1, 1, 0) == -1)
    {
        fprintf(stderr, "Failed to write RGBA alpha image data.\n");
        TIFFClose(tif);
        return 1;
    }

    TIFFClose(tif);
    return 0;
}

static int check_rgba_alpha_metadata(void)
{
    TIFF *tif = NULL;
    uint16_t samples = 0;
    uint16_t extrasamples = 0;
    const uint16_t *sampleinfo = NULL;
    FILE *info = NULL;
    char text[512];
    size_t nread;
    int ret = 1;

    if (write_rgba_alpha_image() != 0)
        return 1;

    tif = TIFFOpen(filename, "r");
    if (!tif)
    {
        fprintf(stderr, "Can't reopen %s for RGBA alpha checks.\n", filename);
        goto done;
    }
    if (!TIFFGetField(tif, TIFFTAG_SAMPLESPERPIXEL, &samples) || samples != 4)
    {
        fprintf(stderr, "RGBA alpha image did not preserve SamplesPerPixel=4.\n");
        goto done;
    }
    if (!TIFFGetField(tif, TIFFTAG_EXTRASAMPLES, &extrasamples, &sampleinfo) ||
        extrasamples != 1 || sampleinfo == NULL ||
        sampleinfo[0] != EXTRASAMPLE_UNASSALPHA)
    {
        fprintf(stderr,
                "RGBA alpha image did not preserve ExtraSamples=unassoc-alpha.\n");
        goto done;
    }

    info = fopen(info_filename, "w");
    if (info == NULL)
    {
        fprintf(stderr, "Can't create %s.\n", info_filename);
        goto done;
    }
    TIFFPrintDirectory(tif, info, 0);
    fclose(info);
    info = NULL;

    info = fopen(info_filename, "r");
    if (info == NULL)
    {
        fprintf(stderr, "Can't reopen %s.\n", info_filename);
        goto done;
    }
    nread = fread(text, 1, sizeof(text) - 1, info);
    text[nread] = '\0';
    if (strstr(text, "Extra Samples: 1<unassoc-alpha>") == NULL)
    {
        fprintf(stderr,
                "TIFFPrintDirectory() did not print unassociated alpha metadata.\n");
        goto done;
    }

    ret = 0;

done:
    if (info)
        fclose(info);
    if (tif)
        TIFFClose(tif);
    if (ret == 0)
    {
        unlink(filename);
        unlink(info_filename);
    }
    return ret;
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
    unlink(info_filename);

    if (write_test_image(1) != 0)
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

    TIFFClose(tif);
    tif = NULL;

    if (write_test_image(2) != 0)
        goto failure;

    tif = TIFFOpen(filename, "r");
    if (!tif)
    {
        fprintf(stderr, "Can't reopen %s for strip alignment checks.\n",
                filename);
        goto failure;
    }

    if (TIFFReadRGBAStripExt(tif, 1, strip_raster, 1))
    {
        fprintf(stderr,
                "TIFFReadRGBAStripExt() accepted a row that is not strip-aligned.\n");
        goto failure;
    }

    if (TIFFReadRGBATileExt(tif, 0, 0, raster, 1))
    {
        fprintf(stderr,
                "TIFFReadRGBATileExt() accepted a striped image.\n");
        goto failure;
    }
    TIFFClose(tif);
    tif = NULL;

    if (check_rgba_alpha_metadata() != 0)
        goto failure;

    ret = 0;

failure:
    if (tif)
        TIFFClose(tif);
    if (ret == 0)
    {
        unlink(filename);
        unlink(info_filename);
    }
    return ret;
}
