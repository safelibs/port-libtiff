cmake_minimum_required(VERSION 3.20)

project(libtiff_safe VERSION 4.5.1 LANGUAGES C CXX)

include(GNUInstallDirs)

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

set(HAVE_GETOPT TRUE CACHE BOOL "Use the host getopt implementation" FORCE)
set(JPEG_SUPPORT OFF)
set(OJPEG_SUPPORT OFF)
set(OPENGL_SUPPORT OFF)
set(WEBP_SUPPORT OFF)
set(EMSCRIPTEN OFF)

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
    ${CMAKE_CURRENT_SOURCE_DIR}/src/strile.rs
    ${CMAKE_CURRENT_SOURCE_DIR}/src/abi/mod.rs
    ${CMAKE_CURRENT_SOURCE_DIR}/src/core/mod.rs
    ${CMAKE_CURRENT_SOURCE_DIR}/src/core/codec.rs
    ${CMAKE_CURRENT_SOURCE_DIR}/src/core/directory.rs
    ${CMAKE_CURRENT_SOURCE_DIR}/src/core/field_registry.rs
    ${CMAKE_CURRENT_SOURCE_DIR}/src/core/field_tables.rs)

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

add_library(tiff SHARED capi/tiff_placeholder.c)
target_include_directories(
  tiff
  PUBLIC
    $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
    $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/libtiff>
    $<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>)
target_link_libraries(tiff PRIVATE tiff_safe_core)
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
target_include_directories(
  tiffxx
  PUBLIC
    $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
    $<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>)
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
      "-Wl,--version-script=${CMAKE_CURRENT_SOURCE_DIR}/capi/libtiffxx-safe.map")
endif()
safe_mirror_shared_library_for_verifier(tiffxx)

install(
  TARGETS tiff tiffxx
  LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}
  ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR}
  RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR})
install(
  DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}/include/
  DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}
  FILES_MATCHING
    PATTERN "*.h"
    PATTERN "*.hxx")

if(tiff-tools)
  add_subdirectory(tools)
endif()

if(tiff-tests)
  add_subdirectory(test)
endif()
