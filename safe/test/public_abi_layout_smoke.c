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
    if (probe->version != 3)
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

    expect_equal_size(probe->tiff_codec_size, sizeof(TIFFCodec),
                      "TIFFCodec.sizeof");
    expect_equal_size(probe->tiff_codec_name_offset,
                      offsetof(TIFFCodec, name),
                      "TIFFCodec.name");
    expect_equal_size(probe->tiff_codec_scheme_offset,
                      offsetof(TIFFCodec, scheme),
                      "TIFFCodec.scheme");
    expect_equal_size(probe->tiff_codec_init_offset,
                      offsetof(TIFFCodec, init),
                      "TIFFCodec.init");

    expect_equal_size(probe->tiff_display_size, sizeof(TIFFDisplay),
                      "TIFFDisplay.sizeof");
    expect_equal_size(probe->tiff_display_d_mat_offset,
                      offsetof(TIFFDisplay, d_mat), "TIFFDisplay.d_mat");
    expect_equal_size(probe->tiff_display_d_ycr_offset,
                      offsetof(TIFFDisplay, d_YCR), "TIFFDisplay.d_YCR");
    expect_equal_size(probe->tiff_display_d_ycg_offset,
                      offsetof(TIFFDisplay, d_YCG), "TIFFDisplay.d_YCG");
    expect_equal_size(probe->tiff_display_d_ycb_offset,
                      offsetof(TIFFDisplay, d_YCB), "TIFFDisplay.d_YCB");
    expect_equal_size(probe->tiff_display_d_vrwr_offset,
                      offsetof(TIFFDisplay, d_Vrwr), "TIFFDisplay.d_Vrwr");
    expect_equal_size(probe->tiff_display_d_vrwg_offset,
                      offsetof(TIFFDisplay, d_Vrwg), "TIFFDisplay.d_Vrwg");
    expect_equal_size(probe->tiff_display_d_vrwb_offset,
                      offsetof(TIFFDisplay, d_Vrwb), "TIFFDisplay.d_Vrwb");
    expect_equal_size(probe->tiff_display_d_y0r_offset,
                      offsetof(TIFFDisplay, d_Y0R), "TIFFDisplay.d_Y0R");
    expect_equal_size(probe->tiff_display_d_y0g_offset,
                      offsetof(TIFFDisplay, d_Y0G), "TIFFDisplay.d_Y0G");
    expect_equal_size(probe->tiff_display_d_y0b_offset,
                      offsetof(TIFFDisplay, d_Y0B), "TIFFDisplay.d_Y0B");
    expect_equal_size(probe->tiff_display_d_gammar_offset,
                      offsetof(TIFFDisplay, d_gammaR),
                      "TIFFDisplay.d_gammaR");
    expect_equal_size(probe->tiff_display_d_gammag_offset,
                      offsetof(TIFFDisplay, d_gammaG),
                      "TIFFDisplay.d_gammaG");
    expect_equal_size(probe->tiff_display_d_gammab_offset,
                      offsetof(TIFFDisplay, d_gammaB),
                      "TIFFDisplay.d_gammaB");

    expect_equal_size(probe->tiff_ycbcr_to_rgb_size, sizeof(TIFFYCbCrToRGB),
                      "TIFFYCbCrToRGB.sizeof");
    expect_equal_size(probe->tiff_ycbcr_to_rgb_clamptab_offset,
                      offsetof(TIFFYCbCrToRGB, clamptab),
                      "TIFFYCbCrToRGB.clamptab");
    expect_equal_size(probe->tiff_ycbcr_to_rgb_cr_r_tab_offset,
                      offsetof(TIFFYCbCrToRGB, Cr_r_tab),
                      "TIFFYCbCrToRGB.Cr_r_tab");
    expect_equal_size(probe->tiff_ycbcr_to_rgb_cb_b_tab_offset,
                      offsetof(TIFFYCbCrToRGB, Cb_b_tab),
                      "TIFFYCbCrToRGB.Cb_b_tab");
    expect_equal_size(probe->tiff_ycbcr_to_rgb_cr_g_tab_offset,
                      offsetof(TIFFYCbCrToRGB, Cr_g_tab),
                      "TIFFYCbCrToRGB.Cr_g_tab");
    expect_equal_size(probe->tiff_ycbcr_to_rgb_cb_g_tab_offset,
                      offsetof(TIFFYCbCrToRGB, Cb_g_tab),
                      "TIFFYCbCrToRGB.Cb_g_tab");
    expect_equal_size(probe->tiff_ycbcr_to_rgb_y_tab_offset,
                      offsetof(TIFFYCbCrToRGB, Y_tab),
                      "TIFFYCbCrToRGB.Y_tab");

    expect_equal_size(probe->tiff_cielab_to_rgb_size, sizeof(TIFFCIELabToRGB),
                      "TIFFCIELabToRGB.sizeof");
    expect_equal_size(probe->tiff_cielab_to_rgb_range_offset,
                      offsetof(TIFFCIELabToRGB, range),
                      "TIFFCIELabToRGB.range");
    expect_equal_size(probe->tiff_cielab_to_rgb_rstep_offset,
                      offsetof(TIFFCIELabToRGB, rstep),
                      "TIFFCIELabToRGB.rstep");
    expect_equal_size(probe->tiff_cielab_to_rgb_gstep_offset,
                      offsetof(TIFFCIELabToRGB, gstep),
                      "TIFFCIELabToRGB.gstep");
    expect_equal_size(probe->tiff_cielab_to_rgb_bstep_offset,
                      offsetof(TIFFCIELabToRGB, bstep),
                      "TIFFCIELabToRGB.bstep");
    expect_equal_size(probe->tiff_cielab_to_rgb_x0_offset,
                      offsetof(TIFFCIELabToRGB, X0), "TIFFCIELabToRGB.X0");
    expect_equal_size(probe->tiff_cielab_to_rgb_y0_offset,
                      offsetof(TIFFCIELabToRGB, Y0), "TIFFCIELabToRGB.Y0");
    expect_equal_size(probe->tiff_cielab_to_rgb_z0_offset,
                      offsetof(TIFFCIELabToRGB, Z0), "TIFFCIELabToRGB.Z0");
    expect_equal_size(probe->tiff_cielab_to_rgb_display_offset,
                      offsetof(TIFFCIELabToRGB, display),
                      "TIFFCIELabToRGB.display");
    expect_equal_size(probe->tiff_cielab_to_rgb_yr2r_offset,
                      offsetof(TIFFCIELabToRGB, Yr2r),
                      "TIFFCIELabToRGB.Yr2r");
    expect_equal_size(probe->tiff_cielab_to_rgb_yg2g_offset,
                      offsetof(TIFFCIELabToRGB, Yg2g),
                      "TIFFCIELabToRGB.Yg2g");
    expect_equal_size(probe->tiff_cielab_to_rgb_yb2b_offset,
                      offsetof(TIFFCIELabToRGB, Yb2b),
                      "TIFFCIELabToRGB.Yb2b");

    expect_equal_size(probe->tiff_rgba_image_size, sizeof(TIFFRGBAImage),
                      "TIFFRGBAImage.sizeof");
    expect_equal_size(probe->tiff_rgba_image_tif_offset,
                      offsetof(TIFFRGBAImage, tif), "TIFFRGBAImage.tif");
    expect_equal_size(probe->tiff_rgba_image_stoponerr_offset,
                      offsetof(TIFFRGBAImage, stoponerr),
                      "TIFFRGBAImage.stoponerr");
    expect_equal_size(probe->tiff_rgba_image_is_contig_offset,
                      offsetof(TIFFRGBAImage, isContig),
                      "TIFFRGBAImage.isContig");
    expect_equal_size(probe->tiff_rgba_image_alpha_offset,
                      offsetof(TIFFRGBAImage, alpha), "TIFFRGBAImage.alpha");
    expect_equal_size(probe->tiff_rgba_image_width_offset,
                      offsetof(TIFFRGBAImage, width), "TIFFRGBAImage.width");
    expect_equal_size(probe->tiff_rgba_image_height_offset,
                      offsetof(TIFFRGBAImage, height), "TIFFRGBAImage.height");
    expect_equal_size(probe->tiff_rgba_image_bitspersample_offset,
                      offsetof(TIFFRGBAImage, bitspersample),
                      "TIFFRGBAImage.bitspersample");
    expect_equal_size(probe->tiff_rgba_image_samplesperpixel_offset,
                      offsetof(TIFFRGBAImage, samplesperpixel),
                      "TIFFRGBAImage.samplesperpixel");
    expect_equal_size(probe->tiff_rgba_image_orientation_offset,
                      offsetof(TIFFRGBAImage, orientation),
                      "TIFFRGBAImage.orientation");
    expect_equal_size(probe->tiff_rgba_image_req_orientation_offset,
                      offsetof(TIFFRGBAImage, req_orientation),
                      "TIFFRGBAImage.req_orientation");
    expect_equal_size(probe->tiff_rgba_image_photometric_offset,
                      offsetof(TIFFRGBAImage, photometric),
                      "TIFFRGBAImage.photometric");
    expect_equal_size(probe->tiff_rgba_image_redcmap_offset,
                      offsetof(TIFFRGBAImage, redcmap),
                      "TIFFRGBAImage.redcmap");
    expect_equal_size(probe->tiff_rgba_image_greencmap_offset,
                      offsetof(TIFFRGBAImage, greencmap),
                      "TIFFRGBAImage.greencmap");
    expect_equal_size(probe->tiff_rgba_image_bluecmap_offset,
                      offsetof(TIFFRGBAImage, bluecmap),
                      "TIFFRGBAImage.bluecmap");
    expect_equal_size(probe->tiff_rgba_image_get_offset,
                      offsetof(TIFFRGBAImage, get), "TIFFRGBAImage.get");
    expect_equal_size(probe->tiff_rgba_image_put_offset,
                      offsetof(TIFFRGBAImage, put), "TIFFRGBAImage.put");
    expect_equal_size(probe->tiff_rgba_image_map_offset,
                      offsetof(TIFFRGBAImage, Map), "TIFFRGBAImage.Map");
    expect_equal_size(probe->tiff_rgba_image_bwmap_offset,
                      offsetof(TIFFRGBAImage, BWmap), "TIFFRGBAImage.BWmap");
    expect_equal_size(probe->tiff_rgba_image_palmap_offset,
                      offsetof(TIFFRGBAImage, PALmap), "TIFFRGBAImage.PALmap");
    expect_equal_size(probe->tiff_rgba_image_ycbcr_offset,
                      offsetof(TIFFRGBAImage, ycbcr), "TIFFRGBAImage.ycbcr");
    expect_equal_size(probe->tiff_rgba_image_cielab_offset,
                      offsetof(TIFFRGBAImage, cielab), "TIFFRGBAImage.cielab");
    expect_equal_size(probe->tiff_rgba_image_uatoaa_offset,
                      offsetof(TIFFRGBAImage, UaToAa), "TIFFRGBAImage.UaToAa");
    expect_equal_size(probe->tiff_rgba_image_bitdepth16to8_offset,
                      offsetof(TIFFRGBAImage, Bitdepth16To8),
                      "TIFFRGBAImage.Bitdepth16To8");
    expect_equal_size(probe->tiff_rgba_image_row_offset_offset,
                      offsetof(TIFFRGBAImage, row_offset),
                      "TIFFRGBAImage.row_offset");
    expect_equal_size(probe->tiff_rgba_image_col_offset_offset,
                      offsetof(TIFFRGBAImage, col_offset),
                      "TIFFRGBAImage.col_offset");

    return 0;
}
