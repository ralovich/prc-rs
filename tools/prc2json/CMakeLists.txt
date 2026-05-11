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
if (WIN32)
    set(LIB_NAME "prc_rs.lib")
endif (WIN32)
if (UNIX)
    set(LIB_NAME "libprc_rs.a")
endif (UNIX)

ExternalProject_Add(
    prc_rs
    SOURCE_DIR ${CMAKE_CURRENT_SOURCE_DIR}/${RUST_DIR}
    DOWNLOAD_COMMAND ""
    CONFIGURE_COMMAND ""
    BUILD_COMMAND "" cargo build "$<IF:$<CONFIG:Release>,--release,>"
    BUILD_BYPRODUCTS     ${CMAKE_CURRENT_SOURCE_DIR}/${RUST_DIR}/target/$<IF:$<CONFIG:Release>,release,debug>/${LIB_NAME}
                         ${CMAKE_CURRENT_SOURCE_DIR}/${RUST_DIR}/include/prc.h
    #BINARY_DIR "${PROJECT_SOURCE_DIR}/${RUST_DIR}"
    #INSTALL_COMMAND "Get-ChildItem -Force -Path ${CMAKE_CURRENT_SOURCE_DIR}/${RUST_DIR}/target/release" COMMAND "Get-ChildItem -Force -Path ${CMAKE_CURRENT_SOURCE_DIR}/${RUST_DIR}/target/debug"
    INSTALL_COMMAND ""
    LOG_BUILD ON
    LOG_OUTPUT_ON_FAILURE ON
    )


#set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -march=native -std=gnu++2a")
#set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -fcoroutines")
ADD_EXECUTABLE(prc2json
  main.cpp
  )
add_dependencies(prc2json prc_rs)

SET(ADDITIONAL_LIBS "")
if (WIN32)
  SET(ADDITIONAL_LIBS "ntdll;ws2_32;wininet;userenv")
endif ()
target_link_libraries(prc2json
  PRIVATE
  debug     ${CMAKE_CURRENT_SOURCE_DIR}/${RUST_DIR}/target/debug/${LIB_NAME}
  optimized ${CMAKE_CURRENT_SOURCE_DIR}/${RUST_DIR}/target/release/${LIB_NAME}
  ${ADDITIONAL_LIBS}
)
target_include_directories(prc2json PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/${RUST_DIR}/include)

add_test(NAME test_prc2json
    COMMAND prc2json ${CMAKE_CURRENT_SOURCE_DIR}/${RUST_DIR}/testdata/pmi_sample.stream-23.prc
    )
