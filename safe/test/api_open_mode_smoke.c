/*
 * Smoke coverage for tif_open.c-compatible mode handling.
 */

#include "tif_config.h"

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

#include "tiffiop.h"

typedef struct
{
    int fd;
    int close_called;
} ClientFile;

static int g_warning_count = 0;
static char g_warning_buffer[256];

static void fail(const char *message)
{
    fprintf(stderr, "%s\n", message);
    exit(1);
}

static void expect(bool condition, const char *message)
{
    if (!condition)
        fail(message);
}

static void write_full(int fd, const void *buffer, size_t size)
{
    const uint8_t *bytes = (const uint8_t *)buffer;
    size_t written = 0;
    while (written < size)
    {
        ssize_t rc = write(fd, bytes + written, size - written);
        if (rc <= 0)
            fail("write failed");
        written += (size_t)rc;
    }
}

static void read_exact_fd(int fd, void *buffer, size_t size)
{
    uint8_t *bytes = (uint8_t *)buffer;
    size_t total = 0;
    while (total < size)
    {
        ssize_t rc = read(fd, bytes + total, size - total);
        if (rc <= 0)
            fail("read failed");
        total += (size_t)rc;
    }
}

static void read_file_prefix(const char *path, void *buffer, size_t size)
{
    int fd = open(path, O_RDONLY);
    if (fd < 0)
        fail("open for read failed");
    read_exact_fd(fd, buffer, size);
    close(fd);
}

static void write_empty_classic_tiff(const char *path)
{
    static const unsigned char classic_le[] = {
        'I', 'I', 42, 0, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    };
    int fd = open(path, O_WRONLY | O_TRUNC);
    if (fd < 0)
        fail("open for write failed");
    write_full(fd, classic_le, sizeof(classic_le));
    close(fd);
}

static void write_empty_big_tiff(const char *path)
{
    static const unsigned char big_le[] = {
        'I', 'I', 43, 0, 8, 0, 0, 0, 16, 0, 0, 0, 0, 0, 0, 0,
        0,   0,   0,  0, 0, 0, 0, 0, 0,  0, 0, 0, 0, 0, 0, 0,
    };
    int fd = open(path, O_WRONLY | O_TRUNC);
    if (fd < 0)
        fail("open for BigTIFF write failed");
    write_full(fd, big_le, sizeof(big_le));
    close(fd);
}

static void capture_warning(const char *module, const char *fmt, va_list ap)
{
    (void)module;
    vsnprintf(g_warning_buffer, sizeof(g_warning_buffer), fmt, ap);
    g_warning_count++;
}

static tmsize_t client_read(thandle_t handle, void *buf, tmsize_t size)
{
    ClientFile *client = (ClientFile *)handle;
    return (tmsize_t)read(client->fd, buf, (size_t)size);
}

static tmsize_t client_write(thandle_t handle, void *buf, tmsize_t size)
{
    ClientFile *client = (ClientFile *)handle;
    return (tmsize_t)write(client->fd, buf, (size_t)size);
}

static toff_t client_seek(thandle_t handle, toff_t off, int whence)
{
    ClientFile *client = (ClientFile *)handle;
    return (toff_t)lseek(client->fd, (off_t)off, whence);
}

static int client_close(thandle_t handle)
{
    ClientFile *client = (ClientFile *)handle;
    int rc = close(client->fd);
    client->fd = -1;
    client->close_called++;
    return rc;
}

static toff_t client_size(thandle_t handle)
{
    ClientFile *client = (ClientFile *)handle;
    struct stat st;
    if (fstat(client->fd, &st) != 0)
        return 0;
    return (toff_t)st.st_size;
}

static TIFF *open_client_read(const char *path, const char *mode,
                              ClientFile *client)
{
    client->fd = open(path, O_RDONLY);
    client->close_called = 0;
    if (client->fd < 0)
        fail("client open failed");
    return TIFFClientOpen(path, mode, (thandle_t)client, client_read,
                          client_write, client_seek, client_close, client_size,
                          NULL, NULL);
}

static void expect_prefix(const char *path, const unsigned char *expected,
                          size_t expected_size, const char *label)
{
    unsigned char actual[16] = {0};
    read_file_prefix(path, actual, expected_size);
    if (memcmp(actual, expected, expected_size) != 0)
    {
        fprintf(stderr, "unexpected header for %s\n", label);
        exit(1);
    }
}

int main(void)
{
    static const unsigned char classic_be[] = {'M', 'M', 0, 42, 0, 0, 0, 0};
    static const unsigned char classic_le[] = {'I', 'I', 42, 0, 0, 0, 0, 0};
    static const unsigned char big_le[] = {'I', 'I', 43, 0, 8, 0, 0, 0,
                                           0,   0,   0,  0, 0, 0, 0, 0};
    char path[] = "api_open_mode_smokeXXXXXX";
    char read_path[] = "api_open_mode_readXXXXXX";
    char big_read_path[] = "api_open_mode_big_readXXXXXX";
    TIFF *tif;
    int fd;
    ClientFile client;
    TIFFErrorHandler previous_warning_handler;

    fd = mkstemp(path);
    if (fd < 0)
        fail("mkstemp failed");
    close(fd);

    fd = mkstemp(read_path);
    if (fd < 0)
        fail("mkstemp read path failed");
    close(fd);
    write_empty_classic_tiff(read_path);

    fd = mkstemp(big_read_path);
    if (fd < 0)
        fail("mkstemp big read path failed");
    close(fd);
    write_empty_big_tiff(big_read_path);

    tif = TIFFOpen(path, "wb");
    expect(tif != NULL, "TIFFOpen(wb) failed");
    expect(TIFFGetMode(tif) == O_RDWR, "wb should use O_RDWR");
    expect(TIFFIsBigEndian(tif), "wb should create a big-endian file");
#if HOST_BIGENDIAN
    expect(!TIFFIsByteSwapped(tif), "wb should not swab on big-endian hosts");
#else
    expect(TIFFIsByteSwapped(tif), "wb should swab on little-endian hosts");
#endif
    TIFFClose(tif);
    expect_prefix(path, classic_be, sizeof(classic_be), "wb");

    tif = TIFFOpen(path, "wl");
    expect(tif != NULL, "TIFFOpen(wl) failed");
    expect(!TIFFIsBigTIFF(tif), "wl should create classic TIFF");
    expect(!TIFFIsBigEndian(tif), "wl should create little-endian TIFF");
    TIFFClose(tif);
    expect_prefix(path, classic_le, sizeof(classic_le), "wl");

    tif = TIFFOpen(path, "w4");
    expect(tif != NULL, "TIFFOpen(w4) failed");
    expect(!TIFFIsBigTIFF(tif), "w4 should force classic TIFF");
    TIFFClose(tif);
    expect_prefix(path, classic_le, sizeof(classic_le), "w4");

    tif = TIFFOpen(path, "w8");
    expect(tif != NULL, "TIFFOpen(w8) failed");
    expect(TIFFIsBigTIFF(tif), "w8 should create BigTIFF");
    TIFFClose(tif);
    expect_prefix(path, big_le, sizeof(big_le), "w8");

    tif = TIFFOpen(path, "w8DO");
    expect(tif != NULL, "TIFFOpen(w8DO) failed");
    expect(TIFFIsBigTIFF(tif), "w8DO should create BigTIFF");
    expect((tif->tif_flags & TIFF_DEFERSTRILELOAD) != 0,
           "w8DO should enable deferred strile loading");
    expect((tif->tif_flags & TIFF_LAZYSTRILELOAD) == 0,
           "w8DO should not enable lazy strile loading on write handles");
    TIFFClose(tif);
    expect_prefix(path, big_le, sizeof(big_le), "w8DO");

    tif = TIFFOpen(path, "w+");
    expect(tif != NULL, "TIFFOpen(w+) failed");
    expect(TIFFGetMode(tif) == O_RDWR, "w+ should use O_RDWR");
    TIFFClose(tif);
    expect_prefix(path, classic_le, sizeof(classic_le), "w+");

    fd = open(path, O_RDWR | O_CREAT | O_TRUNC, 0666);
    if (fd < 0)
        fail("open for a+ failed");
    tif = TIFFFdOpen(fd, path, "a+");
    expect(tif != NULL, "TIFFFdOpen(a+) failed");
    expect(TIFFGetMode(tif) == O_RDWR, "a+ should use O_RDWR");
    TIFFClose(tif);
    expect_prefix(path, classic_le, sizeof(classic_le), "a+");

    tif = TIFFOpen(read_path, "rM");
    expect(tif != NULL, "TIFFOpen(rM) failed");
    expect((tif->tif_flags & TIFF_MAPPED) != 0, "rM should keep mmap enabled");
    expect(TIFFCurrentDirectory(tif) == 0, "rM should read the first directory");
    TIFFClose(tif);

    fd = open(read_path, O_RDONLY);
    if (fd < 0)
        fail("open rm failed");
    tif = TIFFFdOpen(fd, read_path, "rm");
    expect(tif != NULL, "TIFFFdOpen(rm) failed");
    expect((tif->tif_flags & TIFF_MAPPED) == 0, "rm should disable mmap");
    TIFFClose(tif);

    fd = open(read_path, O_RDONLY);
    if (fd < 0)
        fail("open rC failed");
    tif = TIFFFdOpen(fd, read_path, "rC");
    expect(tif != NULL, "TIFFFdOpen(rC) failed");
    expect((tif->tif_flags & TIFF_STRIPCHOP) != 0,
           "rC should enable strip chopping");
    TIFFClose(tif);

    tif = open_client_read(read_path, "rc", &client);
    expect(tif != NULL, "TIFFClientOpen(rc) failed");
    expect((tif->tif_flags & TIFF_STRIPCHOP) == 0,
           "rc should disable strip chopping");
    TIFFClose(tif);
    expect(client.close_called == 1, "client close should run for rc");

    fd = open(read_path, O_RDONLY);
    if (fd < 0)
        fail("open rD failed");
    tif = TIFFFdOpen(fd, read_path, "rD");
    expect(tif != NULL, "TIFFFdOpen(rD) failed");
    expect((tif->tif_flags & TIFF_DEFERSTRILELOAD) != 0,
           "rD should enable deferred strile loading");
    TIFFClose(tif);

    tif = open_client_read(read_path, "rO", &client);
    expect(tif != NULL, "TIFFClientOpen(rO) failed");
    expect((tif->tif_flags & TIFF_DEFERSTRILELOAD) != 0,
           "rO should imply deferred strile loading");
    expect((tif->tif_flags & TIFF_LAZYSTRILELOAD) != 0,
           "rO should enable lazy strile loading");
    TIFFClose(tif);

    tif = TIFFOpen(read_path, "rh");
    expect(tif != NULL, "TIFFOpen(rh) failed");
    expect((tif->tif_flags & TIFF_HEADERONLY) != 0,
           "rh should enable header-only mode");
    expect(TIFFCurrentDirectory(tif) == TIFF_NON_EXISTENT_DIR_NUMBER,
           "rh should skip the first directory");
    TIFFClose(tif);

    tif = TIFFOpen(big_read_path, "rh");
    expect(tif != NULL, "TIFFOpen(BigTIFF rh) failed");
    expect(TIFFIsBigTIFF(tif), "BigTIFF rh should preserve BigTIFF state");
    expect((tif->tif_flags & TIFF_HEADERONLY) != 0,
           "BigTIFF rh should enable header-only mode");
    TIFFClose(tif);

    g_warning_count = 0;
    g_warning_buffer[0] = '\0';
    previous_warning_handler = TIFFSetWarningHandler(capture_warning);
    tif = TIFFOpen(read_path, "rH");
    expect(tif != NULL, "TIFFOpen(rH) failed");
    expect(g_warning_count == 1, "rH should emit the host-order deprecation warning");
    expect(strstr(g_warning_buffer, "deprecated") != NULL,
           "rH warning text mismatch");
    expect(TIFFIsMSB2LSB(tif), "rH should alias to MSB2LSB");
    TIFFClose(tif);
    TIFFSetWarningHandler(previous_warning_handler);

    tif = open_client_read(read_path, "rL", &client);
    expect(tif != NULL, "TIFFClientOpen(rL) failed");
    expect(!TIFFIsMSB2LSB(tif), "rL should select LSB2MSB fill order");
    TIFFClose(tif);

    unlink(path);
    unlink(read_path);
    unlink(big_read_path);
    return 0;
}
