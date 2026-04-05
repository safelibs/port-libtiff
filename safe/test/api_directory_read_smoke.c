/*
 * Smoke coverage for directory reading, navigation, getters, and printing.
 */

#include "tif_config.h"

#include <stdbool.h>
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

static TIFF *expect_open(const char *path, const char *label)
{
    TIFF *tif = TIFFOpen(path, "r");
    char message[512];

    if (tif != NULL)
        return tif;

    snprintf(message, sizeof(message), "failed to open %s: %s", label, path);
    fail(message);
    return NULL;
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
    char print_buffer[4096];
    TIFF *tif;
    char *page_name = NULL;
    uint16_t subifd_count = 0;
    void *subifd_ptr = NULL;
    uint64_t *subifd_offsets = NULL;
    uint64_t first_subifd_offset = 0;
    tdir_t saved_main_dir = 0;
    toff_t saved_main_offset = 0;
    toff_t exif_offset = 0;
    toff_t gps_offset = 0;
    char *date_time_original = NULL;
    char *exif_version = NULL;

    tif = expect_open(SOURCE_DIR "/images/test_ifd_loop_subifd.tif",
                      "loop-subifd smoke image");
    expect(TIFFCurrentDirectory(tif) == 0, "first directory must be loaded");
    expect(TIFFGetField(tif, TIFFTAG_PAGENAME, &page_name) == 1,
           "missing PageName in first directory");
    expect(strcmp(page_name, "0 th. IFD") == 0,
           "unexpected first directory PageName");
    capture_print_directory(tif, 0, print_buffer, sizeof(print_buffer));
    expect_contains(print_buffer, "TIFF Directory at offset", "missing print header");
    expect_contains(print_buffer, "PageName: 0 th. IFD", "missing printed PageName");

    expect(TIFFSetDirectory(tif, 1) == 1, "TIFFSetDirectory(1) failed");
    expect(TIFFGetField(tif, TIFFTAG_PAGENAME, &page_name) == 1,
           "missing PageName in second directory");
    expect(strcmp(page_name, "1 th. IFD") == 0,
           "unexpected second directory PageName");
    expect(TIFFGetField(tif, TIFFTAG_SUBIFD, &subifd_count, &subifd_ptr) == 1,
           "missing SubIFD tag");
    expect(subifd_count == 3, "unexpected SubIFD count");
    expect(subifd_ptr != NULL, "SubIFD pointer must not be NULL");
    subifd_offsets = (uint64_t *)subifd_ptr;
    first_subifd_offset = subifd_offsets[0];
    capture_print_directory(tif, 0, print_buffer, sizeof(print_buffer));
    expect_contains(print_buffer, "PageName: 1 th. IFD",
                    "missing printed second-directory PageName");
    expect_contains(print_buffer, "SubIFD:", "missing printed SubIFD line");
    saved_main_dir = TIFFCurrentDirectory(tif);
    saved_main_offset = TIFFCurrentDirOffset(tif);
    expect(TIFFNumberOfDirectories(tif) == 5,
           "unexpected directory count for loop-subifd image");
    expect(TIFFCurrentDirectory(tif) == saved_main_dir,
           "TIFFNumberOfDirectories changed current directory");
    expect(TIFFCurrentDirOffset(tif) == saved_main_offset,
           "TIFFNumberOfDirectories changed current directory offset");
    expect(TIFFGetField(tif, TIFFTAG_PAGENAME, &page_name) == 1,
           "PageName should remain readable after TIFFNumberOfDirectories");
    expect(strcmp(page_name, "1 th. IFD") == 0,
           "unexpected PageName after TIFFNumberOfDirectories");

    TIFFFreeDirectory(tif);
    expect(TIFFCurrentDirectory(tif) == saved_main_dir,
           "TIFFFreeDirectory changed current directory");
    expect(TIFFCurrentDirOffset(tif) == saved_main_offset,
           "TIFFFreeDirectory changed current directory offset");
    expect(TIFFSetDirectory(tif, saved_main_dir) == 1,
           "failed to reload current directory after TIFFFreeDirectory");

    expect(TIFFSetSubDirectory(tif, first_subifd_offset) == 1,
           "TIFFSetSubDirectory failed");
    expect(TIFFCurrentDirOffset(tif) == first_subifd_offset,
           "SubIFD current offset mismatch");
    expect(TIFFGetField(tif, TIFFTAG_PAGENAME, &page_name) == 1,
           "missing PageName in first SubIFD");
    expect(strcmp(page_name, "200 th. IFD") == 0,
           "unexpected first SubIFD PageName");
    expect(TIFFReadDirectory(tif) == 1, "TIFFReadDirectory should advance in SubIFD chain");
    expect(TIFFGetField(tif, TIFFTAG_PAGENAME, &page_name) == 1,
           "missing PageName in second SubIFD");
    expect(strcmp(page_name, "201 th. IFD") == 0,
           "unexpected second SubIFD PageName");

    TIFFClose(tif);

    tif = expect_open(custom_dir_exif_gps_path(), "EXIF/GPS smoke image");
    expect(TIFFGetField(tif, TIFFTAG_EXIFIFD, &exif_offset) == 1,
           "missing EXIF directory offset");
    expect(TIFFReadEXIFDirectory(tif, exif_offset) == 1,
           "TIFFReadEXIFDirectory failed");
    expect(TIFFCurrentDirOffset(tif) == exif_offset,
           "EXIF current offset mismatch");
    expect(TIFFGetField(tif, EXIFTAG_DATETIMEORIGINAL, &date_time_original) == 1,
           "missing DateTimeOriginal");
    expect(strncmp(date_time_original, "N13-String-13", 13) == 0,
           "unexpected DateTimeOriginal");
    expect(TIFFGetField(tif, EXIFTAG_EXIFVERSION, &exif_version) == 1,
           "missing ExifVersion");
    expect(memcmp(exif_version, "0231", 4) == 0, "unexpected ExifVersion bytes");
    capture_print_directory(tif, 0, print_buffer, sizeof(print_buffer));
    expect_contains(print_buffer, "DateTimeOriginal:", "missing EXIF print line");
    expect_contains(print_buffer, "ExifVersion:", "missing ExifVersion print line");

    expect(TIFFSetDirectory(tif, 0) == 1, "failed to return to main directory");
    expect(TIFFGetField(tif, TIFFTAG_GPSIFD, &gps_offset) == 1,
           "missing GPS directory offset");
    expect(TIFFReadGPSDirectory(tif, gps_offset) == 1,
           "TIFFReadGPSDirectory failed");
    expect(TIFFCurrentDirOffset(tif) == gps_offset,
           "GPS current offset mismatch");
    capture_print_directory(tif, 0, print_buffer, sizeof(print_buffer));
    expect_contains(print_buffer, "VersionID:", "missing GPS VersionID print line");

    TIFFClose(tif);
    return 0;
}
