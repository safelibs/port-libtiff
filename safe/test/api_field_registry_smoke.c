/*
 * Smoke coverage for the field-registry helper surface.
 */

#include "tif_config.h"

#include <assert.h>
#include <fcntl.h>
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
    TIFFTAG_SMOKE_CUSTOM = 65010,
    TIFFTAG_EXTENDER_CUSTOM = 65011,
    TIFFTAG_SMOKE_GENERIC = 65012,
};

static const TIFFFieldInfo custom_field_info[] = {
    {TIFFTAG_SMOKE_CUSTOM, TIFF_VARIABLE2, TIFF_VARIABLE2, TIFF_DOUBLE,
     FIELD_CUSTOM, 1, 1, "SmokeCustom"},
};

static const TIFFFieldInfo extender_field_info[] = {
    {TIFFTAG_EXTENDER_CUSTOM, 1, 1, TIFF_LONG, FIELD_CUSTOM, 1, 0,
     "ExtenderCustom"},
};

static const TIFFFieldInfo generic_field_info[] = {
    {TIFFTAG_SMOKE_GENERIC, 1, 1, TIFF_ANY, FIELD_CUSTOM, 1, 0,
     "SmokeGeneric"},
};

static TIFFExtendProc g_parent_extender = NULL;
static int g_extender_calls = 0;

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

static void smoke_extender(TIFF *tif)
{
    g_extender_calls++;
    TIFFMergeFieldInfo(tif, extender_field_info,
                       (uint32_t)(sizeof(extender_field_info) /
                                  sizeof(extender_field_info[0])));
    if (g_parent_extender)
        (*g_parent_extender)(tif);
}

static void expect_field(const TIFFField *field, uint32_t tag,
                         TIFFDataType type, int read_count, int write_count,
                         int passcount, const char *name)
{
    expect(field != NULL, "expected field metadata");
    expect(TIFFFieldTag(field) == tag, "unexpected field tag");
    expect(TIFFFieldDataType(field) == type, "unexpected field type");
    expect(TIFFFieldReadCount(field) == read_count, "unexpected read count");
    expect(TIFFFieldWriteCount(field) == write_count,
           "unexpected write count");
    expect(TIFFFieldPassCount(field) == passcount, "unexpected passcount");
    expect(strcmp(TIFFFieldName(field), name) == 0, "unexpected field name");
}

int main(void)
{
    static const struct
    {
        TIFFDataType type;
        int expected;
    } width_cases[] = {
        {TIFF_NOTYPE, 1},   {TIFF_BYTE, 1},     {TIFF_ASCII, 1},
        {TIFF_SHORT, 2},    {TIFF_LONG, 4},     {TIFF_RATIONAL, 8},
        {TIFF_SBYTE, 1},    {TIFF_UNDEFINED, 1}, {TIFF_SSHORT, 2},
        {TIFF_SLONG, 4},    {TIFF_SRATIONAL, 8}, {TIFF_FLOAT, 4},
        {TIFF_DOUBLE, 8},   {TIFF_IFD, 4},      {TIFF_LONG8, 8},
        {TIFF_SLONG8, 8},   {TIFF_IFD8, 8},
    };
    char path[] = "api_field_registry_smokeXXXXXX";
    char extender_path[] = "api_field_registry_extenderXXXXXX";
    int fd;
    TIFF *tif;
    TIFF *ext_tif;
    TIFFTagMethods *tag_methods;
    const TIFFFieldArray *exif_fields;
    const TIFFFieldArray *gps_fields;
    const TIFFField *field;
    const TIFFField *custom_field;
    double custom_value = 7.25;
    int i;
    int client_marker = 42;

    for (i = 0; i < (int)(sizeof(width_cases) / sizeof(width_cases[0])); ++i)
    {
        if (TIFFDataWidth(width_cases[i].type) != width_cases[i].expected)
            fail("TIFFDataWidth returned an unexpected width");
    }
    expect(TIFFDataWidth((TIFFDataType)99) == 0,
           "TIFFDataWidth should return 0 for unknown types");

    exif_fields = _TIFFGetExifFields();
    gps_fields = _TIFFGetGpsFields();
    expect(exif_fields != NULL && exif_fields->count == 81 &&
               exif_fields->fields != NULL,
           "EXIF field helper returned unexpected metadata");
    expect(gps_fields != NULL && gps_fields->count == 32 &&
               gps_fields->fields != NULL,
           "GPS field helper returned unexpected metadata");
    expect_field(&exif_fields->fields[0], 33434, TIFF_RATIONAL, 1, 1, 0,
                 "ExposureTime");
    expect_field(&gps_fields->fields[2], 2, TIFF_RATIONAL, 3, 3, 0,
                 "Latitude");
    expect(TIFFFieldSetGetSize(&gps_fields->fields[6]) == 8,
           "unexpected GPS set/get size");
    expect(TIFFFieldSetGetCountSize(&gps_fields->fields[27]) == 2,
           "unexpected GPS count size");

    fd = mkstemp(path);
    if (fd < 0)
        fail("mkstemp failed");
    close(fd);
    unlink(path);

    tif = TIFFOpen(path, "w");
    expect(tif != NULL, "TIFFOpen failed for field-registry smoke");

    field = TIFFFindField(tif, TIFFTAG_IMAGEWIDTH, TIFF_ANY);
    expect_field(field, TIFFTAG_IMAGEWIDTH, TIFF_LONG, 1, 1, 0, "ImageWidth");
    expect(TIFFFieldWithTag(tif, TIFFTAG_IMAGEWIDTH) == field,
           "TIFFFieldWithTag mismatch");
    expect(TIFFFieldWithName(tif, "ImageWidth") == field,
           "TIFFFieldWithName mismatch");
    expect(TIFFFindField(tif, TIFFTAG_SMINSAMPLEVALUE, TIFF_ANY) != NULL,
           "expected TIFF_ANY lookup for SMinSampleValue");
    expect(TIFFFindField(tif, TIFFTAG_SMINSAMPLEVALUE, TIFF_SHORT) == NULL,
           "typed lookup must not match a TIFF_ANY descriptor");

    tag_methods = TIFFAccessTagMethods(tif);
    expect(tag_methods != NULL, "TIFFAccessTagMethods returned NULL");
    expect(TIFFAccessTagMethods(tif) == tag_methods,
           "TIFFAccessTagMethods should be stable");
    expect(tag_methods->vsetfield != NULL,
           "TIFFAccessTagMethods should expose a default vsetfield callback");
    expect(tag_methods->vgetfield != NULL,
           "TIFFAccessTagMethods should expose a default vgetfield callback");
    expect(tag_methods->printdir == NULL,
           "TIFFAccessTagMethods default printdir should be NULL");
    expect(tag_methods->vsetfield != TIFFVSetField,
           "TIFFAccessTagMethods should expose the internal vsetfield callback");
    expect(tag_methods->vgetfield != TIFFVGetField,
           "TIFFAccessTagMethods should expose the internal vgetfield callback");

    expect(TIFFGetClientInfo(tif, "smoke-client") == NULL,
           "unexpected preexisting client info");
    TIFFSetClientInfo(tif, &client_marker, "smoke-client");
    expect(TIFFGetClientInfo(tif, "smoke-client") == &client_marker,
           "client info roundtrip failed");

    expect(TIFFGetTagListCount(tif) == 0, "unexpected initial custom tag list count");
    expect(TIFFGetTagListEntry(tif, 0) == (uint32_t)-1,
           "unexpected initial custom tag list entry");

    expect(TIFFMergeFieldInfo(tif, custom_field_info,
                              (uint32_t)(sizeof(custom_field_info) /
                                         sizeof(custom_field_info[0]))) == 0,
           "TIFFMergeFieldInfo failed");
    custom_field = TIFFFindField(tif, TIFFTAG_SMOKE_CUSTOM, TIFF_ANY);
    expect_field(custom_field, TIFFTAG_SMOKE_CUSTOM, TIFF_DOUBLE,
                 TIFF_VARIABLE2, TIFF_VARIABLE2, 1, "SmokeCustom");
    expect(TIFFFieldSetGetSize(custom_field) == 8,
           "unexpected custom field set/get size");
    expect(TIFFFieldSetGetCountSize(custom_field) == 4,
           "unexpected custom field count size");
    expect(TIFFFieldIsAnonymous(custom_field) == 0,
           "merged custom field should not be anonymous");
    expect(TIFFMergeFieldInfo(tif, generic_field_info,
                              (uint32_t)(sizeof(generic_field_info) /
                                         sizeof(generic_field_info[0]))) == 0,
           "TIFFMergeFieldInfo failed for generic field");
    field = TIFFFindField(tif, TIFFTAG_SMOKE_GENERIC, TIFF_ANY);
    expect_field(field, TIFFTAG_SMOKE_GENERIC, TIFF_ANY, 1, 1, 0,
                 "SmokeGeneric");
    expect(TIFFFindField(tif, TIFFTAG_SMOKE_GENERIC, TIFF_SHORT) == NULL,
           "typed lookup must miss a generic custom descriptor");
    expect(TIFFSetField(tif, TIFFTAG_SMOKE_CUSTOM, (uint32_t)1, &custom_value) == 1,
           "TIFFSetField failed for merged custom field");
    expect(TIFFGetTagListCount(tif) == 1,
           "custom tag set should be recorded in the tag list");
    expect(TIFFGetTagListEntry(tif, 0) == TIFFTAG_SMOKE_CUSTOM,
           "unexpected recorded custom tag");
    expect(TIFFSetField(tif, TIFFTAG_FILLORDER, FILLORDER_LSB2MSB) == 1,
           "TIFFSetField failed for built-in field");
    expect(TIFFGetTagListCount(tif) == 1,
           "built-in tags must not be added to the custom tag list");
    expect(TIFFSetField(tif, TIFFTAG_SMOKE_CUSTOM, (uint32_t)1, &custom_value) == 1,
           "TIFFSetField failed when updating merged custom field");
    expect(TIFFGetTagListCount(tif) == 1,
           "setting the same custom tag twice must not duplicate it");
    expect(TIFFUnsetField(tif, TIFFTAG_SMOKE_CUSTOM) == 1,
           "TIFFUnsetField failed for merged custom field");
    expect(TIFFGetTagListCount(tif) == 0,
           "unsetting the custom tag should clear it from the tag list");
    expect(TIFFGetTagListEntry(tif, 0) == (uint32_t)-1,
           "unexpected tag list entry after unsetting the custom field");

    TIFFClose(tif);
    unlink(path);

    fd = mkstemp(extender_path);
    if (fd < 0)
        fail("mkstemp extender failed");
    close(fd);
    unlink(extender_path);

    g_extender_calls = 0;
    g_parent_extender = TIFFSetTagExtender(smoke_extender);
    ext_tif = TIFFOpen(extender_path, "w");
    TIFFSetTagExtender(g_parent_extender);
    expect(ext_tif != NULL, "TIFFOpen failed for extender smoke");
    expect(g_extender_calls == 1, "tag extender did not run exactly once");
    expect(TIFFFindField(ext_tif, TIFFTAG_EXTENDER_CUSTOM, TIFF_ANY) != NULL,
           "tag extender field was not merged");
    TIFFClose(ext_tif);
    unlink(extender_path);

    return 0;
}
