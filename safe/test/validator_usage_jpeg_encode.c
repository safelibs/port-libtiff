#include "tiffio.h"

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

#define IMAGE_WIDTH 32U
#define IMAGE_HEIGHT 16U
#define TILE_WIDTH 16U
#define TILE_LENGTH 16U

static int fail(const char *message)
{
    fprintf(stderr, "%s\n", message);
    return 0;
}

static void fill_rgb_row(uint8_t *row, uint32_t y)
{
    uint32_t x;
    for (x = 0; x < IMAGE_WIDTH; ++x)
    {
        row[x * 3] = (uint8_t)((x * 7U) & 0xffU);
        row[x * 3 + 1] = (uint8_t)(((y * 11U) + 30U) & 0xffU);
        row[x * 3 + 2] = (uint8_t)(((x + y) * 5U) & 0xffU);
    }
}

static int write_jpeg_tiff(const char *path)
{
    TIFF *tif;
    uint8_t row[IMAGE_WIDTH * 3U];
    uint32_t y;

    tif = TIFFOpen(path, "w");
    if (tif == NULL)
        return fail("failed to open JPEG output");

    if (!TIFFSetField(tif, TIFFTAG_IMAGEWIDTH, IMAGE_WIDTH) ||
        !TIFFSetField(tif, TIFFTAG_IMAGELENGTH, IMAGE_HEIGHT) ||
        !TIFFSetField(tif, TIFFTAG_BITSPERSAMPLE, 8) ||
        !TIFFSetField(tif, TIFFTAG_SAMPLESPERPIXEL, 3) ||
        !TIFFSetField(tif, TIFFTAG_PLANARCONFIG, PLANARCONFIG_CONTIG) ||
        !TIFFSetField(tif, TIFFTAG_PHOTOMETRIC, PHOTOMETRIC_RGB) ||
        !TIFFSetField(tif, TIFFTAG_COMPRESSION, COMPRESSION_JPEG) ||
        !TIFFSetField(tif, TIFFTAG_JPEGCOLORMODE, JPEGCOLORMODE_RGB) ||
        !TIFFSetField(tif, TIFFTAG_JPEGQUALITY, 75) ||
        !TIFFSetField(tif, TIFFTAG_ROWSPERSTRIP, IMAGE_HEIGHT))
    {
        TIFFClose(tif);
        return fail("failed to set JPEG output tags");
    }

    for (y = 0; y < IMAGE_HEIGHT; ++y)
    {
        fill_rgb_row(row, y);
        if (TIFFWriteScanline(tif, row, y, 0) != 1)
        {
            TIFFClose(tif);
            return fail("TIFFWriteScanline failed for JPEG output");
        }
    }

    TIFFClose(tif);
    return 1;
}

static int read_jpeg_tiff(const char *path)
{
    TIFF *tif;
    uint16_t compression = 0;
    uint32_t rows_per_strip = 0;
    uint32_t *raster;
    int ok;

    tif = TIFFOpen(path, "r");
    if (tif == NULL)
        return fail("failed to reopen JPEG output");
    if (!TIFFGetField(tif, TIFFTAG_COMPRESSION, &compression) ||
        compression != COMPRESSION_JPEG)
    {
        TIFFClose(tif);
        return fail("JPEG output did not preserve Compression=JPEG");
    }
    if (!TIFFGetField(tif, TIFFTAG_ROWSPERSTRIP, &rows_per_strip) ||
        rows_per_strip != IMAGE_HEIGHT)
    {
        TIFFClose(tif);
        return fail("JPEG output did not preserve RowsPerStrip");
    }

    raster = (uint32_t *)calloc(IMAGE_WIDTH * IMAGE_HEIGHT, sizeof(uint32_t));
    if (raster == NULL)
    {
        TIFFClose(tif);
        return fail("failed to allocate JPEG RGBA raster");
    }
    ok = TIFFReadRGBAImageOriented(tif, IMAGE_WIDTH, IMAGE_HEIGHT, raster,
                                   ORIENTATION_TOPLEFT, 1);
    free(raster);
    TIFFClose(tif);
    return ok ? 1 : fail("TIFFReadRGBAImageOriented failed for JPEG output");
}

static void fill_tile(uint8_t *tile, uint8_t base)
{
    uint32_t i;
    for (i = 0; i < TILE_WIDTH * TILE_LENGTH; ++i)
    {
        tile[i * 3] = (uint8_t)(base + 1U);
        tile[i * 3 + 1] = (uint8_t)(base + 2U);
        tile[i * 3 + 2] = (uint8_t)(base + 3U);
    }
}

static int write_tiled_tiff(const char *path)
{
    TIFF *tif;
    uint8_t tile[TILE_WIDTH * TILE_LENGTH * 3U];
    uint32_t x;
    uint32_t y;

    tif = TIFFOpen(path, "w");
    if (tif == NULL)
        return fail("failed to open tiled output");
    if (!TIFFSetField(tif, TIFFTAG_IMAGEWIDTH, IMAGE_WIDTH) ||
        !TIFFSetField(tif, TIFFTAG_IMAGELENGTH, IMAGE_HEIGHT) ||
        !TIFFSetField(tif, TIFFTAG_BITSPERSAMPLE, 8) ||
        !TIFFSetField(tif, TIFFTAG_SAMPLESPERPIXEL, 3) ||
        !TIFFSetField(tif, TIFFTAG_PLANARCONFIG, PLANARCONFIG_CONTIG) ||
        !TIFFSetField(tif, TIFFTAG_PHOTOMETRIC, PHOTOMETRIC_RGB) ||
        !TIFFSetField(tif, TIFFTAG_COMPRESSION, COMPRESSION_NONE) ||
        !TIFFSetField(tif, TIFFTAG_TILEWIDTH, TILE_WIDTH) ||
        !TIFFSetField(tif, TIFFTAG_TILELENGTH, TILE_LENGTH))
    {
        TIFFClose(tif);
        return fail("failed to set tiled output tags");
    }

    for (y = 0; y < IMAGE_HEIGHT; y += TILE_LENGTH)
    {
        for (x = 0; x < IMAGE_WIDTH; x += TILE_WIDTH)
        {
            fill_tile(tile, (uint8_t)(x + y));
            if (TIFFWriteTile(tif, tile, x, y, 0, 0) != (tmsize_t)sizeof(tile))
            {
                TIFFClose(tif);
                return fail("TIFFWriteTile failed");
            }
        }
    }
    TIFFClose(tif);
    return 1;
}

static int read_tiled_tiff(const char *path)
{
    TIFF *tif;
    uint32_t tile_width = 0;
    uint32_t tile_length = 0;
    uint32_t rows_per_strip = 0;

    tif = TIFFOpen(path, "r");
    if (tif == NULL)
        return fail("failed to reopen tiled output");
    if (!TIFFIsTiled(tif) ||
        !TIFFGetField(tif, TIFFTAG_TILEWIDTH, &tile_width) ||
        !TIFFGetField(tif, TIFFTAG_TILELENGTH, &tile_length) ||
        tile_width != TILE_WIDTH || tile_length != TILE_LENGTH)
    {
        TIFFClose(tif);
        return fail("tiled output did not preserve tile geometry");
    }
    if (TIFFGetField(tif, TIFFTAG_ROWSPERSTRIP, &rows_per_strip))
    {
        TIFFClose(tif);
        return fail("tiled output unexpectedly stored RowsPerStrip");
    }
    TIFFClose(tif);
    return 1;
}

int main(void)
{
    const char *jpeg_path = "o-validator-usage-jpeg-encode.tiff";
    const char *tile_path = "o-validator-usage-tiled-no-rowsperstrip.tiff";

    remove(jpeg_path);
    remove(tile_path);
    if (!write_jpeg_tiff(jpeg_path) || !read_jpeg_tiff(jpeg_path) ||
        !write_tiled_tiff(tile_path) || !read_tiled_tiff(tile_path))
        return EXIT_FAILURE;
    remove(jpeg_path);
    remove(tile_path);
    return EXIT_SUCCESS;
}
