#include "tiffio.h"

#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

static void fail(const char *message)
{
    fprintf(stderr, "%s\n", message);
    exit(1);
}

static void expect(int condition, const char *message)
{
    if (!condition)
        fail(message);
}

int main(void)
{
    const double ref_u = 0.21052631578947367;
    const double ref_v = 0.47368421052631576;
    const double y16 = LogL16toY(12345);
    const double y10 = LogL10toY(321);
    float xyz[3] = {0.25f, 0.50f, 0.75f};
    float xyz24[3] = {0.0f, 0.0f, 0.0f};
    float xyz32[3] = {0.0f, 0.0f, 0.0f};
    uint8_t rgb[3] = {0, 0, 0};
    double decoded_u = 0.0;
    double decoded_v = 0.0;
    int encoded_uv = 0;
    uint32_t luv24 = 0;
    uint32_t luv32 = 0;

    expect(isfinite(y16) && y16 > 0.0, "LogL16toY returned an invalid value");
    expect(isfinite(y10) && y10 > 0.0, "LogL10toY returned an invalid value");
    expect(LogL16fromY(y16, SGILOGENCODE_NODITHER) >= 0,
           "LogL16fromY failed");
    expect(LogL10fromY(y10, SGILOGENCODE_NODITHER) >= 0,
           "LogL10fromY failed");

    encoded_uv = uv_encode(ref_u, ref_v, SGILOGENCODE_NODITHER);
    expect(encoded_uv >= 0, "uv_encode failed");
    expect(uv_decode(&decoded_u, &decoded_v, encoded_uv) == 0,
           "uv_decode failed");
    expect(isfinite(decoded_u) && isfinite(decoded_v),
           "uv_decode produced non-finite values");

    luv24 = LogLuv24fromXYZ(xyz, SGILOGENCODE_NODITHER);
    luv32 = LogLuv32fromXYZ(xyz, SGILOGENCODE_NODITHER);
    LogLuv24toXYZ(luv24, xyz24);
    LogLuv32toXYZ(luv32, xyz32);
    XYZtoRGB24(xyz24, rgb);

    expect(isfinite(xyz24[0]) && isfinite(xyz24[1]) && isfinite(xyz24[2]),
           "LogLuv24toXYZ produced non-finite values");
    expect(isfinite(xyz32[0]) && isfinite(xyz32[1]) && isfinite(xyz32[2]),
           "LogLuv32toXYZ produced non-finite values");
    expect(rgb[0] <= 255 && rgb[1] <= 255 && rgb[2] <= 255,
           "XYZtoRGB24 produced an invalid RGB triple");

    return 0;
}
