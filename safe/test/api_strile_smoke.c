/*
 * Smoke coverage for strile I/O helpers, public swab helpers, and flush
 * behavior without depending on RGBA readers.
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

static void expect_bytes(const void *lhs, const void *rhs, size_t size,
                         const char *message)
{
    if (memcmp(lhs, rhs, size) != 0)
        fail(message);
}

static void check_public_helpers(void)
{
    const unsigned char *identity = TIFFGetBitRevTable(0);
    const unsigned char *reversed = TIFFGetBitRevTable(1);
    uint8_t bit_buffer[2] = {0x16, 0x80};
    uint16_t short_value = 0x1234;
    uint16_t short_array[2] = {0x1234, 0xABCD};
    uint8_t triples[6] = {0, 1, 2, 3, 4, 5};
    uint32_t long_value = 0x11223344U;
    uint32_t long_array[2] = {0x11223344U, 0x55667788U};
    uint64_t long8_value = 0x0102030405060708ULL;
    uint64_t long8_array[2] = {
        0x0102030405060708ULL,
        0x1112131415161718ULL,
    };
    union
    {
        uint32_t u;
        float f;
    } float_value = {0x11223344U}, float_array[2] = {{0x11223344U}, {0x55667788U}};
    union
    {
        uint64_t u;
        double d;
    } double_value = {0x0102030405060708ULL},
      double_array[2] = {{0x0102030405060708ULL}, {0x1112131415161718ULL}};

    expect(identity != NULL && reversed != NULL, "bit-reversal tables must exist");
    expect(identity[0x16] == 0x16, "identity bit-reversal table is wrong");
    expect(reversed[0x16] == 0x68, "reversed bit-reversal table is wrong");

    TIFFReverseBits(bit_buffer, 2);
    expect(bit_buffer[0] == 0x68 && bit_buffer[1] == 0x01,
           "TIFFReverseBits produced an unexpected result");

    TIFFSwabShort(&short_value);
    expect(short_value == 0x3412, "TIFFSwabShort produced an unexpected result");

    TIFFSwabArrayOfShort(short_array, 2);
    expect(short_array[0] == 0x3412 && short_array[1] == 0xCDAB,
           "TIFFSwabArrayOfShort produced an unexpected result");

    TIFFSwabArrayOfTriples(triples, 2);
    expect(triples[0] == 2 && triples[1] == 1 && triples[2] == 0 &&
               triples[3] == 5 && triples[4] == 4 && triples[5] == 3,
           "TIFFSwabArrayOfTriples produced an unexpected result");

    TIFFSwabLong(&long_value);
    expect(long_value == 0x44332211U,
           "TIFFSwabLong produced an unexpected result");

    TIFFSwabArrayOfLong(long_array, 2);
    expect(long_array[0] == 0x44332211U && long_array[1] == 0x88776655U,
           "TIFFSwabArrayOfLong produced an unexpected result");

    TIFFSwabLong8(&long8_value);
    expect(long8_value == 0x0807060504030201ULL,
           "TIFFSwabLong8 produced an unexpected result");

    TIFFSwabArrayOfLong8(long8_array, 2);
    expect(long8_array[0] == 0x0807060504030201ULL &&
               long8_array[1] == 0x1817161514131211ULL,
           "TIFFSwabArrayOfLong8 produced an unexpected result");

    TIFFSwabFloat(&float_value.f);
    expect(float_value.u == 0x44332211U,
           "TIFFSwabFloat produced an unexpected result");

    TIFFSwabArrayOfFloat(&float_array[0].f, 2);
    expect(float_array[0].u == 0x44332211U && float_array[1].u == 0x88776655U,
           "TIFFSwabArrayOfFloat produced an unexpected result");

    TIFFSwabDouble(&double_value.d);
    expect(double_value.u == 0x0807060504030201ULL,
           "TIFFSwabDouble produced an unexpected result");

    TIFFSwabArrayOfDouble(&double_array[0].d, 2);
    expect(double_array[0].u == 0x0807060504030201ULL &&
               double_array[1].u == 0x1817161514131211ULL,
           "TIFFSwabArrayOfDouble produced an unexpected result");
}

int main(void)
{
    char path[] = "api_strile_smokeXXXXXX";
    unsigned char tile[16 * 16];
    unsigned char decoded_tile[sizeof(tile)];
    unsigned char tile_from_user_buffer[sizeof(tile)];
    uint64_t strile_offset = 0;
    uint64_t strile_size = 0;
    uint32_t width = 0;
    uint32_t height = 0;
    char *page_name = NULL;
    FILE *raw_file = NULL;
    TIFF *tif = NULL;
    int fd;
    int err = 0;

    for (size_t i = 0; i < sizeof(tile); ++i)
        tile[i] = (unsigned char)i;

    check_public_helpers();

    fd = mkstemp(path);
    if (fd < 0)
        fail("mkstemp failed");
    close(fd);
    unlink(path);

    tif = TIFFOpen(path, "w+");
    expect(tif != NULL, "TIFFOpen failed for strile smoke");
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
    expect(TIFFSetField(tif, TIFFTAG_PAGENAME, "api-strile-smoke") == 1,
           "failed to set PageName");

    expect(TIFFCheckTile(tif, 0, 0, 0, 0) == 1,
           "TIFFCheckTile rejected an in-range tile");
    expect(TIFFCheckTile(tif, 16, 0, 0, 0) == 0,
           "TIFFCheckTile accepted an out-of-range column");
    expect(TIFFCheckTile(tif, 0, 16, 0, 0) == 0,
           "TIFFCheckTile accepted an out-of-range row");

    expect(TIFFWriteTile(tif, tile, 0, 0, 0, 0) == (tmsize_t)sizeof(tile),
           "TIFFWriteTile failed");
    expect(TIFFFlushData(tif) == 1, "TIFFFlushData failed");
    expect(TIFFFlush(tif) == 1, "TIFFFlush failed");
    TIFFClose(tif);
    tif = NULL;

    tif = TIFFOpen(path, "r");
    expect(tif != NULL, "failed to reopen strile smoke output");
    expect(TIFFGetField(tif, TIFFTAG_IMAGEWIDTH, &width) == 1 && width == 16,
           "unexpected ImageWidth after reopen");
    expect(TIFFGetField(tif, TIFFTAG_IMAGELENGTH, &height) == 1 && height == 16,
           "unexpected ImageLength after reopen");
    expect(TIFFGetField(tif, TIFFTAG_PAGENAME, &page_name) == 1,
           "missing PageName after reopen");
    expect(strcmp(page_name, "api-strile-smoke") == 0,
           "unexpected PageName after reopen");
    expect(TIFFCheckTile(tif, 0, 0, 0, 0) == 1,
           "TIFFCheckTile failed after reopen");

    memset(decoded_tile, 0, sizeof(decoded_tile));
    expect(TIFFReadTile(tif, decoded_tile, 0, 0, 0, 0) == (tmsize_t)sizeof(tile),
           "TIFFReadTile failed");
    expect_bytes(decoded_tile, tile, sizeof(tile),
                 "TIFFReadTile returned unexpected bytes");

    strile_offset = TIFFGetStrileOffsetWithErr(tif, 0, &err);
    expect(err == 0 && strile_offset != 0,
           "TIFFGetStrileOffsetWithErr failed for tile 0");
    strile_size = TIFFGetStrileByteCountWithErr(tif, 0, &err);
    expect(err == 0 && strile_size == sizeof(tile),
           "TIFFGetStrileByteCountWithErr failed for tile 0");

    raw_file = fopen(path, "rb");
    expect(raw_file != NULL, "failed to open raw TIFF bytes");
    expect(fseek(raw_file, (long)strile_offset, SEEK_SET) == 0,
           "failed to seek to raw tile bytes");
    expect(fread(tile_from_user_buffer, 1, sizeof(tile_from_user_buffer),
                 raw_file) == sizeof(tile_from_user_buffer),
           "failed to read raw tile bytes");
    fclose(raw_file);
    raw_file = NULL;

    memset(decoded_tile, 0, sizeof(decoded_tile));
    expect(TIFFReadFromUserBuffer(tif, 0, tile_from_user_buffer,
                                  (tmsize_t)strile_size, decoded_tile,
                                  (tmsize_t)sizeof(decoded_tile)) == 1,
           "TIFFReadFromUserBuffer failed");
    expect_bytes(decoded_tile, tile, sizeof(tile),
                 "TIFFReadFromUserBuffer returned unexpected bytes");

    TIFFClose(tif);
    unlink(path);
    return 0;
}
