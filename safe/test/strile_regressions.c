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

static int host_is_big_endian(void)
{
    const uint16_t value = 0x0102;
    return ((const unsigned char *)&value)[0] == 0x01;
}

static uint16_t read_u16_value(const unsigned char *bytes, int big_endian)
{
    if (big_endian)
        return (uint16_t)(((uint16_t)bytes[0] << 8) | bytes[1]);
    return (uint16_t)(((uint16_t)bytes[1] << 8) | bytes[0]);
}

static uint32_t read_u32_value(const unsigned char *bytes, int big_endian)
{
    if (big_endian)
    {
        return ((uint32_t)bytes[0] << 24) | ((uint32_t)bytes[1] << 16) |
               ((uint32_t)bytes[2] << 8) | bytes[3];
    }
    return ((uint32_t)bytes[3] << 24) | ((uint32_t)bytes[2] << 16) |
           ((uint32_t)bytes[1] << 8) | bytes[0];
}

static uint64_t read_first_ifd_offset(FILE *file, int *big_endian)
{
    unsigned char header[8];

    expect(fseek(file, 0, SEEK_SET) == 0, "failed to seek TIFF header");
    expect(fread(header, 1, sizeof(header), file) == sizeof(header),
           "failed to read TIFF header");
    expect((header[0] == 'I' && header[1] == 'I') ||
               (header[0] == 'M' && header[1] == 'M'),
           "invalid TIFF byte order");

    *big_endian = header[0] == 'M';
    expect(read_u16_value(header + 2, *big_endian) == 42,
           "expected a Classic TIFF fixture");
    return read_u32_value(header + 4, *big_endian);
}

static int find_classic_ifd_entry(FILE *file, uint64_t ifd_offset, uint16_t tag,
                                  int big_endian, uint16_t *type,
                                  uint32_t *count, uint32_t *value)
{
    unsigned char count_bytes[2];
    uint16_t entry_count = 0;

    expect(fseek(file, (long)ifd_offset, SEEK_SET) == 0,
           "failed to seek classic IFD");
    expect(fread(count_bytes, 1, sizeof(count_bytes), file) == sizeof(count_bytes),
           "failed to read classic IFD entry count");
    entry_count = read_u16_value(count_bytes, big_endian);
    for (uint16_t i = 0; i < entry_count; ++i)
    {
        unsigned char entry[12];

        expect(fread(entry, 1, sizeof(entry), file) == sizeof(entry),
               "failed to read classic IFD entry");
        if (read_u16_value(entry, big_endian) != tag)
            continue;

        *type = read_u16_value(entry + 2, big_endian);
        *count = read_u32_value(entry + 4, big_endian);
        *value = read_u32_value(entry + 8, big_endian);
        return 1;
    }

    return 0;
}

static void set_basic_strip_fields(TIFF *tif, uint32_t height)
{
    expect(TIFFSetField(tif, TIFFTAG_COMPRESSION, COMPRESSION_NONE) == 1,
           "failed to set Compression");
    expect(TIFFSetField(tif, TIFFTAG_IMAGEWIDTH, (uint32_t)1) == 1,
           "failed to set ImageWidth");
    expect(TIFFSetField(tif, TIFFTAG_IMAGELENGTH, height) == 1,
           "failed to set ImageLength");
    expect(TIFFSetField(tif, TIFFTAG_BITSPERSAMPLE, 8) == 1,
           "failed to set BitsPerSample");
    expect(TIFFSetField(tif, TIFFTAG_SAMPLESPERPIXEL, 1) == 1,
           "failed to set SamplesPerPixel");
    expect(TIFFSetField(tif, TIFFTAG_PLANARCONFIG, PLANARCONFIG_CONTIG) == 1,
           "failed to set PlanarConfiguration");
    expect(TIFFSetField(tif, TIFFTAG_PHOTOMETRIC, PHOTOMETRIC_MINISBLACK) == 1,
           "failed to set Photometric");
    expect(TIFFSetField(tif, TIFFTAG_ROWSPERSTRIP, (uint32_t)1) == 1,
           "failed to set RowsPerStrip");
}

static void write_fixture(const char *path, const char *mode, uint16_t bits_per_sample,
                          uint16_t fill_order, uint16_t sample_format,
                          const void *tile_data, size_t tile_size)
{
    TIFF *tif = TIFFOpen(path, mode);

    expect(tif != NULL, "TIFFOpen failed for strile regression fixture");
    expect(TIFFSetField(tif, TIFFTAG_IMAGEWIDTH, (uint32_t)16) == 1,
           "failed to set ImageWidth");
    expect(TIFFSetField(tif, TIFFTAG_IMAGELENGTH, (uint32_t)16) == 1,
           "failed to set ImageLength");
    expect(TIFFSetField(tif, TIFFTAG_BITSPERSAMPLE, bits_per_sample) == 1,
           "failed to set BitsPerSample");
    expect(TIFFSetField(tif, TIFFTAG_SAMPLESPERPIXEL, 1) == 1,
           "failed to set SamplesPerPixel");
    expect(TIFFSetField(tif, TIFFTAG_SAMPLEFORMAT, sample_format) == 1,
           "failed to set SampleFormat");
    expect(TIFFSetField(tif, TIFFTAG_COMPRESSION, COMPRESSION_NONE) == 1,
           "failed to set Compression");
    expect(TIFFSetField(tif, TIFFTAG_PLANARCONFIG, PLANARCONFIG_CONTIG) == 1,
           "failed to set PlanarConfiguration");
    expect(TIFFSetField(tif, TIFFTAG_PHOTOMETRIC, PHOTOMETRIC_MINISBLACK) == 1,
           "failed to set Photometric");
    expect(TIFFSetField(tif, TIFFTAG_FILLORDER, fill_order) == 1,
           "failed to set FillOrder");
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

static void run_fillorder_regression(void)
{
    char path[] = "strile_fillorderXXXXXX";
    unsigned char tile[16 * 16];
    unsigned char raw_tile[sizeof(tile)];
    unsigned char decoded_tile[sizeof(tile)];
    unsigned char readback_tile[sizeof(tile)];
    FILE *raw_file = NULL;
    TIFF *tif = NULL;
    uint64_t strile_offset = 0;
    uint64_t strile_size = 0;
    int fd;
    int err = 0;

    memset(tile, 0, sizeof(tile));
    tile[0] = 0x16;
    tile[1] = 0x80;
    for (size_t i = 2; i < sizeof(tile); ++i)
        tile[i] = (unsigned char)i;

    fd = mkstemp(path);
    if (fd < 0)
        fail("mkstemp failed for fill-order regression");
    close(fd);
    unlink(path);

    write_fixture(path, "w+", 8, FILLORDER_LSB2MSB, SAMPLEFORMAT_UINT, tile,
                  sizeof(tile));

    tif = TIFFOpen(path, "r");
    expect(tif != NULL, "failed to reopen fill-order regression fixture");
    expect(TIFFReadTile(tif, readback_tile, 0, 0, 0, 0) == (tmsize_t)sizeof(tile),
           "TIFFReadTile failed for fill-order regression");
    expect(memcmp(readback_tile, tile, sizeof(tile)) == 0,
           "TIFFReadTile lost fill-order semantics");

    strile_offset = TIFFGetStrileOffsetWithErr(tif, 0, &err);
    expect(err == 0 && strile_offset != 0,
           "TIFFGetStrileOffsetWithErr failed for fill-order regression");
    strile_size = TIFFGetStrileByteCountWithErr(tif, 0, &err);
    expect(err == 0 && strile_size == sizeof(tile),
           "TIFFGetStrileByteCountWithErr failed for fill-order regression");

    raw_file = fopen(path, "rb");
    expect(raw_file != NULL, "failed to open fill-order regression raw bytes");
    expect(fseek(raw_file, (long)strile_offset, SEEK_SET) == 0,
           "failed to seek fill-order raw bytes");
    expect(fread(raw_tile, 1, sizeof(raw_tile), raw_file) == sizeof(raw_tile),
           "failed to read fill-order raw bytes");
    fclose(raw_file);
    raw_file = NULL;

    expect(raw_tile[0] == 0x68 && raw_tile[1] == 0x01,
           "tile payload was not bit-reversed on write");

    memset(decoded_tile, 0, sizeof(decoded_tile));
    expect(TIFFReadFromUserBuffer(tif, 0, raw_tile, (tmsize_t)strile_size,
                                  decoded_tile,
                                  (tmsize_t)sizeof(decoded_tile)) == 1,
           "TIFFReadFromUserBuffer failed for fill-order regression");
    expect(memcmp(decoded_tile, tile, sizeof(decoded_tile)) == 0,
           "TIFFReadFromUserBuffer lost fill-order semantics");

    TIFFClose(tif);
    unlink(path);
}

static void run_byteswap_regression(void)
{
    char path[] = "strile_swabXXXXXX";
    uint16_t tile[16 * 16];
    uint16_t decoded_tile[16 * 16];
    uint16_t readback_tile[16 * 16];
    unsigned char raw_tile[sizeof(tile)];
    FILE *raw_file = NULL;
    TIFF *tif = NULL;
    uint64_t strile_offset = 0;
    uint64_t strile_size = 0;
    const char *mode = host_is_big_endian() ? "wl" : "wb";
    const unsigned char *host_first_sample = (const unsigned char *)&tile[0];
    int fd;
    int err = 0;

    for (size_t i = 0; i < sizeof(tile) / sizeof(tile[0]); ++i)
        tile[i] = (uint16_t)(0x1200U + (uint16_t)i);

    fd = mkstemp(path);
    if (fd < 0)
        fail("mkstemp failed for byte-swap regression");
    close(fd);
    unlink(path);

    write_fixture(path, mode, 16, FILLORDER_MSB2LSB, SAMPLEFORMAT_UINT, tile,
                  sizeof(tile));

    tif = TIFFOpen(path, "r");
    expect(tif != NULL, "failed to reopen byte-swap regression fixture");
    expect(TIFFReadTile(tif, readback_tile, 0, 0, 0, 0) ==
               (tmsize_t)sizeof(tile),
           "TIFFReadTile failed for byte-swap regression");
    expect(memcmp(readback_tile, tile, sizeof(tile)) == 0,
           "TIFFReadTile lost byte-swap semantics");

    strile_offset = TIFFGetStrileOffsetWithErr(tif, 0, &err);
    expect(err == 0 && strile_offset != 0,
           "TIFFGetStrileOffsetWithErr failed for byte-swap regression");
    strile_size = TIFFGetStrileByteCountWithErr(tif, 0, &err);
    expect(err == 0 && strile_size == sizeof(tile),
           "TIFFGetStrileByteCountWithErr failed for byte-swap regression");

    raw_file = fopen(path, "rb");
    expect(raw_file != NULL, "failed to open byte-swap regression raw bytes");
    expect(fseek(raw_file, (long)strile_offset, SEEK_SET) == 0,
           "failed to seek byte-swap raw bytes");
    expect(fread(raw_tile, 1, sizeof(raw_tile), raw_file) == sizeof(raw_tile),
           "failed to read byte-swap raw bytes");
    fclose(raw_file);
    raw_file = NULL;

    expect(raw_tile[0] == host_first_sample[1] &&
               raw_tile[1] == host_first_sample[0],
           "tile payload was not byte-swapped on write");

    memset(decoded_tile, 0, sizeof(decoded_tile));
    expect(TIFFReadFromUserBuffer(tif, 0, raw_tile, (tmsize_t)strile_size,
                                  decoded_tile,
                                  (tmsize_t)sizeof(decoded_tile)) == 1,
           "TIFFReadFromUserBuffer failed for byte-swap regression");
    expect(memcmp(decoded_tile, tile, sizeof(decoded_tile)) == 0,
           "TIFFReadFromUserBuffer lost byte-swap semantics");

    TIFFClose(tif);
    unlink(path);
}

static void run_deferred_flush_regression(void)
{
    char path[] = "strile_deferXXXXXX";
    unsigned char strips[2] = {17, 99};
    FILE *raw_file = NULL;
    TIFF *tif = NULL;
    uint64_t ifd_offset = 0;
    uint64_t dir_offset = 0;
    uint16_t type = 0;
    uint32_t count = 0;
    uint32_t value = 0;
    unsigned char decoded = 0;
    int big_endian = 0;
    int fd;

    fd = mkstemp(path);
    if (fd < 0)
        fail("mkstemp failed for deferred strile regression");
    close(fd);
    unlink(path);

    tif = TIFFOpen(path, "w+");
    expect(tif != NULL, "failed to open deferred strile regression fixture");

    set_basic_strip_fields(tif, 2);
    expect(TIFFDeferStrileArrayWriting(tif) == 1,
           "TIFFDeferStrileArrayWriting failed for first directory");
    expect(TIFFWriteCheck(tif, 0, "strile_regressions") == 1,
           "TIFFWriteCheck failed for deferred first directory");
    expect(TIFFWriteDirectory(tif) == 1,
           "TIFFWriteDirectory failed for deferred first directory");

    set_basic_strip_fields(tif, 1);
    expect(TIFFSetField(tif, TIFFTAG_SUBFILETYPE, FILETYPE_PAGE) == 1,
           "failed to set SubFileType on second directory");
    expect(TIFFDeferStrileArrayWriting(tif) == 1,
           "TIFFDeferStrileArrayWriting failed for second directory");
    expect(TIFFWriteCheck(tif, 0, "strile_regressions") == 1,
           "TIFFWriteCheck failed for deferred second directory");
    expect(TIFFWriteDirectory(tif) == 1,
           "TIFFWriteDirectory failed for deferred second directory");

    raw_file = fopen(path, "rb");
    expect(raw_file != NULL, "failed to open deferred fixture raw bytes");
    ifd_offset = read_first_ifd_offset(raw_file, &big_endian);
    expect(find_classic_ifd_entry(raw_file, ifd_offset, TIFFTAG_STRIPOFFSETS,
                                  big_endian, &type, &count, &value) == 1,
           "missing StripOffsets entry in deferred directory");
    expect(type == 0 && count == 0 && value == 0,
           "deferred StripOffsets entry was not written as a dummy");
    expect(find_classic_ifd_entry(raw_file, ifd_offset, TIFFTAG_STRIPBYTECOUNTS,
                                  big_endian, &type, &count, &value) == 1,
           "missing StripByteCounts entry in deferred directory");
    expect(type == 0 && count == 0 && value == 0,
           "deferred StripByteCounts entry was not written as a dummy");
    fclose(raw_file);
    raw_file = NULL;

    expect(TIFFSetDirectory(tif, 0) == 1,
           "failed to reload first deferred directory");
    dir_offset = TIFFCurrentDirOffset(tif);
    expect(dir_offset != 0, "first deferred directory has no on-disk offset");
    expect(TIFFForceStrileArrayWriting(tif) == 1,
           "TIFFForceStrileArrayWriting failed for first directory");
    expect(TIFFCurrentDirOffset(tif) == dir_offset,
           "TIFFForceStrileArrayWriting rewrote the whole first directory");

    raw_file = fopen(path, "rb");
    expect(raw_file != NULL, "failed to reopen deferred fixture raw bytes");
    ifd_offset = read_first_ifd_offset(raw_file, &big_endian);
    expect(find_classic_ifd_entry(raw_file, ifd_offset, TIFFTAG_STRIPOFFSETS,
                                  big_endian, &type, &count, &value) == 1,
           "missing rewritten StripOffsets entry");
    expect(type != 0 && count == 2,
           "TIFFForceStrileArrayWriting did not patch StripOffsets in place");
    expect(find_classic_ifd_entry(raw_file, ifd_offset, TIFFTAG_STRIPBYTECOUNTS,
                                  big_endian, &type, &count, &value) == 1,
           "missing rewritten StripByteCounts entry");
    expect(type != 0 && count == 2,
           "TIFFForceStrileArrayWriting did not patch StripByteCounts in place");
    fclose(raw_file);
    raw_file = NULL;

    expect(TIFFSetDirectory(tif, 1) == 1,
           "failed to reload second deferred directory");
    dir_offset = TIFFCurrentDirOffset(tif);
    expect(dir_offset != 0, "second deferred directory has no on-disk offset");
    expect(TIFFForceStrileArrayWriting(tif) == 1,
           "TIFFForceStrileArrayWriting failed for second directory");
    expect(TIFFCurrentDirOffset(tif) == dir_offset,
           "TIFFForceStrileArrayWriting rewrote the whole second directory");

    expect(TIFFSetDirectory(tif, 0) == 1,
           "failed to return to first deferred directory");
    expect(TIFFWriteEncodedStrip(tif, 0, &strips[0], 1) == 1,
           "TIFFWriteEncodedStrip failed for strip 0");
    expect(TIFFWriteEncodedStrip(tif, 1, &strips[1], 1) == 1,
           "TIFFWriteEncodedStrip failed for strip 1");
    dir_offset = TIFFCurrentDirOffset(tif);
    expect(TIFFFlush(tif) == 1, "TIFFFlush failed for deferred strip update");
    expect(TIFFCurrentDirOffset(tif) == dir_offset,
           "TIFFFlush rewrote the directory instead of forcing strile arrays");
    TIFFClose(tif);

    tif = TIFFOpen(path, "r");
    expect(tif != NULL, "failed to reopen deferred fixture for verification");
    expect(TIFFReadEncodedStrip(tif, 0, &decoded, 1) == 1,
           "failed to read first deferred strip");
    expect(decoded == strips[0], "first deferred strip has wrong value");
    expect(TIFFReadEncodedStrip(tif, 1, &decoded, 1) == 1,
           "failed to read second deferred strip");
    expect(decoded == strips[1], "second deferred strip has wrong value");
    TIFFClose(tif);
    unlink(path);
}

int main(void)
{
    char path[] = "strile_regressionsXXXXXX";
    unsigned char tile[16 * 16];
    unsigned char raw_tile[sizeof(tile)];
    unsigned char decoded_tile[sizeof(tile)];
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

    write_fixture(path, "w+", 8, FILLORDER_MSB2LSB, SAMPLEFORMAT_UINT, tile,
                  sizeof(tile));

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

    expect(TIFFReadFromUserBuffer(tif, 0, raw_tile,
                                  (tmsize_t)(strile_size - 1), decoded_tile,
                                  (tmsize_t)sizeof(decoded_tile)) == 0,
           "TIFFReadFromUserBuffer accepted truncated input");

    memset(decoded_tile, 0, sizeof(decoded_tile));
    expect(TIFFReadFromUserBuffer(tif, 0, raw_tile, (tmsize_t)strile_size,
                                  decoded_tile,
                                  (tmsize_t)sizeof(decoded_tile)) == 1,
           "TIFFReadFromUserBuffer failed for a full tile");
    expect(memcmp(decoded_tile, tile, sizeof(decoded_tile)) == 0,
           "TIFFReadFromUserBuffer returned unexpected bytes");

    TIFFClose(tif);
    unlink(path);

    run_fillorder_regression();
    run_byteswap_regression();
    run_deferred_flush_regression();
    return 0;
}
