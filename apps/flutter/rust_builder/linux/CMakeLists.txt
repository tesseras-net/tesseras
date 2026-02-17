# The Flutter tooling requires that developers have CMake 3.10 or later
# installed. You should not increase this version, as doing so will cause
# the plugin to fail to compile for some customers of the plugin.
cmake_minimum_required(VERSION 3.10)

# Project-level configuration.
set(PROJECT_NAME "tesseras_embedded")
project(${PROJECT_NAME} LANGUAGES CXX)

include("../cargokit/cmake/cargokit.cmake")
apply_cargokit(${PROJECT_NAME} ../../../../crates/tesseras-embedded tesseras_embedded "")

# Workaround: cargokit build tool may not copy the .so to the output dir.
# Explicitly copy from the cargo target dir if the expected output is missing.
if(CMAKE_BUILD_TYPE STREQUAL "Release")
  set(CARGOKIT_CARGO_PROFILE "release")
else()
  set(CARGOKIT_CARGO_PROFILE "debug")
endif()
set(CARGOKIT_BUILT_LIB "${CMAKE_CURRENT_BINARY_DIR}/cargokit_build/x86_64-unknown-linux-gnu/${CARGOKIT_CARGO_PROFILE}/libtesseras_embedded.so")
add_custom_command(
  TARGET "${PROJECT_NAME}_cargokit" POST_BUILD
  COMMAND ${CMAKE_COMMAND} -E copy_if_different
    "${CARGOKIT_BUILT_LIB}"
    "${${PROJECT_NAME}_cargokit_lib}"
  COMMENT "Ensuring libtesseras_embedded.so is in the expected location"
)

# List of absolute paths to libraries that should be bundled with the plugin.
# This list could contain prebuilt libraries, or libraries created by an
# external build triggered from this build file.
set(tesseras_embedded_bundled_libraries
  "${${PROJECT_NAME}_cargokit_lib}"
  PARENT_SCOPE
)
