extern "C" const char *TIFFGetVersion(void);

int tiff_safe_cxx_placeholder() { return TIFFGetVersion() != nullptr; }
