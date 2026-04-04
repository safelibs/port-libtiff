#include <stddef.h>
#include <stdio.h>

#include <jpeglib.h>
#include <setjmp.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

struct safe_jpeg_error_mgr
{
    struct jpeg_error_mgr pub;
    jmp_buf setjmp_buffer;
    char *message;
    size_t message_len;
};

static void safe_jpeg_set_message(struct safe_jpeg_error_mgr *err,
                                  const char *message)
{
    if (err == NULL || err->message == NULL || err->message_len == 0)
        return;
    snprintf(err->message, err->message_len, "%s", message);
}

static void safe_jpeg_error_exit(j_common_ptr cinfo)
{
    struct safe_jpeg_error_mgr *err =
        (struct safe_jpeg_error_mgr *)cinfo->err;
    if (err != NULL && err->message != NULL && err->message_len != 0)
        (*cinfo->err->format_message)(cinfo, err->message);
    longjmp(err->setjmp_buffer, 1);
}

static int safe_jpeg_decode_rgb_impl(const uint8_t *jpeg_data, size_t jpeg_len,
                                     uint8_t *out, size_t out_len,
                                     uint32_t *out_width,
                                     uint32_t *out_height, char *errbuf,
                                     size_t errbuf_len)
{
    struct jpeg_decompress_struct cinfo;
    struct safe_jpeg_error_mgr err;
    size_t row_stride;
    size_t expected_size;

    if (jpeg_data == NULL || jpeg_len == 0 || out == NULL)
        return 0;

    memset(&cinfo, 0, sizeof(cinfo));
    memset(&err, 0, sizeof(err));
    err.message = errbuf;
    err.message_len = errbuf_len;

    cinfo.err = jpeg_std_error(&err.pub);
    err.pub.error_exit = safe_jpeg_error_exit;

    if (setjmp(err.setjmp_buffer) != 0)
    {
        jpeg_destroy_decompress(&cinfo);
        return 0;
    }

    jpeg_create_decompress(&cinfo);
    jpeg_mem_src(&cinfo, (unsigned char *)jpeg_data, jpeg_len);
    if (jpeg_read_header(&cinfo, TRUE) != JPEG_HEADER_OK)
    {
        safe_jpeg_set_message(&err, "Invalid JPEG header");
        jpeg_destroy_decompress(&cinfo);
        return 0;
    }

    cinfo.out_color_space = JCS_RGB;
    if (!jpeg_start_decompress(&cinfo))
    {
        safe_jpeg_set_message(&err, "JPEG decompressor rejected the stream");
        jpeg_destroy_decompress(&cinfo);
        return 0;
    }

    row_stride = (size_t)cinfo.output_width * (size_t)cinfo.output_components;
    expected_size = row_stride * (size_t)cinfo.output_height;
    if (cinfo.output_components != 3 || row_stride == 0 || out_len < expected_size)
    {
        safe_jpeg_set_message(&err, "Decoded JPEG size does not match output buffer");
        jpeg_finish_decompress(&cinfo);
        jpeg_destroy_decompress(&cinfo);
        return 0;
    }

    while (cinfo.output_scanline < cinfo.output_height)
    {
        JSAMPROW row = out + (size_t)cinfo.output_scanline * row_stride;
        if (jpeg_read_scanlines(&cinfo, &row, 1) != 1)
        {
            safe_jpeg_set_message(&err, "JPEG scanline decode failed");
            jpeg_finish_decompress(&cinfo);
            jpeg_destroy_decompress(&cinfo);
            return 0;
        }
    }

    if (!jpeg_finish_decompress(&cinfo))
    {
        safe_jpeg_set_message(&err, "JPEG decompressor did not finish cleanly");
        jpeg_destroy_decompress(&cinfo);
        return 0;
    }

    if (out_width != NULL)
        *out_width = cinfo.output_width;
    if (out_height != NULL)
        *out_height = cinfo.output_height;

    jpeg_destroy_decompress(&cinfo);
    return 1;
}

static int safe_jpeg_decode_raw_impl(const uint8_t *jpeg_data, size_t jpeg_len,
                                     uint8_t *out, size_t out_len,
                                     uint32_t subsampling_h,
                                     uint32_t subsampling_v, char *errbuf,
                                     size_t errbuf_len)
{
    struct jpeg_decompress_struct cinfo;
    struct safe_jpeg_error_mgr err;
    JSAMPARRAY planes[3] = {NULL, NULL, NULL};
    size_t written = 0;
    int hsamp;
    int vsamp;
    JDIMENSION clumps_per_line;
    size_t bytes_per_line;
    int samples_per_clump;
    int scan_count = DCTSIZE;
    JDIMENSION remaining_rows;

    if (jpeg_data == NULL || jpeg_len == 0 || out == NULL)
        return 0;

    memset(&cinfo, 0, sizeof(cinfo));
    memset(&err, 0, sizeof(err));
    err.message = errbuf;
    err.message_len = errbuf_len;

    cinfo.err = jpeg_std_error(&err.pub);
    err.pub.error_exit = safe_jpeg_error_exit;

    if (setjmp(err.setjmp_buffer) != 0)
    {
        jpeg_destroy_decompress(&cinfo);
        return 0;
    }

    jpeg_create_decompress(&cinfo);
    jpeg_mem_src(&cinfo, (unsigned char *)jpeg_data, jpeg_len);
    if (jpeg_read_header(&cinfo, TRUE) != JPEG_HEADER_OK)
    {
        safe_jpeg_set_message(&err, "Invalid JPEG header");
        jpeg_destroy_decompress(&cinfo);
        return 0;
    }

    if (cinfo.num_components != 3 || cinfo.jpeg_color_space != JCS_YCbCr)
    {
        safe_jpeg_set_message(&err, "JPEG raw decode requires YCbCr input");
        jpeg_destroy_decompress(&cinfo);
        return 0;
    }

    if (cinfo.comp_info[1].h_samp_factor == 0 || cinfo.comp_info[1].v_samp_factor == 0)
    {
        safe_jpeg_set_message(&err, "Invalid JPEG sampling factors");
        jpeg_destroy_decompress(&cinfo);
        return 0;
    }

    hsamp = cinfo.comp_info[0].h_samp_factor / cinfo.comp_info[1].h_samp_factor;
    vsamp = cinfo.comp_info[0].v_samp_factor / cinfo.comp_info[1].v_samp_factor;
    if (hsamp <= 0 || vsamp <= 0)
    {
        safe_jpeg_set_message(&err, "Unsupported JPEG sampling layout");
        jpeg_destroy_decompress(&cinfo);
        return 0;
    }
    if ((subsampling_h != 0 && subsampling_h != (uint32_t)hsamp) ||
        (subsampling_v != 0 && subsampling_v != (uint32_t)vsamp))
    {
        safe_jpeg_set_message(&err, "JPEG sampling factors do not match TIFF tags");
        jpeg_destroy_decompress(&cinfo);
        return 0;
    }

    cinfo.raw_data_out = TRUE;
    cinfo.out_color_space = JCS_YCbCr;
    if (!jpeg_start_decompress(&cinfo))
    {
        safe_jpeg_set_message(&err, "JPEG raw decompressor rejected the stream");
        jpeg_destroy_decompress(&cinfo);
        return 0;
    }

    planes[0] = (*cinfo.mem->alloc_sarray)((j_common_ptr)&cinfo, JPOOL_IMAGE,
                                           cinfo.comp_info[0].width_in_blocks *
                                               DCTSIZE,
                                           cinfo.comp_info[0].v_samp_factor *
                                               DCTSIZE);
    planes[1] = (*cinfo.mem->alloc_sarray)((j_common_ptr)&cinfo, JPOOL_IMAGE,
                                           cinfo.comp_info[1].width_in_blocks *
                                               DCTSIZE,
                                           cinfo.comp_info[1].v_samp_factor *
                                               DCTSIZE);
    planes[2] = (*cinfo.mem->alloc_sarray)((j_common_ptr)&cinfo, JPOOL_IMAGE,
                                           cinfo.comp_info[2].width_in_blocks *
                                               DCTSIZE,
                                           cinfo.comp_info[2].v_samp_factor *
                                               DCTSIZE);
    clumps_per_line = cinfo.comp_info[1].downsampled_width;
    samples_per_clump = hsamp * vsamp + 2;
    bytes_per_line = (size_t)clumps_per_line * (size_t)samples_per_clump;
    remaining_rows = cinfo.image_height;

    while (remaining_rows > 0)
    {
        int ci;
        int clumpoffset;
        jpeg_component_info *compptr;

        if (written + bytes_per_line > out_len)
        {
            safe_jpeg_set_message(
                &err, "JPEG raw-data output exceeded TIFF buffer");
            jpeg_finish_decompress(&cinfo);
            jpeg_destroy_decompress(&cinfo);
            return 0;
        }

        if (scan_count >= DCTSIZE)
        {
            int n = cinfo.max_v_samp_factor * DCTSIZE;
            JDIMENSION got = jpeg_read_raw_data(&cinfo, planes, (JDIMENSION)n);
            if (got != (JDIMENSION)n)
            {
                safe_jpeg_set_message(
                    &err, "JPEG raw-data decode returned malformed row groups");
                jpeg_finish_decompress(&cinfo);
                jpeg_destroy_decompress(&cinfo);
                return 0;
            }
            scan_count = 0;
        }

        clumpoffset = 0;
        for (ci = 0, compptr = cinfo.comp_info; ci < cinfo.num_components;
             ci++, compptr++)
        {
            int component_hsamp = compptr->h_samp_factor;
            int component_vsamp = compptr->v_samp_factor;
            int ypos;

            for (ypos = 0; ypos < component_vsamp; ypos++)
            {
                JSAMPROW inptr =
                    planes[ci][scan_count * component_vsamp + ypos];
                uint8_t *outptr = out + written + (size_t)clumpoffset;
                JDIMENSION nclump;

                if (component_hsamp == 1)
                {
                    for (nclump = clumps_per_line; nclump-- > 0;)
                    {
                        outptr[0] = *inptr++;
                        outptr += samples_per_clump;
                    }
                }
                else
                {
                    JDIMENSION nclump;
                    for (nclump = clumps_per_line; nclump-- > 0;)
                    {
                        int xpos;
                        for (xpos = 0; xpos < component_hsamp; xpos++)
                            outptr[xpos] = *inptr++;
                        outptr += samples_per_clump;
                    }
                }
                clumpoffset += component_hsamp;
            }
        }

        scan_count++;
        written += bytes_per_line;
        if (remaining_rows > (JDIMENSION)vsamp)
            remaining_rows -= (JDIMENSION)vsamp;
        else
            remaining_rows = 0;
    }

    if (!jpeg_finish_decompress(&cinfo))
    {
        safe_jpeg_set_message(&err,
                              "JPEG raw decompressor did not finish cleanly");
        jpeg_destroy_decompress(&cinfo);
        return 0;
    }

    jpeg_destroy_decompress(&cinfo);

    if (written != out_len)
    {
        char message[128];
        snprintf(message, sizeof(message),
                 "JPEG raw-data output size mismatch (%zu != %zu, clumps=%u, "
                 "samples=%d)",
                 written, out_len, (unsigned)clumps_per_line,
                 samples_per_clump);
        safe_jpeg_set_message(&err, message);
        return 0;
    }

    return 1;
}

int safe_tiff_jpeg_decode_rgb(const uint8_t *jpeg_data, size_t jpeg_len,
                              uint8_t *out, size_t out_len,
                              uint32_t *out_width, uint32_t *out_height,
                              char *errbuf, size_t errbuf_len)
{
    return safe_jpeg_decode_rgb_impl(jpeg_data, jpeg_len, out, out_len,
                                     out_width, out_height, errbuf, errbuf_len);
}

int safe_tiff_jpeg_decode_raw_ycbcr(const uint8_t *jpeg_data, size_t jpeg_len,
                                    uint8_t *out, size_t out_len,
                                    uint32_t subsampling_h,
                                    uint32_t subsampling_v, char *errbuf,
                                    size_t errbuf_len)
{
    return safe_jpeg_decode_raw_impl(jpeg_data, jpeg_len, out, out_len,
                                     subsampling_h, subsampling_v, errbuf,
                                     errbuf_len);
}
