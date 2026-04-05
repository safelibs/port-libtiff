/*
 * Minimal libtiffxx compatibility shim.
 *
 * This is the upstream TIFFStreamOpen implementation adapted to link against
 * the safe libtiff C surface.
 */

#include "tiffiop.h"

#include <iostream>

using namespace std;

struct tiffis_data;
struct tiffos_data;

extern "C"
{

static tmsize_t _tiffosReadProc(thandle_t, void *, tmsize_t);
static tmsize_t _tiffisReadProc(thandle_t fd, void *buf, tmsize_t size);
static tmsize_t _tiffosWriteProc(thandle_t fd, void *buf, tmsize_t size);
static tmsize_t _tiffisWriteProc(thandle_t, void *, tmsize_t);
static uint64_t _tiffosSeekProc(thandle_t fd, uint64_t off, int whence);
static uint64_t _tiffisSeekProc(thandle_t fd, uint64_t off, int whence);
static uint64_t _tiffosSizeProc(thandle_t fd);
static uint64_t _tiffisSizeProc(thandle_t fd);
static int _tiffosCloseProc(thandle_t fd);
static int _tiffisCloseProc(thandle_t fd);
static int _tiffDummyMapProc(thandle_t, void **base, toff_t *size);
static void _tiffDummyUnmapProc(thandle_t, void *base, toff_t size);
static TIFF *_tiffStreamOpen(const char *name, const char *mode, void *fd);

struct tiffis_data
{
    istream *stream;
    ios::pos_type start_pos;
};

struct tiffos_data
{
    ostream *stream;
    ios::pos_type start_pos;
};

static tmsize_t _tiffosReadProc(thandle_t, void *, tmsize_t) { return 0; }

static tmsize_t _tiffisReadProc(thandle_t fd, void *buf, tmsize_t size)
{
    tiffis_data *data = reinterpret_cast<tiffis_data *>(fd);

    streamsize request_size = size;
    if (static_cast<tmsize_t>(request_size) != size)
        return static_cast<tmsize_t>(-1);

    data->stream->read(static_cast<char *>(buf), request_size);

    return static_cast<tmsize_t>(data->stream->gcount());
}

static tmsize_t _tiffosWriteProc(thandle_t fd, void *buf, tmsize_t size)
{
    tiffos_data *data = reinterpret_cast<tiffos_data *>(fd);
    ostream *os = data->stream;
    ios::pos_type pos = os->tellp();

    streamsize request_size = size;
    if (static_cast<tmsize_t>(request_size) != size)
        return static_cast<tmsize_t>(-1);

    os->write(reinterpret_cast<const char *>(buf), request_size);

    return static_cast<tmsize_t>(os->tellp() - pos);
}

static tmsize_t _tiffisWriteProc(thandle_t, void *, tmsize_t) { return 0; }

static uint64_t _tiffosSeekProc(thandle_t fd, uint64_t off, int whence)
{
    tiffos_data *data = reinterpret_cast<tiffos_data *>(fd);
    ostream *os = data->stream;

    if (os->fail())
        return static_cast<uint64_t>(-1);

    switch (whence)
    {
        case SEEK_SET:
        {
            uint64_t new_offset = static_cast<uint64_t>(data->start_pos) + off;
            ios::off_type offset = static_cast<ios::off_type>(new_offset);
            if (static_cast<uint64_t>(offset) != new_offset)
                return static_cast<uint64_t>(-1);
            os->seekp(offset, ios::beg);
            break;
        }
        case SEEK_CUR:
        {
            ios::off_type offset = static_cast<ios::off_type>(off);
            if (static_cast<uint64_t>(offset) != off)
                return static_cast<uint64_t>(-1);
            os->seekp(offset, ios::cur);
            break;
        }
        case SEEK_END:
        {
            ios::off_type offset = static_cast<ios::off_type>(off);
            if (static_cast<uint64_t>(offset) != off)
                return static_cast<uint64_t>(-1);
            os->seekp(offset, ios::end);
            break;
        }
    }

    if (os->fail())
    {
        ios::iostate old_state;
        ios::pos_type origin;

        old_state = os->rdstate();
        os->clear(os->rdstate() & ~ios::failbit);
        switch (whence)
        {
            case SEEK_SET:
            default:
                origin = data->start_pos;
                break;
            case SEEK_CUR:
                origin = os->tellp();
                break;
            case SEEK_END:
                os->seekp(0, ios::end);
                origin = os->tellp();
                break;
        }
        os->clear(old_state);

        if ((static_cast<uint64_t>(origin) + off) >
            static_cast<uint64_t>(data->start_pos))
        {
            uint64_t num_fill;

            os->clear(os->rdstate() & ~ios::failbit);
            os->seekp(0, ios::end);
            num_fill = (static_cast<uint64_t>(origin)) + off - os->tellp();
            for (uint64_t i = 0; i < num_fill; i++)
                os->put('\0');

            os->seekp(
                static_cast<ios::off_type>(static_cast<uint64_t>(origin) + off),
                ios::beg);
        }
    }

    return static_cast<uint64_t>(os->tellp());
}

static uint64_t _tiffisSeekProc(thandle_t fd, uint64_t off, int whence)
{
    tiffis_data *data = reinterpret_cast<tiffis_data *>(fd);

    switch (whence)
    {
        case SEEK_SET:
        {
            uint64_t new_offset = static_cast<uint64_t>(data->start_pos) + off;
            ios::off_type offset = static_cast<ios::off_type>(new_offset);
            if (static_cast<uint64_t>(offset) != new_offset)
                return static_cast<uint64_t>(-1);
            data->stream->seekg(offset, ios::beg);
            break;
        }
        case SEEK_CUR:
        {
            ios::off_type offset = static_cast<ios::off_type>(off);
            if (static_cast<uint64_t>(offset) != off)
                return static_cast<uint64_t>(-1);
            data->stream->seekg(offset, ios::cur);
            break;
        }
        case SEEK_END:
        {
            ios::off_type offset = static_cast<ios::off_type>(off);
            if (static_cast<uint64_t>(offset) != off)
                return static_cast<uint64_t>(-1);
            data->stream->seekg(offset, ios::end);
            break;
        }
    }

    return static_cast<uint64_t>(data->stream->tellg() - data->start_pos);
}

static uint64_t _tiffosSizeProc(thandle_t fd)
{
    tiffos_data *data = reinterpret_cast<tiffos_data *>(fd);
    ostream *os = data->stream;
    ios::pos_type pos = os->tellp();
    ios::pos_type len;

    os->seekp(0, ios::end);
    len = os->tellp();
    os->seekp(pos);

    return static_cast<uint64_t>(len);
}

static uint64_t _tiffisSizeProc(thandle_t fd)
{
    tiffis_data *data = reinterpret_cast<tiffis_data *>(fd);
    ios::pos_type pos = data->stream->tellg();
    ios::pos_type len;

    data->stream->seekg(0, ios::end);
    len = data->stream->tellg();
    data->stream->seekg(pos);

    return static_cast<uint64_t>(len);
}

static int _tiffosCloseProc(thandle_t fd)
{
    delete reinterpret_cast<tiffos_data *>(fd);
    return 0;
}

static int _tiffisCloseProc(thandle_t fd)
{
    delete reinterpret_cast<tiffis_data *>(fd);
    return 0;
}

static int _tiffDummyMapProc(thandle_t, void **base, toff_t *size)
{
    (void)base;
    (void)size;
    return 0;
}

static void _tiffDummyUnmapProc(thandle_t, void *base, toff_t size)
{
    (void)base;
    (void)size;
}

static TIFF *_tiffStreamOpen(const char *name, const char *mode, void *fd)
{
    TIFF *tif;

    if (strchr(mode, 'w'))
    {
        tiffos_data *data = new tiffos_data;
        data->stream = reinterpret_cast<ostream *>(fd);
        data->start_pos = data->stream->tellp();

        tif = TIFFClientOpen(name, mode, reinterpret_cast<thandle_t>(data),
                             _tiffosReadProc, _tiffosWriteProc,
                             _tiffosSeekProc, _tiffosCloseProc,
                             _tiffosSizeProc, _tiffDummyMapProc,
                             _tiffDummyUnmapProc);
        if (!tif)
            delete data;
    }
    else
    {
        tiffis_data *data = new tiffis_data;
        data->stream = reinterpret_cast<istream *>(fd);
        data->start_pos = data->stream->tellg();

        tif = TIFFClientOpen(name, mode, reinterpret_cast<thandle_t>(data),
                             _tiffisReadProc, _tiffisWriteProc,
                             _tiffisSeekProc, _tiffisCloseProc,
                             _tiffisSizeProc, _tiffDummyMapProc,
                             _tiffDummyUnmapProc);
        if (!tif)
            delete data;
    }

    return tif;
}

} /* extern "C" */

TIFF *TIFFStreamOpen(const char *name, ostream *os)
{
    if (!os->fail() && static_cast<int>(os->tellp()) < 0)
    {
        *os << '\0';
        os->seekp(0);
    }

    return _tiffStreamOpen(name, "wm", os);
}

TIFF *TIFFStreamOpen(const char *name, istream *is)
{
    return _tiffStreamOpen(name, "rm", is);
}
