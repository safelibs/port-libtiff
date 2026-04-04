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

#include "tif_config.h"

#include <stdio.h>
#include <string.h>

#ifdef HAVE_UNISTD_H
#include <unistd.h>
#endif

#include "tiffio.h"

static const char rgb_filename[] = "api_rgba_image_helpers_rgb.tif";
static const char float_filename[] = "api_rgba_image_helpers_float.tif";

static int write_rgb_image(void)
{
    static const unsigned char row[] = {10, 20, 30};
    TIFF *tif = TIFFOpen(rgb_filename, "w");

    if (!tif)
    {
        fprintf(stderr, "Unable to create %s.\n", rgb_filename);
        return 1;
    }

    if (!TIFFSetField(tif, TIFFTAG_IMAGEWIDTH, 1U) ||
        !TIFFSetField(tif, TIFFTAG_IMAGELENGTH, 1U) ||
        !TIFFSetField(tif, TIFFTAG_BITSPERSAMPLE, 8) ||
        !TIFFSetField(tif, TIFFTAG_SAMPLESPERPIXEL, 3) ||
        !TIFFSetField(tif, TIFFTAG_ROWSPERSTRIP, 1U) ||
        !TIFFSetField(tif, TIFFTAG_PLANARCONFIG, PLANARCONFIG_CONTIG) ||
        !TIFFSetField(tif, TIFFTAG_PHOTOMETRIC, PHOTOMETRIC_RGB) ||
        TIFFWriteScanline(tif, (void *)row, 0, 0) == -1)
    {
        fprintf(stderr, "Unable to write %s.\n", rgb_filename);
        TIFFClose(tif);
        return 1;
    }

    TIFFClose(tif);
    return 0;
}

static int write_float_image(void)
{
    uint16_t sample = 1;
    TIFF *tif = TIFFOpen(float_filename, "w");

    if (!tif)
    {
        fprintf(stderr, "Unable to create %s.\n", float_filename);
        return 1;
    }

    if (!TIFFSetField(tif, TIFFTAG_IMAGEWIDTH, 1U) ||
        !TIFFSetField(tif, TIFFTAG_IMAGELENGTH, 1U) ||
        !TIFFSetField(tif, TIFFTAG_BITSPERSAMPLE, 16) ||
        !TIFFSetField(tif, TIFFTAG_SAMPLESPERPIXEL, 1) ||
        !TIFFSetField(tif, TIFFTAG_SAMPLEFORMAT, SAMPLEFORMAT_IEEEFP) ||
        !TIFFSetField(tif, TIFFTAG_ROWSPERSTRIP, 1U) ||
        !TIFFSetField(tif, TIFFTAG_PLANARCONFIG, PLANARCONFIG_CONTIG) ||
        !TIFFSetField(tif, TIFFTAG_PHOTOMETRIC, PHOTOMETRIC_MINISBLACK) ||
        TIFFWriteScanline(tif, &sample, 0, 0) == -1)
    {
        fprintf(stderr, "Unable to write %s.\n", float_filename);
        TIFFClose(tif);
        return 1;
    }

    TIFFClose(tif);
    return 0;
}

int main(void)
{
    TIFF *tif = NULL;
    TIFFRGBAImage img;
    char emsg[1024];
    int ret = 1;

    unlink(rgb_filename);
    unlink(float_filename);

    if (write_rgb_image() != 0 || write_float_image() != 0)
        goto cleanup;

    tif = TIFFOpen(rgb_filename, "r");
    if (!tif)
    {
        fprintf(stderr, "Unable to reopen %s.\n", rgb_filename);
        goto cleanup;
    }

    memset(emsg, 0, sizeof(emsg));
    if (!TIFFRGBAImageOK(tif, emsg))
    {
        fprintf(stderr, "TIFFRGBAImageOK() rejected RGB input: %s\n", emsg);
        goto cleanup;
    }

    memset(&img, 0, sizeof(img));
    if (!TIFFRGBAImageBegin(&img, tif, 0, emsg))
    {
        fprintf(stderr, "TIFFRGBAImageBegin() failed for RGB input: %s\n",
                emsg);
        goto cleanup;
    }

    if (img.alpha != 0)
    {
        fprintf(stderr, "TIFFRGBAImageBegin() reported alpha=%d for RGB input.\n",
                img.alpha);
        TIFFRGBAImageEnd(&img);
        goto cleanup;
    }

    if (img.get == NULL || img.put.any == NULL)
    {
        fprintf(stderr,
                "TIFFRGBAImageBegin() did not populate RGBA helper routines.\n");
        TIFFRGBAImageEnd(&img);
        goto cleanup;
    }

    if (img.isContig != 1 || img.photometric != PHOTOMETRIC_RGB ||
        img.samplesperpixel != 3 || img.bitspersample != 8 ||
        img.req_orientation != ORIENTATION_BOTLEFT)
    {
        fprintf(stderr, "TIFFRGBAImageBegin() exposed incompatible image state.\n");
        TIFFRGBAImageEnd(&img);
        goto cleanup;
    }

    TIFFRGBAImageEnd(&img);
    TIFFClose(tif);
    tif = NULL;

    tif = TIFFOpen(float_filename, "r");
    if (!tif)
    {
        fprintf(stderr, "Unable to reopen %s.\n", float_filename);
        goto cleanup;
    }

    memset(emsg, 0, sizeof(emsg));
    if (TIFFRGBAImageOK(tif, emsg))
    {
        fprintf(stderr,
                "TIFFRGBAImageOK() unexpectedly accepted IEEE floating-point input.\n");
        goto cleanup;
    }

    if (strstr(emsg, "floating-point") == NULL)
    {
        fprintf(stderr,
                "TIFFRGBAImageOK() returned an unexpected rejection reason: %s\n",
                emsg);
        goto cleanup;
    }

    ret = 0;

cleanup:
    if (tif)
        TIFFClose(tif);
    if (ret == 0)
    {
        unlink(rgb_filename);
        unlink(float_filename);
    }
    return ret;
}
