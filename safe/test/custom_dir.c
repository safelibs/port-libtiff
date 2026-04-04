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
 * Public-API regression for EXIF directory creation and round-tripping.
 */

#include "tif_config.h"

#include <stdio.h>
#include <string.h>

#ifdef HAVE_UNISTD_H
#include <unistd.h>
#endif

#include "tiffio.h"

static const char filename[] = "custom_dir.tif";

#define SPP 3
static const uint16_t width = 1;
static const uint16_t length = 1;
static const uint16_t bps = 8;
static const uint16_t photometric = PHOTOMETRIC_RGB;
static const uint16_t rows_per_strip = 1;
static const uint16_t planarconfig = PLANARCONFIG_CONTIG;

int main(void)
{
    TIFF *tif = NULL;
    unsigned char buf[SPP] = {0, 127, 255};
    uint64_t exif_offset = 0;
    uint64_t read_exif_offset = 0;
    char *ascii_value = NULL;
    uint32_t value_u32 = 0;
    uint8_t exif_version[4] = {'0', '2', '3', '1'};

    unlink(filename);

    tif = TIFFOpen(filename, "w+");
    if (!tif)
    {
        fprintf(stderr, "Can't create test TIFF file %s.\n", filename);
        return 1;
    }

    if (!TIFFSetField(tif, TIFFTAG_IMAGEWIDTH, width) ||
        !TIFFSetField(tif, TIFFTAG_IMAGELENGTH, length) ||
        !TIFFSetField(tif, TIFFTAG_BITSPERSAMPLE, bps) ||
        !TIFFSetField(tif, TIFFTAG_SAMPLESPERPIXEL, SPP) ||
        !TIFFSetField(tif, TIFFTAG_ROWSPERSTRIP, rows_per_strip) ||
        !TIFFSetField(tif, TIFFTAG_PLANARCONFIG, planarconfig) ||
        !TIFFSetField(tif, TIFFTAG_PHOTOMETRIC, photometric) ||
        !TIFFSetField(tif, TIFFTAG_EXIFIFD, exif_offset))
    {
        fprintf(stderr, "Failed to initialize the main directory.\n");
        goto failure;
    }

    if (TIFFWriteScanline(tif, buf, 0, 0) == -1)
    {
        fprintf(stderr, "Can't write image data.\n");
        goto failure;
    }

    if (!TIFFWriteDirectory(tif))
    {
        fprintf(stderr, "TIFFWriteDirectory() failed.\n");
        goto failure;
    }

    if (TIFFCreateEXIFDirectory(tif) != 0)
    {
        fprintf(stderr, "TIFFCreateEXIFDirectory() failed.\n");
        goto failure;
    }

    if (!TIFFSetField(tif, EXIFTAG_EXIFVERSION, exif_version) ||
        !TIFFSetField(tif, EXIFTAG_SPECTRALSENSITIVITY,
                      "EXIF Spectral Sensitivity"))
    {
        fprintf(stderr, "Failed to populate the EXIF directory.\n");
        goto failure;
    }

    if (!TIFFWriteCustomDirectory(tif, &exif_offset))
    {
        fprintf(stderr, "TIFFWriteCustomDirectory() failed.\n");
        goto failure;
    }

    if (!TIFFSetDirectory(tif, 0) ||
        !TIFFSetField(tif, TIFFTAG_EXIFIFD, exif_offset) ||
        !TIFFRewriteDirectory(tif))
    {
        fprintf(stderr, "Failed to link the EXIF directory from the main IFD.\n");
        goto failure;
    }

    TIFFClose(tif);
    tif = NULL;

    tif = TIFFOpen(filename, "r");
    if (!tif)
    {
        fprintf(stderr, "Can't reopen %s.\n", filename);
        return 1;
    }

    if (TIFFNumberOfDirectories(tif) != 1)
    {
        fprintf(stderr, "Unexpected number of main directories.\n");
        goto failure;
    }

    if (!TIFFGetField(tif, TIFFTAG_EXIFIFD, &read_exif_offset) ||
        read_exif_offset != exif_offset)
    {
        fprintf(stderr, "Did not get the expected EXIFIFD value.\n");
        goto failure;
    }

    if (!TIFFReadEXIFDirectory(tif, read_exif_offset))
    {
        fprintf(stderr, "TIFFReadEXIFDirectory() failed.\n");
        goto failure;
    }

    if (!TIFFGetField(tif, EXIFTAG_SPECTRALSENSITIVITY, &ascii_value) ||
        strcmp(ascii_value, "EXIF Spectral Sensitivity") != 0)
    {
        fprintf(stderr, "Got the wrong SPECTRALSENSITIVITY value.\n");
        goto failure;
    }

    if (!TIFFSetDirectory(tif, 0) ||
        !TIFFGetField(tif, TIFFTAG_IMAGEWIDTH, &value_u32) ||
        value_u32 != width)
    {
        fprintf(stderr, "Failed to return to the main IFD.\n");
        goto failure;
    }

    TIFFClose(tif);
    unlink(filename);
    return 0;

failure:
    if (tif)
        TIFFClose(tif);
    return 1;
}
