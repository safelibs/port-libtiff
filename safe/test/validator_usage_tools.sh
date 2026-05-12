#!/bin/sh
#
# Validator usage regressions for tiff2pdf, tiffcp JPEG output, and tiled output.
#
. ${srcdir:-.}/common.sh

pdf=o-validator-usage-tools.pdf
jpeg_pdf=o-validator-usage-tools-jpeg.pdf
tiled=o-validator-usage-tools-tiled.tiff
jpeg=o-validator-usage-tools-jpeg.tiff
info=o-validator-usage-tools-info.txt

rm -f "$pdf" "$jpeg_pdf" "$tiled" "$jpeg" "$info"

"${TIFF2PDF}" -o "$pdf" "${IMG_RGB_3C_8B}"
head=$(dd if="$pdf" bs=5 count=1 2>/dev/null)
if test "x$head" != "x%PDF-"; then
    echo "tiff2pdf -o did not produce a PDF header" >&2
    exit 1
fi

"${TIFFCP}" -t -w 16 -l 16 "${IMG_RGB_3C_8B}" "$tiled"
"${TIFFDUMP}" "$tiled" > "$info"
grep "TileWidth" "$info" >/dev/null || {
    echo "tiffcp tiled output is missing TileWidth" >&2
    exit 1
}
grep "TileLength" "$info" >/dev/null || {
    echo "tiffcp tiled output is missing TileLength" >&2
    exit 1
}
if grep "RowsPerStrip" "$info" >/dev/null; then
    echo "tiffcp tiled output unexpectedly stored RowsPerStrip" >&2
    exit 1
fi

"${TIFFCP}" -c jpeg -r 16 "${IMG_RGB_3C_8B}" "$jpeg"
"${TIFFDUMP}" "$jpeg" > "$info"
grep "Compression (259).*<7>" "$info" >/dev/null || {
    echo "tiffcp JPEG output is missing JPEG compression" >&2
    exit 1
}
grep "RowsPerStrip.*16" "$info" >/dev/null || {
    echo "tiffcp JPEG output did not preserve RowsPerStrip=16" >&2
    exit 1
}

"${TIFF2PDF}" -j -o "$jpeg_pdf" "$jpeg"
head=$(dd if="$jpeg_pdf" bs=5 count=1 2>/dev/null)
if test "x$head" != "x%PDF-"; then
    echo "tiff2pdf -j did not produce a PDF header" >&2
    exit 1
fi
