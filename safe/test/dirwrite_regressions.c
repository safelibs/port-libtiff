/*
 * Regression coverage for directory-only write-path validation and failures.
 */

#include "tif_config.h"

#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifdef HAVE_UNISTD_H
#include <unistd.h>
#endif

#include "tif_dir.h"
#include "tiffio.h"

enum
{
    TIFFTAG_DIRWRITE_ANON = 65040
};

extern int _TIFFRewriteField(TIFF *tif, uint16_t tag, TIFFDataType in_datatype,
                             tmsize_t count, void *data);

typedef struct
{
    char message[512];
    char module[64];
} ErrorState;

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

static void write_minimal_anonymous_tiff(const char *path)
{
    FILE *fp = fopen(path, "wb");

    if (fp == NULL)
        fail("fopen failed");

    write_full(fp, "II", 2);
    write_le16(fp, TIFF_VERSION_CLASSIC);
    write_le32(fp, 8);

    write_le16(fp, 1);
    write_le16(fp, TIFFTAG_DIRWRITE_ANON);
    write_le16(fp, TIFF_LONG);
    write_le32(fp, 1);
    write_le32(fp, 7);
    write_le32(fp, 0);

    fclose(fp);
}

static int capture_error(TIFF *tif, void *user_data, const char *module,
                         const char *fmt, va_list ap)
{
    ErrorState *state = (ErrorState *)user_data;
    (void)tif;
    vsnprintf(state->message, sizeof(state->message), fmt, ap);
    snprintf(state->module, sizeof(state->module), "%s",
             module ? module : "");
    return 1;
}

static void clear_error(ErrorState *state)
{
    state->message[0] = '\0';
    state->module[0] = '\0';
}

static TIFF *open_with_error_handler(const char *path, const char *mode,
                                     ErrorState *state)
{
    TIFFOpenOptions *opts = TIFFOpenOptionsAlloc();
    TIFF *tif;

    if (opts == NULL)
        fail("TIFFOpenOptionsAlloc failed");

    TIFFOpenOptionsSetErrorHandlerExtR(opts, capture_error, state);
    tif = TIFFOpenExt(path, mode, opts);
    TIFFOpenOptionsFree(opts);
    return tif;
}

static void init_basic_main_directory(TIFF *tif)
{
    expect(TIFFCreateDirectory(tif) == 0, "TIFFCreateDirectory failed");
    expect(TIFFSetField(tif, TIFFTAG_IMAGEWIDTH, (uint32_t)1) == 1,
           "failed to set ImageWidth");
    expect(TIFFSetField(tif, TIFFTAG_IMAGELENGTH, (uint32_t)1) == 1,
           "failed to set ImageLength");
    expect(TIFFSetField(tif, TIFFTAG_BITSPERSAMPLE, 8) == 1,
           "failed to set BitsPerSample");
    expect(TIFFSetField(tif, TIFFTAG_SAMPLESPERPIXEL, 1) == 1,
           "failed to set SamplesPerPixel");
}

static void test_setfield_rational_validation(void)
{
    char path[] = "dirwrite_setfield_rationalXXXXXX";
    int fd = mkstemp(path);
    ErrorState error_state = {{0}};
    TIFF *tif;

    if (fd < 0)
        fail("mkstemp failed");
    close(fd);
    unlink(path);

    tif = open_with_error_handler(path, "w", &error_state);
    expect(tif != NULL, "failed to open rational validation output");
    init_basic_main_directory(tif);

    clear_error(&error_state);
    expect(TIFFSetField(tif, TIFFTAG_XRESOLUTION, -1.0) == 0,
           "negative XResolution should be rejected");
    expect(error_state.message[0] != '\0',
           "negative XResolution should report an error");

    TIFFClose(tif);
    unlink(path);
}

static void test_write_time_rational_validation(void)
{
    char path[] = "dirwrite_rational_writeXXXXXX";
    int fd = mkstemp(path);
    ErrorState error_state = {{0}};
    TIFF *tif;
    double latitude[3] = {1.0, NAN, 3.0};
    uint64_t offset = 0;

    if (fd < 0)
        fail("mkstemp failed");
    close(fd);
    unlink(path);

    tif = open_with_error_handler(path, "w+", &error_state);
    expect(tif != NULL, "failed to open GPS rational validation output");
    expect(TIFFCreateGPSDirectory(tif) == 0, "TIFFCreateGPSDirectory failed");
    expect(TIFFSetField(tif, GPSTAG_LATITUDE, latitude) == 1,
           "setting GPS latitude should defer validation until write");

    clear_error(&error_state);
    expect(TIFFWriteCustomDirectory(tif, &offset) == 0,
           "non-finite GPS rational data should fail at write time");
    expect(error_state.message[0] != '\0',
           "write-time rational failure should report an error");

    TIFFClose(tif);
    unlink(path);
}

static void test_transferfunction_validation(void)
{
    char path[] = "dirwrite_transferfunctionXXXXXX";
    int fd = mkstemp(path);
    ErrorState error_state = {{0}};
    TIFF *tif;
    uint16_t dummy = 0;

    if (fd < 0)
        fail("mkstemp failed");
    close(fd);
    unlink(path);

    tif = open_with_error_handler(path, "w", &error_state);
    expect(tif != NULL, "failed to open transferfunction validation output");
    expect(TIFFCreateDirectory(tif) == 0, "TIFFCreateDirectory failed");
    expect(TIFFSetField(tif, TIFFTAG_SAMPLESPERPIXEL, 3) == 1,
           "failed to set SamplesPerPixel");
    expect(TIFFSetField(tif, TIFFTAG_BITSPERSAMPLE, 31) == 1,
           "failed to set oversized BitsPerSample");

    clear_error(&error_state);
    expect(TIFFSetField(tif, TIFFTAG_TRANSFERFUNCTION, &dummy, &dummy, &dummy) == 0,
           "invalid TransferFunction should be rejected");
    expect(error_state.message[0] != '\0',
           "TransferFunction rejection should report an error");

    TIFFClose(tif);
    unlink(path);
}

static void test_allocation_cap_failure(void)
{
    char path[] = "dirwrite_alloc_capXXXXXX";
    int fd = mkstemp(path);
    TIFFOpenOptions *opts;
    ErrorState error_state = {{0}};
    TIFF *tif;

    if (fd < 0)
        fail("mkstemp failed");
    close(fd);
    unlink(path);

    opts = TIFFOpenOptionsAlloc();
    if (opts == NULL)
        fail("TIFFOpenOptionsAlloc failed");
    TIFFOpenOptionsSetMaxSingleMemAlloc(opts, 4096);
    TIFFOpenOptionsSetErrorHandlerExtR(opts, capture_error, &error_state);
    tif = TIFFOpenExt(path, "w", opts);
    TIFFOpenOptionsFree(opts);
    expect(tif != NULL, "failed to open allocation-cap output");

    expect(TIFFCreateDirectory(tif) == 0, "TIFFCreateDirectory failed");
    expect(TIFFSetField(tif, TIFFTAG_IMAGEWIDTH, (uint32_t)1) == 1,
           "failed to set ImageWidth");
    expect(TIFFSetField(tif, TIFFTAG_IMAGELENGTH, (uint32_t)1) == 1,
           "failed to set ImageLength");
    expect(TIFFSetField(tif, TIFFTAG_SAMPLESPERPIXEL, 10000) == 1,
           "failed to set large SamplesPerPixel");
    expect(TIFFSetField(tif, TIFFTAG_BITSPERSAMPLE, 8) == 1,
           "failed to set BitsPerSample");

    clear_error(&error_state);
    expect(TIFFWriteDirectory(tif) == 0,
           "allocation cap should fail during directory serialization");
    expect(error_state.message[0] != '\0',
           "allocation-cap failure should report an error");

    TIFFClose(tif);
    unlink(path);
}

static void test_anonymous_field_and_malformed_operations(void)
{
    char path[] = "dirwrite_anonymousXXXXXX";
    int fd = mkstemp(path);
    ErrorState error_state = {{0}};
    TIFF *tif;
    TIFF *reopen;
    uint32_t anon_value = 1234;
    uint32_t read_count = 0;
    void *read_anon_ptr = NULL;

    if (fd < 0)
        fail("mkstemp failed");
    close(fd);
    write_minimal_anonymous_tiff(path);

    tif = open_with_error_handler(path, "r+", &error_state);
    expect(tif != NULL, "failed to open anonymous field output");
    expect(TIFFSetField(tif, TIFFTAG_DIRWRITE_ANON, (uint32_t)1, &anon_value) == 1,
           "failed to set anonymous field");
    expect(TIFFRewriteDirectory(tif) == 1,
           "failed to rewrite directory with anonymous field");

    expect(TIFFCreateEXIFDirectory(tif) == 0, "TIFFCreateEXIFDirectory failed");

    clear_error(&error_state);
    expect(TIFFWriteDirectory(tif) == 0,
           "TIFFWriteDirectory should reject custom directories");
    expect(error_state.message[0] != '\0',
           "wrong-directory write should report an error");

    clear_error(&error_state);
    expect(_TIFFRewriteField(tif, TIFFTAG_IMAGEWIDTH, TIFF_LONG, 1, &anon_value) == 0,
           "_TIFFRewriteField should reject unwritten directories");
    expect(error_state.message[0] != '\0',
           "rewrite failure should report an error");

    clear_error(&error_state);
    expect(TIFFUnlinkDirectory(tif, 0) == 0,
           "TIFFUnlinkDirectory should reject directory number 0");
    expect(error_state.message[0] != '\0',
           "unlink failure should report an error");

    TIFFClose(tif);

    reopen = TIFFOpen(path, "r");
    expect(reopen != NULL, "failed to reopen anonymous field output");
    expect(TIFFGetField(reopen, TIFFTAG_DIRWRITE_ANON, &read_count, &read_anon_ptr) == 1,
           "failed to read anonymous field after reopen");
    expect(read_count == 1 && read_anon_ptr != NULL &&
               ((const uint32_t *)read_anon_ptr)[0] == anon_value,
           "unexpected anonymous field value after reopen");
    TIFFClose(reopen);
    unlink(path);
}

int main(void)
{
    test_setfield_rational_validation();
    test_write_time_rational_validation();
    test_transferfunction_validation();
    test_allocation_cap_failure();
    test_anonymous_field_and_malformed_operations();
    return 0;
}
