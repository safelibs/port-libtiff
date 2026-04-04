#include <Lerc_c_api.h>
#include <jbig.h>
#include <lzma.h>
#include <webp/decode.h>
#include <webp/encode.h>
#include <zstd.h>

#include <math.h>
#include <limits.h>
#include <stdarg.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static void safe_set_message(char *errbuf, size_t errbuf_len, const char *fmt, ...)
{
    va_list ap;

    if (errbuf == NULL || errbuf_len == 0)
        return;

    va_start(ap, fmt);
    vsnprintf(errbuf, errbuf_len, fmt, ap);
    va_end(ap);
}

static uint8_t safe_reverse_byte(uint8_t value)
{
    value = (uint8_t)(((value & 0xF0u) >> 4) | ((value & 0x0Fu) << 4));
    value = (uint8_t)(((value & 0xCCu) >> 2) | ((value & 0x33u) << 2));
    value = (uint8_t)(((value & 0xAAu) >> 1) | ((value & 0x55u) << 1));
    return value;
}

static void safe_reverse_bits_in_place(uint8_t *data, size_t len)
{
    size_t i;
    if (data == NULL)
        return;
    for (i = 0; i < len; ++i)
        data[i] = safe_reverse_byte(data[i]);
}

static int safe_checked_mul_size(size_t a, size_t b, size_t *out)
{
    if (out == NULL)
        return 0;
    if (a != 0 && b > SIZE_MAX / a)
        return 0;
    *out = a * b;
    return 1;
}

static int safe_checked_mul3_size(size_t a, size_t b, size_t c, size_t *out)
{
    size_t ab = 0;
    if (!safe_checked_mul_size(a, b, &ab))
        return 0;
    return safe_checked_mul_size(ab, c, out);
}

static uint8_t *safe_malloc_bytes(size_t len, char *errbuf, size_t errbuf_len)
{
    uint8_t *ptr;
    if (len == 0)
        len = 1;
    ptr = (uint8_t *)malloc(len);
    if (ptr == NULL)
        safe_set_message(errbuf, errbuf_len, "Out of memory");
    return ptr;
}

void safe_tiff_external_codec_free(void *ptr)
{
    free(ptr);
}

int safe_tiff_zstd_max_c_level(void)
{
    return ZSTD_maxCLevel();
}

struct safe_jbig_buffer
{
    uint8_t *data;
    size_t size;
    size_t capacity;
    int reverse_output;
    char *errbuf;
    size_t errbuf_len;
    int failed;
};

static int safe_jbig_reserve(struct safe_jbig_buffer *buffer, size_t extra)
{
    size_t needed;
    size_t new_capacity;
    uint8_t *new_data;

    if (buffer == NULL)
        return 0;
    if (buffer->failed)
        return 0;
    if (extra > SIZE_MAX - buffer->size)
        return 0;
    needed = buffer->size + extra;
    if (needed <= buffer->capacity)
        return 1;

    new_capacity = buffer->capacity == 0 ? 1024 : buffer->capacity;
    while (new_capacity < needed)
    {
        if (new_capacity > SIZE_MAX / 2)
        {
            new_capacity = needed;
            break;
        }
        new_capacity *= 2;
    }

    new_data = (uint8_t *)realloc(buffer->data, new_capacity);
    if (new_data == NULL)
    {
        safe_set_message(buffer->errbuf, buffer->errbuf_len, "Out of memory");
        buffer->failed = 1;
        return 0;
    }

    buffer->data = new_data;
    buffer->capacity = new_capacity;
    return 1;
}

static void safe_jbig_output(unsigned char *chunk, size_t len, void *user_data)
{
    struct safe_jbig_buffer *buffer = (struct safe_jbig_buffer *)user_data;
    size_t old_size;

    if (buffer == NULL || buffer->failed || len == 0)
        return;
    if (!safe_jbig_reserve(buffer, len))
        return;

    old_size = buffer->size;
    memcpy(buffer->data + old_size, chunk, len);
    if (buffer->reverse_output)
        safe_reverse_bits_in_place(buffer->data + old_size, len);
    buffer->size += len;
}

int safe_tiff_jbig_decode(const uint8_t *input, size_t input_len, int reverse_input,
                          uint8_t *out, size_t out_len, char *errbuf,
                          size_t errbuf_len)
{
    struct jbg_dec_state decoder;
    uint8_t *owned_input = NULL;
    unsigned char *image;
    unsigned long decoded_size;
    int status;

    if (input == NULL || input_len == 0 || out == NULL)
    {
        safe_set_message(errbuf, errbuf_len, "Invalid JBIG decode arguments");
        return 0;
    }

    if (reverse_input)
    {
        owned_input = safe_malloc_bytes(input_len, errbuf, errbuf_len);
        if (owned_input == NULL)
            return 0;
        memcpy(owned_input, input, input_len);
        safe_reverse_bits_in_place(owned_input, input_len);
        input = owned_input;
    }

    memset(out, 0, out_len);
    jbg_dec_init(&decoder);
#if defined(HAVE_JBG_NEWLEN)
    (void)jbg_newlen((unsigned char *)input, input_len);
#endif
    status = jbg_dec_in(&decoder, (unsigned char *)input, input_len, NULL);
    if (status != JBG_EOK)
    {
        safe_set_message(errbuf, errbuf_len, "JBIG decode failed: %s",
                         jbg_strerror(status));
        jbg_dec_free(&decoder);
        free(owned_input);
        return 0;
    }

    decoded_size = jbg_dec_getsize(&decoder);
    if (decoded_size > out_len)
    {
        safe_set_message(errbuf, errbuf_len,
                         "JBIG decoded %lu bytes, expected at most %zu",
                         decoded_size, out_len);
        jbg_dec_free(&decoder);
        free(owned_input);
        return 0;
    }

    image = jbg_dec_getimage(&decoder, 0);
    if (image == NULL)
    {
        safe_set_message(errbuf, errbuf_len, "JBIG returned no decoded image");
        jbg_dec_free(&decoder);
        free(owned_input);
        return 0;
    }

    memcpy(out, image, decoded_size);
    jbg_dec_free(&decoder);
    free(owned_input);
    return 1;
}

int safe_tiff_jbig_encode(const uint8_t *input, uint32_t width, uint32_t height,
                          int reverse_output, uint8_t **out_ptr,
                          size_t *out_len, char *errbuf, size_t errbuf_len)
{
    struct jbg_enc_state encoder;
    struct safe_jbig_buffer buffer;
    unsigned char *planes[1];

    if (input == NULL || width == 0 || height == 0 || out_ptr == NULL ||
        out_len == NULL)
    {
        safe_set_message(errbuf, errbuf_len, "Invalid JBIG encode arguments");
        return 0;
    }

    memset(&buffer, 0, sizeof(buffer));
    buffer.reverse_output = reverse_output;
    buffer.errbuf = errbuf;
    buffer.errbuf_len = errbuf_len;
    planes[0] = (unsigned char *)input;

    jbg_enc_init(&encoder, width, height, 1, planes, safe_jbig_output, &buffer);
    jbg_enc_out(&encoder);
    jbg_enc_free(&encoder);

    if (buffer.failed)
    {
        free(buffer.data);
        return 0;
    }

    *out_ptr = buffer.data;
    *out_len = buffer.size;
    return 1;
}

static const char *safe_lzma_error_name(lzma_ret ret)
{
    switch (ret)
    {
        case LZMA_OK:
            return "success";
        case LZMA_FORMAT_ERROR:
            return "format error";
        case LZMA_OPTIONS_ERROR:
            return "options error";
        case LZMA_DATA_ERROR:
            return "data error";
        case LZMA_MEM_ERROR:
            return "memory error";
        case LZMA_MEMLIMIT_ERROR:
            return "memory limit error";
        case LZMA_BUF_ERROR:
            return "buffer too small";
        case LZMA_PROG_ERROR:
            return "programming error";
        default:
            return "unknown liblzma error";
    }
}

int safe_tiff_lzma_decode(const uint8_t *input, size_t input_len, uint8_t *out,
                          size_t out_len, char *errbuf, size_t errbuf_len)
{
    uint64_t memlimit = UINT64_MAX;
    size_t in_pos = 0;
    size_t out_pos = 0;
    lzma_ret ret;

    if (input == NULL || out == NULL)
    {
        safe_set_message(errbuf, errbuf_len, "Invalid LZMA decode arguments");
        return 0;
    }

    ret = lzma_stream_buffer_decode(&memlimit, 0, NULL, input, &in_pos,
                                    input_len, out, &out_pos, out_len);
    if (ret != LZMA_OK)
    {
        safe_set_message(errbuf, errbuf_len, "LZMA decode failed: %s",
                         safe_lzma_error_name(ret));
        return 0;
    }
    if (out_pos != out_len)
    {
        safe_set_message(errbuf, errbuf_len,
                         "LZMA decoded %zu bytes, expected %zu", out_pos,
                         out_len);
        return 0;
    }
    return 1;
}

int safe_tiff_lzma_encode(const uint8_t *input, size_t input_len, uint32_t preset,
                          uint8_t **out_ptr, size_t *out_len, char *errbuf,
                          size_t errbuf_len)
{
    size_t bound;
    size_t out_pos = 0;
    uint8_t *out;
    lzma_ret ret;

    if (input == NULL || out_ptr == NULL || out_len == NULL)
    {
        safe_set_message(errbuf, errbuf_len, "Invalid LZMA encode arguments");
        return 0;
    }

    bound = lzma_stream_buffer_bound(input_len);
    out = safe_malloc_bytes(bound, errbuf, errbuf_len);
    if (out == NULL)
        return 0;

    ret = lzma_easy_buffer_encode(preset, LZMA_CHECK_CRC64, NULL, input,
                                  input_len, out, &out_pos, bound);
    if (ret != LZMA_OK)
    {
        safe_set_message(errbuf, errbuf_len, "LZMA encode failed: %s",
                         safe_lzma_error_name(ret));
        free(out);
        return 0;
    }

    *out_ptr = out;
    *out_len = out_pos;
    return 1;
}

int safe_tiff_zstd_decode(const uint8_t *input, size_t input_len, uint8_t *out,
                          size_t out_len, char *errbuf, size_t errbuf_len)
{
    size_t ret;

    if (input == NULL || out == NULL)
    {
        safe_set_message(errbuf, errbuf_len, "Invalid ZSTD decode arguments");
        return 0;
    }

    ret = ZSTD_decompress(out, out_len, input, input_len);
    if (ZSTD_isError(ret))
    {
        safe_set_message(errbuf, errbuf_len, "ZSTD decode failed: %s",
                         ZSTD_getErrorName(ret));
        return 0;
    }
    if (ret != out_len)
    {
        safe_set_message(errbuf, errbuf_len,
                         "ZSTD decoded %zu bytes, expected %zu", ret, out_len);
        return 0;
    }

    return 1;
}

int safe_tiff_zstd_decode_alloc(const uint8_t *input, size_t input_len,
                                uint8_t **out_ptr, size_t *out_len,
                                char *errbuf, size_t errbuf_len)
{
    unsigned long long expected_size;
    uint8_t *out;
    size_t ret;

    if (input == NULL || out_ptr == NULL || out_len == NULL)
    {
        safe_set_message(errbuf, errbuf_len,
                         "Invalid ZSTD allocation decode arguments");
        return 0;
    }

    expected_size = ZSTD_getFrameContentSize(input, input_len);
    if (expected_size == ZSTD_CONTENTSIZE_ERROR ||
        expected_size == ZSTD_CONTENTSIZE_UNKNOWN || expected_size > SIZE_MAX)
    {
        safe_set_message(errbuf, errbuf_len,
                         "ZSTD frame content size is unavailable");
        return 0;
    }

    out = safe_malloc_bytes((size_t)expected_size, errbuf, errbuf_len);
    if (out == NULL)
        return 0;

    ret = ZSTD_decompress(out, (size_t)expected_size, input, input_len);
    if (ZSTD_isError(ret))
    {
        safe_set_message(errbuf, errbuf_len, "ZSTD decode failed: %s",
                         ZSTD_getErrorName(ret));
        free(out);
        return 0;
    }

    *out_ptr = out;
    *out_len = ret;
    return 1;
}

int safe_tiff_zstd_encode(const uint8_t *input, size_t input_len, int level,
                          uint8_t **out_ptr, size_t *out_len, char *errbuf,
                          size_t errbuf_len)
{
    size_t bound;
    uint8_t *out;
    size_t ret;

    if (input == NULL || out_ptr == NULL || out_len == NULL)
    {
        safe_set_message(errbuf, errbuf_len, "Invalid ZSTD encode arguments");
        return 0;
    }

    bound = ZSTD_compressBound(input_len);
    out = safe_malloc_bytes(bound, errbuf, errbuf_len);
    if (out == NULL)
        return 0;

    ret = ZSTD_compress(out, bound, input, input_len, level);
    if (ZSTD_isError(ret))
    {
        safe_set_message(errbuf, errbuf_len, "ZSTD encode failed: %s",
                         ZSTD_getErrorName(ret));
        free(out);
        return 0;
    }

    *out_ptr = out;
    *out_len = ret;
    return 1;
}

int safe_tiff_webp_decode(const uint8_t *input, size_t input_len, int samples,
                          uint32_t width, uint32_t height, uint8_t *out,
                          size_t out_len, char *errbuf, size_t errbuf_len)
{
    WebPBitstreamFeatures features;
    VP8StatusCode status;
    size_t expected_size = 0;
    uint8_t *decoded;

    if (input == NULL || out == NULL || (samples != 3 && samples != 4))
    {
        safe_set_message(errbuf, errbuf_len, "Invalid WebP decode arguments");
        return 0;
    }

    if (!safe_checked_mul3_size(width, height, (size_t)samples, &expected_size) ||
        expected_size > out_len)
    {
        safe_set_message(errbuf, errbuf_len, "WebP output buffer is too small");
        return 0;
    }

    status = WebPGetFeatures(input, input_len, &features);
    if (status != VP8_STATUS_OK)
    {
        safe_set_message(errbuf, errbuf_len, "WebPGetFeatures() failed");
        return 0;
    }
    if ((uint32_t)features.width != width || (uint32_t)features.height != height)
    {
        safe_set_message(errbuf, errbuf_len,
                         "WebP blob dimensions are %dx%d, expected %ux%u",
                         features.width, features.height, width, height);
        return 0;
    }
    if (samples == 3 && features.has_alpha)
    {
        safe_set_message(errbuf, errbuf_len,
                         "WebP blob contains alpha but TIFF expects RGB");
        return 0;
    }

    if (samples == 4)
    {
        decoded = WebPDecodeRGBAInto(input, input_len, out, expected_size,
                                     (int)(width * 4u));
    }
    else
    {
        decoded = WebPDecodeRGBInto(input, input_len, out, expected_size,
                                    (int)(width * 3u));
    }
    if (decoded == NULL)
    {
        safe_set_message(errbuf, errbuf_len, "WebP decode failed");
        return 0;
    }

    return 1;
}

int safe_tiff_webp_encode(const uint8_t *input, uint32_t width, uint32_t height,
                          int samples, float quality, int lossless, int exact,
                          uint8_t **out_ptr, size_t *out_len, char *errbuf,
                          size_t errbuf_len)
{
    size_t stride = 0;
    size_t encoded_len = 0;
    uint8_t *encoded = NULL;
    uint8_t *owned = NULL;

    (void)exact;

    if (input == NULL || out_ptr == NULL || out_len == NULL ||
        (samples != 3 && samples != 4))
    {
        safe_set_message(errbuf, errbuf_len, "Invalid WebP encode arguments");
        return 0;
    }

    if (!safe_checked_mul_size(width, (size_t)samples, &stride) || stride > INT_MAX)
    {
        safe_set_message(errbuf, errbuf_len, "WebP stride is too large");
        return 0;
    }

    if (samples == 4)
    {
        encoded_len = lossless
                          ? WebPEncodeLosslessRGBA(input, (int)width,
                                                   (int)height, (int)stride,
                                                   &encoded)
                          : WebPEncodeRGBA(input, (int)width, (int)height,
                                           (int)stride, quality, &encoded);
    }
    else
    {
        encoded_len = lossless
                          ? WebPEncodeLosslessRGB(input, (int)width,
                                                  (int)height, (int)stride,
                                                  &encoded)
                          : WebPEncodeRGB(input, (int)width, (int)height,
                                          (int)stride, quality, &encoded);
    }

    if (encoded_len == 0 || encoded == NULL)
    {
        safe_set_message(errbuf, errbuf_len, "WebP encode failed");
        return 0;
    }

    owned = safe_malloc_bytes(encoded_len, errbuf, errbuf_len);
    if (owned == NULL)
    {
        WebPFree(encoded);
        return 0;
    }
    memcpy(owned, encoded, encoded_len);
    WebPFree(encoded);

    *out_ptr = owned;
    *out_len = encoded_len;
    return 1;
}

static int safe_lerc_expected_size(int width, int height, int depth, int bands,
                                   int sample_bytes, size_t *out)
{
    if (width <= 0 || height <= 0 || depth <= 0 || bands <= 0 || sample_bytes <= 0)
        return 0;
    return safe_checked_mul3_size((size_t)width * (size_t)height,
                                  (size_t)depth * (size_t)bands,
                                  (size_t)sample_bytes, out);
}

int safe_tiff_lerc_decode(const uint8_t *blob, size_t blob_len,
                          unsigned int data_type, int width, int height,
                          int depth, int bands, int mask_mode, int sample_bytes,
                          int samples_per_pixel, uint8_t *out, size_t out_len,
                          char *errbuf, size_t errbuf_len)
{
    unsigned int info[11] = {0};
    size_t expected_size = 0;
    int blob_depth;
    int blob_masks;
    int want_alpha_mask;
    int want_float_mask;
    unsigned char *mask = NULL;
    uint8_t *temp = NULL;
    size_t temp_size = 0;
    lerc_status status;
    size_t pixel_count;
    size_t i;

    if (blob == NULL || out == NULL || blob_len == 0)
    {
        safe_set_message(errbuf, errbuf_len, "Invalid LERC decode arguments");
        return 0;
    }
    if (blob_len > UINT_MAX)
    {
        safe_set_message(errbuf, errbuf_len, "LERC blob is too large");
        return 0;
    }
    if (!safe_lerc_expected_size(width, height, depth, bands, sample_bytes,
                                 &expected_size) ||
        expected_size > out_len)
    {
        safe_set_message(errbuf, errbuf_len, "LERC output buffer is too small");
        return 0;
    }

    status = lerc_getBlobInfo(blob, (unsigned int)blob_len, info, NULL, 11, 0);
    if (status != 0)
    {
        safe_set_message(errbuf, errbuf_len, "lerc_getBlobInfo() failed");
        return 0;
    }

    blob_depth = (int)info[2];
    blob_masks = info[8] > INT_MAX ? 0 : (int)info[8];
    if ((int)info[1] != (int)data_type || (int)info[3] != width ||
        (int)info[4] != height || (int)info[5] != bands)
    {
        safe_set_message(errbuf, errbuf_len,
                         "LERC blob metadata does not match TIFF geometry");
        return 0;
    }

    want_alpha_mask = mask_mode == 1 && depth == samples_per_pixel &&
                      samples_per_pixel > 1 && blob_depth == depth - 1;
    want_float_mask = mask_mode == 2 && depth == 1 && bands == 1 &&
                      (sample_bytes == 4 || sample_bytes == 8) && blob_masks == 1;

    if (!want_alpha_mask && blob_depth != depth)
    {
        safe_set_message(errbuf, errbuf_len,
                         "LERC blob depth %d does not match expected depth %d",
                         blob_depth, depth);
        return 0;
    }

    pixel_count = (size_t)width * (size_t)height;
    if (want_alpha_mask || want_float_mask)
    {
        mask = safe_malloc_bytes(pixel_count, errbuf, errbuf_len);
        if (mask == NULL)
            return 0;
    }

    if (want_alpha_mask)
    {
        if (!safe_lerc_expected_size(width, height, depth - 1, bands,
                                     sample_bytes, &temp_size))
        {
            safe_set_message(errbuf, errbuf_len, "LERC alpha decode is too large");
            free(mask);
            return 0;
        }
        temp = safe_malloc_bytes(temp_size, errbuf, errbuf_len);
        if (temp == NULL)
        {
            free(mask);
            return 0;
        }
        status = lerc_decode(blob, (unsigned int)blob_len, 1, mask, depth - 1,
                             width, height, bands, data_type, temp);
    }
    else
    {
        status = lerc_decode(blob, (unsigned int)blob_len,
                             want_float_mask ? 1 : 0, mask, depth, width,
                             height, bands, data_type, out);
    }

    if (status != 0)
    {
        safe_set_message(errbuf, errbuf_len, "lerc_decode() failed");
        free(mask);
        free(temp);
        return 0;
    }

    if (want_alpha_mask)
    {
        size_t src_stride = (size_t)(samples_per_pixel - 1) * (size_t)sample_bytes;
        size_t dst_stride = (size_t)samples_per_pixel * (size_t)sample_bytes;
        memset(out, 0, out_len);
        for (i = 0; i < pixel_count; ++i)
        {
            memcpy(out + i * dst_stride, temp + i * src_stride, src_stride);
            out[i * dst_stride + (size_t)(samples_per_pixel - 1)] =
                mask[i] ? 255u : 0u;
        }
    }
    else if (want_float_mask)
    {
        if (sample_bytes == 4)
        {
            float *values = (float *)out;
            for (i = 0; i < pixel_count; ++i)
            {
                if (mask[i] == 0)
                    values[i] = NAN;
            }
        }
        else
        {
            double *values = (double *)out;
            for (i = 0; i < pixel_count; ++i)
            {
                if (mask[i] == 0)
                    values[i] = NAN;
            }
        }
    }

    free(mask);
    free(temp);
    return 1;
}

int safe_tiff_lerc_encode(const uint8_t *input, size_t input_len,
                          int codec_version, unsigned int data_type, int width,
                          int height, int depth, int bands, double max_z_error,
                          int mask_mode, int sample_bytes, int samples_per_pixel,
                          uint8_t **out_ptr, size_t *out_len, char *errbuf,
                          size_t errbuf_len)
{
    const void *raw_data = input;
    size_t raw_size = input_len;
    int raw_depth = depth;
    int n_masks = 0;
    unsigned char *mask = NULL;
    uint8_t *temp = NULL;
    unsigned int needed = 0;
    unsigned int written = 0;
    uint8_t *out;
    lerc_status status;
    size_t pixel_count;
    size_t i;

    if (input == NULL || out_ptr == NULL || out_len == NULL)
    {
        safe_set_message(errbuf, errbuf_len, "Invalid LERC encode arguments");
        return 0;
    }

    if (!safe_lerc_expected_size(width, height, depth, bands, sample_bytes,
                                 &raw_size) ||
        raw_size != input_len)
    {
        safe_set_message(errbuf, errbuf_len, "LERC input buffer size is invalid");
        return 0;
    }

    pixel_count = (size_t)width * (size_t)height;

    if (mask_mode == 1 && data_type == 1 && sample_bytes == 1 &&
        depth == samples_per_pixel && samples_per_pixel > 1)
    {
        int use_mask = 1;
        size_t src_stride = (size_t)samples_per_pixel;
        size_t dst_stride = (size_t)(samples_per_pixel - 1);

        mask = safe_malloc_bytes(pixel_count, errbuf, errbuf_len);
        if (mask == NULL)
            return 0;
        temp = safe_malloc_bytes(pixel_count * dst_stride, errbuf, errbuf_len);
        if (temp == NULL)
        {
            free(mask);
            return 0;
        }

        for (i = 0; i < pixel_count; ++i)
        {
            uint8_t alpha = input[i * src_stride + (size_t)(samples_per_pixel - 1)];
            if (alpha != 0 && alpha != 255)
            {
                use_mask = 0;
                break;
            }
            mask[i] = alpha != 0 ? 1u : 0u;
            memcpy(temp + i * dst_stride, input + i * src_stride, dst_stride);
        }

        if (use_mask)
        {
            raw_data = temp;
            raw_depth = samples_per_pixel - 1;
            n_masks = 1;
        }
        else
        {
            free(mask);
            free(temp);
            mask = NULL;
            temp = NULL;
        }
    }
    else if (mask_mode == 2 && depth == 1 && bands == 1 &&
             (sample_bytes == 4 || sample_bytes == 8))
    {
        int has_nan = 0;

        mask = safe_malloc_bytes(pixel_count, errbuf, errbuf_len);
        if (mask == NULL)
            return 0;
        temp = safe_malloc_bytes(input_len, errbuf, errbuf_len);
        if (temp == NULL)
        {
            free(mask);
            return 0;
        }
        memcpy(temp, input, input_len);

        if (sample_bytes == 4)
        {
            const float *src = (const float *)input;
            float *dst = (float *)temp;
            for (i = 0; i < pixel_count; ++i)
            {
                if (src[i] == src[i])
                {
                    mask[i] = 1;
                }
                else
                {
                    has_nan = 1;
                    mask[i] = 0;
                    dst[i] = 0.0f;
                }
            }
        }
        else
        {
            const double *src = (const double *)input;
            double *dst = (double *)temp;
            for (i = 0; i < pixel_count; ++i)
            {
                if (src[i] == src[i])
                {
                    mask[i] = 1;
                }
                else
                {
                    has_nan = 1;
                    mask[i] = 0;
                    dst[i] = 0.0;
                }
            }
        }

        if (has_nan)
        {
            raw_data = temp;
            n_masks = 1;
        }
        else
        {
            free(mask);
            free(temp);
            mask = NULL;
            temp = NULL;
        }
    }

    status = lerc_computeCompressedSizeForVersion(
        raw_data, codec_version, data_type, raw_depth, width, height, bands,
        n_masks, mask, max_z_error, &needed);
    if (status != 0 || needed == 0)
    {
        safe_set_message(errbuf, errbuf_len,
                         "lerc_computeCompressedSizeForVersion() failed");
        free(mask);
        free(temp);
        return 0;
    }

    out = safe_malloc_bytes(needed, errbuf, errbuf_len);
    if (out == NULL)
    {
        free(mask);
        free(temp);
        return 0;
    }

    status = lerc_encodeForVersion(raw_data, codec_version, data_type, raw_depth,
                                   width, height, bands, n_masks, mask,
                                   max_z_error, out, needed, &written);
    free(mask);
    free(temp);
    if (status != 0)
    {
        safe_set_message(errbuf, errbuf_len, "lerc_encodeForVersion() failed");
        free(out);
        return 0;
    }

    *out_ptr = out;
    *out_len = written;
    return 1;
}
