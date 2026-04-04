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
        size_t tiff_codec_size;
        size_t tiff_codec_name_offset;
        size_t tiff_codec_scheme_offset;
        size_t tiff_codec_init_offset;
        size_t tiff_display_size;
        size_t tiff_display_d_mat_offset;
        size_t tiff_display_d_ycr_offset;
        size_t tiff_display_d_ycg_offset;
        size_t tiff_display_d_ycb_offset;
        size_t tiff_display_d_vrwr_offset;
        size_t tiff_display_d_vrwg_offset;
        size_t tiff_display_d_vrwb_offset;
        size_t tiff_display_d_y0r_offset;
        size_t tiff_display_d_y0g_offset;
        size_t tiff_display_d_y0b_offset;
        size_t tiff_display_d_gammar_offset;
        size_t tiff_display_d_gammag_offset;
        size_t tiff_display_d_gammab_offset;
        size_t tiff_ycbcr_to_rgb_size;
        size_t tiff_ycbcr_to_rgb_clamptab_offset;
        size_t tiff_ycbcr_to_rgb_cr_r_tab_offset;
        size_t tiff_ycbcr_to_rgb_cb_b_tab_offset;
        size_t tiff_ycbcr_to_rgb_cr_g_tab_offset;
        size_t tiff_ycbcr_to_rgb_cb_g_tab_offset;
        size_t tiff_ycbcr_to_rgb_y_tab_offset;
        size_t tiff_cielab_to_rgb_size;
        size_t tiff_cielab_to_rgb_range_offset;
        size_t tiff_cielab_to_rgb_rstep_offset;
        size_t tiff_cielab_to_rgb_gstep_offset;
        size_t tiff_cielab_to_rgb_bstep_offset;
        size_t tiff_cielab_to_rgb_x0_offset;
        size_t tiff_cielab_to_rgb_y0_offset;
        size_t tiff_cielab_to_rgb_z0_offset;
        size_t tiff_cielab_to_rgb_display_offset;
        size_t tiff_cielab_to_rgb_yr2r_offset;
        size_t tiff_cielab_to_rgb_yg2g_offset;
        size_t tiff_cielab_to_rgb_yb2b_offset;
        size_t tiff_rgba_image_size;
        size_t tiff_rgba_image_tif_offset;
        size_t tiff_rgba_image_stoponerr_offset;
        size_t tiff_rgba_image_is_contig_offset;
        size_t tiff_rgba_image_alpha_offset;
        size_t tiff_rgba_image_width_offset;
        size_t tiff_rgba_image_height_offset;
        size_t tiff_rgba_image_bitspersample_offset;
        size_t tiff_rgba_image_samplesperpixel_offset;
        size_t tiff_rgba_image_orientation_offset;
        size_t tiff_rgba_image_req_orientation_offset;
        size_t tiff_rgba_image_photometric_offset;
        size_t tiff_rgba_image_redcmap_offset;
        size_t tiff_rgba_image_greencmap_offset;
        size_t tiff_rgba_image_bluecmap_offset;
        size_t tiff_rgba_image_get_offset;
        size_t tiff_rgba_image_put_offset;
        size_t tiff_rgba_image_map_offset;
        size_t tiff_rgba_image_bwmap_offset;
        size_t tiff_rgba_image_palmap_offset;
        size_t tiff_rgba_image_ycbcr_offset;
        size_t tiff_rgba_image_cielab_offset;
        size_t tiff_rgba_image_uatoaa_offset;
        size_t tiff_rgba_image_bitdepth16to8_offset;
        size_t tiff_rgba_image_row_offset_offset;
        size_t tiff_rgba_image_col_offset_offset;
    } SafeTiffAbiLayoutProbe;

    const SafeTiffAbiLayoutProbe *safe_tiff_abi_layout_probe(void);

#ifdef __cplusplus
}
#endif

#endif
