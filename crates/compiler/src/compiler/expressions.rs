//! Expression parser for Control Path expressions
//!
//! Parses expression strings into Expression AST nodes.
//! Supports all operators and functions from the expression language specification.

use crate::ast::{BinaryOp, FuncCode, LogicalOp};
use crate::error::{CompilationError, CompilerError};

/// Intermediate expression type used during parsing.
/// Properties and string literals use strings initially, which are then
/// converted to string table indices by the string table processor.
#[derive(Debug, Clone, PartialEq)]
pub enum IntermediateExpression {
    /// binary_op: [0, op_code, left, right]
    BinaryOp {
        op_code: u8,
        left: Box<IntermediateExpression>,
        right: Box<IntermediateExpression>,
    },
    /// logical_op: [1, op_code, left, right?] (NOT has no right)
    LogicalOp {
        op_code: u8,
        left: Box<IntermediateExpression>,
        right: Option<Box<IntermediateExpression>>,
    },
    /// property: [2, prop_path] (prop_path is string, will be converted to index)
    Property(String),
    /// literal: [3, value] (value may be string, will be converted to index if string)
    Literal(serde_json::Value),
    /// func: [4, func_code, args[]]
    Func {
        func_code: u8,
        args: Vec<IntermediateExpression>,
    },
}

/// Token types for the lexer
#[derive(Debug, Clone, PartialEq)]
enum TokenType {
    Identifier(String),
    String(String),
    Number(f64),
    Boolean(bool),
    Null,
    Operator(String),
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    Comma,
    Eof,
}

/// Token with position information
#[derive(Debug, Clone)]
struct Token {
    token_type: TokenType,
    position: usize,
}

/// Expression parser using recursive descent
pub struct ExpressionParser {
    tokens: Vec<Token>,
    current: usize,
}

impl ExpressionParser {
    /// Create a new expression parser
    pub fn new() -> Self {
        Self {
            tokens: Vec::new(),
            current: 0,
        }
    }

    /// Parse an expression string into an intermediate Expression.
    /// The result will be processed by StringTable to convert strings to indices.
    pub fn parse(&mut self, expr: &str) -> Result<IntermediateExpression, CompilerError> {
        self.tokens = Self::tokenize(expr)?;
        self.current = 0;
        let result = self.parse_logical_or()?;
        if !self.is_at_end() {
            let token = self.peek();
            return Err(CompilerError::Compilation(
                CompilationError::ExpressionParsing(format!(
                    "Unexpected token at position {}: {:?}",
                    token.position, token.token_type
                )),
            ));
        }
        Ok(result)
    }

    /// Tokenize the input string into tokens
    fn tokenize(expr: &str) -> Result<Vec<Token>, CompilerError> {
        let mut tokens = Vec::new();
        let mut i = 0;
        let chars: Vec<char> = expr.chars().collect();

        while i < chars.len() {
            let char = chars[i];

            // Skip whitespace
            if char.is_whitespace() {
                i += 1;
                continue;
            }

            // String literals (single or double quotes)
            if char == '\'' || char == '"' {
                let quote = char;
                i += 1; // Skip opening quote
                let start_pos = i;
                let mut value = String::new();
                while i < chars.len() && chars[i] != quote {
                    if chars[i] == '\\' && i + 1 < chars.len() {
                        i += 1;
                        value.push(chars[i]);
                    } else {
                        value.push(chars[i]);
                    }
                    i += 1;
                }
                if i >= chars.len() {
                    return Err(CompilerError::Compilation(
                        CompilationError::ExpressionParsing(format!(
                            "Unterminated string literal at position {}",
                            start_pos - 1
                        )),
                    ));
                }
                tokens.push(Token {
                    token_type: TokenType::String(value),
                    position: start_pos - 1,
                });
                i += 1; // Skip closing quote
                continue;
            }

            // Numbers
            if char.is_ascii_digit() {
                let start_pos = i;
                let mut value = String::new();
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    value.push(chars[i]);
                    i += 1;
                }
                // Parse as f64 first to handle both integers and floats, and avoid overflow issues
                let num = value.parse::<f64>().map_err(|e| {
                    CompilerError::Compilation(CompilationError::ExpressionParsing(format!(
                        "Invalid number at position {}: {}",
                        start_pos, e
                    )))
                })?;
                tokens.push(Token {
                    token_type: TokenType::Number(num),
                    position: start_pos,
                });
                continue;
            }

            // Two-character operators
            if i + 1 < chars.len() {
                let two_char = format!("{}{}", chars[i], chars[i + 1]);
                match two_char.as_str() {
                    "==" => {
                        tokens.push(Token {
                            token_type: TokenType::Operator("==".to_string()),
                            position: i,
                        });
                        i += 2;
                        continue;
                    }
                    "!=" => {
                        tokens.push(Token {
                            token_type: TokenType::Operator("!=".to_string()),
                            position: i,
                        });
                        i += 2;
                        continue;
                    }
                    ">=" => {
                        tokens.push(Token {
                            token_type: TokenType::Operator(">=".to_string()),
                            position: i,
                        });
                        i += 2;
                        continue;
                    }
                    "<=" => {
                        tokens.push(Token {
                            token_type: TokenType::Operator("<=".to_string()),
                            position: i,
                        });
                        i += 2;
                        continue;
                    }
                    _ => {}
                }
            }

            // Single-character operators
            match char {
                '>' => {
                    tokens.push(Token {
                        token_type: TokenType::Operator(">".to_string()),
                        position: i,
                    });
                    i += 1;
                    continue;
                }
                '<' => {
                    tokens.push(Token {
                        token_type: TokenType::Operator("<".to_string()),
                        position: i,
                    });
                    i += 1;
                    continue;
                }
                '(' => {
                    tokens.push(Token {
                        token_type: TokenType::LeftParen,
                        position: i,
                    });
                    i += 1;
                    continue;
                }
                ')' => {
                    tokens.push(Token {
                        token_type: TokenType::RightParen,
                        position: i,
                    });
                    i += 1;
                    continue;
                }
                '[' => {
                    tokens.push(Token {
                        token_type: TokenType::LeftBracket,
                        position: i,
                    });
                    i += 1;
                    continue;
                }
                ']' => {
                    tokens.push(Token {
                        token_type: TokenType::RightBracket,
                        position: i,
                    });
                    i += 1;
                    continue;
                }
                ',' => {
                    tokens.push(Token {
                        token_type: TokenType::Comma,
                        position: i,
                    });
                    i += 1;
                    continue;
                }
                _ => {}
            }

            // Identifiers and keywords
            if char.is_ascii_alphabetic() || char == '_' {
                let start_pos = i;
                let mut value = String::new();
                while i < chars.len()
                    && (chars[i].is_ascii_alphanumeric() || chars[i] == '_' || chars[i] == '.')
                {
                    value.push(chars[i]);
                    i += 1;
                }

                // Check for boolean literals and null
                match value.as_str() {
                    "true" => {
                        tokens.push(Token {
                            token_type: TokenType::Boolean(true),
                            position: start_pos,
                        });
                    }
                    "false" => {
                        tokens.push(Token {
                            token_type: TokenType::Boolean(false),
                            position: start_pos,
                        });
                    }
                    "null" => {
                        tokens.push(Token {
                            token_type: TokenType::Null,
                            position: start_pos,
                        });
                    }
                    _ => {
                        tokens.push(Token {
                            token_type: TokenType::Identifier(value),
                            position: start_pos,
                        });
                    }
                }
                continue;
            }

            return Err(CompilerError::Compilation(
                CompilationError::ExpressionParsing(format!(
                    "Unexpected character at position {}: {}",
                    i, char
                )),
            ));
        }

        tokens.push(Token {
            token_type: TokenType::Eof,
            position: i,
        });
        Ok(tokens)
    }

    /// Check if we're at the end of tokens
    fn is_at_end(&self) -> bool {
        self.current >= self.tokens.len() || matches!(self.peek().token_type, TokenType::Eof)
    }

    /// Peek at the current token without advancing
    fn peek(&self) -> &Token {
        if self.current < self.tokens.len() {
            &self.tokens[self.current]
        } else {
            &self.tokens[self.tokens.len() - 1]
        }
    }

    /// Advance to the next token
    fn advance(&mut self) -> &Token {
        if self.current < self.tokens.len() {
            let token = &self.tokens[self.current];
            self.current += 1;
            token
        } else {
            &self.tokens[self.tokens.len() - 1]
        }
    }

    /// Parse logical OR (lowest precedence)
    fn parse_logical_or(&mut self) -> Result<IntermediateExpression, CompilerError> {
        let mut left = self.parse_logical_and()?;

        while self.check_identifier("OR") {
            self.advance(); // consume OR
            let right = self.parse_logical_and()?;
            left = IntermediateExpression::LogicalOp {
                op_code: LogicalOp::Or as u8,
                left: Box::new(left),
                right: Some(Box::new(right)),
            };
        }

        Ok(left)
    }

    /// Parse logical AND
    fn parse_logical_and(&mut self) -> Result<IntermediateExpression, CompilerError> {
        let mut left = self.parse_logical_not()?;

        while self.check_identifier("AND") {
            self.advance(); // consume AND
            let right = self.parse_logical_not()?;
            left = IntermediateExpression::LogicalOp {
                op_code: LogicalOp::And as u8,
                left: Box::new(left),
                right: Some(Box::new(right)),
            };
        }

        Ok(left)
    }

    /// Parse logical NOT
    fn parse_logical_not(&mut self) -> Result<IntermediateExpression, CompilerError> {
        if self.check_identifier("NOT") {
            self.advance(); // consume NOT
            let operand = self.parse_logical_not()?;
            Ok(IntermediateExpression::LogicalOp {
                op_code: LogicalOp::Not as u8,
                left: Box::new(operand),
                right: None,
            })
        } else {
            self.parse_comparison()
        }
    }

    /// Parse comparison operators and IN operator
    fn parse_comparison(&mut self) -> Result<IntermediateExpression, CompilerError> {
        let left = self.parse_primary()?;

        // Check for IN operator (infix syntax: value IN array)
        if self.check_identifier("IN") {
            self.advance(); // consume IN
            let right = self.parse_primary()?;
            // Transform IN operator to function call: IN(value, array)
            return Ok(IntermediateExpression::Func {
                func_code: FuncCode::In as u8,
                args: vec![left, right],
            });
        }

        // Check for comparison operators
        if let TokenType::Operator(ref op) = self.peek().token_type {
            let op_code = match op.as_str() {
                "==" => Some(BinaryOp::Eq as u8),
                "!=" => Some(BinaryOp::Ne as u8),
                ">" => Some(BinaryOp::Gt as u8),
                "<" => Some(BinaryOp::Lt as u8),
                ">=" => Some(BinaryOp::Gte as u8),
                "<=" => Some(BinaryOp::Lte as u8),
                _ => None,
            };

            if let Some(code) = op_code {
                self.advance(); // consume operator
                let right = self.parse_primary()?;
                return Ok(IntermediateExpression::BinaryOp {
                    op_code: code,
                    left: Box::new(left),
                    right: Box::new(right),
                });
            }
        }

        Ok(left)
    }

    /// Parse primary expressions (literals, properties, functions, parentheses, arrays)
    fn parse_primary(&mut self) -> Result<IntermediateExpression, CompilerError> {
        let position = self.peek().position;

        match &self.peek().token_type {
            TokenType::Boolean(b) => {
                let value = *b;
                self.advance(); // consume token
                Ok(IntermediateExpression::Literal(serde_json::Value::Bool(
                    value,
                )))
            }
            TokenType::String(s) => {
                let value = s.clone();
                self.advance(); // consume token
                Ok(IntermediateExpression::Literal(serde_json::Value::String(
                    value,
                )))
            }
            TokenType::Number(n) => {
                let num = *n;
                self.advance(); // consume token
                                // Convert to integer if it's a whole number and fits in i64, otherwise keep as float
                if num.fract() == 0.0 && num >= (i64::MIN as f64) && num <= (i64::MAX as f64) {
                    Ok(IntermediateExpression::Literal(serde_json::Value::Number(
                        serde_json::Number::from(num as i64),
                    )))
                } else {
                    Ok(IntermediateExpression::Literal(serde_json::Value::Number(
                        serde_json::Number::from_f64(num).ok_or_else(|| {
                            CompilerError::Compilation(CompilationError::ExpressionParsing(
                                format!("Invalid number at position {}", position),
                            ))
                        })?,
                    )))
                }
            }
            TokenType::Null => {
                self.advance(); // consume token
                Ok(IntermediateExpression::Literal(serde_json::Value::Null))
            }
            TokenType::Identifier(ident) => {
                let value = ident.clone();
                self.advance(); // consume identifier
                                // Check if it's a function call
                if self.check_token_type(&TokenType::LeftParen) {
                    self.parse_function_call(value)
                } else {
                    // Property access (e.g., user.role, context.environment, or role, environment)
                    // Note: user. and context. prefixes are accepted for backward compatibility
                    // but will be normalized (stripped) during string table processing
                    Ok(IntermediateExpression::Property(value))
                }
            }
            TokenType::LeftParen => {
                self.advance(); // consume '('
                let expr = self.parse_logical_or()?;
                if !self.check_token_type(&TokenType::RightParen) {
                    return Err(CompilerError::Compilation(
                        CompilationError::ExpressionParsing(format!(
                            "Expected ')' at position {}",
                            self.peek().position
                        )),
                    ));
                }
                self.advance(); // consume ')'
                Ok(expr)
            }
            TokenType::LeftBracket => {
                self.advance(); // consume '['
                                // Array literal
                let mut elements = Vec::new();
                if !self.check_token_type(&TokenType::RightBracket) {
                    loop {
                        // Parse array element as a primary expression (literal)
                        let elem = self.parse_primary()?;
                        match elem {
                            IntermediateExpression::Literal(v) => elements.push(v),
                            other => {
                                return Err(CompilerError::Compilation(
                                    CompilationError::ExpressionParsing(format!(
                                        "Array elements must be literals, found: {:?}",
                                        other
                                    )),
                                ));
                            }
                        }
                        if self.check_token_type(&TokenType::RightBracket) {
                            break;
                        }
                        if !self.check_token_type(&TokenType::Comma) {
                            return Err(CompilerError::Compilation(
                                CompilationError::ExpressionParsing(format!(
                                    "Expected ',' or ']' at position {}",
                                    self.peek().position
                                )),
                            ));
                        }
                        self.advance(); // consume ','
                    }
                }
                self.advance(); // consume ']'
                Ok(IntermediateExpression::Literal(serde_json::Value::Array(
                    elements,
                )))
            }
            _ => {
                let position = self.peek().position;
                let token_type = format!("{:?}", self.peek().token_type);
                Err(CompilerError::Compilation(
                    CompilationError::ExpressionParsing(format!(
                        "Unexpected token at position {}: {}",
                        position, token_type
                    )),
                ))
            }
        }
    }

    /// Parse a function call
    fn parse_function_call(
        &mut self,
        func_name: String,
    ) -> Result<IntermediateExpression, CompilerError> {
        // Map function name to function code
        let func_code = Self::get_function_code(&func_name)?;

        // Parse arguments
        self.advance(); // consume '('
        let mut args = Vec::new();

        if !self.check_token_type(&TokenType::RightParen) {
            loop {
                args.push(self.parse_logical_or()?);
                if self.check_token_type(&TokenType::RightParen) {
                    break;
                }
                if !self.check_token_type(&TokenType::Comma) {
                    return Err(CompilerError::Compilation(
                        CompilationError::ExpressionParsing(format!(
                            "Expected ',' or ')' at position {}",
                            self.peek().position
                        )),
                    ));
                }
                self.advance(); // consume ','
            }
        }

        self.advance(); // consume ')'

        Ok(IntermediateExpression::Func { func_code, args })
    }

    /// Get function code from function name
    fn get_function_code(func_name: &str) -> Result<u8, CompilerError> {
        match func_name {
            "STARTS_WITH" => Ok(FuncCode::StartsWith as u8),
            "ENDS_WITH" => Ok(FuncCode::EndsWith as u8),
            "CONTAINS" => Ok(FuncCode::Contains as u8),
            "IN" => Ok(FuncCode::In as u8),
            "MATCHES" => Ok(FuncCode::Matches as u8),
            "UPPER" => Ok(FuncCode::Upper as u8),
            "LOWER" => Ok(FuncCode::Lower as u8),
            "LENGTH" => Ok(FuncCode::Length as u8),
            "INTERSECTS" => Ok(FuncCode::Intersects as u8),
            "SEMVER_EQ" => Ok(FuncCode::SemverEq as u8),
            "SEMVER_GT" => Ok(FuncCode::SemverGt as u8),
            "SEMVER_GTE" => Ok(FuncCode::SemverGte as u8),
            "SEMVER_LT" => Ok(FuncCode::SemverLt as u8),
            "SEMVER_LTE" => Ok(FuncCode::SemverLte as u8),
            "HASHED_PARTITION" => Ok(FuncCode::Hash as u8),
            "COALESCE" => Ok(FuncCode::Coalesce as u8),
            "IS_BETWEEN" => Ok(FuncCode::IsBetween as u8),
            "IS_AFTER" => Ok(FuncCode::IsAfter as u8),
            "IS_BEFORE" => Ok(FuncCode::IsBefore as u8),
            "CURRENT_DAY_OF_WEEK_UTC" => Ok(FuncCode::DayOfWeek as u8),
            "CURRENT_HOUR_UTC" => Ok(FuncCode::HourOfDay as u8),
            "CURRENT_DAY_OF_MONTH_UTC" => Ok(FuncCode::DayOfMonth as u8),
            "CURRENT_MONTH_UTC" => Ok(FuncCode::Month as u8),
            "CURRENT_TIMESTAMP" => Ok(FuncCode::CurrentTimestamp as u8),
            "IN_SEGMENT" => Ok(FuncCode::InSegment as u8),
            _ => Err(CompilerError::Compilation(
                CompilationError::ExpressionParsing(format!("Unknown function: {}", func_name)),
            )),
        }
    }

    /// Check if current token is an identifier with the given value
    fn check_identifier(&self, value: &str) -> bool {
        matches!(&self.peek().token_type, TokenType::Identifier(s) if s == value)
    }

    /// Check if current token matches the given token type
    fn check_token_type(&self, token_type: &TokenType) -> bool {
        std::mem::discriminant(&self.peek().token_type) == std::mem::discriminant(token_type)
    }
}

impl Default for ExpressionParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse an expression string into an intermediate Expression.
/// The result should be processed by StringTable.processExpression() to convert
/// strings to string table indices.
///
/// # Arguments
///
/// * `expr` - Expression string (e.g., "user.role == 'admin'" or "role == 'admin'")
///   Note: user. and context. prefixes are accepted but will be normalized during compilation
///
/// # Returns
///
/// Intermediate Expression (with strings, not indices)
///
/// # Errors
///
/// Returns `CompilerError::Parse` if expression is invalid
pub fn parse_expression(expr: &str) -> Result<IntermediateExpression, CompilerError> {
    let mut parser = ExpressionParser::new();
    parser.parse(expr.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_comparison() {
        let result = parse_expression("user.role == 'admin'").unwrap();
        match result {
            IntermediateExpression::BinaryOp {
                op_code,
                left,
                right,
            } => {
                assert_eq!(op_code, BinaryOp::Eq as u8);
                match *left {
                    IntermediateExpression::Property(ref prop) => {
                        assert_eq!(prop, "user.role");
                    }
                    _ => panic!("Expected Property"),
                }
                match *right {
                    IntermediateExpression::Literal(serde_json::Value::String(ref s)) => {
                        assert_eq!(s, "admin");
                    }
                    _ => panic!("Expected String literal"),
                }
            }
            _ => panic!("Expected BinaryOp"),
        }
    }

    #[test]
    fn test_parse_logical_and() {
        let result =
            parse_expression("user.role == 'admin' AND environment == 'production'").unwrap();
        match result {
            IntermediateExpression::LogicalOp { op_code, right, .. } => {
                assert_eq!(op_code, LogicalOp::And as u8);
                assert!(right.is_some());
            }
            _ => panic!("Expected LogicalOp"),
        }
    }

    #[test]
    fn test_parse_logical_or() {
        let result = parse_expression("user.role == 'admin' OR user.role == 'moderator'").unwrap();
        match result {
            IntermediateExpression::LogicalOp { op_code, .. } => {
                assert_eq!(op_code, LogicalOp::Or as u8);
            }
            _ => panic!("Expected LogicalOp"),
        }
    }

    #[test]
    fn test_parse_logical_not() {
        let result = parse_expression("NOT user.role == 'guest'").unwrap();
        match result {
            IntermediateExpression::LogicalOp { op_code, right, .. } => {
                assert_eq!(op_code, LogicalOp::Not as u8);
                assert!(right.is_none());
            }
            _ => panic!("Expected LogicalOp with NOT"),
        }
    }

    #[test]
    fn test_parse_parentheses() {
        let result = parse_expression(
            "(user.role == 'admin' AND environment == 'production') OR user.role == 'moderator'",
        )
        .unwrap();
        match result {
            IntermediateExpression::LogicalOp { op_code, .. } => {
                assert_eq!(op_code, LogicalOp::Or as u8);
            }
            _ => panic!("Expected LogicalOp"),
        }
    }

    #[test]
    fn test_parse_function_call() {
        let result = parse_expression("STARTS_WITH(user.id, 'guest_')").unwrap();
        match result {
            IntermediateExpression::Func { func_code, args } => {
                assert_eq!(func_code, FuncCode::StartsWith as u8);
                assert_eq!(args.len(), 2);
            }
            _ => panic!("Expected Func"),
        }
    }

    #[test]
    fn test_parse_array_literal() {
        let result = parse_expression("user.role IN ['admin', 'moderator']").unwrap();
        // The IN operator should be parsed as a function call
        match result {
            IntermediateExpression::Func { func_code, args } => {
                assert_eq!(func_code, FuncCode::In as u8);
                assert_eq!(args.len(), 2);
            }
            _ => panic!("Expected Func for IN operator"),
        }
    }

    #[test]
    fn test_parse_number_literal() {
        let result = parse_expression("user.account_age_days > 30").unwrap();
        match result {
            IntermediateExpression::BinaryOp { op_code, right, .. } => {
                assert_eq!(op_code, BinaryOp::Gt as u8);
                match *right {
                    IntermediateExpression::Literal(serde_json::Value::Number(n)) => {
                        assert_eq!(n.as_i64(), Some(30));
                    }
                    _ => panic!("Expected Number literal"),
                }
            }
            _ => panic!("Expected BinaryOp"),
        }
    }

    #[test]
    fn test_parse_boolean_literal() {
        let result = parse_expression("true").unwrap();
        match result {
            IntermediateExpression::Literal(serde_json::Value::Bool(b)) => {
                assert!(b);
            }
            _ => panic!("Expected Boolean literal"),
        }
    }

    #[test]
    fn test_parse_null_literal() {
        let result = parse_expression("user.preferred_theme == null").unwrap();
        match result {
            IntermediateExpression::BinaryOp { right, .. } => match *right {
                IntermediateExpression::Literal(serde_json::Value::Null) => {}
                _ => panic!("Expected Null literal"),
            },
            _ => panic!("Expected BinaryOp"),
        }
    }

    #[test]
    fn test_parse_complex_expression() {
        let result = parse_expression(
            "user.role == 'admin' AND (environment == 'production' OR environment == 'staging')",
        )
        .unwrap();
        match result {
            IntermediateExpression::LogicalOp { op_code, .. } => {
                assert_eq!(op_code, LogicalOp::And as u8);
            }
            _ => panic!("Expected LogicalOp"),
        }
    }

    #[test]
    fn test_parse_all_comparison_operators() {
        let ops = vec![
            ("==", BinaryOp::Eq),
            ("!=", BinaryOp::Ne),
            (">", BinaryOp::Gt),
            ("<", BinaryOp::Lt),
            (">=", BinaryOp::Gte),
            ("<=", BinaryOp::Lte),
        ];

        for (op_str, op_enum) in ops {
            let expr = format!("user.score {} 100", op_str);
            let result = parse_expression(&expr).unwrap();
            match result {
                IntermediateExpression::BinaryOp { op_code, .. } => {
                    assert_eq!(op_code, op_enum as u8, "Failed for operator: {}", op_str);
                }
                _ => panic!("Expected BinaryOp for operator: {}", op_str),
            }
        }
    }

    #[test]
    fn test_parse_all_functions() {
        let functions = vec![
            ("STARTS_WITH", FuncCode::StartsWith),
            ("ENDS_WITH", FuncCode::EndsWith),
            ("CONTAINS", FuncCode::Contains),
            ("MATCHES", FuncCode::Matches),
            ("UPPER", FuncCode::Upper),
            ("LOWER", FuncCode::Lower),
            ("LENGTH", FuncCode::Length),
            ("INTERSECTS", FuncCode::Intersects),
            ("SEMVER_EQ", FuncCode::SemverEq),
            ("SEMVER_GT", FuncCode::SemverGt),
            ("SEMVER_GTE", FuncCode::SemverGte),
            ("SEMVER_LT", FuncCode::SemverLt),
            ("SEMVER_LTE", FuncCode::SemverLte),
            ("HASHED_PARTITION", FuncCode::Hash),
            ("COALESCE", FuncCode::Coalesce),
            ("IS_BETWEEN", FuncCode::IsBetween),
            ("IS_AFTER", FuncCode::IsAfter),
            ("IS_BEFORE", FuncCode::IsBefore),
            ("CURRENT_DAY_OF_WEEK_UTC", FuncCode::DayOfWeek),
            ("CURRENT_HOUR_UTC", FuncCode::HourOfDay),
            ("CURRENT_DAY_OF_MONTH_UTC", FuncCode::DayOfMonth),
            ("CURRENT_MONTH_UTC", FuncCode::Month),
            ("CURRENT_TIMESTAMP", FuncCode::CurrentTimestamp),
            ("IN_SEGMENT", FuncCode::InSegment),
        ];

        for (func_name, func_code) in functions {
            // Create a simple function call expression
            let expr = if func_code == FuncCode::DayOfWeek
                || func_code == FuncCode::HourOfDay
                || func_code == FuncCode::DayOfMonth
                || func_code == FuncCode::Month
                || func_code == FuncCode::CurrentTimestamp
            {
                // Functions with no arguments
                format!("{}()", func_name)
            } else if func_code == FuncCode::IsBetween {
                format!("{}(start, end)", func_name)
            } else if func_code == FuncCode::IsAfter || func_code == FuncCode::IsBefore {
                format!("{}(timestamp)", func_name)
            } else if func_code == FuncCode::InSegment {
                format!("{}(user, 'segment')", func_name)
            } else if func_code == FuncCode::Hash {
                format!("{}(user.id, 100)", func_name)
            } else if func_code == FuncCode::Coalesce {
                format!("{}(user.name, 'default')", func_name)
            } else if func_code == FuncCode::Intersects {
                format!("{}(user.groups, ['admin'])", func_name)
            } else if func_code == FuncCode::In {
                format!("{}(user.role, ['admin'])", func_name)
            } else {
                format!("{}(user.email, 'test')", func_name)
            };

            let result = parse_expression(&expr);
            match result {
                Ok(IntermediateExpression::Func {
                    func_code: code, ..
                }) => {
                    assert_eq!(code, func_code as u8, "Failed for function: {}", func_name);
                }
                Ok(_) => panic!("Expected Func for function: {}", func_name),
                Err(e) => panic!("Failed to parse function {}: {:?}", func_name, e),
            }
        }
    }

    #[test]
    fn test_parse_error_unterminated_string() {
        let result = parse_expression("user.role == 'admin");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_unexpected_token() {
        let result = parse_expression("user.role == 'admin' AND");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_missing_closing_paren() {
        let result = parse_expression("(user.role == 'admin'");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_string_escaping() {
        // Test escaped single quote
        let result = parse_expression("user.name == 'it\\'s a test'").unwrap();
        match result {
            IntermediateExpression::BinaryOp { right, .. } => match *right {
                IntermediateExpression::Literal(serde_json::Value::String(s)) => {
                    assert_eq!(s, "it's a test");
                }
                _ => panic!("Expected string literal"),
            },
            _ => panic!("Expected BinaryOp"),
        }

        // Test escaped backslash
        let result = parse_expression("'path\\\\to\\\\file'").unwrap();
        match result {
            IntermediateExpression::Literal(serde_json::Value::String(s)) => {
                assert_eq!(s, "path\\to\\file");
            }
            _ => panic!("Expected string literal"),
        }
    }

    #[test]
    fn test_parse_empty_string() {
        let result = parse_expression("user.name == ''").unwrap();
        match result {
            IntermediateExpression::BinaryOp { right, .. } => match *right {
                IntermediateExpression::Literal(serde_json::Value::String(s)) => {
                    assert_eq!(s, "");
                }
                _ => panic!("Expected empty string literal"),
            },
            _ => panic!("Expected BinaryOp"),
        }
    }

    #[test]
    fn test_parse_empty_array() {
        let result = parse_expression("user.role IN []").unwrap();
        match result {
            IntermediateExpression::Func { args, .. } => {
                assert_eq!(args.len(), 2);
                match &args[1] {
                    IntermediateExpression::Literal(serde_json::Value::Array(arr)) => {
                        assert!(arr.is_empty());
                    }
                    _ => panic!("Expected array literal"),
                }
            }
            _ => panic!("Expected Func"),
        }
    }

    #[test]
    fn test_parse_deep_property_access() {
        let result = parse_expression("user.profile.settings.theme == 'dark'").unwrap();
        match result {
            IntermediateExpression::BinaryOp { left, .. } => match *left {
                IntermediateExpression::Property(prop) => {
                    assert_eq!(prop, "user.profile.settings.theme");
                }
                _ => panic!("Expected Property"),
            },
            _ => panic!("Expected BinaryOp"),
        }
    }

    #[test]
    fn test_parse_nested_function_calls() {
        let result = parse_expression("STARTS_WITH(UPPER(user.email), 'ADMIN')").unwrap();
        match result {
            IntermediateExpression::Func { args, .. } => {
                assert_eq!(args.len(), 2);
                // First arg should be a function call (UPPER)
                match &args[0] {
                    IntermediateExpression::Func { func_code, .. } => {
                        assert_eq!(*func_code, FuncCode::Upper as u8);
                    }
                    _ => panic!("Expected nested Func"),
                }
            }
            _ => panic!("Expected Func"),
        }
    }

    #[test]
    fn test_parse_in_operator_variations() {
        // Test infix syntax with array literal
        let result = parse_expression("user.role IN ['admin', 'moderator']").unwrap();
        match result {
            IntermediateExpression::Func {
                func_code, args, ..
            } => {
                assert_eq!(func_code, FuncCode::In as u8);
                assert_eq!(args.len(), 2);
            }
            _ => panic!("Expected Func for IN operator"),
        }

        // Test function call syntax
        let result = parse_expression("IN(user.role, ['admin', 'moderator'])").unwrap();
        match result {
            IntermediateExpression::Func {
                func_code, args, ..
            } => {
                assert_eq!(func_code, FuncCode::In as u8);
                assert_eq!(args.len(), 2);
            }
            _ => panic!("Expected Func for IN function"),
        }
    }

    #[test]
    fn test_parse_complex_precedence() {
        // Test that AND has higher precedence than OR
        let result = parse_expression(
            "user.role == 'admin' OR user.role == 'moderator' AND environment == 'production'",
        )
        .unwrap();
        // Should parse as: (user.role == 'admin') OR ((user.role == 'moderator') AND (environment == 'production'))
        match result {
            IntermediateExpression::LogicalOp { op_code, .. } => {
                assert_eq!(op_code, LogicalOp::Or as u8);
            }
            _ => panic!("Expected LogicalOp with OR"),
        }
    }

    #[test]
    fn test_parse_parentheses_override_precedence() {
        // Test that parentheses override operator precedence
        let result = parse_expression(
            "(user.role == 'admin' OR user.role == 'moderator') AND environment == 'production'",
        )
        .unwrap();
        // Should parse as: ((user.role == 'admin') OR (user.role == 'moderator')) AND (environment == 'production')
        match result {
            IntermediateExpression::LogicalOp { op_code, .. } => {
                assert_eq!(op_code, LogicalOp::And as u8);
            }
            _ => panic!("Expected LogicalOp with AND"),
        }
    }

    #[test]
    fn test_parse_zero_argument_function() {
        let result = parse_expression("CURRENT_TIMESTAMP()").unwrap();
        match result {
            IntermediateExpression::Func {
                func_code, args, ..
            } => {
                assert_eq!(func_code, FuncCode::CurrentTimestamp as u8);
                assert_eq!(args.len(), 0);
            }
            _ => panic!("Expected Func"),
        }
    }

    // Behavior tests: Verify that parsed expressions can be compiled and used
    // These tests verify the expressions work end-to-end, not just their structure

    #[test]
    fn test_parse_and_compile_simple_comparison() {
        use crate::compiler::string_table::StringTable;

        let expr = parse_expression("user.role == 'admin'").unwrap();
        let mut string_table = StringTable::new();

        // Verify the expression can be compiled (processed through string table)
        // This is a behavior test - it verifies the expression is usable
        let compiled = string_table.process_expression(&expr).unwrap();

        // Verify it compiled to a BinaryOp expression
        match compiled {
            crate::ast::Expression::BinaryOp { .. } => {
                // Success - expression compiled correctly
            }
            _ => panic!("Expected BinaryOp after compilation"),
        }
    }

    #[test]
    fn test_parse_and_compile_logical_and() {
        use crate::compiler::string_table::StringTable;

        let expr =
            parse_expression("user.role == 'admin' AND environment == 'production'").unwrap();
        let mut string_table = StringTable::new();

        // Verify the expression can be compiled
        let compiled = string_table.process_expression(&expr).unwrap();

        match compiled {
            crate::ast::Expression::LogicalOp { .. } => {
                // Success - expression compiled correctly
            }
            _ => panic!("Expected LogicalOp after compilation"),
        }
    }

    #[test]
    fn test_parse_and_compile_logical_or() {
        use crate::compiler::string_table::StringTable;

        let expr = parse_expression("user.role == 'admin' OR user.role == 'moderator'").unwrap();
        let mut string_table = StringTable::new();

        // Verify the expression can be compiled
        let compiled = string_table.process_expression(&expr).unwrap();

        match compiled {
            crate::ast::Expression::LogicalOp { .. } => {
                // Success - expression compiled correctly
            }
            _ => panic!("Expected LogicalOp after compilation"),
        }
    }

    #[test]
    fn test_parse_and_compile_logical_not() {
        use crate::compiler::string_table::StringTable;

        let expr = parse_expression("NOT user.role == 'guest'").unwrap();
        let mut string_table = StringTable::new();

        // Verify the expression can be compiled
        let compiled = string_table.process_expression(&expr).unwrap();

        match compiled {
            crate::ast::Expression::LogicalOp { .. } => {
                // Success - expression compiled correctly
            }
            _ => panic!("Expected LogicalOp after compilation"),
        }
    }

    #[test]
    fn test_parse_and_compile_function_call() {
        use crate::compiler::string_table::StringTable;

        let expr = parse_expression("STARTS_WITH(user.id, 'guest_')").unwrap();
        let mut string_table = StringTable::new();

        // Verify the expression can be compiled
        let compiled = string_table.process_expression(&expr).unwrap();

        match compiled {
            crate::ast::Expression::Func { .. } => {
                // Success - expression compiled correctly
            }
            _ => panic!("Expected Func after compilation"),
        }
    }

    #[test]
    fn test_parse_and_compile_parentheses() {
        use crate::compiler::string_table::StringTable;

        let expr = parse_expression(
            "(user.role == 'admin' AND environment == 'production') OR user.role == 'moderator'",
        )
        .unwrap();
        let mut string_table = StringTable::new();

        // Verify the expression can be compiled
        let compiled = string_table.process_expression(&expr).unwrap();

        match compiled {
            crate::ast::Expression::LogicalOp { .. } => {
                // Success - expression compiled correctly
            }
            _ => panic!("Expected LogicalOp after compilation"),
        }
    }

    // Note: Full evaluation behavior is tested in the TypeScript runtime integration tests
    // (runtime/typescript/src/integration.test.ts), which verify that compiled expressions
    // evaluate correctly with actual user attributes.
}
