// Copyright 2026 tiylabs / TiyGate Authors.
// SPDX-License-Identifier: Apache-2.0
//
// Extracted from tiygate (commit: 6db40e54663455fb99d675bd8985e4dc50f68a14) under Apache-2.0 License.

extern crate self as proxyweave_protocols;
extern crate self as tiygate_core;
extern crate self as tiygate_protocols;

pub mod ir;
pub mod protocol;
pub mod routing;

pub mod chat_completions;
pub mod error_body;
pub mod messages;
pub mod responses;

// Re-exports matching tiygate_core root
pub use ir::*;
pub use protocol::*;
pub use routing::*;
