/*
 * Copyright (c) 2007, Frank Warmerdam <warmerdam@pobox.com>
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
 * Public rewrite regressions:
 * - overwriting encoded strip data in-place
 * - rewriting a directory through TIFFRewriteDirectory()
 */

#include "tif_config.h"

#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifdef HAVE_UNISTD_H
#include <unistd.h>
#endif

#include "tiffio.h"

static const uint32_t rows_per_strip = 1;

static int initialize_grayscale_image(TIFF *tif, uint32_t width,
                                      uint32_t length, const char *description)
{
    if (!TIFFSetField(tif, TIFFTAG_COMPRESSION, COMPRESSION_PACKBITS) ||
        !TIFFSetField(tif, TIFFTAG_IMAGEWIDTH, width) ||
        !TIFFSetField(tif, TIFFTAG_IMAGELENGTH, length) ||
        !TIFFSetField(tif, TIFFTAG_BITSPERSAMPLE, 8) ||
        !TIFFSetField(tif, TIFFTAG_SAMPLESPERPIXEL, 1) ||
        !TIFFSetField(tif, TIFFTAG_ROWSPERSTRIP, rows_per_strip) ||
        !TIFFSetField(tif, TIFFTAG_PLANARCONFIG, PLANARCONFIG_CONTIG) ||
        !TIFFSetField(tif, TIFFTAG_PHOTOMETRIC, PHOTOMETRIC_MINISBLACK) ||
        !TIFFSetField(tif, TIFFTAG_IMAGEDESCRIPTION, description))
    {
        fprintf(stderr, "Failed to initialize the TIFF directory.\n");
        return 1;
    }
    return 0;
}

static int test_packbits(void)
{
    TIFF *tif = NULL;
    int i;
    unsigned char buf[10] = {0, 0, 0, 0, 0, 0, 0, 0, 0, 0};
    unsigned char verify[10] = {0};
    const uint32_t width = 10;
    const uint32_t length = 20;
    const char *filename = "test_packbits.tif";

    tif = TIFFOpen(filename, "w");
    if (!tif)
    {
        fprintf(stderr, "Can't create %s.\n", filename);
        return 1;
    }

    if (initialize_grayscale_image(tif, width, length, "packbits before"))
    {
        TIFFClose(tif);
        return 1;
    }

    for (i = 0; i < (int)length; i++)
    {
        if (TIFFWriteEncodedStrip(tif, (uint32_t)i, buf, sizeof(buf)) !=
            (tmsize_t)sizeof(buf))
        {
            fprintf(stderr, "Can't write strip %d.\n", i);
            TIFFClose(tif);
            return 1;
        }
    }

    TIFFClose(tif);
    tif = NULL;

    tif = TIFFOpen(filename, "r+");
    if (!tif)
    {
        fprintf(stderr, "Can't reopen %s.\n", filename);
        return 1;
    }

    buf[3] = 17;
    buf[6] = 12;
    if (TIFFWriteEncodedStrip(tif, 6, buf, sizeof(buf)) !=
        (tmsize_t)sizeof(buf))
    {
        fprintf(stderr, "Can't overwrite strip 6.\n");
        TIFFClose(tif);
        return 1;
    }

    TIFFClose(tif);
    tif = TIFFOpen(filename, "r");
    if (!tif)
    {
        fprintf(stderr, "Can't reopen %s for verification.\n", filename);
        return 1;
    }

    if (TIFFReadEncodedStrip(tif, 6, verify, (tmsize_t)sizeof(verify)) !=
            (tmsize_t)sizeof(verify) ||
        memcmp(buf, verify, sizeof(buf)) != 0)
    {
        fprintf(stderr, "Did not read back the overwritten strip data.\n");
        TIFFClose(tif);
        return 1;
    }

    TIFFClose(tif);
    unlink(filename);
    return 0;
}

static int rewrite_directory_test(const char *filename, const char *mode)
{
    TIFF *tif = NULL;
    unsigned char buf[8] = {0};
    uint32_t width = sizeof(buf);
    uint32_t length = 4;
    char *description = NULL;
    char *software = NULL;

    tif = TIFFOpen(filename, mode);
    if (!tif)
    {
        fprintf(stderr, "Can't create %s.\n", filename);
        return 1;
    }

    if (initialize_grayscale_image(tif, width, length, "before rewrite"))
    {
        TIFFClose(tif);
        return 1;
    }

    if (!TIFFSetField(tif, TIFFTAG_SOFTWARE, "rewrite initial"))
    {
        fprintf(stderr, "Can't set initial software tag.\n");
        TIFFClose(tif);
        return 1;
    }

    for (uint32_t row = 0; row < length; row++)
    {
        if (TIFFWriteScanline(tif, buf, row, 0) == -1)
        {
            fprintf(stderr, "Can't write row %" PRIu32 ".\n", row);
            TIFFClose(tif);
            return 1;
        }
    }

    TIFFClose(tif);
    tif = TIFFOpen(filename, "r+");
    if (!tif)
    {
        fprintf(stderr, "Can't reopen %s for rewriting.\n", filename);
        return 1;
    }

    if (!TIFFSetField(tif, TIFFTAG_IMAGEDESCRIPTION, "after rewrite") ||
        !TIFFSetField(tif, TIFFTAG_SOFTWARE, "rewrite updated") ||
        !TIFFRewriteDirectory(tif))
    {
        fprintf(stderr, "Failed to rewrite directory metadata for %s.\n",
                filename);
        TIFFClose(tif);
        return 1;
    }

    TIFFClose(tif);
    tif = TIFFOpen(filename, "r");
    if (!tif)
    {
        fprintf(stderr, "Can't reopen %s for verification.\n", filename);
        return 1;
    }

    if (!TIFFGetField(tif, TIFFTAG_IMAGEDESCRIPTION, &description) ||
        strcmp(description, "after rewrite") != 0)
    {
        fprintf(stderr, "ImageDescription rewrite verification failed.\n");
        TIFFClose(tif);
        return 1;
    }

    if (!TIFFGetField(tif, TIFFTAG_SOFTWARE, &software) ||
        strcmp(software, "rewrite updated") != 0)
    {
        fprintf(stderr, "Software rewrite verification failed.\n");
        TIFFClose(tif);
        return 1;
    }

    TIFFClose(tif);
    unlink(filename);
    return 0;
}

int main(void)
{
    int failure = 0;

    failure |= test_packbits();
    failure |= rewrite_directory_test("rewrite_classic.tif", "w");
    failure |= rewrite_directory_test("rewrite_big.tif", "w8");

    return failure;
}
