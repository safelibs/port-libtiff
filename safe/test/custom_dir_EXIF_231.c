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
 * Public-API regression for EXIF and GPS directory round-tripping.
 */

#include "tif_config.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

#ifdef HAVE_UNISTD_H
#include <unistd.h>
#endif

#include "tiffio.h"

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
        fprintf(stderr, "Unexpected field metadata for tag %" PRIu32 ".\n",
                tag);
        return 1;
    }

    return 0;
}

static int run_test(const char *filename, const char *mode)
{
    TIFF *tif = NULL;
    unsigned char row[3] = {0, 127, 255};
    uint64_t gps_offset = 0;
    uint64_t exif_offset = 0;
    uint64_t read_offset = 0;
    char *ascii_value = NULL;
    uint8_t *u8_array = NULL;
    void *void_array = NULL;
    uint16_t count16 = 0;
    uint16_t value_u16 = 0;
    uint32_t value_u32 = 0;
    const uint8_t gps_version[4] = {2, 2, 0, 1};
    const uint8_t exif_version[4] = {'0', '2', '3', '1'};
    const uint8_t gps_altitude_ref = 0;
    const uint8_t gps_processing_method[] = {'A', 'S', 'C', 'I', 'I'};

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
        !TIFFSetField(tif, TIFFTAG_SAMPLESPERPIXEL, 3) ||
        !TIFFSetField(tif, TIFFTAG_ROWSPERSTRIP, 1U) ||
        !TIFFSetField(tif, TIFFTAG_PLANARCONFIG, PLANARCONFIG_CONTIG) ||
        !TIFFSetField(tif, TIFFTAG_PHOTOMETRIC, PHOTOMETRIC_RGB) ||
        !TIFFSetField(tif, TIFFTAG_GPSIFD, gps_offset) ||
        !TIFFSetField(tif, TIFFTAG_EXIFIFD, exif_offset))
    {
        fprintf(stderr, "Failed to initialize the main directory.\n");
        goto failure;
    }

    if (TIFFWriteScanline(tif, row, 0, 0) == -1 || !TIFFWriteDirectory(tif))
    {
        fprintf(stderr, "Failed to write the main directory.\n");
        goto failure;
    }

    if (TIFFCreateGPSDirectory(tif) != 0)
    {
        fprintf(stderr, "TIFFCreateGPSDirectory() failed.\n");
        goto failure;
    }

    if (!TIFFSetField(tif, GPSTAG_VERSIONID, gps_version) ||
        !TIFFSetField(tif, GPSTAG_LATITUDEREF, "N") ||
        !TIFFSetField(tif, GPSTAG_LONGITUDEREF, "W") ||
        !TIFFSetField(tif, GPSTAG_ALTITUDEREF, gps_altitude_ref) ||
        !TIFFSetField(tif, GPSTAG_DATESTAMP, "2026:04:03") ||
        !TIFFSetField(tif, GPSTAG_DIFFERENTIAL, 1) ||
        !TIFFSetField(tif, GPSTAG_PROCESSINGMETHOD,
                      (uint16_t)sizeof(gps_processing_method),
                      gps_processing_method))
    {
        fprintf(stderr, "Failed to populate the GPS directory.\n");
        goto failure;
    }

    if (check_field_metadata(tif, GPSTAG_VERSIONID, TIFF_BYTE, 4, 4, 0))
        goto failure;

    if (!TIFFWriteCustomDirectory(tif, &gps_offset))
    {
        fprintf(stderr, "TIFFWriteCustomDirectory() for GPS failed.\n");
        goto failure;
    }

    if (!TIFFSetDirectory(tif, 0) || TIFFCreateEXIFDirectory(tif) != 0)
    {
        fprintf(stderr, "Failed to switch to the EXIF directory.\n");
        goto failure;
    }

    if (!TIFFSetField(tif, EXIFTAG_EXIFVERSION, exif_version) ||
        !TIFFSetField(tif, EXIFTAG_DATETIMEORIGINAL, "2026:04:03 12:34:56") ||
        !TIFFSetField(tif, EXIFTAG_FLASH, 1) ||
        !TIFFSetField(tif, EXIFTAG_EXPOSUREPROGRAM, 2) ||
        !TIFFSetField(tif, EXIFTAG_PIXELXDIMENSION, 1U) ||
        !TIFFSetField(tif, EXIFTAG_PIXELYDIMENSION, 1U))
    {
        fprintf(stderr, "Failed to populate the EXIF directory.\n");
        goto failure;
    }

    if (check_field_metadata(tif, EXIFTAG_EXIFVERSION, TIFF_UNDEFINED, 4, 4,
                             0))
        goto failure;

    if (!TIFFWriteCustomDirectory(tif, &exif_offset))
    {
        fprintf(stderr, "TIFFWriteCustomDirectory() for EXIF failed.\n");
        goto failure;
    }

    if (!TIFFSetDirectory(tif, 0) ||
        !TIFFSetField(tif, TIFFTAG_GPSIFD, gps_offset) ||
        !TIFFSetField(tif, TIFFTAG_EXIFIFD, exif_offset) ||
        !TIFFRewriteDirectory(tif))
    {
        fprintf(stderr, "Failed to rewrite the main directory.\n");
        goto failure;
    }

    TIFFClose(tif);
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

    if (!TIFFGetField(tif, TIFFTAG_GPSIFD, &read_offset) ||
        read_offset != gps_offset || !TIFFReadGPSDirectory(tif, read_offset))
    {
        fprintf(stderr, "Failed to read back the GPS directory.\n");
        goto failure;
    }

    if (!TIFFGetField(tif, GPSTAG_VERSIONID, &u8_array) ||
        memcmp(u8_array, gps_version, sizeof(gps_version)) != 0 ||
        !TIFFGetField(tif, GPSTAG_LATITUDEREF, &ascii_value) ||
        strcmp(ascii_value, "N") != 0 ||
        !TIFFGetField(tif, GPSTAG_LONGITUDEREF, &ascii_value) ||
        strcmp(ascii_value, "W") != 0 ||
        !TIFFGetField(tif, GPSTAG_DATESTAMP, &ascii_value) ||
        strcmp(ascii_value, "2026:04:03") != 0 ||
        !TIFFGetField(tif, GPSTAG_DIFFERENTIAL, &value_u16) || value_u16 != 1 ||
        !TIFFGetField(tif, GPSTAG_PROCESSINGMETHOD, &count16, &void_array) ||
        count16 != sizeof(gps_processing_method) ||
        memcmp(void_array, gps_processing_method,
               sizeof(gps_processing_method)) != 0)
    {
        fprintf(stderr, "GPS verification failed.\n");
        goto failure;
    }

    if (!TIFFSetDirectory(tif, 0) ||
        !TIFFGetField(tif, TIFFTAG_EXIFIFD, &read_offset) ||
        read_offset != exif_offset || !TIFFReadEXIFDirectory(tif, read_offset))
    {
        fprintf(stderr, "Failed to read back the EXIF directory.\n");
        goto failure;
    }

    if (!TIFFGetField(tif, EXIFTAG_EXIFVERSION, &ascii_value) ||
        memcmp(ascii_value, exif_version, sizeof(exif_version)) != 0 ||
        !TIFFGetField(tif, EXIFTAG_DATETIMEORIGINAL, &ascii_value) ||
        strcmp(ascii_value, "2026:04:03 12:34:56") != 0 ||
        !TIFFGetField(tif, EXIFTAG_FLASH, &value_u16) || value_u16 != 1 ||
        !TIFFGetField(tif, EXIFTAG_EXPOSUREPROGRAM, &value_u16) ||
        value_u16 != 2 || !TIFFGetField(tif, EXIFTAG_PIXELXDIMENSION,
                                        &value_u32) ||
        value_u32 != 1U ||
        !TIFFGetField(tif, EXIFTAG_PIXELYDIMENSION, &value_u32) ||
        value_u32 != 1U)
    {
        fprintf(stderr, "EXIF verification failed.\n");
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

int main(void)
{
    int ret = 0;

    ret |= run_test("custom_dir_EXIF_231.tif", "w");
    ret |= run_test("custom_dir_EXIF_231_big.tif", "w8");

    return ret;
}
