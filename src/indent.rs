// -*- mode: rust; coding: utf-8-unix -*-

// SPDX-License-Identifier: MIT
//
// SPDX-FileCopyrightText: Copyright Kristóf Ralovich (C) 2025-2026.
// All rights reserved.

#![allow(unused)]

use std::cell::Cell;
use std::string::String;

thread_local! {
    pub static INDENT: std::cell::Cell<std::string::String> = const { std::cell::Cell::new(std::string::String::new()) };
}

pub fn indent() {
    //INDENT.with(|&mut indent| {
    //    let mut prev: *mut std::string::String = Default::default();
    //    prev = indent.get_mut()
    //});

    INDENT.with(|indent| {
        let mut prev: *mut std::string::String = Default::default();
        prev = indent.as_ptr();
        unsafe {
            (*prev).push(' ');
        }
    });
    //let a = INDENT.get().clone();
    //let b = a.deref_mut();
    //INDENT.with(|indent| {(*indent).get_mut().push_str(" ")});
    //INDENT.get_mut().push_str(" ");
}
pub fn dedent() {
    //INDENT.pop();
    INDENT.with(|indent| {
        let mut prev: *mut std::string::String = Default::default();
        prev = indent.as_ptr();
        unsafe {
            (*prev).pop();
        }
    });
}

pub fn get() -> std::string::String {
    let mut s: std::string::String = String::new();
    INDENT.with(|indent| {
        let mut prev: *mut std::string::String = Default::default();
        prev = indent.as_ptr();
        unsafe {
            s = (*prev).clone();
        }
    });
    s
}

pub struct IndentGuard {}
impl IndentGuard {
    pub fn new() -> IndentGuard {
        indent();
        Self {}
    }
}
impl Drop for IndentGuard {
    fn drop(&mut self) {
        dedent();
    }
}
