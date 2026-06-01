// Copyright 2026 The Libernet Team
// SPDX-License-Identifier: Apache-2.0

pub mod lexer;
pub mod parser;

pub mod ast {
    include!(concat!(env!("OUT_DIR"), "/starkom.ast.v1.rs"));
}
