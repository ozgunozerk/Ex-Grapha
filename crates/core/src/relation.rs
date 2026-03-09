//! Relation expression parser for propositional logic.
//!
//! Parses expressions like `(n-4a7b2c AND n-3f8a1d) IMPLIES n-7c1d3e` into an
//! AST and validates that operands match the node's dependency list.

use std::collections::HashSet;

// ── AST ──────────────────────────────────────────────────

/// AST node for a propositional logic relation expression.
///
/// `Box` is required because `RelationExpr` is recursive — without heap
/// indirection, the enum would have infinite size.
#[derive(Debug, Clone, PartialEq)]
pub enum RelationExpr {
    /// A leaf node referencing a dependency by its node ID.
    Operand(String),
    /// Logical NOT (unary prefix).
    Not(Box<RelationExpr>),
    /// Logical AND (binary infix).
    And(Box<RelationExpr>, Box<RelationExpr>),
    /// Logical OR (binary infix).
    Or(Box<RelationExpr>, Box<RelationExpr>),
    /// Logical implication (binary infix).
    Implies(Box<RelationExpr>, Box<RelationExpr>),
    /// Logical equivalence / biconditional (binary infix).
    Iff(Box<RelationExpr>, Box<RelationExpr>),
}

/// Collect all node-ID operands from a parsed AST.
pub fn collect_operands(expr: &RelationExpr) -> HashSet<String> {
    let mut out = HashSet::new();
    collect_operands_inner(expr, &mut out);
    out
}

fn collect_operands_inner(expr: &RelationExpr, out: &mut HashSet<String>) {
    match expr {
        RelationExpr::Operand(id) => {
            out.insert(id.clone());
        }
        RelationExpr::Not(inner) => collect_operands_inner(inner, out),
        RelationExpr::And(l, r)
        | RelationExpr::Or(l, r)
        | RelationExpr::Implies(l, r)
        | RelationExpr::Iff(l, r) => {
            collect_operands_inner(l, out);
            collect_operands_inner(r, out);
        }
    }
}

// ── Errors ───────────────────────────────────────────────

/// A single error from parsing or validating a relation expression.
#[derive(Debug, Clone, PartialEq)]
pub struct RelationError {
    pub kind: RelationErrorKind,
    /// Byte position in the input where the error occurred (if applicable).
    pub position: Option<usize>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RelationErrorKind {
    UnexpectedToken,
    UnexpectedEnd,
    UnmatchedParen,
    /// Operand in expression not found in dependency list.
    UnknownOperand,
    /// Dependency not referenced in the expression.
    MissingOperand,
    EmptyExpression,
}

impl std::fmt::Display for RelationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(pos) = self.position {
            write!(f, "{} (at position {})", self.message, pos)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

// ── Tokenizer ────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum TokenKind {
    LParen,
    RParen,
    And,
    Or,
    Not,
    Implies,
    Iff,
    /// A node ID like `n-4a7b2c`.
    NodeId(String),
}

#[derive(Debug, Clone)]
struct Token {
    kind: TokenKind,
    /// Byte position in the input where this token starts.
    position: usize,
}

fn tokenize(input: &str) -> Result<Vec<Token>, RelationError> {
    let mut tokens = Vec::new();
    let bytes = input.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        // Skip whitespace.
        if bytes[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }

        // Parentheses.
        if bytes[i] == b'(' {
            tokens.push(Token {
                kind: TokenKind::LParen,
                position: i,
            });
            i += 1;
            continue;
        }
        if bytes[i] == b')' {
            tokens.push(Token {
                kind: TokenKind::RParen,
                position: i,
            });
            i += 1;
            continue;
        }

        // Keywords or node IDs: read an alphanumeric+hyphen word.
        if bytes[i].is_ascii_alphanumeric() || bytes[i] == b'n' {
            let start = i;
            while i < bytes.len()
                && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-' || bytes[i] == b'_')
            {
                i += 1;
            }
            let word = &input[start..i];
            let kind = match word {
                "AND" => TokenKind::And,
                "OR" => TokenKind::Or,
                "NOT" => TokenKind::Not,
                "IMPLIES" => TokenKind::Implies,
                "IFF" => TokenKind::Iff,
                _ => TokenKind::NodeId(word.to_string()),
            };
            tokens.push(Token {
                kind,
                position: start,
            });
            continue;
        }

        return Err(RelationError {
            kind: RelationErrorKind::UnexpectedToken,
            position: Some(i),
            message: format!(
                "unexpected character '{}'",
                input[i..].chars().next().unwrap()
            ),
        });
    }

    Ok(tokens)
}

// ── Parser ───────────────────────────────────────────────
//
// Recursive descent parser implementing the grammar:
//
//   expr       → iff
//   iff        → implies (IFF implies)*           // left-assoc
//   implies    → or (IMPLIES implies)?             // right-assoc
//   or         → and (OR and)*                     // left-assoc
//   and        → unary (AND unary)*                // left-assoc
//   unary      → NOT unary | atom
//   atom       → LPAREN expr RPAREN | NODE_ID
//
// Operator precedence (lowest → highest):
//   IFF < IMPLIES < OR < AND < NOT < atoms

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    /// Input string (for error messages).
    input_len: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>, input_len: usize) -> Self {
        Self {
            tokens,
            pos: 0,
            input_len,
        }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<&Token> {
        let tok = self.tokens.get(self.pos);
        if tok.is_some() {
            self.pos += 1;
        }
        tok
    }

    fn expect_end(&self) -> Result<(), RelationError> {
        if let Some(tok) = self.peek() {
            Err(RelationError {
                kind: RelationErrorKind::UnexpectedToken,
                position: Some(tok.position),
                message: format!("unexpected token after expression: {:?}", tok.kind),
            })
        } else {
            Ok(())
        }
    }

    // ── Grammar rules ────────────────────────────────

    fn parse_expr(&mut self) -> Result<RelationExpr, RelationError> {
        self.parse_iff()
    }

    /// iff → implies (IFF implies)*
    fn parse_iff(&mut self) -> Result<RelationExpr, RelationError> {
        let mut left = self.parse_implies()?;
        while self.peek().is_some_and(|t| t.kind == TokenKind::Iff) {
            self.advance();
            let right = self.parse_implies()?;
            left = RelationExpr::Iff(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    /// implies → or (IMPLIES implies)?   (right-associative via recursion)
    fn parse_implies(&mut self) -> Result<RelationExpr, RelationError> {
        let left = self.parse_or()?;
        if self.peek().is_some_and(|t| t.kind == TokenKind::Implies) {
            self.advance();
            let right = self.parse_implies()?; // recurse for right-assoc
            Ok(RelationExpr::Implies(Box::new(left), Box::new(right)))
        } else {
            Ok(left)
        }
    }

    /// or → and (OR and)*
    fn parse_or(&mut self) -> Result<RelationExpr, RelationError> {
        let mut left = self.parse_and()?;
        while self.peek().is_some_and(|t| t.kind == TokenKind::Or) {
            self.advance();
            let right = self.parse_and()?;
            left = RelationExpr::Or(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    /// and → unary (AND unary)*
    fn parse_and(&mut self) -> Result<RelationExpr, RelationError> {
        let mut left = self.parse_unary()?;
        while self.peek().is_some_and(|t| t.kind == TokenKind::And) {
            self.advance();
            let right = self.parse_unary()?;
            left = RelationExpr::And(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    /// unary → NOT unary | atom
    fn parse_unary(&mut self) -> Result<RelationExpr, RelationError> {
        if self.peek().is_some_and(|t| t.kind == TokenKind::Not) {
            self.advance();
            let inner = self.parse_unary()?;
            Ok(RelationExpr::Not(Box::new(inner)))
        } else {
            self.parse_atom()
        }
    }

    /// atom → LPAREN expr RPAREN | NODE_ID
    fn parse_atom(&mut self) -> Result<RelationExpr, RelationError> {
        let end_pos = self.input_len;
        let tok = self.advance().ok_or_else(|| RelationError {
            kind: RelationErrorKind::UnexpectedEnd,
            position: Some(end_pos),
            message: "unexpected end of expression, expected operand".to_string(),
        })?;

        match &tok.kind {
            TokenKind::LParen => {
                let paren_pos = tok.position;
                let inner = self.parse_expr()?;
                // Expect closing paren.
                let close = self.advance().ok_or_else(|| RelationError {
                    kind: RelationErrorKind::UnmatchedParen,
                    position: Some(paren_pos),
                    message: "unmatched opening parenthesis".to_string(),
                })?;
                if close.kind != TokenKind::RParen {
                    return Err(RelationError {
                        kind: RelationErrorKind::UnmatchedParen,
                        position: Some(close.position),
                        message: format!("expected closing parenthesis, found {:?}", close.kind),
                    });
                }
                Ok(inner)
            }
            TokenKind::NodeId(id) => Ok(RelationExpr::Operand(id.clone())),
            other => Err(RelationError {
                kind: RelationErrorKind::UnexpectedToken,
                position: Some(tok.position),
                message: format!("expected operand or '(', found {:?}", other),
            }),
        }
    }
}

// ── Public API ───────────────────────────────────────────

/// Parse a relation expression string (syntax only, no dependency validation).
pub fn parse_expression(expression: &str) -> Result<RelationExpr, Vec<RelationError>> {
    let trimmed = expression.trim();
    if trimmed.is_empty() {
        return Err(vec![RelationError {
            kind: RelationErrorKind::EmptyExpression,
            position: None,
            message: "relation expression is empty".to_string(),
        }]);
    }

    let tokens = tokenize(trimmed).map_err(|e| vec![e])?;
    let mut parser = Parser::new(tokens, trimmed.len());
    let ast = parser.parse_expr().map_err(|e| vec![e])?;
    parser.expect_end().map_err(|e| vec![e])?;
    Ok(ast)
}

/// Parse a relation expression and validate operands against the dependency
/// list.
///
/// Returns the AST on success. On failure, returns **all** errors found
/// (parse errors and/or dependency mismatches).
pub fn parse_relation(
    expression: &str,
    dependency_ids: &[String],
) -> Result<RelationExpr, Vec<RelationError>> {
    let ast = parse_expression(expression)?;

    let operands = collect_operands(&ast);
    let dep_set: HashSet<&str> = dependency_ids.iter().map(|s| s.as_str()).collect();
    let mut errors = Vec::new();

    // Check: every operand in the expression must be in the dependency list.
    for op in &operands {
        if !dep_set.contains(op.as_str()) {
            errors.push(RelationError {
                kind: RelationErrorKind::UnknownOperand,
                position: None,
                message: format!("operand `{op}` in expression is not in the dependency list"),
            });
        }
    }

    // Check: every dependency must appear in the expression.
    for dep in dependency_ids {
        if !operands.contains(dep.as_str()) {
            errors.push(RelationError {
                kind: RelationErrorKind::MissingOperand,
                position: None,
                message: format!("dependency `{dep}` is not referenced in the relation expression"),
            });
        }
    }

    if errors.is_empty() {
        Ok(ast)
    } else {
        Err(errors)
    }
}
