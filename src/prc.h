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

  //! Return textual representation of a parsed PRC. \c dst is caller allocated
  //! and needs to be large enough to hold the parsed representation.
  //!
  //! \param[in] src_len Number of bytes in PRC data.
  //! \param[in] src Pointer to PRC data.
  //! \param[in] dst_size Number of bytes allocated in \c dst.
  //! \param[out] dst Pointer to data being returned.
  //! \param[out] dst_actual_size Number of bytes used in \c dst.
  //! \returns 0 on success.
  extern int32_t prc_parse(const uint64_t src_len,
                           const char *const src,
                           const uint64_t dst_size,
                           char* dst,
                           uint64_t* dst_actual_size);

#ifdef __cplusplus
}
#endif
