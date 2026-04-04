/*
 * Copyright (c) 2026, LibTIFF Contributors
 *
 * Permission to use, copy, modify, distribute, and sell this software and
 * its documentation for any purpose is hereby granted without fee, provided
 * that (i) the above copyright notices and this permission notice appear in
 * all copies of the software and related documentation, and (ii) the names of
 * Sam Leffler and Silicon Graphics may not be used in any advertising or
 * publicity relating to this software without the specific, prior written
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

#include "tif_config.h"

#include <stdint.h>
#include <stdio.h>
#include <string.h>

#ifdef HAVE_UNISTD_H
#include <unistd.h>
#endif

#include "tiffio.h"

static const char rgb_filename[] = "api_rgba_image_helpers_rgb.tif";
static const char float_filename[] = "api_rgba_image_helpers_float.tif";
static const char ycbcr_filename[] = "api_rgba_image_helpers_ycbcr.tif";
static const char cmyk_filename[] = "api_rgba_image_helpers_cmyk.tif";
static const char lab16_filename[] = "api_rgba_image_helpers_lab16.tif";
static const char window_filename[] = "api_rgba_image_helpers_window.tif";

static const TIFFDisplay srgb_display = {
    {{3.2410f, -1.5374f, -0.4986f},
     {-0.9692f, 1.8760f, 0.0416f},
     {0.0556f, -0.2040f, 1.0570f}},
    100.0f,
    100.0f,
    100.0f,
    255,
    255,
    255,
    0.0f,
    0.0f,
    0.0f,
    2.2f,
    2.2f,
    2.2f,
};

static int expect_channels(const char *label, uint32_t pixel, uint32_t r, uint32_t g,
                           uint32_t b, uint32_t a)
{
    if (TIFFGetR(pixel) != r || TIFFGetG(pixel) != g || TIFFGetB(pixel) != b ||
        TIFFGetA(pixel) != a)
    {
        fprintf(stderr,
                "%s pixel mismatch: got (%u,%u,%u,%u), expected (%u,%u,%u,%u).\n",
                label, TIFFGetR(pixel), TIFFGetG(pixel), TIFFGetB(pixel),
                TIFFGetA(pixel), r, g, b, a);
        return 1;
    }
    return 0;
}

static int write_rgb_image(void)
{
    static const unsigned char row[] = {10, 20, 30};
    TIFF *tif = TIFFOpen(rgb_filename, "w");

    if (!tif)
    {
        fprintf(stderr, "Unable to create %s.\n", rgb_filename);
        return 1;
    }

    if (!TIFFSetField(tif, TIFFTAG_IMAGEWIDTH, 1U) ||
        !TIFFSetField(tif, TIFFTAG_IMAGELENGTH, 1U) ||
        !TIFFSetField(tif, TIFFTAG_BITSPERSAMPLE, 8) ||
        !TIFFSetField(tif, TIFFTAG_SAMPLESPERPIXEL, 3) ||
        !TIFFSetField(tif, TIFFTAG_ROWSPERSTRIP, 1U) ||
        !TIFFSetField(tif, TIFFTAG_PLANARCONFIG, PLANARCONFIG_CONTIG) ||
        !TIFFSetField(tif, TIFFTAG_PHOTOMETRIC, PHOTOMETRIC_RGB) ||
        TIFFWriteScanline(tif, (void *)row, 0, 0) == -1)
    {
        fprintf(stderr, "Unable to write %s.\n", rgb_filename);
        TIFFClose(tif);
        return 1;
    }

    TIFFClose(tif);
    return 0;
}

static int write_window_image(void)
{
    static const unsigned char row0[] = {255, 0,   0,   0,   255,
                                         0,   0,   0,   255};
    static const unsigned char row1[] = {255, 255, 0,   255, 0,
                                         255, 0,   255, 255};
    TIFF *tif = TIFFOpen(window_filename, "w");

    if (!tif)
    {
        fprintf(stderr, "Unable to create %s.\n", window_filename);
        return 1;
    }

    if (!TIFFSetField(tif, TIFFTAG_IMAGEWIDTH, 3U) ||
        !TIFFSetField(tif, TIFFTAG_IMAGELENGTH, 2U) ||
        !TIFFSetField(tif, TIFFTAG_BITSPERSAMPLE, 8) ||
        !TIFFSetField(tif, TIFFTAG_SAMPLESPERPIXEL, 3) ||
        !TIFFSetField(tif, TIFFTAG_ROWSPERSTRIP, 1U) ||
        !TIFFSetField(tif, TIFFTAG_PLANARCONFIG, PLANARCONFIG_CONTIG) ||
        !TIFFSetField(tif, TIFFTAG_PHOTOMETRIC, PHOTOMETRIC_RGB) ||
        TIFFWriteScanline(tif, (void *)row0, 0, 0) == -1 ||
        TIFFWriteScanline(tif, (void *)row1, 1, 0) == -1)
    {
        fprintf(stderr, "Unable to write %s.\n", window_filename);
        TIFFClose(tif);
        return 1;
    }

    TIFFClose(tif);
    return 0;
}

static int write_float_image(void)
{
    uint16_t sample = 1;
    TIFF *tif = TIFFOpen(float_filename, "w");

    if (!tif)
    {
        fprintf(stderr, "Unable to create %s.\n", float_filename);
        return 1;
    }

    if (!TIFFSetField(tif, TIFFTAG_IMAGEWIDTH, 1U) ||
        !TIFFSetField(tif, TIFFTAG_IMAGELENGTH, 1U) ||
        !TIFFSetField(tif, TIFFTAG_BITSPERSAMPLE, 16) ||
        !TIFFSetField(tif, TIFFTAG_SAMPLESPERPIXEL, 1) ||
        !TIFFSetField(tif, TIFFTAG_SAMPLEFORMAT, SAMPLEFORMAT_IEEEFP) ||
        !TIFFSetField(tif, TIFFTAG_ROWSPERSTRIP, 1U) ||
        !TIFFSetField(tif, TIFFTAG_PLANARCONFIG, PLANARCONFIG_CONTIG) ||
        !TIFFSetField(tif, TIFFTAG_PHOTOMETRIC, PHOTOMETRIC_MINISBLACK) ||
        TIFFWriteScanline(tif, &sample, 0, 0) == -1)
    {
        fprintf(stderr, "Unable to write %s.\n", float_filename);
        TIFFClose(tif);
        return 1;
    }

    TIFFClose(tif);
    return 0;
}

static int write_ycbcr_image(void)
{
    const float luma[3] = {0.299f, 0.587f, 0.114f};
    const float ref_black_white[6] = {0.0f, 255.0f, 128.0f,
                                      255.0f, 128.0f, 255.0f};
    const unsigned char row[] = {90, 140, 190};
    TIFF *tif = TIFFOpen(ycbcr_filename, "w");

    if (!tif)
    {
        fprintf(stderr, "Unable to create %s.\n", ycbcr_filename);
        return 1;
    }

    if (!TIFFSetField(tif, TIFFTAG_IMAGEWIDTH, 1U) ||
        !TIFFSetField(tif, TIFFTAG_IMAGELENGTH, 1U) ||
        !TIFFSetField(tif, TIFFTAG_BITSPERSAMPLE, 8) ||
        !TIFFSetField(tif, TIFFTAG_SAMPLESPERPIXEL, 3) ||
        !TIFFSetField(tif, TIFFTAG_ROWSPERSTRIP, 1U) ||
        !TIFFSetField(tif, TIFFTAG_PLANARCONFIG, PLANARCONFIG_CONTIG) ||
        !TIFFSetField(tif, TIFFTAG_PHOTOMETRIC, PHOTOMETRIC_YCBCR) ||
        !TIFFSetField(tif, TIFFTAG_COMPRESSION, COMPRESSION_NONE) ||
        !TIFFSetField(tif, TIFFTAG_YCBCRSUBSAMPLING, 1, 1) ||
        !TIFFSetField(tif, TIFFTAG_YCBCRCOEFFICIENTS, (float *)luma) ||
        !TIFFSetField(tif, TIFFTAG_REFERENCEBLACKWHITE,
                      (float *)ref_black_white) ||
        TIFFWriteScanline(tif, (void *)row, 0, 0) == -1)
    {
        fprintf(stderr, "Unable to write %s.\n", ycbcr_filename);
        TIFFClose(tif);
        return 1;
    }

    TIFFClose(tif);
    return 0;
}

static int write_separate_cmyk_image(void)
{
    const unsigned char c = 10;
    const unsigned char m = 50;
    const unsigned char y = 100;
    const unsigned char k = 40;
    TIFF *tif = TIFFOpen(cmyk_filename, "w");

    if (!tif)
    {
        fprintf(stderr, "Unable to create %s.\n", cmyk_filename);
        return 1;
    }

    if (!TIFFSetField(tif, TIFFTAG_IMAGEWIDTH, 1U) ||
        !TIFFSetField(tif, TIFFTAG_IMAGELENGTH, 1U) ||
        !TIFFSetField(tif, TIFFTAG_BITSPERSAMPLE, 8) ||
        !TIFFSetField(tif, TIFFTAG_SAMPLESPERPIXEL, 4) ||
        !TIFFSetField(tif, TIFFTAG_ROWSPERSTRIP, 1U) ||
        !TIFFSetField(tif, TIFFTAG_PLANARCONFIG, PLANARCONFIG_SEPARATE) ||
        !TIFFSetField(tif, TIFFTAG_PHOTOMETRIC, PHOTOMETRIC_SEPARATED) ||
        !TIFFSetField(tif, TIFFTAG_INKSET, INKSET_CMYK) ||
        TIFFWriteScanline(tif, (void *)&c, 0, 0) == -1 ||
        TIFFWriteScanline(tif, (void *)&m, 0, 1) == -1 ||
        TIFFWriteScanline(tif, (void *)&y, 0, 2) == -1 ||
        TIFFWriteScanline(tif, (void *)&k, 0, 3) == -1)
    {
        fprintf(stderr, "Unable to write %s.\n", cmyk_filename);
        TIFFClose(tif);
        return 1;
    }

    TIFFClose(tif);
    return 0;
}

static int write_cielab16_image(void)
{
    const float white_point[2] = {0.34570292f, 0.3585386f};
    uint16_t row[3];
    int16_t a = 4000;
    int16_t b = -6000;
    TIFF *tif = TIFFOpen(lab16_filename, "w");

    if (!tif)
    {
        fprintf(stderr, "Unable to create %s.\n", lab16_filename);
        return 1;
    }

    row[0] = 42000;
    memcpy(&row[1], &a, sizeof(a));
    memcpy(&row[2], &b, sizeof(b));

    if (!TIFFSetField(tif, TIFFTAG_IMAGEWIDTH, 1U) ||
        !TIFFSetField(tif, TIFFTAG_IMAGELENGTH, 1U) ||
        !TIFFSetField(tif, TIFFTAG_BITSPERSAMPLE, 16) ||
        !TIFFSetField(tif, TIFFTAG_SAMPLESPERPIXEL, 3) ||
        !TIFFSetField(tif, TIFFTAG_ROWSPERSTRIP, 1U) ||
        !TIFFSetField(tif, TIFFTAG_PLANARCONFIG, PLANARCONFIG_CONTIG) ||
        !TIFFSetField(tif, TIFFTAG_PHOTOMETRIC, PHOTOMETRIC_CIELAB) ||
        !TIFFSetField(tif, TIFFTAG_WHITEPOINT, (float *)white_point) ||
        TIFFWriteScanline(tif, row, 0, 0) == -1)
    {
        fprintf(stderr, "Unable to write %s.\n", lab16_filename);
        TIFFClose(tif);
        return 1;
    }

    TIFFClose(tif);
    return 0;
}

static int read_single_pixel_rgba(const char *filename, uint32_t *pixel)
{
    TIFF *tif = TIFFOpen(filename, "r");
    uint32_t raster = 0;

    if (!tif)
    {
        fprintf(stderr, "Unable to reopen %s.\n", filename);
        return 1;
    }

    if (!TIFFReadRGBAImageOriented(tif, 1U, 1U, &raster, ORIENTATION_TOPLEFT,
                                   1))
    {
        fprintf(stderr, "TIFFReadRGBAImageOriented() failed for %s.\n", filename);
        TIFFClose(tif);
        return 1;
    }

    TIFFClose(tif);
    *pixel = raster;
    return 0;
}

static int check_rgb_helper_state(void)
{
    TIFF *tif = TIFFOpen(rgb_filename, "r");
    TIFFRGBAImage img;
    char emsg[1024];

    if (!tif)
    {
        fprintf(stderr, "Unable to reopen %s.\n", rgb_filename);
        return 1;
    }

    memset(emsg, 0, sizeof(emsg));
    if (!TIFFRGBAImageOK(tif, emsg))
    {
        fprintf(stderr, "TIFFRGBAImageOK() rejected RGB input: %s\n", emsg);
        TIFFClose(tif);
        return 1;
    }

    memset(&img, 0, sizeof(img));
    if (!TIFFRGBAImageBegin(&img, tif, 0, emsg))
    {
        fprintf(stderr, "TIFFRGBAImageBegin() failed for RGB input: %s\n",
                emsg);
        TIFFClose(tif);
        return 1;
    }

    if (img.alpha != 0)
    {
        fprintf(stderr, "TIFFRGBAImageBegin() reported alpha=%d for RGB input.\n",
                img.alpha);
        TIFFRGBAImageEnd(&img);
        TIFFClose(tif);
        return 1;
    }

    if (img.get == NULL || img.put.any == NULL)
    {
        fprintf(stderr,
                "TIFFRGBAImageBegin() did not populate RGBA helper routines.\n");
        TIFFRGBAImageEnd(&img);
        TIFFClose(tif);
        return 1;
    }

    if (img.isContig != 1 || img.photometric != PHOTOMETRIC_RGB ||
        img.samplesperpixel != 3 || img.bitspersample != 8 ||
        img.req_orientation != ORIENTATION_BOTLEFT)
    {
        fprintf(stderr, "TIFFRGBAImageBegin() exposed incompatible image state.\n");
        TIFFRGBAImageEnd(&img);
        TIFFClose(tif);
        return 1;
    }

    TIFFRGBAImageEnd(&img);
    TIFFClose(tif);
    return 0;
}

static int check_windowed_get(void)
{
    TIFF *tif = TIFFOpen(window_filename, "r");
    TIFFRGBAImage img;
    uint32_t raster[2];
    char emsg[1024];

    if (!tif)
    {
        fprintf(stderr, "Unable to reopen %s.\n", window_filename);
        return 1;
    }

    memset(emsg, 0, sizeof(emsg));
    memset(&img, 0, sizeof(img));
    if (!TIFFRGBAImageBegin(&img, tif, 1, emsg))
    {
        fprintf(stderr, "TIFFRGBAImageBegin() failed for windowed RGB input: %s\n",
                emsg);
        TIFFClose(tif);
        return 1;
    }

    img.req_orientation = ORIENTATION_TOPLEFT;
    img.row_offset = 1;
    img.col_offset = 1;
    memset(raster, 0, sizeof(raster));
    if (!TIFFRGBAImageGet(&img, raster, 2U, 1U))
    {
        fprintf(stderr, "TIFFRGBAImageGet() failed for windowed RGB input.\n");
        TIFFRGBAImageEnd(&img);
        TIFFClose(tif);
        return 1;
    }

    TIFFRGBAImageEnd(&img);
    TIFFClose(tif);

    if (expect_channels("Window[0]", raster[0], 255, 0, 255, 255) != 0 ||
        expect_channels("Window[1]", raster[1], 0, 255, 255, 255) != 0)
        return 1;
    return 0;
}

static int check_float_rejection(void)
{
    TIFF *tif = TIFFOpen(float_filename, "r");
    char emsg[1024];

    if (!tif)
    {
        fprintf(stderr, "Unable to reopen %s.\n", float_filename);
        return 1;
    }

    memset(emsg, 0, sizeof(emsg));
    if (TIFFRGBAImageOK(tif, emsg))
    {
        fprintf(stderr,
                "TIFFRGBAImageOK() unexpectedly accepted IEEE floating-point input.\n");
        TIFFClose(tif);
        return 1;
    }

    if (strstr(emsg, "floating-point") == NULL)
    {
        fprintf(stderr,
                "TIFFRGBAImageOK() returned an unexpected rejection reason: %s\n",
                emsg);
        TIFFClose(tif);
        return 1;
    }

    TIFFClose(tif);
    return 0;
}

static int check_ycbcr_read(void)
{
    const float luma[3] = {0.299f, 0.587f, 0.114f};
    const float ref_black_white[6] = {0.0f, 255.0f, 128.0f,
                                      255.0f, 128.0f, 255.0f};
    TIFFYCbCrToRGB ycbcr;
    uint32_t pixel = 0;
    uint32_t r = 0, g = 0, b = 0;
    TIFF *tif = TIFFOpen(ycbcr_filename, "r");
    TIFFRGBAImage img;
    char emsg[1024];
    memset(&ycbcr, 0, sizeof(ycbcr));

    if (!tif)
    {
        fprintf(stderr, "Unable to reopen %s.\n", ycbcr_filename);
        return 1;
    }

    memset(emsg, 0, sizeof(emsg));
    if (!TIFFRGBAImageBegin(&img, tif, 1, emsg))
    {
        fprintf(stderr, "TIFFRGBAImageBegin() failed for YCbCr input: %s\n",
                emsg);
        TIFFClose(tif);
        return 1;
    }
    if (img.ycbcr == NULL)
    {
        fprintf(stderr, "TIFFRGBAImageBegin() did not initialize YCbCr state.\n");
        TIFFRGBAImageEnd(&img);
        TIFFClose(tif);
        return 1;
    }
    TIFFRGBAImageEnd(&img);
    TIFFClose(tif);

    if (read_single_pixel_rgba(ycbcr_filename, &pixel) != 0)
        return 1;

    if (TIFFYCbCrToRGBInit(&ycbcr, (float *)luma, (float *)ref_black_white) < 0)
    {
        fprintf(stderr, "TIFFYCbCrToRGBInit() failed.\n");
        return 1;
    }
    TIFFYCbCrtoRGB(&ycbcr, 90, 140, 190, &r, &g, &b);
    return expect_channels("YCbCr", pixel, r, g, b, 255);
}

static int check_cmyk_read(void)
{
    uint32_t pixel = 0;
    uint32_t k = 255U - 40U;
    uint32_t r = (k * (255U - 10U)) / 255U;
    uint32_t g = (k * (255U - 50U)) / 255U;
    uint32_t b = (k * (255U - 100U)) / 255U;

    if (read_single_pixel_rgba(cmyk_filename, &pixel) != 0)
        return 1;
    return expect_channels("CMYK", pixel, r, g, b, 255);
}

static void cielab16_to_xyz(const TIFFCIELabToRGB *cielab, uint16_t l,
                            int16_t a, int16_t b, float *x, float *y, float *z)
{
    float l_value = l * 100.0f / 65535.0f;
    float cby;

    if (l_value < 8.856f)
    {
        *y = (l_value * cielab->Y0) / 903.292f;
        cby = 7.787f * (*y / cielab->Y0) + 16.0f / 116.0f;
    }
    else
    {
        cby = (l_value + 16.0f) / 116.0f;
        *y = cielab->Y0 * cby * cby * cby;
    }

    {
        float tmp = a / 256.0f / 500.0f + cby;
        if (tmp < 0.2069f)
            *x = cielab->X0 * (tmp - 0.13793f) / 7.787f;
        else
            *x = cielab->X0 * tmp * tmp * tmp;

        tmp = cby - b / 256.0f / 200.0f;
        if (tmp < 0.2069f)
            *z = cielab->Z0 * (tmp - 0.13793f) / 7.787f;
        else
            *z = cielab->Z0 * tmp * tmp * tmp;
    }
}

static int check_cielab16_read(void)
{
    const float white_point[2] = {0.34570292f, 0.3585386f};
    const float ref_white[3] = {white_point[0] / white_point[1] * 100.0f, 100.0f,
                                (1.0f - white_point[0] - white_point[1]) /
                                    white_point[1] * 100.0f};
    TIFFCIELabToRGB cielab;
    uint32_t pixel = 0;
    uint32_t r = 0, g = 0, b = 0;
    float x = 0.0f, y = 0.0f, z = 0.0f;
    TIFF *tif = TIFFOpen(lab16_filename, "r");
    TIFFRGBAImage img;
    char emsg[1024];

    if (!tif)
    {
        fprintf(stderr, "Unable to reopen %s.\n", lab16_filename);
        return 1;
    }

    memset(emsg, 0, sizeof(emsg));
    if (!TIFFRGBAImageBegin(&img, tif, 1, emsg))
    {
        fprintf(stderr, "TIFFRGBAImageBegin() failed for 16-bit Lab input: %s\n",
                emsg);
        TIFFClose(tif);
        return 1;
    }
    if (img.cielab == NULL)
    {
        fprintf(stderr, "TIFFRGBAImageBegin() did not initialize CIELab state.\n");
        TIFFRGBAImageEnd(&img);
        TIFFClose(tif);
        return 1;
    }
    TIFFRGBAImageEnd(&img);
    TIFFClose(tif);

    if (read_single_pixel_rgba(lab16_filename, &pixel) != 0)
        return 1;

    memset(&cielab, 0, sizeof(cielab));
    if (TIFFCIELabToRGBInit(&cielab, (TIFFDisplay *)&srgb_display,
                            (float *)ref_white) < 0)
    {
        fprintf(stderr, "TIFFCIELabToRGBInit() failed.\n");
        return 1;
    }

    cielab16_to_xyz(&cielab, 42000, 4000, -6000, &x, &y, &z);
    TIFFXYZToRGB(&cielab, x, y, z, &r, &g, &b);
    return expect_channels("CIELab16", pixel, r, g, b, 255);
}

int main(void)
{
    int ret = 1;

    unlink(rgb_filename);
    unlink(float_filename);
    unlink(ycbcr_filename);
    unlink(cmyk_filename);
    unlink(lab16_filename);
    unlink(window_filename);

    if (write_rgb_image() != 0 || write_float_image() != 0 ||
        write_ycbcr_image() != 0 || write_separate_cmyk_image() != 0 ||
        write_cielab16_image() != 0 || write_window_image() != 0)
        goto cleanup;

    if (check_rgb_helper_state() != 0 || check_float_rejection() != 0 ||
        check_ycbcr_read() != 0 || check_cmyk_read() != 0 ||
        check_cielab16_read() != 0 || check_windowed_get() != 0)
        goto cleanup;

    ret = 0;

cleanup:
    if (ret == 0)
    {
        unlink(rgb_filename);
        unlink(float_filename);
        unlink(ycbcr_filename);
        unlink(cmyk_filename);
        unlink(lab16_filename);
        unlink(window_filename);
    }
    return ret;
}
