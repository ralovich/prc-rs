# -*- mode: python; coding: utf-8-unix -*-
#
# SPDX-License-Identifier: MIT
#
# SPDX-FileCopyrightText: Copyright Kristóf Ralovich (C) 2025-2026.
# All rights reserved.

#
# https://github.com/search?q=gdb_script_file%20language%3ARust%20&type=code&p=1
# https://walnut356.github.io/posts/so-you-want-better-debug-info/
# https://github.com/rust-lang/rust/blob/43ca9d18e333797f0aa3b525501a7cec8d61a96b/src/etc/lldb_providers.py
# https://doc.rust-lang.org/reference/attributes/debugger.html#r-attributes.debugger.debugger_visualizer.gdb
#

#
# mkdir -p ~/.config/gdb/
# echo "add-auto-load-safe-path /PATH/TO/BINARY/prc-rs/target/debug/prc_describe" > ~/.config/gdb/gdbinit
#

import gdb
import gdb.printing

class MyClassPrinterBoolean:
    "Print a prc_rs::prc_builtin::Boolean"
    def __init__(self, val):
        self.val = val
        self.value = bool(val['value'])
    def to_string(self):
        return f"{str(self.value).lower()}"

class MyClassPrinterCharacter:
    "Print a prc_rs::prc_builtin::Character"
    def __init__(self, val):
        self.val = val
        self.value = val['value']
    def to_string(self):
        return f"{self.value}"

class MyClassPrinterDouble:
    "Print a prc_rs::prc_builtin::Double"
    def __init__(self, val):
        self.val = val
        self.value = float(val['value'])
    def to_string(self):
        return str(self.value)

class MyClassPrinterString:
    "Print a prc_rs::prc_builtin::String"
    def __init__(self, val):
        self.val = val
        self.value = val['value']
    def to_string(self):
        return str(self.value)

class MyClassPrinterUnsignedInteger:
    "Print a prc_rs::prc_builtin::UnsignedInteger"
    def __init__(self, val):
        self.val = val
        self.value = val['value']
    def to_string(self):
        return f"{self.value}"

class MyClassPrinterInteger:
    "Print a prc_rs::prc_builtin::Integer"
    def __init__(self, val):
        self.val = val
        self.value = val['value']
    def to_string(self):
        return f"{self.value}"

def lookup_function(val):
    if str(val.type) == "prc_rs::prc_builtin::Boolean":
        return MyClassPrinterBoolean(val)
    if str(val.type) == "prc_rs::prc_builtin::Character":
        return MyClassPrinterCharacter(val)
    if str(val.type) == "prc_rs::prc_builtin::Double":
        return MyClassPrinterDouble(val)
    if str(val.type) == "prc_rs::prc_builtin::String":
        return MyClassPrinterString(val)
    if str(val.type) == "prc_rs::prc_builtin::Integer":
        return MyClassPrinterInteger(val)
    if (str(val.type) == "prc_rs::prc_builtin::UnsignedInteger"
        or str(val.type) == "prc_rs::prc_builtin::UnsignedIntegerWithVariableBitNumber"
        or str(val.type) == "prc_rs::prc_builtin::NumberOfBitsThenUnsignedInteger"):
        return MyClassPrinterUnsignedInteger(val)
    return None

gdb.pretty_printers.append(lookup_function)
