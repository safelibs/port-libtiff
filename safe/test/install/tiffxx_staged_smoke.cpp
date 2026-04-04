#include "tiffio.hxx"

#include <cstdint>
#include <iostream>
#include <sstream>
#include <string>

namespace
{

[[noreturn]] void fail(const char *message)
{
    std::cerr << message << '\n';
    std::exit(1);
}

void expect(bool condition, const char *message)
{
    if (!condition)
        fail(message);
}

} // namespace

int main()
{
    std::ostringstream output(std::ios::out | std::ios::binary);
    TIFF *writer = TIFFStreamOpen("staged-writer", &output);
    uint8_t pixel = 0x5A;
    std::string encoded;

    expect(writer != nullptr, "TIFFStreamOpen(writer) failed");
    expect(TIFFSetField(writer, TIFFTAG_IMAGEWIDTH, static_cast<uint32_t>(1)) == 1,
           "failed to set ImageWidth");
    expect(TIFFSetField(writer, TIFFTAG_IMAGELENGTH, static_cast<uint32_t>(1)) == 1,
           "failed to set ImageLength");
    expect(TIFFSetField(writer, TIFFTAG_SAMPLESPERPIXEL, 1) == 1,
           "failed to set SamplesPerPixel");
    expect(TIFFSetField(writer, TIFFTAG_BITSPERSAMPLE, 8) == 1,
           "failed to set BitsPerSample");
    expect(TIFFSetField(writer, TIFFTAG_ROWSPERSTRIP, static_cast<uint32_t>(1)) == 1,
           "failed to set RowsPerStrip");
    expect(TIFFSetField(writer, TIFFTAG_PLANARCONFIG, PLANARCONFIG_CONTIG) == 1,
           "failed to set PlanarConfiguration");
    expect(TIFFSetField(writer, TIFFTAG_PHOTOMETRIC, PHOTOMETRIC_MINISBLACK) == 1,
           "failed to set Photometric");
    expect(TIFFSetField(writer, TIFFTAG_COMPRESSION, COMPRESSION_NONE) == 1,
           "failed to set Compression");
    expect(TIFFWriteScanline(writer, &pixel, 0, 0) == 1,
           "TIFFWriteScanline failed");
    TIFFClose(writer);

    encoded = output.str();
    expect(!encoded.empty(), "writer stream is empty");

    std::istringstream input(encoded, std::ios::in | std::ios::binary);
    TIFF *reader = TIFFStreamOpen("staged-reader", &input);
    uint32_t width = 0;
    uint32_t height = 0;
    uint8_t decoded = 0;

    expect(reader != nullptr, "TIFFStreamOpen(reader) failed");
    expect(TIFFGetField(reader, TIFFTAG_IMAGEWIDTH, &width) == 1 && width == 1,
           "unexpected ImageWidth");
    expect(TIFFGetField(reader, TIFFTAG_IMAGELENGTH, &height) == 1 && height == 1,
           "unexpected ImageLength");
    expect(TIFFReadScanline(reader, &decoded, 0, 0) == 1,
           "TIFFReadScanline failed");
    expect(decoded == pixel, "decoded pixel mismatch");
    TIFFClose(reader);

    return 0;
}
