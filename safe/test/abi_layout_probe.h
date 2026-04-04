#ifndef SAFE_TEST_ABI_LAYOUT_PROBE_H
#define SAFE_TEST_ABI_LAYOUT_PROBE_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C"
{
#endif

    typedef struct
    {
        uint32_t version;
        size_t struct_size;
        size_t tiff_field_info_size;
        size_t tiff_field_info_field_tag_offset;
        size_t tiff_field_info_field_readcount_offset;
        size_t tiff_field_info_field_writecount_offset;
        size_t tiff_field_info_field_type_offset;
        size_t tiff_field_info_field_bit_offset;
        size_t tiff_field_info_field_oktochange_offset;
        size_t tiff_field_info_field_passcount_offset;
        size_t tiff_field_info_field_name_offset;
        size_t tiff_tag_methods_size;
        size_t tiff_tag_methods_vsetfield_offset;
        size_t tiff_tag_methods_vgetfield_offset;
        size_t tiff_tag_methods_printdir_offset;
    } SafeTiffAbiLayoutProbe;

    const SafeTiffAbiLayoutProbe *safe_tiff_abi_layout_probe(void);

#ifdef __cplusplus
}
#endif

#endif
