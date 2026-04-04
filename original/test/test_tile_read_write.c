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
 * Regression coverage for public tiled I/O helpers.
 */

#include "tif_config.h"

#include <stdio.h>
#include <string.h>

#ifdef HAVE_UNISTD_H
#include <unistd.h>
#endif

#include "tiffio.h"

static const char write_tile_filename[] = "test_write_tile.tif";
static const char write_raw_tile_filename[] = "test_write_raw_tile.tif";

enum
{
    IMAGE_WIDTH = 32,
    IMAGE_HEIGHT = 32,
    TILE_WIDTH = 16,
    TILE_HEIGHT = 16,
    TILE_BYTES = TILE_WIDTH * TILE_HEIGHT
};

static int initialize_tiled_image(TIFF *tif)
{
    if (!TIFFSetField(tif, TIFFTAG_COMPRESSION, COMPRESSION_NONE) ||
        !TIFFSetField(tif, TIFFTAG_IMAGEWIDTH, IMAGE_WIDTH) ||
        !TIFFSetField(tif, TIFFTAG_IMAGELENGTH, IMAGE_HEIGHT) ||
        !TIFFSetField(tif, TIFFTAG_BITSPERSAMPLE, 8) ||
        !TIFFSetField(tif, TIFFTAG_SAMPLESPERPIXEL, 1) ||
        !TIFFSetField(tif, TIFFTAG_PLANARCONFIG, PLANARCONFIG_CONTIG) ||
        !TIFFSetField(tif, TIFFTAG_PHOTOMETRIC, PHOTOMETRIC_MINISBLACK) ||
        !TIFFSetField(tif, TIFFTAG_TILEWIDTH, TILE_WIDTH) ||
        !TIFFSetField(tif, TIFFTAG_TILELENGTH, TILE_HEIGHT))
    {
        fprintf(stderr, "Failed to initialize tiled image parameters.\n");
        return 1;
    }

    return 0;
}

static int check_filled_bytes(const unsigned char *buffer, unsigned char value,
                              const char *label)
{
    tmsize_t i;

    for (i = 0; i < (tmsize_t)TILE_BYTES; i++)
    {
        if (buffer[i] != value)
        {
            fprintf(stderr,
                    "%s: got byte %ld = %u, expected every byte to be %u.\n",
                    label, (long)i, (unsigned int)buffer[i],
                    (unsigned int)value);
            return 1;
        }
    }

    return 0;
}

static int check_gray_rgba(uint32_t pixel, unsigned char value,
                           const char *label)
{
    if (TIFFGetR(pixel) != (uint32_t)value ||
        TIFFGetG(pixel) != (uint32_t)value ||
        TIFFGetB(pixel) != (uint32_t)value || TIFFGetA(pixel) != 255U)
    {
        fprintf(stderr,
                "%s: got RGBA=(%u,%u,%u,%u), expected (%u,%u,%u,255)\n", label,
                TIFFGetR(pixel), TIFFGetG(pixel), TIFFGetB(pixel),
                TIFFGetA(pixel), (unsigned int)value, (unsigned int)value,
                (unsigned int)value);
        return 1;
    }

    return 0;
}

static int collect_tile_ids(TIFF *tif, uint32_t *tile00, uint32_t *tile10,
                            uint32_t *tile01, uint32_t *tile11)
{
    if (!TIFFIsTiled(tif))
    {
        fprintf(stderr, "TIFFIsTiled() returned false for a tiled image.\n");
        return 1;
    }

    if (TIFFTileSize(tif) != (tmsize_t)TILE_BYTES)
    {
        fprintf(stderr, "TIFFTileSize() returned an unexpected value.\n");
        return 1;
    }

    if (TIFFNumberOfTiles(tif) != 4)
    {
        fprintf(stderr, "TIFFNumberOfTiles() returned an unexpected value.\n");
        return 1;
    }

    *tile00 = TIFFComputeTile(tif, 0, 0, 0, 0);
    *tile10 = TIFFComputeTile(tif, TILE_WIDTH, 0, 0, 0);
    *tile01 = TIFFComputeTile(tif, 0, TILE_HEIGHT, 0, 0);
    *tile11 = TIFFComputeTile(tif, TILE_WIDTH, TILE_HEIGHT, 0, 0);

    if (*tile00 == *tile10 || *tile00 == *tile01 || *tile00 == *tile11 ||
        *tile10 == *tile01 || *tile10 == *tile11 || *tile01 == *tile11)
    {
        fprintf(stderr, "TIFFComputeTile() returned duplicate tile numbers.\n");
        return 1;
    }

    return 0;
}

static int test_write_tile_roundtrip(void)
{
    TIFF *tif = NULL;
    unsigned char tile_buffer[TILE_BYTES] = {0};
    uint32_t rgba_tile[TILE_BYTES] = {0};
    uint32_t tile00 = 0, tile10 = 0, tile01 = 0, tile11 = 0;
    int ret = 1;

    unlink(write_tile_filename);

    tif = TIFFOpen(write_tile_filename, "w");
    if (!tif)
    {
        fprintf(stderr, "Can't create %s.\n", write_tile_filename);
        goto failure;
    }

    if (initialize_tiled_image(tif) != 0 ||
        collect_tile_ids(tif, &tile00, &tile10, &tile01, &tile11) != 0)
    {
        goto failure;
    }

    (void)tile00;
    (void)tile10;
    (void)tile01;
    (void)tile11;

    memset(tile_buffer, 0x11, sizeof(tile_buffer));
    if (TIFFWriteTile(tif, tile_buffer, 0, 0, 0, 0) !=
        (tmsize_t)sizeof(tile_buffer))
    {
        fprintf(stderr, "TIFFWriteTile() failed for the top-left tile.\n");
        goto failure;
    }

    memset(tile_buffer, 0x22, sizeof(tile_buffer));
    if (TIFFWriteTile(tif, tile_buffer, TILE_WIDTH, 0, 0, 0) !=
        (tmsize_t)sizeof(tile_buffer))
    {
        fprintf(stderr, "TIFFWriteTile() failed for the top-right tile.\n");
        goto failure;
    }

    memset(tile_buffer, 0x33, sizeof(tile_buffer));
    if (TIFFWriteTile(tif, tile_buffer, 0, TILE_HEIGHT, 0, 0) !=
        (tmsize_t)sizeof(tile_buffer))
    {
        fprintf(stderr, "TIFFWriteTile() failed for the bottom-left tile.\n");
        goto failure;
    }

    memset(tile_buffer, 0x44, sizeof(tile_buffer));
    if (TIFFWriteTile(tif, tile_buffer, TILE_WIDTH, TILE_HEIGHT, 0, 0) !=
        (tmsize_t)sizeof(tile_buffer))
    {
        fprintf(stderr, "TIFFWriteTile() failed for the bottom-right tile.\n");
        goto failure;
    }

    TIFFClose(tif);
    tif = TIFFOpen(write_tile_filename, "r");
    if (!tif)
    {
        fprintf(stderr, "Can't reopen %s.\n", write_tile_filename);
        goto failure;
    }

    if (collect_tile_ids(tif, &tile00, &tile10, &tile01, &tile11) != 0)
        goto failure;

    if (TIFFReadTile(tif, tile_buffer, 0, 0, 0, 0) !=
            (tmsize_t)sizeof(tile_buffer) ||
        check_filled_bytes(tile_buffer, 0x11, "TIFFReadTile top-left"))
    {
        goto failure;
    }

    if (TIFFReadTile(tif, tile_buffer, TILE_WIDTH, 0, 0, 0) !=
            (tmsize_t)sizeof(tile_buffer) ||
        check_filled_bytes(tile_buffer, 0x22, "TIFFReadTile top-right"))
    {
        goto failure;
    }

    if (TIFFReadTile(tif, tile_buffer, 0, TILE_HEIGHT, 0, 0) !=
            (tmsize_t)sizeof(tile_buffer) ||
        check_filled_bytes(tile_buffer, 0x33, "TIFFReadTile bottom-left"))
    {
        goto failure;
    }

    if (TIFFReadTile(tif, tile_buffer, TILE_WIDTH, TILE_HEIGHT, 0, 0) !=
            (tmsize_t)sizeof(tile_buffer) ||
        check_filled_bytes(tile_buffer, 0x44, "TIFFReadTile bottom-right"))
    {
        goto failure;
    }

    if (!TIFFReadRGBATile(tif, TILE_WIDTH, TILE_HEIGHT, rgba_tile) ||
        check_gray_rgba(rgba_tile[0], 0x44, "TIFFReadRGBATile raster[0]") ||
        check_gray_rgba(rgba_tile[TILE_BYTES - 1], 0x44,
                        "TIFFReadRGBATile raster[last]"))
    {
        fprintf(stderr, "TIFFReadRGBATile() failed for the bottom-right tile.\n");
        goto failure;
    }

    ret = 0;

failure:
    if (tif)
        TIFFClose(tif);
    if (ret == 0)
        unlink(write_tile_filename);
    return ret;
}

static int test_write_raw_tile_roundtrip(void)
{
    TIFF *tif = NULL;
    unsigned char tile_buffer[TILE_BYTES] = {0};
    uint32_t tile00 = 0, tile10 = 0, tile01 = 0, tile11 = 0;
    int ret = 1;

    unlink(write_raw_tile_filename);

    tif = TIFFOpen(write_raw_tile_filename, "w");
    if (!tif)
    {
        fprintf(stderr, "Can't create %s.\n", write_raw_tile_filename);
        goto failure;
    }

    if (initialize_tiled_image(tif) != 0 ||
        collect_tile_ids(tif, &tile00, &tile10, &tile01, &tile11) != 0)
    {
        goto failure;
    }

    memset(tile_buffer, 0x55, sizeof(tile_buffer));
    if (TIFFWriteRawTile(tif, tile00, tile_buffer, sizeof(tile_buffer)) !=
        (tmsize_t)sizeof(tile_buffer))
    {
        fprintf(stderr, "TIFFWriteRawTile() failed for the top-left tile.\n");
        goto failure;
    }

    memset(tile_buffer, 0x66, sizeof(tile_buffer));
    if (TIFFWriteRawTile(tif, tile10, tile_buffer, sizeof(tile_buffer)) !=
        (tmsize_t)sizeof(tile_buffer))
    {
        fprintf(stderr, "TIFFWriteRawTile() failed for the top-right tile.\n");
        goto failure;
    }

    memset(tile_buffer, 0x77, sizeof(tile_buffer));
    if (TIFFWriteRawTile(tif, tile01, tile_buffer, sizeof(tile_buffer)) !=
        (tmsize_t)sizeof(tile_buffer))
    {
        fprintf(stderr, "TIFFWriteRawTile() failed for the bottom-left tile.\n");
        goto failure;
    }

    memset(tile_buffer, 0x88, sizeof(tile_buffer));
    if (TIFFWriteRawTile(tif, tile11, tile_buffer, sizeof(tile_buffer)) !=
        (tmsize_t)sizeof(tile_buffer))
    {
        fprintf(stderr, "TIFFWriteRawTile() failed for the bottom-right tile.\n");
        goto failure;
    }

    TIFFClose(tif);
    tif = TIFFOpen(write_raw_tile_filename, "r");
    if (!tif)
    {
        fprintf(stderr, "Can't reopen %s.\n", write_raw_tile_filename);
        goto failure;
    }

    if (collect_tile_ids(tif, &tile00, &tile10, &tile01, &tile11) != 0)
        goto failure;

    if (TIFFReadRawTile(tif, tile00, tile_buffer, sizeof(tile_buffer)) !=
            (tmsize_t)sizeof(tile_buffer) ||
        check_filled_bytes(tile_buffer, 0x55, "TIFFReadRawTile top-left"))
    {
        goto failure;
    }

    if (TIFFReadRawTile(tif, tile10, tile_buffer, sizeof(tile_buffer)) !=
            (tmsize_t)sizeof(tile_buffer) ||
        check_filled_bytes(tile_buffer, 0x66, "TIFFReadRawTile top-right"))
    {
        goto failure;
    }

    if (TIFFReadRawTile(tif, tile01, tile_buffer, sizeof(tile_buffer)) !=
            (tmsize_t)sizeof(tile_buffer) ||
        check_filled_bytes(tile_buffer, 0x77, "TIFFReadRawTile bottom-left"))
    {
        goto failure;
    }

    if (TIFFReadRawTile(tif, tile11, tile_buffer, sizeof(tile_buffer)) !=
            (tmsize_t)sizeof(tile_buffer) ||
        check_filled_bytes(tile_buffer, 0x88, "TIFFReadRawTile bottom-right"))
    {
        goto failure;
    }

    if (TIFFReadTile(tif, tile_buffer, TILE_WIDTH, 0, 0, 0) !=
            (tmsize_t)sizeof(tile_buffer) ||
        check_filled_bytes(tile_buffer, 0x66,
                           "TIFFReadTile after TIFFWriteRawTile"))
    {
        goto failure;
    }

    ret = 0;

failure:
    if (tif)
        TIFFClose(tif);
    if (ret == 0)
        unlink(write_raw_tile_filename);
    return ret;
}

int main(void)
{
    if (test_write_tile_roundtrip() != 0)
        return 1;
    if (test_write_raw_tile_roundtrip() != 0)
        return 1;
    return 0;
}
