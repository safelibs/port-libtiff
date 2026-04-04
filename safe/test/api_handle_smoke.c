/*
 * Smoke coverage for the phase-2 handle/query/open-options surface.
 */

#include "tif_config.h"

#include <assert.h>
#include <fcntl.h>
#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>

#ifdef HAVE_UNISTD_H
#include <unistd.h>
#endif

#include "tiffio.h"

#define ERROR_BUFFER_SIZE 256

typedef struct
{
    int fd;
    int close_called;
} ClientContext;

typedef struct
{
    char buffer[ERROR_BUFFER_SIZE];
    TIFF *seen_tif;
    char module[64];
} HandlerContext;

static void fail(const char *message)
{
    fprintf(stderr, "%s\n", message);
    exit(1);
}

static tmsize_t client_read(thandle_t handle, void *buf, tmsize_t size)
{
    ClientContext *ctx = (ClientContext *)handle;
    return (tmsize_t)read(ctx->fd, buf, (size_t)size);
}

static tmsize_t client_write(thandle_t handle, void *buf, tmsize_t size)
{
    ClientContext *ctx = (ClientContext *)handle;
    return (tmsize_t)write(ctx->fd, buf, (size_t)size);
}

static toff_t client_seek(thandle_t handle, toff_t off, int whence)
{
    ClientContext *ctx = (ClientContext *)handle;
    return (toff_t)lseek(ctx->fd, (off_t)off, whence);
}

static int client_close(thandle_t handle)
{
    ClientContext *ctx = (ClientContext *)handle;
    int rc = close(ctx->fd);
    ctx->fd = -1;
    ctx->close_called++;
    return rc;
}

static toff_t client_size(thandle_t handle)
{
    ClientContext *ctx = (ClientContext *)handle;
    struct stat st;
    if (fstat(ctx->fd, &st) != 0)
        return 0;
    return (toff_t)st.st_size;
}

static int client_map(thandle_t handle, void **base, toff_t *size)
{
    (void)handle;
    (void)base;
    (void)size;
    return 0;
}

static void client_unmap(thandle_t handle, void *base, toff_t size)
{
    (void)handle;
    (void)base;
    (void)size;
}

static int capture_handler(TIFF *tif, void *user_data, const char *module,
                           const char *fmt, va_list ap)
{
    HandlerContext *ctx = (HandlerContext *)user_data;
    vsnprintf(ctx->buffer, sizeof(ctx->buffer), fmt, ap);
    ctx->seen_tif = tif;
    snprintf(ctx->module, sizeof(ctx->module), "%s", module);
    return 1;
}

int main(void)
{
    uint32_t *calloc_buffer = (uint32_t *)_TIFFcalloc(4, sizeof(uint32_t));
    char path[] = "api_handle_smokeXXXXXX";
    int fd;
    ClientContext client = {0};
    HandlerContext error_ctx = {{0}, NULL, {0}};
    HandlerContext warn_ctx = {{0}, NULL, {0}};
    TIFFOpenOptions *opts;
    TIFF *tif;
    const char *version = TIFFGetVersion();
    const char *old_name;
    ClientContext replacement = {.fd = -1, .close_called = 0};

    if (calloc_buffer == NULL)
        fail("_TIFFcalloc returned NULL");
    for (int i = 0; i < 4; ++i)
    {
        if (calloc_buffer[i] != 0)
            fail("_TIFFcalloc did not zero-initialize");
    }
    _TIFFfree(calloc_buffer);

    if (version == NULL || strstr(version, "LIBTIFF, Version 4.5.1") == NULL)
        fail("TIFFGetVersion returned an unexpected string");

    fd = mkstemp(path);
    if (fd < 0)
        fail("mkstemp failed");

    client.fd = fd;

    opts = TIFFOpenOptionsAlloc();
    if (opts == NULL)
        fail("TIFFOpenOptionsAlloc failed");
    TIFFOpenOptionsSetMaxSingleMemAlloc(opts, 4096);
    TIFFOpenOptionsSetErrorHandlerExtR(opts, capture_handler, &error_ctx);
    TIFFOpenOptionsSetWarningHandlerExtR(opts, capture_handler, &warn_ctx);

    tif = TIFFClientOpenExt(path, "w8", (thandle_t)&client, client_read,
                            client_write, client_seek, client_close,
                            client_size, client_map, client_unmap, opts);
    TIFFOpenOptionsFree(opts);
    if (tif == NULL)
        fail("TIFFClientOpenExt failed");

    if (!TIFFIsBigTIFF(tif))
        fail("TIFFIsBigTIFF did not report a BigTIFF handle");
    if (TIFFIsTiled(tif))
        fail("new handle should not be tiled");
    if (TIFFIsUpSampled(tif))
        fail("new handle should not be upsampled");
    if (!TIFFIsMSB2LSB(tif))
        fail("new handle should default to MSB2LSB fill order");
#if HOST_BIGENDIAN
    if (!TIFFIsBigEndian(tif))
        fail("host-endian create path should be big-endian on big-endian hosts");
    if (TIFFIsByteSwapped(tif))
        fail("host-endian create path should not be byte-swapped");
#else
    if (TIFFIsBigEndian(tif))
        fail("host-endian create path should be little-endian on little-endian hosts");
    if (TIFFIsByteSwapped(tif))
        fail("host-endian create path should not be byte-swapped");
#endif

    if (TIFFGetReadProc(tif) != client_read)
        fail("TIFFGetReadProc mismatch");
    if (TIFFGetWriteProc(tif) != client_write)
        fail("TIFFGetWriteProc mismatch");
    if (TIFFGetSeekProc(tif) != client_seek)
        fail("TIFFGetSeekProc mismatch");
    if (TIFFGetCloseProc(tif) != client_close)
        fail("TIFFGetCloseProc mismatch");
    if (TIFFGetSizeProc(tif) != client_size)
        fail("TIFFGetSizeProc mismatch");
    if (TIFFGetMapFileProc(tif) != client_map)
        fail("TIFFGetMapFileProc mismatch");
    if (TIFFGetUnmapFileProc(tif) != client_unmap)
        fail("TIFFGetUnmapFileProc mismatch");

    if (TIFFClientdata(tif) != (thandle_t)&client)
        fail("TIFFClientdata mismatch");
    if (TIFFSetClientdata(tif, (thandle_t)&replacement) != (thandle_t)&client)
        fail("TIFFSetClientdata did not return the previous clientdata");
    if (TIFFClientdata(tif) != (thandle_t)&replacement)
        fail("TIFFSetClientdata did not update the clientdata");
    TIFFSetClientdata(tif, (thandle_t)&client);

    if (TIFFGetMode(tif) != O_RDWR)
        fail("TIFFGetMode should report O_RDWR for write handles");
    if (TIFFSetMode(tif, O_RDONLY) != O_RDWR)
        fail("TIFFSetMode did not return the previous mode");
    if (TIFFGetMode(tif) != O_RDONLY)
        fail("TIFFSetMode did not update the mode");
    TIFFSetMode(tif, O_RDWR);

    if (TIFFSetFileno(tif, 1234) != -1)
        fail("TIFFSetFileno did not return the previous fd");
    if (TIFFFileno(tif) != 1234)
        fail("TIFFSetFileno did not update the fd");
    TIFFSetFileno(tif, -1);

    old_name = TIFFSetFileName(tif, "override-name");
    if (old_name == NULL || strcmp(old_name, path) != 0)
        fail("TIFFSetFileName did not return the previous name");
    if (strcmp(TIFFFileName(tif), "override-name") != 0)
        fail("TIFFSetFileName did not update the name");
    TIFFSetFileName(tif, path);

    TIFFErrorExtR(tif, "handle_error", "%s", "captured error");
    if (strcmp(error_ctx.buffer, "captured error") != 0 ||
        error_ctx.seen_tif != tif ||
        strcmp(error_ctx.module, "handle_error") != 0)
    {
        fail("per-handle error handler capture failed");
    }

    TIFFWarningExtR(tif, "handle_warn", "%s", "captured warning");
    if (strcmp(warn_ctx.buffer, "captured warning") != 0 ||
        warn_ctx.seen_tif != tif ||
        strcmp(warn_ctx.module, "handle_warn") != 0)
    {
        fail("per-handle warning handler capture failed");
    }

    TIFFClose(tif);
    if (client.close_called != 1)
        fail("TIFFClose did not invoke the client close callback");

    unlink(path);
    return 0;
}
