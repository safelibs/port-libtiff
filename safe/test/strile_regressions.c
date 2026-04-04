/*
 * Regression coverage for strile edge cases and user-buffer decoding.
 */

#include "tif_config.h"

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifdef HAVE_UNISTD_H
#include <unistd.h>
#endif

#include "tiffio.h"

static void fail(const char *message)
{
    fprintf(stderr, "%s\n", message);
    exit(1);
}

static void expect(int condition, const char *message)
{
    if (!condition)
        fail(message);
}

static void write_fixture(const char *path, const unsigned char *tile_data,
                          size_t tile_size)
{
    TIFF *tif = TIFFOpen(path, "w+");

    expect(tif != NULL, "TIFFOpen failed for strile regression fixture");
    expect(TIFFSetField(tif, TIFFTAG_IMAGEWIDTH, (uint32_t)16) == 1,
           "failed to set ImageWidth");
    expect(TIFFSetField(tif, TIFFTAG_IMAGELENGTH, (uint32_t)16) == 1,
           "failed to set ImageLength");
    expect(TIFFSetField(tif, TIFFTAG_BITSPERSAMPLE, 8) == 1,
           "failed to set BitsPerSample");
    expect(TIFFSetField(tif, TIFFTAG_SAMPLESPERPIXEL, 1) == 1,
           "failed to set SamplesPerPixel");
    expect(TIFFSetField(tif, TIFFTAG_COMPRESSION, COMPRESSION_NONE) == 1,
           "failed to set Compression");
    expect(TIFFSetField(tif, TIFFTAG_PLANARCONFIG, PLANARCONFIG_CONTIG) == 1,
           "failed to set PlanarConfiguration");
    expect(TIFFSetField(tif, TIFFTAG_PHOTOMETRIC, PHOTOMETRIC_MINISBLACK) == 1,
           "failed to set Photometric");
    expect(TIFFSetField(tif, TIFFTAG_TILEWIDTH, (uint32_t)16) == 1,
           "failed to set TileWidth");
    expect(TIFFSetField(tif, TIFFTAG_TILELENGTH, (uint32_t)16) == 1,
           "failed to set TileLength");
    expect(TIFFWriteTile(tif, (void *)tile_data, 0, 0, 0, 0) ==
               (tmsize_t)tile_size,
           "TIFFWriteTile failed");
    expect(TIFFFlushData(tif) == 1, "TIFFFlushData failed");
    expect(TIFFFlush(tif) == 1, "TIFFFlush failed");
    TIFFClose(tif);
}

int main(void)
{
    char path[] = "strile_regressionsXXXXXX";
    unsigned char tile[16 * 16];
    unsigned char raw_tile[sizeof(tile)];
    unsigned char decoded_tile[sizeof(tile)];
    unsigned char small_buffer[8];
    FILE *raw_file = NULL;
    TIFF *tif = NULL;
    uint64_t strile_offset = 0;
    uint64_t strile_size = 0;
    int fd;
    int err = 0;

    for (size_t i = 0; i < sizeof(tile); ++i)
        tile[i] = (unsigned char)(255U - (unsigned char)i);

    fd = mkstemp(path);
    if (fd < 0)
        fail("mkstemp failed");
    close(fd);
    unlink(path);

    write_fixture(path, tile, sizeof(tile));

    tif = TIFFOpen(path, "r");
    expect(tif != NULL, "failed to reopen strile regression fixture");

    expect(TIFFCheckTile(tif, 0, 0, 0, 0) == 1,
           "TIFFCheckTile rejected an in-range tile");
    expect(TIFFCheckTile(tif, 16, 0, 0, 0) == 0,
           "TIFFCheckTile accepted an out-of-range column");
    expect(TIFFCheckTile(tif, 0, 16, 0, 0) == 0,
           "TIFFCheckTile accepted an out-of-range row");

    strile_offset = TIFFGetStrileOffsetWithErr(tif, 0, &err);
    expect(err == 0 && strile_offset != 0,
           "TIFFGetStrileOffsetWithErr failed for tile 0");
    strile_size = TIFFGetStrileByteCountWithErr(tif, 0, &err);
    expect(err == 0 && strile_size == sizeof(tile),
           "TIFFGetStrileByteCountWithErr failed for tile 0");

    expect(TIFFGetStrileOffsetWithErr(tif, 1, &err) == 0 && err == 1,
           "TIFFGetStrileOffsetWithErr accepted an invalid strile");
    expect(TIFFGetStrileByteCountWithErr(tif, 1, &err) == 0 && err == 1,
           "TIFFGetStrileByteCountWithErr accepted an invalid strile");

    raw_file = fopen(path, "rb");
    expect(raw_file != NULL, "failed to open raw regression fixture");
    expect(fseek(raw_file, (long)strile_offset, SEEK_SET) == 0,
           "failed to seek to raw tile data");
    expect(fread(raw_tile, 1, sizeof(raw_tile), raw_file) == sizeof(raw_tile),
           "failed to read raw tile payload");
    fclose(raw_file);
    raw_file = NULL;

    expect(TIFFReadFromUserBuffer(tif, 0, raw_tile, (tmsize_t)strile_size,
                                  small_buffer,
                                  (tmsize_t)sizeof(small_buffer)) == 0,
           "TIFFReadFromUserBuffer accepted an undersized output buffer");

    memset(decoded_tile, 0, sizeof(decoded_tile));
    expect(TIFFReadFromUserBuffer(tif, 0, raw_tile, (tmsize_t)strile_size,
                                  decoded_tile,
                                  (tmsize_t)sizeof(decoded_tile)) == 1,
           "TIFFReadFromUserBuffer failed for a full tile");
    expect(memcmp(decoded_tile, tile, sizeof(decoded_tile)) == 0,
           "TIFFReadFromUserBuffer returned unexpected bytes");

    TIFFClose(tif);
    unlink(path);
    return 0;
}
