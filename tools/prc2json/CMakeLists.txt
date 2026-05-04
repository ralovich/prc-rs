cmake_minimum_required(VERSION 3.14)

PROJECT(prc2json C CXX)

# https://cmake.org/cmake/help/latest/prop_tgt/CXX_STANDARD.html
set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS ON)

set(CMAKE_VERBOSE_MAKEFILE ON)
set(BUILD_TESTING ON)
enable_testing()


include (ExternalProject)
set(RUST_DIR ../../../prc-rs)
ExternalProject_Add(
    prc_rs
    SOURCE_DIR ${CMAKE_CURRENT_SOURCE_DIR}/${RUST_DIR}
    DOWNLOAD_COMMAND ""
    CONFIGURE_COMMAND ""
    BUILD_COMMAND "" cargo build "$<IF:$<CONFIG:Release>,--release,>"
    BUILD_BYPRODUCTS     ${RUST_DIR}/target/$<IF:$<CONFIG:Release>,release,debug>/libprc_rs.a
                         ${RUST_DIR}/src/prc.h
    #BINARY_DIR "${PROJECT_SOURCE_DIR}/${RUST_DIR}"
    INSTALL_COMMAND ""
    LOG_BUILD ON
    )


#set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -march=native -std=gnu++2a")
#set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -fcoroutines")
ADD_EXECUTABLE(prc2json
  main.cpp
  )
add_dependencies(prc2json prc_rs)

target_link_libraries(prc2json
    PRIVATE debug "${PROJECT_SOURCE_DIR}/${RUST_DIR}/target/debug/libprc_rs.a"
    optimized "${PROJECT_SOURCE_DIR}/${RUST_DIR}/target/release/libprc_rs.a"
    )
target_include_directories(prc2json PRIVATE ../../src)

add_test(NAME test_prc2json
    COMMAND prc2json ${CMAKE_CURRENT_SOURCE_DIR}/../../../prc-rs/testdata/pmi_sample.stream-23.prc
    )
