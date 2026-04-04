/*
 * Regression coverage for malformed directory and tag cases.
 */

#include "tif_config.h"

#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifdef HAVE_UNISTD_H
#include <unistd.h>
#endif

#include "tiffio.h"

typedef struct
{
    uint16_t tag;
    uint16_t type;
    uint64_t count;
    const uint8_t *data;
    size_t size;
} DirEntrySpec;

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

static void write_full(FILE *fp, const void *buffer, size_t size)
{
    if (fwrite(buffer, 1, size, fp) != size)
        fail("fwrite failed");
}

static void write_le16(FILE *fp, uint16_t value)
{
    uint8_t bytes[2] = {(uint8_t)(value & 0xffU), (uint8_t)(value >> 8)};
    write_full(fp, bytes, sizeof(bytes));
}

static void write_le32(FILE *fp, uint32_t value)
{
    uint8_t bytes[4] = {
        (uint8_t)(value & 0xffU),
        (uint8_t)((value >> 8) & 0xffU),
        (uint8_t)((value >> 16) & 0xffU),
        (uint8_t)((value >> 24) & 0xffU),
    };
    write_full(fp, bytes, sizeof(bytes));
}

static void write_le64(FILE *fp, uint64_t value)
{
    uint8_t bytes[8] = {
        (uint8_t)(value & 0xffU),
        (uint8_t)((value >> 8) & 0xffU),
        (uint8_t)((value >> 16) & 0xffU),
        (uint8_t)((value >> 24) & 0xffU),
        (uint8_t)((value >> 32) & 0xffU),
        (uint8_t)((value >> 40) & 0xffU),
        (uint8_t)((value >> 48) & 0xffU),
        (uint8_t)((value >> 56) & 0xffU),
    };
    write_full(fp, bytes, sizeof(bytes));
}

static size_t type_width(uint16_t type)
{
    switch (type)
    {
        case TIFF_BYTE:
        case TIFF_ASCII:
        case TIFF_SBYTE:
        case TIFF_UNDEFINED:
            return 1;
        case TIFF_SHORT:
        case TIFF_SSHORT:
            return 2;
        case TIFF_LONG:
        case TIFF_SLONG:
        case TIFF_FLOAT:
        case TIFF_IFD:
            return 4;
        case TIFF_RATIONAL:
        case TIFF_SRATIONAL:
        case TIFF_DOUBLE:
        case TIFF_LONG8:
        case TIFF_SLONG8:
        case TIFF_IFD8:
            return 8;
        default:
            return 0;
    }
}

static void write_padding(FILE *fp, uint64_t target_offset)
{
    long position = ftell(fp);
    if (position < 0)
        fail("ftell failed");
    while ((uint64_t)position < target_offset)
    {
        fputc(0, fp);
        ++position;
    }
}

static void write_tiff_file(const char *path, int big_tiff,
                            const DirEntrySpec *entries, size_t entry_count)
{
    FILE *fp;
    size_t i;
    const uint64_t header_size = big_tiff ? 16U : 8U;
    const uint64_t count_size = big_tiff ? 8U : 2U;
    const uint64_t entry_size = big_tiff ? 20U : 12U;
    const uint64_t next_size = big_tiff ? 8U : 4U;
    const uint64_t inline_size = big_tiff ? 8U : 4U;
    const uint64_t alignment = big_tiff ? 8U : 2U;
    uint64_t payload_offset =
        header_size + count_size + entry_size * entry_count + next_size;
    uint64_t entry_payload_offsets[16] = {0};

    if (entry_count > (sizeof(entry_payload_offsets) / sizeof(entry_payload_offsets[0])))
        fail("too many entries in regression writer");

    fp = fopen(path, "wb");
    if (fp == NULL)
        fail("fopen failed");

    for (i = 0; i < entry_count; ++i)
    {
        if (entries[i].size > inline_size)
        {
            payload_offset = (payload_offset + alignment - 1U) & ~(alignment - 1U);
            entry_payload_offsets[i] = payload_offset;
            payload_offset += entries[i].size;
        }
    }

    write_full(fp, "II", 2);
    write_le16(fp, big_tiff ? TIFF_VERSION_BIG : TIFF_VERSION_CLASSIC);
    if (big_tiff)
    {
        write_le16(fp, 8);
        write_le16(fp, 0);
        write_le64(fp, header_size);
        write_le64(fp, entry_count);
    }
    else
    {
        write_le32(fp, (uint32_t)header_size);
        write_le16(fp, (uint16_t)entry_count);
    }

    for (i = 0; i < entry_count; ++i)
    {
        uint8_t inline_bytes[8] = {0};
        size_t width = type_width(entries[i].type);
        uint64_t inline_value = entry_payload_offsets[i];

        if (width == 0)
            fail("unsupported entry type in writer");
        if (entries[i].count != 0 &&
            entries[i].size != entries[i].count * width)
            fail("entry size/count mismatch");

        write_le16(fp, entries[i].tag);
        write_le16(fp, entries[i].type);
        if (big_tiff)
            write_le64(fp, entries[i].count);
        else
            write_le32(fp, (uint32_t)entries[i].count);

        if (entries[i].size <= inline_size && entries[i].data != NULL &&
            entries[i].size > 0)
        {
            memcpy(inline_bytes, entries[i].data, entries[i].size);
            write_full(fp, inline_bytes, inline_size);
        }
        else if (big_tiff)
        {
            write_le64(fp, inline_value);
        }
        else
        {
            write_le32(fp, (uint32_t)inline_value);
        }
    }

    if (big_tiff)
        write_le64(fp, 0);
    else
        write_le32(fp, 0);

    for (i = 0; i < entry_count; ++i)
    {
        if (entry_payload_offsets[i] != 0)
        {
            write_padding(fp, entry_payload_offsets[i]);
            write_full(fp, entries[i].data, entries[i].size);
        }
    }

    fclose(fp);
}

static void capture_print_directory(TIFF *tif, long flags, char *buffer,
                                    size_t buffer_size)
{
    FILE *sink = tmpfile();
    size_t nread;

    if (sink == NULL)
        fail("tmpfile failed");

    TIFFPrintDirectory(tif, sink, flags);
    fflush(sink);
    rewind(sink);
    nread = fread(buffer, 1, buffer_size - 1, sink);
    buffer[nread] = '\0';
    fclose(sink);
}

int main(void)
{
    static const uint8_t u32_one[4] = {1, 0, 0, 0};
    static const uint8_t u32_zero[4] = {0, 0, 0, 0};
    static const uint8_t u32_seven[4] = {7, 0, 0, 0};
    static const uint8_t u32_sixtyfour[4] = {64, 0, 0, 0};
    static const uint8_t u64_eighty[8] = {128, 0, 0, 0, 0, 0, 0, 0};
    static const uint8_t page_unordered[] = "unordered";
    static const uint8_t zero_rational[8] = {1, 0, 0, 0, 0, 0, 0, 0};
    char path_unknown[] = "dirread_unknownXXXXXX";
    char path_zero_rational[] = "dirread_zero_rationalXXXXXX";
    char path_zero_ascii[] = "dirread_zero_asciiXXXXXX";
    char path_strip_missing[] = "dirread_strip_missingXXXXXX";
    char path_tile_missing[] = "dirread_tile_missingXXXXXX";
    char print_buffer[4096];
    TIFF *tif;
    const TIFFField *field;
    char *page_name = NULL;
    uint32_t anon_count = 0;
    const uint32_t *anon_values = NULL;
    float xres = 123.0f;
    uint16_t compression = 0;
    uint16_t fillorder = 0;
    uint16_t extrasamples = 99;
    const uint16_t *sampleinfo = (const uint16_t *)1;
    const uint64_t *offsets = NULL;
    const uint64_t *bytecounts = NULL;
    uint32_t tilewidth = 123;
    int fd;

    const DirEntrySpec unknown_entries[] = {
        {TIFFTAG_PAGENAME, TIFF_ASCII, sizeof(page_unordered), page_unordered,
         sizeof(page_unordered)},
        {65000, TIFF_LONG, 1, u32_seven, sizeof(u32_seven)},
        {TIFFTAG_IMAGEWIDTH, TIFF_LONG, 1, u32_one, sizeof(u32_one)},
        {TIFFTAG_IMAGELENGTH, TIFF_LONG, 1, u32_one, sizeof(u32_one)},
    };
    const DirEntrySpec zero_rational_entries[] = {
        {TIFFTAG_IMAGEWIDTH, TIFF_LONG, 1, u32_one, sizeof(u32_one)},
        {TIFFTAG_IMAGELENGTH, TIFF_LONG, 1, u32_one, sizeof(u32_one)},
        {TIFFTAG_XRESOLUTION, TIFF_RATIONAL, 1, zero_rational,
         sizeof(zero_rational)},
    };
    const DirEntrySpec zero_ascii_entries[] = {
        {TIFFTAG_PAGENAME, TIFF_ASCII, 0, NULL, 0},
        {TIFFTAG_IMAGEWIDTH, TIFF_LONG, 1, u32_one, sizeof(u32_one)},
        {TIFFTAG_IMAGELENGTH, TIFF_LONG, 1, u32_one, sizeof(u32_one)},
    };
    const DirEntrySpec strip_missing_entries[] = {
        {TIFFTAG_IMAGEWIDTH, TIFF_LONG, 1, u32_one, sizeof(u32_one)},
        {TIFFTAG_IMAGELENGTH, TIFF_LONG, 1, u32_one, sizeof(u32_one)},
        {TIFFTAG_ROWSPERSTRIP, TIFF_LONG, 1, u32_one, sizeof(u32_one)},
        {TIFFTAG_STRIPOFFSETS, TIFF_LONG, 1, u32_sixtyfour, sizeof(u32_sixtyfour)},
    };
    const DirEntrySpec tile_missing_entries[] = {
        {TIFFTAG_IMAGEWIDTH, TIFF_LONG, 1, u32_one, sizeof(u32_one)},
        {TIFFTAG_IMAGELENGTH, TIFF_LONG, 1, u32_one, sizeof(u32_one)},
        {TIFFTAG_TILEWIDTH, TIFF_LONG, 1, u32_zero, sizeof(u32_zero)},
        {TIFFTAG_TILELENGTH, TIFF_LONG, 1, u32_one, sizeof(u32_one)},
        {TIFFTAG_TILEOFFSETS, TIFF_LONG8, 1, u64_eighty, sizeof(u64_eighty)},
    };

    fd = mkstemp(path_unknown);
    expect(fd >= 0, "mkstemp failed for unknown-tag regression");
    close(fd);
    write_tiff_file(path_unknown, 0, unknown_entries,
                    sizeof(unknown_entries) / sizeof(unknown_entries[0]));

    tif = TIFFOpen(path_unknown, "r");
    expect(tif != NULL, "failed to open unknown-tag regression TIFF");
    expect(TIFFGetField(tif, TIFFTAG_PAGENAME, &page_name) == 1,
           "missing PageName in unknown-tag TIFF");
    expect(strcmp(page_name, "unordered") == 0,
           "unexpected PageName in unknown-tag TIFF");
    field = TIFFFieldWithTag(tif, 65000);
    expect(field != NULL, "anonymous field was not registered");
    expect(TIFFFieldIsAnonymous(field) == 1, "field should be anonymous");
    expect(TIFFGetField(tif, 65000, &anon_count, &anon_values) == 1,
           "failed to read anonymous LONG tag");
    expect(anon_count == 1, "unexpected anonymous LONG count");
    expect(anon_values != NULL && anon_values[0] == 7,
           "unexpected anonymous LONG value");
    capture_print_directory(tif, 0, print_buffer, sizeof(print_buffer));
    expect(strstr(print_buffer, "Tag 65000: 7") != NULL,
           "anonymous tag print output mismatch");
    expect(TIFFGetFieldDefaulted(tif, TIFFTAG_FILLORDER, &fillorder) == 1,
           "FillOrder default should be available");
    expect(fillorder == FILLORDER_MSB2LSB, "unexpected default FillOrder");
    TIFFClose(tif);
    unlink(path_unknown);

    fd = mkstemp(path_zero_rational);
    expect(fd >= 0, "mkstemp failed for zero-rational regression");
    close(fd);
    write_tiff_file(path_zero_rational, 0, zero_rational_entries,
                    sizeof(zero_rational_entries) / sizeof(zero_rational_entries[0]));

    tif = TIFFOpen(path_zero_rational, "r");
    expect(tif != NULL, "failed to open zero-rational TIFF");
    expect(TIFFGetField(tif, TIFFTAG_XRESOLUTION, &xres) == 0,
           "zero-denominator rational must be rejected");
    expect(TIFFGetFieldDefaulted(tif, TIFFTAG_EXTRASAMPLES, &extrasamples,
                                 &sampleinfo) == 1,
           "defaulted ExtraSamples should succeed");
    expect(extrasamples == 0 && sampleinfo == NULL,
           "unexpected default ExtraSamples state");
    TIFFClose(tif);
    unlink(path_zero_rational);

    fd = mkstemp(path_zero_ascii);
    expect(fd >= 0, "mkstemp failed for zero-ascii regression");
    close(fd);
    write_tiff_file(path_zero_ascii, 0, zero_ascii_entries,
                    sizeof(zero_ascii_entries) / sizeof(zero_ascii_entries[0]));

    tif = TIFFOpen(path_zero_ascii, "r");
    expect(tif != NULL, "failed to open zero-ascii TIFF");
    expect(TIFFGetField(tif, TIFFTAG_PAGENAME, &page_name) == 1,
           "zero-count ASCII tag should still be readable");
    expect(page_name != NULL && page_name[0] == '\0',
           "zero-count ASCII tag should become an empty string");
    expect(TIFFGetField(tif, TIFFTAG_COMPRESSION, &compression) == 1,
           "default Compression should be readable");
    expect(compression == COMPRESSION_NONE,
           "unexpected default Compression value");
    capture_print_directory(tif, 0, print_buffer, sizeof(print_buffer));
    expect(strstr(print_buffer, "Compression:") != NULL,
           "missing default Compression print output");
    TIFFClose(tif);
    unlink(path_zero_ascii);

    fd = mkstemp(path_strip_missing);
    expect(fd >= 0, "mkstemp failed for strip-missing regression");
    close(fd);
    write_tiff_file(path_strip_missing, 0, strip_missing_entries,
                    sizeof(strip_missing_entries) / sizeof(strip_missing_entries[0]));

    tif = TIFFOpen(path_strip_missing, "r");
    expect(tif != NULL, "failed to open missing-strip-bytecounts TIFF");
    expect(TIFFGetField(tif, TIFFTAG_STRIPOFFSETS, &offsets) == 1,
           "StripOffsets should be readable");
    expect(offsets != NULL && offsets[0] == 64, "unexpected StripOffsets value");
    expect(TIFFGetField(tif, TIFFTAG_STRIPBYTECOUNTS, &bytecounts) == 0,
           "missing StripByteCounts must not be synthesized");
    expect(TIFFGetFieldDefaulted(tif, TIFFTAG_YCBCRSUBSAMPLING, &extrasamples,
                                 &fillorder) == 1,
           "defaulted YCbCrSubsampling should succeed");
    expect(extrasamples == 2 && fillorder == 2,
           "unexpected default YCbCrSubsampling");
    capture_print_directory(tif, 0, print_buffer, sizeof(print_buffer));
    expect(strstr(print_buffer, "StripOffsets: 64") != NULL,
           "missing StripOffsets print output");
    TIFFClose(tif);
    unlink(path_strip_missing);

    fd = mkstemp(path_tile_missing);
    expect(fd >= 0, "mkstemp failed for tile-missing regression");
    close(fd);
    write_tiff_file(path_tile_missing, 1, tile_missing_entries,
                    sizeof(tile_missing_entries) / sizeof(tile_missing_entries[0]));

    tif = TIFFOpen(path_tile_missing, "r");
    expect(tif != NULL, "failed to open BigTIFF tile regression");
    expect(TIFFIsBigTIFF(tif), "generated tile regression file should be BigTIFF");
    expect(TIFFIsTiled(tif), "tile regression file should be treated as tiled");
    expect(TIFFGetField(tif, TIFFTAG_TILEWIDTH, &tilewidth) == 1,
           "TileWidth getter failed");
    expect(tilewidth == 0, "TileWidth should preserve zero value");
    expect(TIFFGetField(tif, TIFFTAG_TILEBYTECOUNTS, &bytecounts) == 0,
           "missing TileByteCounts must not be synthesized");
    capture_print_directory(tif, 0, print_buffer, sizeof(print_buffer));
    expect(strstr(print_buffer, "Tile Width: 0 Tile Length: 1") != NULL,
           "missing tile summary print output");
    TIFFClose(tif);
    unlink(path_tile_missing);

    return 0;
}
