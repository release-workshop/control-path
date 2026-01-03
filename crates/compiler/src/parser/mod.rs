/*!
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 *
 * Parser module for parsing flag definitions and deployments from YAML/JSON strings.
 * This module works only with in-memory data (no file I/O).
 */

pub mod definitions;
pub mod deployment;
pub mod error;
pub mod utils;

pub use definitions::parse_definitions;
pub use deployment::parse_deployment;
pub use error::ParseError as ParserError;
