/*
 * Smoke coverage for configured external codecs and their pseudo-tags.
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

static void expect_configured_codec(uint16_t scheme, const char *expected_name)
{
    const TIFFCodec *codec = TIFFFindCODEC(scheme);
    expect(codec != NULL, "missing configured codec");
    expect(TIFFIsCODECConfigured(scheme) == 1, "codec should be configured");
    expect(codec->scheme == scheme, "unexpected codec scheme");
    expect(strcmp(codec->name, expected_name) == 0, "unexpected codec name");
}

int main(void)
{
    char path[] = "api_codec_smokeXXXXXX";
    int fd;
    TIFF *tif;
    TIFFCodec *configured;
    TIFFCodec *configured_start;
    int saw_jbig = 0;
    int saw_sgilog = 0;
    int saw_sgilog24 = 0;
    int saw_lerc = 0;
    int saw_lzma = 0;
    int saw_zstd = 0;
    int saw_webp = 0;
    uint32_t lerc_count = 0;
    uint32_t *lerc_params = NULL;
    double max_z_error = 0.0;
    int zstd_level = 0;
    int lzma_preset = 0;
    int webp_level = 0;
    int webp_lossless = 0;
    int webp_exact = 0;

    expect_configured_codec(COMPRESSION_JBIG, "ISO JBIG");
    expect_configured_codec(COMPRESSION_SGILOG, "SGILog");
    expect_configured_codec(COMPRESSION_SGILOG24, "SGILog24");
    expect_configured_codec(COMPRESSION_LERC, "LERC");
    expect_configured_codec(COMPRESSION_LZMA, "LZMA");
    expect_configured_codec(COMPRESSION_ZSTD, "ZSTD");
    expect_configured_codec(COMPRESSION_WEBP, "WEBP");

    configured = TIFFGetConfiguredCODECs();
    expect(configured != NULL, "TIFFGetConfiguredCODECs failed");
    configured_start = configured;
    for (; configured->name != NULL; ++configured)
    {
        saw_jbig |= configured->scheme == COMPRESSION_JBIG;
        saw_sgilog |= configured->scheme == COMPRESSION_SGILOG;
        saw_sgilog24 |= configured->scheme == COMPRESSION_SGILOG24;
        saw_lerc |= configured->scheme == COMPRESSION_LERC;
        saw_lzma |= configured->scheme == COMPRESSION_LZMA;
        saw_zstd |= configured->scheme == COMPRESSION_ZSTD;
        saw_webp |= configured->scheme == COMPRESSION_WEBP;
    }
    expect(saw_jbig && saw_sgilog && saw_sgilog24 && saw_lerc && saw_lzma &&
               saw_zstd && saw_webp,
           "configured codec listing missed an external codec");
    _TIFFfree(configured_start);

    fd = mkstemp(path);
    if (fd < 0)
        fail("mkstemp failed");
    close(fd);
    unlink(path);

    tif = TIFFOpen(path, "w+");
    expect(tif != NULL, "TIFFOpen failed for codec smoke");
    expect(TIFFSetField(tif, TIFFTAG_IMAGEWIDTH, (uint32_t)4) == 1,
           "failed to set ImageWidth");
    expect(TIFFSetField(tif, TIFFTAG_IMAGELENGTH, (uint32_t)4) == 1,
           "failed to set ImageLength");
    expect(TIFFSetField(tif, TIFFTAG_BITSPERSAMPLE, 8) == 1,
           "failed to set BitsPerSample");
    expect(TIFFSetField(tif, TIFFTAG_SAMPLESPERPIXEL, 3) == 1,
           "failed to set SamplesPerPixel");
    expect(TIFFSetField(tif, TIFFTAG_PHOTOMETRIC, PHOTOMETRIC_RGB) == 1,
           "failed to set Photometric");
    expect(TIFFSetField(tif, TIFFTAG_PLANARCONFIG, PLANARCONFIG_CONTIG) == 1,
           "failed to set PlanarConfiguration");

    expect(TIFFSetField(tif, TIFFTAG_COMPRESSION, COMPRESSION_LERC) == 1,
           "failed to select LERC");
    expect(TIFFSetField(tif, TIFFTAG_LERC_VERSION, LERC_VERSION_2_4) == 1,
           "failed to set LercVersion");
    expect(TIFFSetField(tif, TIFFTAG_LERC_ADD_COMPRESSION,
                        LERC_ADD_COMPRESSION_ZSTD) == 1,
           "failed to set LercAdditionalCompression");
    expect(TIFFSetField(tif, TIFFTAG_LERC_MAXZERROR, 0.25) == 1,
           "failed to set LercMaximumError");
    expect(TIFFGetField(tif, TIFFTAG_LERC_PARAMETERS, &lerc_count,
                        &lerc_params) == 1,
           "failed to get LercParameters");
    expect(lerc_count >= 2 && lerc_params != NULL,
           "LercParameters should contain at least two values");
    expect(lerc_params[0] == LERC_VERSION_2_4,
           "unexpected LercParameters version");
    expect(lerc_params[1] == LERC_ADD_COMPRESSION_ZSTD,
           "unexpected LercParameters additional compression");
    expect(TIFFGetField(tif, TIFFTAG_LERC_MAXZERROR, &max_z_error) == 1,
           "failed to read LercMaximumError");
    expect(max_z_error > 0.24 && max_z_error < 0.26,
           "unexpected LercMaximumError value");
    expect(TIFFUnsetField(tif, TIFFTAG_LERC_ADD_COMPRESSION) == 1,
           "failed to unset LercAdditionalCompression");
    expect(TIFFGetField(tif, TIFFTAG_LERC_PARAMETERS, &lerc_count,
                        &lerc_params) == 1,
           "failed to reload LercParameters after unset");
    expect(lerc_params[1] == LERC_ADD_COMPRESSION_NONE,
           "unsetting LercAdditionalCompression should update LercParameters");

    expect(TIFFSetField(tif, TIFFTAG_COMPRESSION, COMPRESSION_ZSTD) == 1,
           "failed to select ZSTD");
    expect(TIFFSetField(tif, TIFFTAG_ZSTD_LEVEL, 7) == 1,
           "failed to set ZSTD level");
    expect(TIFFGetField(tif, TIFFTAG_ZSTD_LEVEL, &zstd_level) == 1,
           "failed to get ZSTD level");
    expect(zstd_level == 7, "unexpected ZSTD level");

    expect(TIFFSetField(tif, TIFFTAG_COMPRESSION, COMPRESSION_LZMA) == 1,
           "failed to select LZMA");
    expect(TIFFSetField(tif, TIFFTAG_LZMAPRESET, 5) == 1,
           "failed to set LZMA preset");
    expect(TIFFGetField(tif, TIFFTAG_LZMAPRESET, &lzma_preset) == 1,
           "failed to get LZMA preset");
    expect(lzma_preset == 5, "unexpected LZMA preset");

    expect(TIFFSetField(tif, TIFFTAG_COMPRESSION, COMPRESSION_WEBP) == 1,
           "failed to select WEBP");
    expect(TIFFSetField(tif, TIFFTAG_WEBP_LEVEL, 90) == 1,
           "failed to set WEBP level");
    expect(TIFFSetField(tif, TIFFTAG_WEBP_LOSSLESS, 1) == 1,
           "failed to set WEBP lossless");
    expect(TIFFSetField(tif, TIFFTAG_WEBP_LOSSLESS_EXACT, 0) == 1,
           "failed to set WEBP exact");
    expect(TIFFGetField(tif, TIFFTAG_WEBP_LEVEL, &webp_level) == 1,
           "failed to get WEBP level");
    expect(TIFFGetField(tif, TIFFTAG_WEBP_LOSSLESS, &webp_lossless) == 1,
           "failed to get WEBP lossless");
    expect(TIFFGetField(tif, TIFFTAG_WEBP_LOSSLESS_EXACT, &webp_exact) == 1,
           "failed to get WEBP exact");
    expect(webp_level == 90 && webp_lossless == 1 && webp_exact == 0,
           "unexpected WEBP pseudo-tag values");

    TIFFClose(tif);
    unlink(path);
    return 0;
}
