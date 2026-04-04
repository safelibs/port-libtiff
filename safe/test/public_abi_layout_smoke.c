/*
 * Smoke coverage for public non-opaque struct layout compatibility.
 */

#include "tif_config.h"

#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

#include "abi_layout_probe.h"
#include "tiffio.h"

static void fail(const char *message)
{
    fprintf(stderr, "%s\n", message);
    exit(1);
}

static void expect_equal_size(size_t actual, size_t expected,
                              const char *label)
{
    if (actual != expected)
    {
        fprintf(stderr, "%s mismatch: actual=%zu expected=%zu\n", label,
                actual, expected);
        exit(1);
    }
}

int main(void)
{
    const SafeTiffAbiLayoutProbe *probe = safe_tiff_abi_layout_probe();

    if (probe == NULL)
        fail("safe_tiff_abi_layout_probe returned NULL");
    if (probe->version != 1)
        fail("unexpected ABI layout probe version");

    expect_equal_size(probe->struct_size, sizeof(*probe),
                      "SafeTiffAbiLayoutProbe.sizeof");

    expect_equal_size(probe->tiff_field_info_size, sizeof(TIFFFieldInfo),
                      "TIFFFieldInfo.sizeof");
    expect_equal_size(probe->tiff_field_info_field_tag_offset,
                      offsetof(TIFFFieldInfo, field_tag),
                      "TIFFFieldInfo.field_tag");
    expect_equal_size(probe->tiff_field_info_field_readcount_offset,
                      offsetof(TIFFFieldInfo, field_readcount),
                      "TIFFFieldInfo.field_readcount");
    expect_equal_size(probe->tiff_field_info_field_writecount_offset,
                      offsetof(TIFFFieldInfo, field_writecount),
                      "TIFFFieldInfo.field_writecount");
    expect_equal_size(probe->tiff_field_info_field_type_offset,
                      offsetof(TIFFFieldInfo, field_type),
                      "TIFFFieldInfo.field_type");
    expect_equal_size(probe->tiff_field_info_field_bit_offset,
                      offsetof(TIFFFieldInfo, field_bit),
                      "TIFFFieldInfo.field_bit");
    expect_equal_size(probe->tiff_field_info_field_oktochange_offset,
                      offsetof(TIFFFieldInfo, field_oktochange),
                      "TIFFFieldInfo.field_oktochange");
    expect_equal_size(probe->tiff_field_info_field_passcount_offset,
                      offsetof(TIFFFieldInfo, field_passcount),
                      "TIFFFieldInfo.field_passcount");
    expect_equal_size(probe->tiff_field_info_field_name_offset,
                      offsetof(TIFFFieldInfo, field_name),
                      "TIFFFieldInfo.field_name");

    expect_equal_size(probe->tiff_tag_methods_size, sizeof(TIFFTagMethods),
                      "TIFFTagMethods.sizeof");
    expect_equal_size(probe->tiff_tag_methods_vsetfield_offset,
                      offsetof(TIFFTagMethods, vsetfield),
                      "TIFFTagMethods.vsetfield");
    expect_equal_size(probe->tiff_tag_methods_vgetfield_offset,
                      offsetof(TIFFTagMethods, vgetfield),
                      "TIFFTagMethods.vgetfield");
    expect_equal_size(probe->tiff_tag_methods_printdir_offset,
                      offsetof(TIFFTagMethods, printdir),
                      "TIFFTagMethods.printdir");

    return 0;
}
