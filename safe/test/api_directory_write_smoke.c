/*
 * Smoke coverage for directory-only write-side APIs.
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

extern int TIFFSetCompressionScheme(TIFF *tif, int scheme);

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

int main(void)
{
    char path[] = "api_directory_write_smokeXXXXXX";
    int fd;
    TIFF *tif;
    uint64_t checkpoint_offset = 0;
    uint64_t rewritten_offset = 0;
    uint64_t exif_offset = 0;
    uint64_t gps_offset = 0;
    uint64_t read_exif_offset = 0;
    uint64_t read_gps_offset = 0;
    char *page_name = NULL;
    char *date_time_original = NULL;
    char *exif_version = NULL;
    void *gps_version_ptr = NULL;
    uint8_t gps_version[4] = {2, 3, 0, 0};
    uint8_t exif_version_bytes[4] = {'0', '2', '3', '1'};
    double altitude = 0.0;

    fd = mkstemp(path);
    if (fd < 0)
        fail("mkstemp failed");
    close(fd);
    unlink(path);

    tif = TIFFOpen(path, "w+");
    expect(tif != NULL, "TIFFOpen failed for directory write smoke");

    expect(TIFFCreateDirectory(tif) == 0, "TIFFCreateDirectory failed");
    expect(TIFFSetCompressionScheme(tif, 65000) == 1,
           "TIFFSetCompressionScheme should accept unknown schemes");
    expect(TIFFSetField(tif, TIFFTAG_IMAGEWIDTH, (uint32_t)5) == 1,
           "failed to set ImageWidth");
    expect(TIFFSetField(tif, TIFFTAG_IMAGELENGTH, (uint32_t)6) == 1,
           "failed to set ImageLength");
    expect(TIFFSetField(tif, TIFFTAG_BITSPERSAMPLE, 8) == 1,
           "failed to set BitsPerSample");
    expect(TIFFSetField(tif, TIFFTAG_SAMPLESPERPIXEL, 1) == 1,
           "failed to set SamplesPerPixel");
    expect(TIFFSetField(tif, TIFFTAG_PHOTOMETRIC, PHOTOMETRIC_MINISBLACK) == 1,
           "failed to set Photometric");
    expect(TIFFSetField(tif, TIFFTAG_PAGENAME, "primary-checkpoint") == 1,
           "failed to set PageName");
    expect(TIFFSetField(tif, TIFFTAG_SOFTWARE, "safe-directory-write") == 1,
           "failed to set Software");
    expect(TIFFSetField(tif, TIFFTAG_XRESOLUTION, 72.0) == 1,
           "failed to set XResolution");
    expect(TIFFCheckpointDirectory(tif) == 1, "TIFFCheckpointDirectory failed");
    checkpoint_offset = TIFFCurrentDirOffset(tif);
    expect(checkpoint_offset != 0, "checkpoint directory offset was not recorded");

    expect(TIFFSetField(tif, TIFFTAG_PAGENAME, "primary-rewritten") == 1,
           "failed to update primary PageName");
    expect(TIFFRewriteDirectory(tif) == 1, "TIFFRewriteDirectory failed");
    rewritten_offset = TIFFCurrentDirOffset(tif);
    expect(rewritten_offset != 0, "rewritten directory offset was not recorded");
    expect(rewritten_offset != checkpoint_offset,
           "rewrite should relocate the on-disk directory");

    expect(TIFFCreateEXIFDirectory(tif) == 0, "TIFFCreateEXIFDirectory failed");
    expect(TIFFSetField(tif, EXIFTAG_EXIFVERSION, exif_version_bytes) == 1,
           "failed to set ExifVersion");
    expect(TIFFSetField(tif, EXIFTAG_DATETIMEORIGINAL, "2026:04:04 12:34:56") == 1,
           "failed to set DateTimeOriginal");
    expect(TIFFWriteCustomDirectory(tif, &exif_offset) == 1,
           "TIFFWriteCustomDirectory failed for EXIF IFD");
    expect(exif_offset != 0, "EXIF directory offset was not returned");

    expect(TIFFCreateGPSDirectory(tif) == 0, "TIFFCreateGPSDirectory failed");
    expect(TIFFSetField(tif, GPSTAG_VERSIONID, gps_version) == 1,
           "failed to set GPS VersionID");
    expect(TIFFSetField(tif, GPSTAG_ALTITUDE, 123.5) == 1,
           "failed to set GPS Altitude");
    expect(TIFFWriteCustomDirectory(tif, &gps_offset) == 1,
           "TIFFWriteCustomDirectory failed for GPS IFD");
    expect(gps_offset != 0, "GPS directory offset was not returned");

    expect(TIFFSetDirectory(tif, 0) == 1, "failed to reload the primary directory");
    expect(TIFFSetField(tif, TIFFTAG_EXIFIFD, exif_offset) == 1,
           "failed to link EXIF directory");
    expect(TIFFSetField(tif, TIFFTAG_GPSIFD, gps_offset) == 1,
           "failed to link GPS directory");
    expect(TIFFRewriteDirectory(tif) == 1,
           "failed to rewrite the primary directory with custom IFD links");

    expect(TIFFCreateDirectory(tif) == 0, "failed to create a secondary directory");
    expect(TIFFSetField(tif, TIFFTAG_IMAGEWIDTH, (uint32_t)9) == 1,
           "failed to set secondary ImageWidth");
    expect(TIFFSetField(tif, TIFFTAG_IMAGELENGTH, (uint32_t)9) == 1,
           "failed to set secondary ImageLength");
    expect(TIFFSetField(tif, TIFFTAG_BITSPERSAMPLE, 8) == 1,
           "failed to set secondary BitsPerSample");
    expect(TIFFSetField(tif, TIFFTAG_SAMPLESPERPIXEL, 1) == 1,
           "failed to set secondary SamplesPerPixel");
    expect(TIFFSetField(tif, TIFFTAG_PAGENAME, "secondary") == 1,
           "failed to set secondary PageName");
    expect(TIFFSetField(tif, TIFFTAG_COMPRESSION, 65000) == 1,
           "failed to accept unknown compression through TIFFSetField");
    expect(TIFFWriteDirectory(tif) == 1, "TIFFWriteDirectory failed");

    expect(TIFFUnlinkDirectory(tif, 2) == 1,
           "TIFFUnlinkDirectory failed for the secondary directory");

    TIFFClose(tif);

    tif = TIFFOpen(path, "r");
    expect(tif != NULL, "failed to reopen smoke output");
    expect(TIFFNumberOfDirectories(tif) == 1,
           "unlink should leave a single main directory");
    expect(TIFFGetField(tif, TIFFTAG_PAGENAME, &page_name) == 1,
           "missing primary PageName after reopen");
    expect(strcmp(page_name, "primary-rewritten") == 0,
           "unexpected primary PageName after reopen");
    expect(TIFFGetField(tif, TIFFTAG_EXIFIFD, &read_exif_offset) == 1,
           "missing EXIFIFD link after reopen");
    expect(read_exif_offset == exif_offset,
           "unexpected EXIFIFD offset after reopen");
    expect(TIFFGetField(tif, TIFFTAG_GPSIFD, &read_gps_offset) == 1,
           "missing GPSIFD link after reopen");
    expect(read_gps_offset == gps_offset,
           "unexpected GPSIFD offset after reopen");

    expect(TIFFReadEXIFDirectory(tif, read_exif_offset) == 1,
           "TIFFReadEXIFDirectory failed after write smoke");
    expect(TIFFGetField(tif, EXIFTAG_DATETIMEORIGINAL, &date_time_original) == 1,
           "missing DateTimeOriginal after reopen");
    expect(strcmp(date_time_original, "2026:04:04 12:34:56") == 0,
           "unexpected DateTimeOriginal after reopen");
    expect(TIFFGetField(tif, EXIFTAG_EXIFVERSION, &exif_version) == 1,
           "missing ExifVersion after reopen");
    expect(memcmp(exif_version, "0231", 4) == 0,
           "unexpected ExifVersion after reopen");

    expect(TIFFSetDirectory(tif, 0) == 1,
           "failed to return to main directory after EXIF read");
    expect(TIFFReadGPSDirectory(tif, read_gps_offset) == 1,
           "TIFFReadGPSDirectory failed after write smoke");
    expect(TIFFGetField(tif, GPSTAG_VERSIONID, &gps_version_ptr) == 1,
           "missing GPS VersionID after reopen");
    expect(gps_version_ptr != NULL &&
               memcmp(gps_version_ptr, gps_version, sizeof(gps_version)) == 0,
           "unexpected GPS VersionID after reopen");
    expect(TIFFGetField(tif, GPSTAG_ALTITUDE, &altitude) == 1,
           "missing GPS Altitude after reopen");
    expect(altitude > 123.49 && altitude < 123.51,
           "unexpected GPS Altitude after reopen");

    TIFFClose(tif);
    unlink(path);
    return 0;
}
