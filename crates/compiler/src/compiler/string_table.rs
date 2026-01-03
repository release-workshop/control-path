//! String table builder for AST compilation.
//!
//! Collects all strings used in the artifact and provides index-based access.
//! Processes expressions to convert string literals and property paths to string table indices.

use crate::ast::Expression;
use crate::compiler::expressions::IntermediateExpression;

/// String table builder for AST compilation.
/// Collects all strings used in the artifact and provides index-based access.
pub struct StringTable {
    strings: Vec<String>,
    index_map: std::collections::HashMap<String, u16>,
}

impl StringTable {
    /// Create a new empty string table.
    pub fn new() -> Self {
        Self {
            strings: Vec::new(),
            index_map: std::collections::HashMap::new(),
        }
    }

    /// Add a string to the table and return its index.
    /// If the string already exists, returns the existing index.
    ///
    /// # Errors
    ///
    /// Returns an error if the string table exceeds the maximum size (65536 strings).
    pub fn add(&mut self, str: &str) -> Result<u16, crate::error::CompilerError> {
        if let Some(&index) = self.index_map.get(str) {
            return Ok(index);
        }

        let index = self.strings.len();
        if index > u16::MAX as usize {
            return Err(crate::error::CompilerError::Compilation(
                crate::error::CompilationError::InvalidRule(
                    "String table exceeds maximum size (65536 strings)".to_string(),
                ),
            ));
        }
        let index = u16::try_from(index).map_err(|_| {
            crate::error::CompilerError::Compilation(crate::error::CompilationError::InvalidRule(
                "String table index exceeds u16::MAX".to_string(),
            ))
        })?;
        self.strings.push(str.to_string());
        self.index_map.insert(str.to_string(), index);
        Ok(index)
    }

    /// Get the string at the given index.
    #[must_use]
    pub fn get(&self, index: u16) -> Option<&str> {
        self.strings.get(index as usize).map(String::as_str)
    }

    /// Get all strings as a vector (for the artifact).
    #[must_use]
    pub fn to_vec(&self) -> Vec<String> {
        self.strings.clone()
    }

    /// Get the current size of the string table.
    #[must_use]
    pub fn size(&self) -> usize {
        self.strings.len()
    }

    /// Extract all strings from an expression and add them to the table.
    /// Returns a new expression with string references replaced by indices.
    ///
    /// # Errors
    ///
    /// Returns an error if the string table exceeds the maximum size.
    pub fn process_expression(
        &mut self,
        expr: &IntermediateExpression,
    ) -> Result<Expression, crate::error::CompilerError> {
        match expr {
            IntermediateExpression::BinaryOp {
                op_code,
                left,
                right,
            } => Ok(Expression::BinaryOp {
                op_code: *op_code,
                left: Box::new(self.process_expression(left)?),
                right: Box::new(self.process_expression(right)?),
            }),
            IntermediateExpression::LogicalOp {
                op_code,
                left,
                right,
            } => Ok(Expression::LogicalOp {
                op_code: *op_code,
                left: Box::new(self.process_expression(left)?),
                right: right
                    .as_ref()
                    .map(|r| self.process_expression(r).map(Box::new))
                    .transpose()?,
            }),
            IntermediateExpression::Property(prop_path) => {
                let prop_index = self.add(prop_path)?;
                Ok(Expression::Property { prop_index })
            }
            IntermediateExpression::Literal(value) => {
                // If value is a string, add to table and replace with index
                if let serde_json::Value::String(s) = value {
                    let str_index = self.add(s)?;
                    Ok(Expression::Literal {
                        value: serde_json::Value::Number(str_index.into()),
                    })
                } else {
                    // Numbers, booleans, null, and arrays stay as-is
                    // Arrays are kept as arrays intentionally - they are processed during evaluation
                    Ok(Expression::Literal {
                        value: value.clone(),
                    })
                }
            }
            IntermediateExpression::Func { func_code, args } => {
                let processed_args: Result<Vec<Expression>, _> = args
                    .iter()
                    .map(|arg| self.process_expression(arg))
                    .collect();
                Ok(Expression::Func {
                    func_code: *func_code,
                    args: processed_args?,
                })
            }
        }
    }
}

impl Default for StringTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::BinaryOp;
    use crate::compiler::expressions::IntermediateExpression;

    #[test]
    fn test_add_strings() {
        let mut table = StringTable::new();
        assert_eq!(table.add("ON").unwrap(), 0);
        assert_eq!(table.add("OFF").unwrap(), 1);
        assert_eq!(table.add("admin").unwrap(), 2);
    }

    #[test]
    fn test_deduplicate_strings() {
        let mut table = StringTable::new();
        let index1 = table.add("ON").unwrap();
        let index2 = table.add("ON").unwrap();
        assert_eq!(index1, index2);
        assert_eq!(table.size(), 1);
    }

    #[test]
    fn test_get_string() {
        let mut table = StringTable::new();
        let index = table.add("test").unwrap();
        assert_eq!(table.get(index), Some("test"));
    }

    #[test]
    fn test_to_vec() {
        let mut table = StringTable::new();
        table.add("ON").unwrap();
        table.add("OFF").unwrap();
        table.add("admin").unwrap();
        let vec = table.to_vec();
        assert_eq!(vec, vec!["ON", "OFF", "admin"]);
    }

    #[test]
    fn test_process_expression_property() {
        let mut table = StringTable::new();
        let expr = IntermediateExpression::Property("user.role".to_string());
        let processed = table.process_expression(&expr).unwrap();

        match processed {
            Expression::Property { prop_index } => {
                assert_eq!(prop_index, 0);
                assert_eq!(table.get(prop_index), Some("user.role"));
            }
            _ => panic!("Expected Property expression"),
        }
    }

    #[test]
    fn test_process_expression_string_literal() {
        let mut table = StringTable::new();
        let expr = IntermediateExpression::Literal(serde_json::Value::String("admin".to_string()));
        let processed = table.process_expression(&expr).unwrap();

        match processed {
            Expression::Literal { value } => {
                if let serde_json::Value::Number(n) = value {
                    assert_eq!(n.as_u64(), Some(0));
                    assert_eq!(table.get(0), Some("admin"));
                } else {
                    panic!("Expected Number (string table index)");
                }
            }
            _ => panic!("Expected Literal expression"),
        }
    }

    #[test]
    fn test_process_expression_binary_op() {
        let mut table = StringTable::new();
        let expr = IntermediateExpression::BinaryOp {
            op_code: BinaryOp::Eq as u8,
            left: Box::new(IntermediateExpression::Property("user.role".to_string())),
            right: Box::new(IntermediateExpression::Literal(serde_json::Value::String(
                "admin".to_string(),
            ))),
        };
        let processed = table.process_expression(&expr).unwrap();

        match processed {
            Expression::BinaryOp {
                op_code,
                left,
                right,
            } => {
                assert_eq!(op_code, BinaryOp::Eq as u8);
                match *left {
                    Expression::Property { prop_index } => {
                        assert_eq!(prop_index, 0);
                        assert_eq!(table.get(prop_index), Some("user.role"));
                    }
                    _ => panic!("Expected Property in left operand"),
                }
                match *right {
                    Expression::Literal { value } => {
                        if let serde_json::Value::Number(n) = value {
                            assert_eq!(n.as_u64(), Some(1));
                            assert_eq!(table.get(1), Some("admin"));
                        } else {
                            panic!("Expected Number (string table index) in right operand");
                        }
                    }
                    _ => panic!("Expected Literal in right operand"),
                }
            }
            _ => panic!("Expected BinaryOp expression"),
        }
    }

    #[test]
    fn test_process_expression_number_literal() {
        let mut table = StringTable::new();
        let expr = IntermediateExpression::Literal(serde_json::Value::Number(42.into()));
        let processed = table.process_expression(&expr).unwrap();

        match processed {
            Expression::Literal { value } => {
                if let serde_json::Value::Number(n) = value {
                    assert_eq!(n.as_i64(), Some(42));
                } else {
                    panic!("Expected Number literal");
                }
            }
            _ => panic!("Expected Literal expression"),
        }
    }

    #[test]
    fn test_process_expression_boolean_literal() {
        let mut table = StringTable::new();
        let expr = IntermediateExpression::Literal(serde_json::Value::Bool(true));
        let processed = table.process_expression(&expr).unwrap();

        match processed {
            Expression::Literal { value } => {
                if let serde_json::Value::Bool(b) = value {
                    assert!(b);
                } else {
                    panic!("Expected Boolean literal");
                }
            }
            _ => panic!("Expected Literal expression"),
        }
    }

    #[test]
    fn test_process_expression_array_literal() {
        let mut table = StringTable::new();
        let expr = IntermediateExpression::Literal(serde_json::Value::Array(vec![
            serde_json::Value::String("admin".to_string()),
            serde_json::Value::String("moderator".to_string()),
        ]));
        let processed = table.process_expression(&expr).unwrap();

        match processed {
            Expression::Literal { value } => {
                // Arrays remain as arrays (not converted to indices) - this matches TypeScript behavior.
                // Array elements are processed during expression evaluation, not during compilation.
                if let serde_json::Value::Array(arr) = value {
                    assert_eq!(arr.len(), 2);
                    // Array elements remain as strings in the expression AST
                    assert!(arr[0].is_string());
                    assert!(arr[1].is_string());
                } else {
                    panic!("Expected Array literal");
                }
            }
            _ => panic!("Expected Literal expression"),
        }
    }
}
