/*
 * Smoke coverage for reading linked EXIF/GPS custom directories from a
 * pre-existing upstream fixture.
 */

#include "tif_config.h"

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

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

static void expect_contains(const char *haystack, const char *needle,
                            const char *message)
{
    if (strstr(haystack, needle) == NULL)
        fail(message);
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

static const char *custom_dir_exif_gps_path(void)
{
    static const char primary[] = SOURCE_DIR "/images/custom_dir_EXIF_GPS.tiff";
    static const char fallback[] = SOURCE_DIR "/../debian/custom_dir_EXIF_GPS.tiff";
    FILE *probe = fopen(primary, "rb");

    if (probe != NULL)
    {
        fclose(probe);
        return primary;
    }

    probe = fopen(fallback, "rb");
    if (probe != NULL)
    {
        fclose(probe);
        return fallback;
    }

    return primary;
}

int main(void)
{
    char print_buffer[8192];
    TIFF *tif = NULL;
    toff_t main_offset = 0;
    toff_t exif_offset = 0;
    toff_t gps_offset = 0;
    toff_t reread_exif_offset = 0;
    toff_t reread_gps_offset = 0;
    char *ascii_value = NULL;
    uint8_t *gps_version = NULL;
    uint16_t value_u16 = 0;
    uint32_t value_u32 = 0;
    double altitude = 0.0;
    static const uint8_t expected_gps_version[4] = {2, 2, 0, 1};

    tif = TIFFOpen(custom_dir_exif_gps_path(), "r");
    expect(tif != NULL, "failed to open EXIF/GPS fixture");

    expect(TIFFCurrentDirectory(tif) == 0, "main directory must be loaded first");
    main_offset = TIFFCurrentDirOffset(tif);
    expect(main_offset != 0, "main directory offset was not recorded");
    expect(TIFFNumberOfDirectories(tif) == 1,
           "custom EXIF/GPS directories must not count as main IFDs");
    expect(TIFFCurrentDirectory(tif) == 0,
           "TIFFNumberOfDirectories changed the current main directory");
    expect(TIFFCurrentDirOffset(tif) == main_offset,
           "TIFFNumberOfDirectories changed the current main directory offset");

    expect(TIFFGetField(tif, TIFFTAG_EXIFIFD, &exif_offset) == 1,
           "missing EXIFIFD link in main directory");
    expect(TIFFGetField(tif, TIFFTAG_GPSIFD, &gps_offset) == 1,
           "missing GPSIFD link in main directory");
    expect(exif_offset != 0, "EXIFIFD offset must be non-zero");
    expect(gps_offset != 0, "GPSIFD offset must be non-zero");

    capture_print_directory(tif, 0, print_buffer, sizeof(print_buffer));
    expect_contains(print_buffer, "EXIFIFDOffset:", "missing printed EXIFIFD link");
    expect_contains(print_buffer, "GPSIFDOffset:", "missing printed GPSIFD link");

    expect(TIFFReadEXIFDirectory(tif, exif_offset) == 1,
           "TIFFReadEXIFDirectory failed for the upstream fixture");
    expect(TIFFCurrentDirOffset(tif) == exif_offset,
           "EXIF current directory offset mismatch");
    expect(TIFFGetField(tif, EXIFTAG_EXIFVERSION, &ascii_value) == 1,
           "missing ExifVersion in EXIF directory");
    expect(memcmp(ascii_value, "0231", 4) == 0, "unexpected ExifVersion bytes");
    expect(TIFFGetField(tif, EXIFTAG_DATETIMEORIGINAL, &ascii_value) == 1,
           "missing DateTimeOriginal in EXIF directory");
    expect(strncmp(ascii_value, "N13-String-13", 13) == 0,
           "unexpected DateTimeOriginal in EXIF directory");
    expect(TIFFGetField(tif, EXIFTAG_EXPOSUREPROGRAM, &value_u16) == 1,
           "missing ExposureProgram in EXIF directory");
    expect(value_u16 == 399, "unexpected ExposureProgram value");
    expect(TIFFGetField(tif, EXIFTAG_PIXELXDIMENSION, &value_u32) == 1,
           "missing PixelXDimension in EXIF directory");
    expect(value_u32 == 5985U, "unexpected PixelXDimension value");
    capture_print_directory(tif, 0, print_buffer, sizeof(print_buffer));
    expect_contains(print_buffer, "DateTimeOriginal:", "missing printed DateTimeOriginal");
    expect_contains(print_buffer, "ExposureProgram:", "missing printed ExposureProgram");

    expect(TIFFSetDirectory(tif, 0) == 1,
           "failed to restore the main directory after EXIF read");
    expect(TIFFCurrentDirectory(tif) == 0,
           "main directory index changed after EXIF read");
    expect(TIFFCurrentDirOffset(tif) == main_offset,
           "main directory offset changed after EXIF read");
    expect(TIFFGetField(tif, TIFFTAG_EXIFIFD, &reread_exif_offset) == 1,
           "missing EXIFIFD link after returning to main directory");
    expect(reread_exif_offset == exif_offset,
           "EXIFIFD link changed after returning to main directory");
    expect(TIFFGetField(tif, TIFFTAG_GPSIFD, &reread_gps_offset) == 1,
           "missing GPSIFD link after returning to main directory");
    expect(reread_gps_offset == gps_offset,
           "GPSIFD link changed after returning to main directory");

    expect(TIFFReadGPSDirectory(tif, gps_offset) == 1,
           "TIFFReadGPSDirectory failed for the upstream fixture");
    expect(TIFFCurrentDirOffset(tif) == gps_offset,
           "GPS current directory offset mismatch");
    expect(TIFFGetField(tif, GPSTAG_VERSIONID, &gps_version) == 1,
           "missing GPS VersionID");
    expect(gps_version != NULL &&
               memcmp(gps_version, expected_gps_version,
                      sizeof(expected_gps_version)) == 0,
           "unexpected GPS VersionID");
    expect(TIFFGetField(tif, GPSTAG_LATITUDEREF, &ascii_value) == 1,
           "missing GPS LatitudeRef");
    expect(strcmp(ascii_value, "N") == 0, "unexpected GPS LatitudeRef");
    expect(TIFFGetField(tif, GPSTAG_LONGITUDEREF, &ascii_value) == 1,
           "missing GPS LongitudeRef");
    expect(strcmp(ascii_value, "W") == 0, "unexpected GPS LongitudeRef");
    expect(TIFFGetField(tif, GPSTAG_DATESTAMP, &ascii_value) == 1,
           "missing GPS DateStamp");
    expect(strcmp(ascii_value, "2012:11:04") == 0, "unexpected GPS DateStamp");
    expect(TIFFGetField(tif, GPSTAG_DIFFERENTIAL, &value_u16) == 1,
           "missing GPS Differential");
    expect(value_u16 == 42, "unexpected GPS Differential");
    expect(TIFFGetField(tif, GPSTAG_ALTITUDE, &altitude) == 1,
           "missing GPS Altitude");
    expect(altitude > 3455.9 && altitude < 3456.1,
           "unexpected GPS Altitude");
    capture_print_directory(tif, 0, print_buffer, sizeof(print_buffer));
    expect_contains(print_buffer, "VersionID:", "missing printed GPS VersionID");
    expect_contains(print_buffer, "DateStamp: 2012:11:04",
                    "missing printed GPS DateStamp");

    expect(TIFFSetDirectory(tif, 0) == 1,
           "failed to restore the main directory after GPS read");
    expect(TIFFCurrentDirectory(tif) == 0,
           "main directory index changed after GPS read");
    expect(TIFFCurrentDirOffset(tif) == main_offset,
           "main directory offset changed after GPS read");
    expect(TIFFReadEXIFDirectory(tif, exif_offset) == 1,
           "second TIFFReadEXIFDirectory failed");
    expect(TIFFGetField(tif, EXIFTAG_DATETIMEORIGINAL, &ascii_value) == 1,
           "missing DateTimeOriginal on second EXIF read");
    expect(strncmp(ascii_value, "N13-String-13", 13) == 0,
           "unexpected DateTimeOriginal on second EXIF read");

    TIFFClose(tif);
    return 0;
}
