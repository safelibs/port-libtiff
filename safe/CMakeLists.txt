cmake_minimum_required(VERSION 3.20)

project(libtiff_safe VERSION 4.5.1 LANGUAGES C CXX)

include(GNUInstallDirs)
include(CMakePackageConfigHelpers)

set(_safe_default_tools OFF)
set(_safe_default_tests OFF)
if(DEFINED SAFE_BUILD_UPSTREAM_TOOLS)
  set(_safe_default_tools "${SAFE_BUILD_UPSTREAM_TOOLS}")
endif()
if(DEFINED SAFE_BUILD_UPSTREAM_TESTS)
  set(_safe_default_tests "${SAFE_BUILD_UPSTREAM_TESTS}")
endif()

set(tiff-tools "${_safe_default_tools}" CACHE BOOL "Build copied upstream tools")
set(tiff-tests "${_safe_default_tests}" CACHE BOOL "Build copied upstream tests")
set(SAFE_BUILD_UPSTREAM_TOOLS "${tiff-tools}" CACHE BOOL "Build copied upstream tools" FORCE)
set(SAFE_BUILD_UPSTREAM_TESTS "${tiff-tests}" CACHE BOOL "Build copied upstream tests" FORCE)

set(CMAKE_C_STANDARD 99)
set(CMAKE_C_STANDARD_REQUIRED ON)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_POSITION_INDEPENDENT_CODE ON)

enable_testing()

set(LIBTIFF_SONAME_VERSION 6)
set(LIBTIFF_SHARED_VERSION 6.0.1)
set(LIBTIFF_MAJOR_VERSION ${PROJECT_VERSION_MAJOR})
set(LIBTIFF_MINOR_VERSION ${PROJECT_VERSION_MINOR})
set(LIBTIFF_MICRO_VERSION ${PROJECT_VERSION_PATCH})
set(TIFF_CONFIGDIR "${CMAKE_INSTALL_LIBDIR}/cmake/tiff")

set(HAVE_GETOPT TRUE CACHE BOOL "Use the host getopt implementation" FORCE)
set(JPEG_SUPPORT ON)
set(OJPEG_SUPPORT ON)
set(OPENGL_SUPPORT OFF)
set(WEBP_SUPPORT ON)
set(EMSCRIPTEN OFF)

find_package(PkgConfig REQUIRED)
find_package(JPEG REQUIRED)
pkg_check_modules(LERC REQUIRED IMPORTED_TARGET Lerc)
pkg_check_modules(LIBLZMA REQUIRED IMPORTED_TARGET liblzma)
pkg_check_modules(WEBP REQUIRED IMPORTED_TARGET libwebp)
pkg_check_modules(ZSTD REQUIRED IMPORTED_TARGET libzstd)
find_path(JBIG_INCLUDE_DIR NAMES jbig.h REQUIRED)
find_library(JBIG_LIBRARY NAMES jbig REQUIRED)
add_library(JBIG::JBIG UNKNOWN IMPORTED)
set_target_properties(
  JBIG::JBIG
  PROPERTIES
    IMPORTED_LOCATION "${JBIG_LIBRARY}"
    INTERFACE_INCLUDE_DIRECTORIES "${JBIG_INCLUDE_DIR}")

find_program(CARGO_EXECUTABLE cargo REQUIRED)

if(CMAKE_CONFIGURATION_TYPES)
  set(SAFE_CARGO_PROFILE debug)
else()
  string(TOLOWER "${CMAKE_BUILD_TYPE}" SAFE_CARGO_PROFILE)
  if(NOT SAFE_CARGO_PROFILE OR SAFE_CARGO_PROFILE STREQUAL "debug")
    set(SAFE_CARGO_PROFILE debug)
  else()
    set(SAFE_CARGO_PROFILE release)
  endif()
endif()

set(SAFE_CARGO_TARGET_DIR "${CMAKE_BINARY_DIR}/cargo-target"
    CACHE PATH "Cargo target directory for the safe Rust core")

if(SAFE_CARGO_PROFILE STREQUAL "release")
  set(SAFE_CARGO_BUILD_ARGS --release)
else()
  set(SAFE_CARGO_BUILD_ARGS)
endif()

set(SAFE_RUST_STATICLIB
    "${SAFE_CARGO_TARGET_DIR}/${SAFE_CARGO_PROFILE}/libtiff_safe_core.a")

set(SAFE_RUST_SOURCES
    ${CMAKE_CURRENT_SOURCE_DIR}/src/lib.rs
    ${CMAKE_CURRENT_SOURCE_DIR}/src/rgba.rs
    ${CMAKE_CURRENT_SOURCE_DIR}/src/strile.rs
    ${CMAKE_CURRENT_SOURCE_DIR}/src/abi/mod.rs
    ${CMAKE_CURRENT_SOURCE_DIR}/src/core/mod.rs
    ${CMAKE_CURRENT_SOURCE_DIR}/src/core/color.rs
    ${CMAKE_CURRENT_SOURCE_DIR}/src/core/codec.rs
    ${CMAKE_CURRENT_SOURCE_DIR}/src/core/directory.rs
    ${CMAKE_CURRENT_SOURCE_DIR}/src/core/field_registry.rs
    ${CMAKE_CURRENT_SOURCE_DIR}/src/core/field_tables.rs
    ${CMAKE_CURRENT_SOURCE_DIR}/src/core/jpeg.rs)

add_custom_command(
  OUTPUT "${SAFE_RUST_STATICLIB}"
  COMMAND
    ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${SAFE_CARGO_TARGET_DIR}
    ${CARGO_EXECUTABLE} build --manifest-path
    ${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml ${SAFE_CARGO_BUILD_ARGS}
  WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
  DEPENDS
    ${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml
    ${SAFE_RUST_SOURCES}
  COMMENT "Building the safe Rust staticlib"
  VERBATIM)

add_custom_target(tiff_safe_core_build DEPENDS "${SAFE_RUST_STATICLIB}")

add_library(tiff_safe_core STATIC IMPORTED GLOBAL)
set_target_properties(
  tiff_safe_core
  PROPERTIES IMPORTED_LOCATION "${SAFE_RUST_STATICLIB}")
add_dependencies(tiff_safe_core tiff_safe_core_build)

add_library(safe_cmath INTERFACE)
find_library(SAFE_MATH_LIBRARY m)
if(SAFE_MATH_LIBRARY)
  target_link_libraries(safe_cmath INTERFACE "${SAFE_MATH_LIBRARY}")
endif()
add_library(CMath::CMath ALIAS safe_cmath)

add_subdirectory(port)

function(safe_mirror_shared_library_for_verifier target)
  if(CMAKE_SYSTEM_NAME STREQUAL "Linux")
    set(_safe_verifier_dir "${CMAKE_BINARY_DIR}/libtiff")
    add_custom_command(
      TARGET ${target}
      POST_BUILD
      COMMAND ${CMAKE_COMMAND} -E make_directory "${_safe_verifier_dir}"
      COMMAND ${CMAKE_COMMAND} -E copy_if_different
              "$<TARGET_FILE:${target}>"
              "${_safe_verifier_dir}/$<TARGET_FILE_NAME:${target}>"
      COMMAND ${CMAKE_COMMAND} -E copy_if_different
              "$<TARGET_SONAME_FILE:${target}>"
              "${_safe_verifier_dir}/$<TARGET_SONAME_FILE_NAME:${target}>"
      COMMAND ${CMAKE_COMMAND} -E copy_if_different
              "$<TARGET_LINKER_FILE:${target}>"
              "${_safe_verifier_dir}/$<TARGET_LINKER_FILE_NAME:${target}>"
      VERBATIM)
  endif()
endfunction()

add_library(
  tiff SHARED
  capi/tiff_placeholder.c
  capi/jpeg_helper.c
  capi/external_codec_helper.c)
add_library(TIFF::tiff ALIAS tiff)
target_include_directories(
  tiff
  PUBLIC
    $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
    $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/libtiff>
    $<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>)
target_link_libraries(
  tiff
  PRIVATE
    tiff_safe_core
    JPEG::JPEG
    JBIG::JBIG
    PkgConfig::LERC
    PkgConfig::LIBLZMA
    PkgConfig::WEBP
    PkgConfig::ZSTD
    CMath::CMath)
set_target_properties(
  tiff
  PROPERTIES
    OUTPUT_NAME tiff
    VERSION ${LIBTIFF_SHARED_VERSION}
    SOVERSION ${LIBTIFF_SONAME_VERSION})
if(CMAKE_SYSTEM_NAME STREQUAL "Linux")
  target_link_options(
    tiff
    PRIVATE
      "-Wl,--version-script=${CMAKE_CURRENT_SOURCE_DIR}/capi/libtiff-safe.map")
endif()
safe_mirror_shared_library_for_verifier(tiff)

add_library(tiffxx SHARED capi/tiffxx_placeholder.cxx)
add_library(TIFF::CXX ALIAS tiffxx)
target_include_directories(
  tiffxx
  PUBLIC
    $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
    $<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>
  PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/libtiff)
target_compile_options(tiffxx PRIVATE -fno-inline -fno-default-inline)
target_link_libraries(tiffxx PRIVATE tiff)
set_target_properties(
  tiffxx
  PROPERTIES
    OUTPUT_NAME tiffxx
    VERSION ${LIBTIFF_SHARED_VERSION}
    SOVERSION ${LIBTIFF_SONAME_VERSION})
if(CMAKE_SYSTEM_NAME STREQUAL "Linux")
  target_link_options(
    tiffxx
    PRIVATE
      "-Wl,--version-script=${CMAKE_CURRENT_SOURCE_DIR}/capi/libtiffxx-safe.map"
      "-Wl,--undefined=_ZNKSt4fposI11__mbstate_tEcvlEv"
      "-Wl,--undefined=_ZNKSt4fposI11__mbstate_tEmiERKS1_")
endif()
safe_mirror_shared_library_for_verifier(tiffxx)

set(prefix "${CMAKE_INSTALL_PREFIX}")
set(exec_prefix "\${prefix}")
if(IS_ABSOLUTE "${CMAKE_INSTALL_LIBDIR}")
  set(libdir "${CMAKE_INSTALL_LIBDIR}")
else()
  set(libdir "\${exec_prefix}/${CMAKE_INSTALL_LIBDIR}")
endif()
if(IS_ABSOLUTE "${CMAKE_INSTALL_INCLUDEDIR}")
  set(includedir "${CMAKE_INSTALL_INCLUDEDIR}")
else()
  set(includedir "\${prefix}/${CMAKE_INSTALL_INCLUDEDIR}")
endif()
set(tiff_libs_private " -ldeflate -ljbig -lLerc -lm")
set(tiff_requires_private " zlib libjpeg liblzma libzstd libwebp")

configure_file(
  ${CMAKE_CURRENT_SOURCE_DIR}/pkgconfig/libtiff-4.pc.in
  ${CMAKE_CURRENT_BINARY_DIR}/libtiff-4.pc
  @ONLY)
configure_file(
  ${CMAKE_CURRENT_SOURCE_DIR}/cmake/TiffConfig.cmake.in
  ${CMAKE_CURRENT_BINARY_DIR}/TiffConfig.cmake
  @ONLY)
write_basic_package_version_file(
  ${CMAKE_CURRENT_BINARY_DIR}/TiffConfigVersion.cmake
  VERSION ${PROJECT_VERSION}
  COMPATIBILITY SameMajorVersion)

install(
  TARGETS tiff
  EXPORT TiffTargets
  LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}
  ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR}
  RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR})
install(
  TARGETS tiffxx
  LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}
  ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR}
  RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR})
install(
  DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}/include/
  DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}
  FILES_MATCHING
    PATTERN "*.h"
    PATTERN "*.hxx")
install(
  EXPORT TiffTargets
  FILE TiffTargets.cmake
  NAMESPACE TIFF::
  DESTINATION ${TIFF_CONFIGDIR})
install(
  FILES
    ${CMAKE_CURRENT_BINARY_DIR}/TiffConfig.cmake
    ${CMAKE_CURRENT_BINARY_DIR}/TiffConfigVersion.cmake
  DESTINATION ${TIFF_CONFIGDIR})
install(
  FILES ${CMAKE_CURRENT_BINARY_DIR}/libtiff-4.pc
  DESTINATION ${CMAKE_INSTALL_LIBDIR}/pkgconfig)

if(tiff-tools)
  add_subdirectory(tools)
endif()

if(tiff-tests)
  add_subdirectory(test)
endif()
