use ex_grapha_core::relation::{
    collect_operands, parse_expression, parse_relation, RelationErrorKind, RelationExpr,
};

// ── Helpers ──────────────────────────────────────────────

/// Shorthand: wrap a string in `RelationExpr::Operand`.
fn op(id: &str) -> RelationExpr {
    RelationExpr::Operand(id.into())
}

/// Shorthand: wrap two exprs in `RelationExpr::And`.
fn and(l: RelationExpr, r: RelationExpr) -> RelationExpr {
    RelationExpr::And(Box::new(l), Box::new(r))
}

/// Shorthand: wrap two exprs in `RelationExpr::Or`.
fn or(l: RelationExpr, r: RelationExpr) -> RelationExpr {
    RelationExpr::Or(Box::new(l), Box::new(r))
}

/// Shorthand: wrap an expr in `RelationExpr::Not`.
fn not(e: RelationExpr) -> RelationExpr {
    RelationExpr::Not(Box::new(e))
}

/// Shorthand: wrap two exprs in `RelationExpr::Implies`.
fn implies(l: RelationExpr, r: RelationExpr) -> RelationExpr {
    RelationExpr::Implies(Box::new(l), Box::new(r))
}

/// Shorthand: wrap two exprs in `RelationExpr::Iff`.
fn iff(l: RelationExpr, r: RelationExpr) -> RelationExpr {
    RelationExpr::Iff(Box::new(l), Box::new(r))
}

// ── Valid expressions ────────────────────────────────────

#[test]
fn parse_single_operand() {
    let ast = parse_expression("n-abc123").unwrap();
    assert_eq!(ast, op("n-abc123"));
}

#[test]
fn parse_and_expression() {
    let ast = parse_expression("n-aaa AND n-bbb").unwrap();
    assert_eq!(ast, and(op("n-aaa"), op("n-bbb")));
}

#[test]
fn parse_or_expression() {
    let ast = parse_expression("n-aaa OR n-bbb").unwrap();
    assert_eq!(ast, or(op("n-aaa"), op("n-bbb")));
}

#[test]
fn parse_not_expression() {
    let ast = parse_expression("NOT n-aaa").unwrap();
    assert_eq!(ast, not(op("n-aaa")));
}

#[test]
fn parse_implies_expression() {
    let ast = parse_expression("n-aaa IMPLIES n-bbb").unwrap();
    assert_eq!(ast, implies(op("n-aaa"), op("n-bbb")));
}

#[test]
fn parse_iff_expression() {
    let ast = parse_expression("n-aaa IFF n-bbb").unwrap();
    assert_eq!(ast, iff(op("n-aaa"), op("n-bbb")));
}

#[test]
fn parse_parenthesized_expression() {
    let ast = parse_expression("(n-aaa AND n-bbb)").unwrap();
    assert_eq!(ast, and(op("n-aaa"), op("n-bbb")));
}

#[test]
fn parse_nested_parentheses() {
    let ast = parse_expression("((n-aaa))").unwrap();
    assert_eq!(ast, op("n-aaa"));
}

#[test]
fn parse_complex_expression() {
    // (A AND B) IMPLIES C
    let ast = parse_expression("(n-aaa AND n-bbb) IMPLIES n-ccc").unwrap();
    assert_eq!(ast, implies(and(op("n-aaa"), op("n-bbb")), op("n-ccc")));
}

#[test]
fn parse_multi_operator_expression() {
    // (A AND B) OR (C AND D)
    let ast = parse_expression("(n-a AND n-b) OR (n-c AND n-d)").unwrap();
    assert_eq!(
        ast,
        or(and(op("n-a"), op("n-b")), and(op("n-c"), op("n-d")))
    );
}

#[test]
fn parse_chained_and() {
    // A AND B AND C → left-associative: (A AND B) AND C
    let ast = parse_expression("n-a AND n-b AND n-c").unwrap();
    assert_eq!(ast, and(and(op("n-a"), op("n-b")), op("n-c")));
}

#[test]
fn parse_chained_or() {
    // A OR B OR C → left-associative: (A OR B) OR C
    let ast = parse_expression("n-a OR n-b OR n-c").unwrap();
    assert_eq!(ast, or(or(op("n-a"), op("n-b")), op("n-c")));
}

// ── Operator precedence ──────────────────────────────────

#[test]
fn precedence_and_binds_tighter_than_or() {
    // a AND b OR c → (a AND b) OR c
    let ast = parse_expression("n-a AND n-b OR n-c").unwrap();
    assert_eq!(ast, or(and(op("n-a"), op("n-b")), op("n-c")));
}

#[test]
fn precedence_or_binds_tighter_than_implies() {
    // a OR b IMPLIES c → (a OR b) IMPLIES c
    let ast = parse_expression("n-a OR n-b IMPLIES n-c").unwrap();
    assert_eq!(ast, implies(or(op("n-a"), op("n-b")), op("n-c")));
}

#[test]
fn precedence_implies_binds_tighter_than_iff() {
    // a IMPLIES b IFF c IMPLIES d → (a IMPLIES b) IFF (c IMPLIES d)
    let ast = parse_expression("n-a IMPLIES n-b IFF n-c IMPLIES n-d").unwrap();
    assert_eq!(
        ast,
        iff(implies(op("n-a"), op("n-b")), implies(op("n-c"), op("n-d")))
    );
}

#[test]
fn implies_is_right_associative() {
    // a IMPLIES b IMPLIES c → a IMPLIES (b IMPLIES c)
    let ast = parse_expression("n-a IMPLIES n-b IMPLIES n-c").unwrap();
    assert_eq!(ast, implies(op("n-a"), implies(op("n-b"), op("n-c"))));
}

#[test]
fn not_binds_tightest() {
    // NOT a AND b → (NOT a) AND b
    let ast = parse_expression("NOT n-a AND n-b").unwrap();
    assert_eq!(ast, and(not(op("n-a")), op("n-b")));
}

#[test]
fn double_not() {
    let ast = parse_expression("NOT NOT n-a").unwrap();
    assert_eq!(ast, not(not(op("n-a"))));
}

#[test]
fn not_with_parenthesized_expr() {
    // NOT (a OR b)
    let ast = parse_expression("NOT (n-a OR n-b)").unwrap();
    assert_eq!(ast, not(or(op("n-a"), op("n-b"))));
}

#[test]
fn parentheses_override_precedence() {
    // a AND (b OR c) — parens override AND > OR
    let ast = parse_expression("n-a AND (n-b OR n-c)").unwrap();
    assert_eq!(ast, and(op("n-a"), or(op("n-b"), op("n-c"))));
}

// ── Full precedence chain ────────────────────────────────

#[test]
fn full_precedence_chain() {
    // NOT a AND b OR c IMPLIES d IFF e
    // Parsed as: ((((NOT a) AND b) OR c) IMPLIES d) IFF e
    let ast = parse_expression("NOT n-a AND n-b OR n-c IMPLIES n-d IFF n-e").unwrap();
    assert_eq!(
        ast,
        iff(
            implies(or(and(not(op("n-a")), op("n-b")), op("n-c")), op("n-d")),
            op("n-e")
        )
    );
}

// ── collect_operands ─────────────────────────────────────

#[test]
fn collect_operands_from_complex_expr() {
    let ast = parse_expression("(n-a AND n-b) IMPLIES NOT n-c").unwrap();
    let operands = collect_operands(&ast);
    assert_eq!(operands.len(), 3);
    assert!(operands.contains("n-a"));
    assert!(operands.contains("n-b"));
    assert!(operands.contains("n-c"));
}

#[test]
fn collect_operands_deduplicates() {
    // Same operand appears twice
    let ast = parse_expression("n-a AND n-a").unwrap();
    let operands = collect_operands(&ast);
    assert_eq!(operands.len(), 1);
    assert!(operands.contains("n-a"));
}

// ── Dependency validation (parse_relation) ───────────────

#[test]
fn parse_relation_valid() {
    let deps = vec!["n-aaa".to_string(), "n-bbb".to_string()];
    let ast = parse_relation("n-aaa AND n-bbb", &deps).unwrap();
    assert_eq!(ast, and(op("n-aaa"), op("n-bbb")));
}

#[test]
fn parse_relation_single_dep() {
    let deps = vec!["n-aaa".to_string()];
    let ast = parse_relation("n-aaa", &deps).unwrap();
    assert_eq!(ast, op("n-aaa"));
}

#[test]
fn parse_relation_unknown_operand() {
    let deps = vec!["n-aaa".to_string()];
    let errors = parse_relation("n-aaa AND n-bbb", &deps).unwrap_err();

    assert!(errors
        .iter()
        .any(|e| e.kind == RelationErrorKind::UnknownOperand && e.message.contains("n-bbb")));
}

#[test]
fn parse_relation_missing_operand() {
    let deps = vec!["n-aaa".to_string(), "n-bbb".to_string()];
    let errors = parse_relation("n-aaa", &deps).unwrap_err();

    assert!(errors
        .iter()
        .any(|e| e.kind == RelationErrorKind::MissingOperand && e.message.contains("n-bbb")));
}

#[test]
fn parse_relation_both_unknown_and_missing() {
    let deps = vec!["n-aaa".to_string()];
    let errors = parse_relation("n-zzz", &deps).unwrap_err();

    // n-zzz not in deps → UnknownOperand
    assert!(errors
        .iter()
        .any(|e| e.kind == RelationErrorKind::UnknownOperand && e.message.contains("n-zzz")));
    // n-aaa not in expression → MissingOperand
    assert!(errors
        .iter()
        .any(|e| e.kind == RelationErrorKind::MissingOperand && e.message.contains("n-aaa")));
}

#[test]
fn parse_relation_repeated_operand_still_valid() {
    // Using the same operand twice is fine as long as deps match
    let deps = vec!["n-aaa".to_string()];
    let ast = parse_relation("n-aaa AND n-aaa", &deps).unwrap();
    assert_eq!(ast, and(op("n-aaa"), op("n-aaa")));
}

// ── Syntax errors ────────────────────────────────────────

#[test]
fn error_empty_expression() {
    let errors = parse_expression("").unwrap_err();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].kind, RelationErrorKind::EmptyExpression);
}

#[test]
fn error_whitespace_only() {
    let errors = parse_expression("   ").unwrap_err();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].kind, RelationErrorKind::EmptyExpression);
}

#[test]
fn error_unmatched_open_paren() {
    let errors = parse_expression("(n-aaa AND n-bbb").unwrap_err();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].kind, RelationErrorKind::UnmatchedParen);
}

#[test]
fn error_unmatched_close_paren() {
    let errors = parse_expression("n-aaa AND n-bbb)").unwrap_err();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].kind, RelationErrorKind::UnexpectedToken);
}

#[test]
fn error_consecutive_operators() {
    let errors = parse_expression("n-aaa AND AND n-bbb").unwrap_err();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].kind, RelationErrorKind::UnexpectedToken);
}

#[test]
fn error_trailing_operator() {
    let errors = parse_expression("n-aaa AND").unwrap_err();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].kind, RelationErrorKind::UnexpectedEnd);
}

#[test]
fn error_leading_binary_operator() {
    let errors = parse_expression("AND n-aaa").unwrap_err();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].kind, RelationErrorKind::UnexpectedToken);
}

#[test]
fn error_unknown_character() {
    let errors = parse_expression("n-aaa & n-bbb").unwrap_err();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].kind, RelationErrorKind::UnexpectedToken);
    assert!(errors[0].message.contains('&'));
}

#[test]
fn error_empty_parentheses() {
    let errors = parse_expression("()").unwrap_err();
    assert_eq!(errors.len(), 1);
    // RParen is encountered where an operand is expected
    assert_eq!(errors[0].kind, RelationErrorKind::UnexpectedToken);
}

// ── Position info ────────────────────────────────────────

#[test]
fn error_position_for_unknown_char() {
    let errors = parse_expression("n-aaa & n-bbb").unwrap_err();
    assert_eq!(errors[0].position, Some(6));
}

#[test]
fn error_position_for_trailing_operator() {
    // "n-aaa AND " is trimmed to "n-aaa AND" (len 9) before parsing
    let errors = parse_expression("n-aaa AND ").unwrap_err();
    assert_eq!(errors[0].position, Some(9));
}

#[test]
fn error_position_for_unmatched_paren() {
    let errors = parse_expression("(n-aaa").unwrap_err();
    // Position should point to the opening paren
    assert_eq!(errors[0].position, Some(0));
}

// ── Whitespace handling ──────────────────────────────────

#[test]
fn parse_with_extra_whitespace() {
    let ast = parse_expression("  n-aaa   AND   n-bbb  ").unwrap();
    assert_eq!(ast, and(op("n-aaa"), op("n-bbb")));
}

#[test]
fn parse_with_no_spaces_around_parens() {
    let ast = parse_expression("(n-aaa)AND(n-bbb)").unwrap();
    assert_eq!(ast, and(op("n-aaa"), op("n-bbb")));
}

// ── Edge cases ───────────────────────────────────────────

#[test]
fn parse_operand_with_underscores() {
    let ast = parse_expression("n-abc_123").unwrap();
    assert_eq!(ast, op("n-abc_123"));
}

#[test]
fn parse_relation_empty_deps() {
    // An expression with no dependencies should report all operands as unknown
    let deps: Vec<String> = vec![];
    let errors = parse_relation("n-aaa", &deps).unwrap_err();
    assert!(errors
        .iter()
        .any(|e| e.kind == RelationErrorKind::UnknownOperand));
}

#[test]
fn parse_deeply_nested_expression() {
    // ((((n-a))))
    let ast = parse_expression("((((n-a))))").unwrap();
    assert_eq!(ast, op("n-a"));
}

#[test]
fn parse_all_operators_combined() {
    // Use every operator in one expression
    let ast = parse_expression("NOT n-a AND n-b OR n-c IMPLIES n-d IFF n-e").unwrap();
    // Should parse without errors
    let operands = collect_operands(&ast);
    assert_eq!(operands.len(), 5);
}
