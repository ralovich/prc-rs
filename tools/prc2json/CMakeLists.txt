cmake_minimum_required(VERSION 3.14)

PROJECT(prc2json C CXX)

# https://cmake.org/cmake/help/latest/prop_tgt/CXX_STANDARD.html
set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS ON)

set(CMAKE_VERBOSE_MAKEFILE OFF)
set(BUILD_TESTING ON)
enable_testing()


set(RUST_DIR ${CMAKE_CURRENT_SOURCE_DIR}/../../)
set(LIB_NAME "libprc.a")
if (WIN32)
    set(LIB_NAME "prc.lib")
endif (WIN32)

include (ExternalProject)
ExternalProject_Add(
    prc
    BUILD_ALWAYS ON
    PREFIX ${RUST_DIR}
    SOURCE_DIR ${RUST_DIR}
    DOWNLOAD_COMMAND ""
    CONFIGURE_COMMAND ""
    BUILD_COMMAND cargo build --lib --release COMMAND cargo build --lib
    BUILD_BYPRODUCTS     ${RUST_DIR}/target/$<IF:$<CONFIG:Release>,release,debug>/${LIB_NAME}
                         ${RUST_DIR}/include/prc.h
    #BINARY_DIR "${RUST_DIR}"
    #INSTALL_COMMAND "Get-ChildItem -Force -Path ${RUST_DIR}/target/release" COMMAND "Get-ChildItem -Force -Path ${RUST_DIR}/target/debug"
    INSTALL_COMMAND ""
    LOG_BUILD ON
    LOG_OUTPUT_ON_FAILURE ON
)


add_library(prcrs STATIC IMPORTED)
set_target_properties(
  prcrs
  PROPERTIES
  IMPORTED_CONFIGURATIONS "debug;release"
  IMPORTED_LOCATION_RELEASE ${RUST_DIR}/target/release/${LIB_NAME}
  IMPORTED_LOCATION_DEBUG   ${RUST_DIR}/target/debug/${LIB_NAME}
  INTERFACE_INCLUDE_DIRECTORIES "${RUST_DIR}/include"
)

ADD_EXECUTABLE(prc2json
  main.cpp
)
add_dependencies(prc2json prc)
target_link_libraries(prc2json
  PRIVATE
  prcrs
)

SET(ADDITIONAL_LIBS "")
if (WIN32)
  SET(ADDITIONAL_LIBS "ntdll;ws2_32;wininet;userenv")
endif ()
target_link_libraries(prc2json
  PRIVATE
  ${ADDITIONAL_LIBS}
)

add_test(NAME remove_json
    COMMAND ${CMAKE_COMMAND} -E rm -f pmi_sample.stream-23.json
)
add_test(NAME prc2json_runs
    COMMAND prc2json ${RUST_DIR}/testdata/pmi_sample.stream-23.prc pmi_sample.stream-23.json
)
set_tests_properties(prc2json_runs PROPERTIES DEPENDS "remove_json")
add_test(NAME json_as_expected
    COMMAND ${CMAKE_COMMAND} -E compare_files ${RUST_DIR}/testdata/pmi_sample.stream-23.json pmi_sample.stream-23.json
)
set_tests_properties(json_as_expected PROPERTIES DEPENDS "prc2json_runs")
