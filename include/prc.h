// -*- mode: C; coding: utf-8-unix -*-

// SPDX-License-Identifier: MIT
//
// SPDX-FileCopyrightText: Copyright Kristóf Ralovich (C) 2025-2026.
// All rights reserved.

#pragma once

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define PRC_LIBPRC_VER_MAJOR 0
#define PRC_LIBPRC_VER_MINOR 2
#define PRC_LIBPRC_VER_PATCH 0

  //! Function pointer to caller defined memory allocator. Memory allocated
  //! by this function needs to be released by the caller with a matching
  //! deallocator. The returned allocation should be at least 16 byte aligned.
  typedef void* (*allocate_fn_t)(const size_t num_bytes);

  //! Return textual representation of a parsed PRC.
  //!
  //! \param[in] src_len Number of bytes in PRC data.
  //! \param[in] src Pointer to PRC data.
  //! \param[in] allocate function pointer to caller defined memory allocator.
  //! \param[out] dst Address of pointer to data being returned. Allocated by \c allocate.
  //! \param[out] dst_actual_size Number of bytes used in \c dst.
  //! \returns 0 on success.
  extern int32_t prc_parse_to_json(const uint64_t src_len,
                                   const unsigned char *const src,
                                   allocate_fn_t allocate,
                                   unsigned char** dst,
                                   uint64_t* dst_actual_size);


#ifdef __cplusplus
}
#endif
