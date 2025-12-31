/*!
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 *
 * AST (Abstract Syntax Tree) data structures for compiled Control Path artifacts.
 * These structures match the format specified in specs/ast-format.md.
 *
 * The AST uses compact array-based formats optimized for:
 * - Small size (MessagePack encoded)
 * - Fast evaluation (direct array access)
 * - Minimal memory footprint
 */
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_bytes;

/// Top-level AST artifact structure.
/// This is the root structure that gets serialized to MessagePack.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Artifact {
    /// Format version (e.g., "1.0")
    #[serde(rename = "v")]
    pub version: String,
    /// Environment name
    #[serde(rename = "env")]
    pub environment: String,
    /// String table - all strings referenced by index (uint16)
    #[serde(rename = "strs")]
    pub string_table: Vec<String>,
    /// Array of flag rule arrays, indexed by flag definition order
    #[serde(rename = "flags")]
    pub flags: Vec<Vec<Rule>>,
    /// Flag names as string table indices (one per flag, same order as flags array)
    #[serde(rename = "flagNames")]
    pub flag_names: Vec<u16>,
    /// Optional segment definitions as [name_index, expression] tuples
    #[serde(rename = "segments", skip_serializing_if = "Option::is_none", default)]
    pub segments: Option<Vec<(u16, Expression)>>,
    /// Optional Ed25519 signature
    /// Serialized as MessagePack binary type (not array) for compatibility with TypeScript
    /// Note: Using custom serializer to ensure binary format matches TypeScript Uint8Array
    #[serde(rename = "sig", skip_serializing_if = "Option::is_none", default, serialize_with = "serialize_signature", deserialize_with = "deserialize_signature")]
    pub signature: Option<Vec<u8>>,
}

/// Rule type codes (first element of rule array)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum RuleType {
    /// Serve rule - payload is string | number (string table index)
    Serve = 0,
    /// Variations rule - payload is Variation[]
    Variations = 1,
    /// Rollout rule - payload is [value_index, pct] tuple
    Rollout = 2,
}

/// Rule structure: [type, when?, payload]
/// 
/// Types:
/// - serve: [0, undefined, string | number] or [0, Expression, string | number]
/// - variations: [1, undefined, Variation[]] or [1, Expression, Variation[]]
/// - rollout: [2, undefined, [string | number, number]] or [2, Expression, [string | number, number]]
/// 
/// Note: Rules are serialized as arrays to match TypeScript format.
/// We use serde_json::Value for the payload to handle flexible types during serialization.
#[derive(Debug, Clone, PartialEq)]
pub enum Rule {
    /// serve without when: [0, undefined, string | number]
    ServeWithoutWhen(ServePayload),
    /// serve with when: [0, Expression, string | number]
    ServeWithWhen(Expression, ServePayload),
    /// variations without when: [1, undefined, Variation[]]
    VariationsWithoutWhen(Vec<Variation>),
    /// variations with when: [1, Expression, Variation[]]
    VariationsWithWhen(Expression, Vec<Variation>),
    /// rollout without when: [2, undefined, [string | number, number]]
    RolloutWithoutWhen(RolloutPayload),
    /// rollout with when: [2, Expression, [string | number, number]]
    RolloutWithWhen(Expression, RolloutPayload),
}

impl Serialize for Rule {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(None)?;
        match self {
            Rule::ServeWithoutWhen(payload) => {
                seq.serialize_element(&0u8)?;
                seq.serialize_element::<Option<Expression>>(&None)?;
                match payload {
                    ServePayload::String(s) => seq.serialize_element(s)?,
                    ServePayload::Number(n) => seq.serialize_element(n)?,
                }
            }
            Rule::ServeWithWhen(when, payload) => {
                seq.serialize_element(&0u8)?;
                seq.serialize_element(when)?;
                match payload {
                    ServePayload::String(s) => seq.serialize_element(s)?,
                    ServePayload::Number(n) => seq.serialize_element(n)?,
                }
            }
            Rule::VariationsWithoutWhen(variations) => {
                seq.serialize_element(&1u8)?;
                seq.serialize_element::<Option<Expression>>(&None)?;
                seq.serialize_element(variations)?;
            }
            Rule::VariationsWithWhen(when, variations) => {
                seq.serialize_element(&1u8)?;
                seq.serialize_element(when)?;
                seq.serialize_element(variations)?;
            }
            Rule::RolloutWithoutWhen(payload) => {
                seq.serialize_element(&2u8)?;
                seq.serialize_element::<Option<Expression>>(&None)?;
                let rollout_array: [serde_json::Value; 2] = match &payload.value_index {
                    RolloutValue::String(s) => [serde_json::Value::String(s.clone()), serde_json::Value::Number(payload.percentage.into())],
                    RolloutValue::Number(n) => [serde_json::Value::Number((*n).into()), serde_json::Value::Number(payload.percentage.into())],
                };
                seq.serialize_element(&rollout_array)?;
            }
            Rule::RolloutWithWhen(when, payload) => {
                seq.serialize_element(&2u8)?;
                seq.serialize_element(when)?;
                let rollout_array: [serde_json::Value; 2] = match &payload.value_index {
                    RolloutValue::String(s) => [serde_json::Value::String(s.clone()), serde_json::Value::Number(payload.percentage.into())],
                    RolloutValue::Number(n) => [serde_json::Value::Number((*n).into()), serde_json::Value::Number(payload.percentage.into())],
                };
                seq.serialize_element(&rollout_array)?;
            }
        }
        seq.end()
    }
}

impl<'de> Deserialize<'de> for Rule {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, Visitor};
        use std::fmt;

        struct RuleVisitor;

        impl<'de> Visitor<'de> for RuleVisitor {
            type Value = Rule;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a rule array [type, when?, payload]")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let rule_type: u8 = seq.next_element()?.ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let when: Option<Expression> = seq.next_element()?.unwrap_or(None);
                let payload: serde_json::Value = seq.next_element()?.ok_or_else(|| de::Error::invalid_length(1, &self))?;

                match rule_type {
                    0 => {
                        let serve_payload = if payload.is_string() {
                            ServePayload::String(payload.as_str().unwrap().to_string())
                        } else if payload.is_u64() {
                            ServePayload::Number(payload.as_u64().unwrap() as u16)
                        } else {
                            return Err(de::Error::custom("Invalid serve payload type"));
                        };
                        if let Some(when_expr) = when {
                            Ok(Rule::ServeWithWhen(when_expr, serve_payload))
                        } else {
                            Ok(Rule::ServeWithoutWhen(serve_payload))
                        }
                    }
                    1 => {
                        let variations: Vec<Variation> = serde_json::from_value(payload).map_err(de::Error::custom)?;
                        if let Some(when_expr) = when {
                            Ok(Rule::VariationsWithWhen(when_expr, variations))
                        } else {
                            Ok(Rule::VariationsWithoutWhen(variations))
                        }
                    }
                    2 => {
                        let rollout_array: [serde_json::Value; 2] = serde_json::from_value(payload).map_err(de::Error::custom)?;
                        let value_index = if rollout_array[0].is_string() {
                            RolloutValue::String(rollout_array[0].as_str().unwrap().to_string())
                        } else if rollout_array[0].is_u64() {
                            RolloutValue::Number(rollout_array[0].as_u64().unwrap() as u16)
                        } else {
                            return Err(de::Error::custom("Invalid rollout value_index type"));
                        };
                        let percentage = rollout_array[1].as_u64().unwrap() as u8;
                        let rollout_payload = RolloutPayload { value_index, percentage };
                        if let Some(when_expr) = when {
                            Ok(Rule::RolloutWithWhen(when_expr, rollout_payload))
                        } else {
                            Ok(Rule::RolloutWithoutWhen(rollout_payload))
                        }
                    }
                    _ => Err(de::Error::custom(format!("Invalid rule type: {}", rule_type))),
                }
            }
        }

        deserializer.deserialize_seq(RuleVisitor)
    }
}

/// Serve rule payload: string | number (string table index)
#[derive(Debug, Clone, PartialEq)]
pub enum ServePayload {
    String(String),
    Number(u16), // string table index
}

/// Rollout rule payload: [value_index, pct]
#[derive(Debug, Clone, PartialEq)]
pub struct RolloutPayload {
    pub value_index: RolloutValue,
    pub percentage: u8,
}

/// Rollout value: string | number (string table index)
#[derive(Debug, Clone, PartialEq)]
pub enum RolloutValue {
    String(String),
    Number(u16), // string table index
}

/// Variation structure: [var_index, pct]
/// - var_index: string table index (uint16)
/// - pct: percentage (uint8, 0-100)
#[derive(Debug, Clone, PartialEq)]
pub struct Variation {
    pub var_index: u16,
    pub percentage: u8,
}

impl Serialize for Variation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeTuple;
        let mut tuple = serializer.serialize_tuple(2)?;
        tuple.serialize_element(&self.var_index)?;
        tuple.serialize_element(&self.percentage)?;
        tuple.end()
    }
}

impl<'de> Deserialize<'de> for Variation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, Visitor};
        use std::fmt;

        struct VariationVisitor;

        impl<'de> Visitor<'de> for VariationVisitor {
            type Value = Variation;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a variation tuple [var_index, percentage]")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let var_index: u16 = seq.next_element()?.ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let percentage: u8 = seq.next_element()?.ok_or_else(|| de::Error::invalid_length(1, &self))?;
                Ok(Variation { var_index, percentage })
            }
        }

        deserializer.deserialize_tuple(2, VariationVisitor)
    }
}

/// Expression node type codes (first element of expression array)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ExpressionType {
    /// Binary operator: [0, op_code, left, right]
    BinaryOp = 0,
    /// Logical operator: [1, op_code, left, right?] (NOT has no right)
    LogicalOp = 1,
    /// Property access: [2, prop_index]
    Property = 2,
    /// Literal value: [3, value]
    Literal = 3,
    /// Function call: [4, func_code, args[]]
    Func = 4,
}

/// Binary operator codes (second element of binary_op expression)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum BinaryOp {
    Eq = 0,  // ==
    Ne = 1,  // !=
    Gt = 2,  // >
    Lt = 3,  // <
    Gte = 4, // >=
    Lte = 5, // <=
}

/// Logical operator codes (second element of logical_op expression)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum LogicalOp {
    And = 6,
    Or = 7,
    Not = 8,
}

/// Function codes (second element of func expression)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum FuncCode {
    StartsWith = 0,
    EndsWith = 1,
    Contains = 2,
    In = 3,
    Matches = 4,
    Upper = 5,
    Lower = 6,
    Length = 7,
    Intersects = 8,
    SemverEq = 9,
    SemverGt = 10,
    SemverGte = 11,
    SemverLt = 12,
    SemverLte = 13,
    Hash = 14, // Maps from HASHED_PARTITION
    Coalesce = 15,
    IsBetween = 16,
    IsAfter = 17,
    IsBefore = 18,
    DayOfWeek = 19, // Maps from CURRENT_DAY_OF_WEEK_UTC
    HourOfDay = 20, // Maps from CURRENT_HOUR_UTC
    DayOfMonth = 21, // Maps from CURRENT_DAY_OF_MONTH_UTC
    Month = 22, // Maps from CURRENT_MONTH_UTC
    CurrentTimestamp = 23,
    InSegment = 24,
}

/// Expression structure: [type, ...operands]
/// 
/// Types:
/// - binary_op: [0, op_code, left_expr, right_expr]
/// - logical_op: [1, op_code, left_expr, right_expr?] (NOT has no right)
/// - property: [2, prop_index] (prop_index is string table index)
/// - literal: [3, value] (value can be string table index for strings)
/// - func: [4, func_code, [arg_expr, ...]]
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// binary_op: [0, op_code, left, right]
    BinaryOp {
        op_code: u8,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    /// logical_op: [1, op_code, left, right?] (NOT has no right)
    LogicalOp {
        op_code: u8,
        left: Box<Expression>,
        right: Option<Box<Expression>>,
    },
    /// property: [2, prop_index]
    Property {
        prop_index: u16,
    },
    /// literal: [3, value]
    Literal {
        value: serde_json::Value,
    },
    /// func: [4, func_code, args[]]
    Func {
        func_code: u8,
        args: Vec<Expression>,
    },
}

impl Serialize for Expression {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(None)?;
        match self {
            Expression::BinaryOp { op_code, left, right } => {
                seq.serialize_element(&0u8)?;
                seq.serialize_element(op_code)?;
                seq.serialize_element(left)?;
                seq.serialize_element(right)?;
            }
            Expression::LogicalOp { op_code, left, right } => {
                seq.serialize_element(&1u8)?;
                seq.serialize_element(op_code)?;
                seq.serialize_element(left)?;
                if let Some(right_expr) = right {
                    seq.serialize_element(right_expr)?;
                }
            }
            Expression::Property { prop_index } => {
                seq.serialize_element(&2u8)?;
                seq.serialize_element(prop_index)?;
            }
            Expression::Literal { value } => {
                seq.serialize_element(&3u8)?;
                seq.serialize_element(value)?;
            }
            Expression::Func { func_code, args } => {
                seq.serialize_element(&4u8)?;
                seq.serialize_element(func_code)?;
                seq.serialize_element(args)?;
            }
        }
        seq.end()
    }
}

impl<'de> Deserialize<'de> for Expression {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, Visitor};
        use std::fmt;

        struct ExpressionVisitor;

        impl<'de> Visitor<'de> for ExpressionVisitor {
            type Value = Expression;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("an expression array")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let expr_type: u8 = seq.next_element()?.ok_or_else(|| de::Error::invalid_length(0, &self))?;
                
                match expr_type {
                    0 => {
                        let op_code: u8 = seq.next_element()?.ok_or_else(|| de::Error::invalid_length(1, &self))?;
                        let left: Expression = seq.next_element()?.ok_or_else(|| de::Error::invalid_length(2, &self))?;
                        let right: Expression = seq.next_element()?.ok_or_else(|| de::Error::invalid_length(3, &self))?;
                        Ok(Expression::BinaryOp {
                            op_code,
                            left: Box::new(left),
                            right: Box::new(right),
                        })
                    }
                    1 => {
                        let op_code: u8 = seq.next_element()?.ok_or_else(|| de::Error::invalid_length(1, &self))?;
                        let left: Expression = seq.next_element()?.ok_or_else(|| de::Error::invalid_length(2, &self))?;
                        let right: Option<Expression> = seq.next_element()?;
                        Ok(Expression::LogicalOp {
                            op_code,
                            left: Box::new(left),
                            right: right.map(Box::new),
                        })
                    }
                    2 => {
                        let prop_index: u16 = seq.next_element()?.ok_or_else(|| de::Error::invalid_length(1, &self))?;
                        Ok(Expression::Property { prop_index })
                    }
                    3 => {
                        let value: serde_json::Value = seq.next_element()?.ok_or_else(|| de::Error::invalid_length(1, &self))?;
                        Ok(Expression::Literal { value })
                    }
                    4 => {
                        let func_code: u8 = seq.next_element()?.ok_or_else(|| de::Error::invalid_length(1, &self))?;
                        let args: Vec<Expression> = seq.next_element()?.ok_or_else(|| de::Error::invalid_length(2, &self))?;
                        Ok(Expression::Func { func_code, args })
                    }
                    _ => Err(de::Error::custom(format!("Invalid expression type: {}", expr_type))),
                }
            }
        }

        deserializer.deserialize_seq(ExpressionVisitor)
    }
}

/// Custom serializer for signature field to ensure it's serialized as MessagePack binary type
fn serialize_signature<S>(signature: &Option<Vec<u8>>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match signature {
        Some(bytes) => serde_bytes::serialize(bytes, serializer),
        None => serializer.serialize_none(),
    }
}

/// Custom deserializer for signature field to ensure it's deserialized as MessagePack binary type
fn deserialize_signature<'de, D>(deserializer: D) -> Result<Option<Vec<u8>>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Visitor;
    use std::fmt;

    struct SignatureVisitor;

    impl<'de> Visitor<'de> for SignatureVisitor {
        type Value = Option<Vec<u8>>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("an optional byte array")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: Deserializer<'de>,
        {
            let buf: serde_bytes::ByteBuf = serde_bytes::deserialize(deserializer)?;
            Ok(Some(buf.into_vec()))
        }
    }

    deserializer.deserialize_option(SignatureVisitor)
}

