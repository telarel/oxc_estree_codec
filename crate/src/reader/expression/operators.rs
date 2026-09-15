use oxc::ast::ast::{BinaryOperator, LogicalOperator, UnaryOperator};
use sonic_rs::JsonValueTrait;

use crate::errors::read::ReadError;
use crate::reader::engine::context::Cx;
use crate::reader::json::Value;

pub fn binary_operator(
    cx: &Cx<'_>,
    node: &Value,
) -> Result<BinaryOperator, ReadError> {
    let op: &str = node.get("operator").and_then(Value::as_str).unwrap_or("");

    let operator: BinaryOperator = match op {
        | "==" => BinaryOperator::Equality,
        | "!=" => BinaryOperator::Inequality,
        | "===" => BinaryOperator::StrictEquality,
        | "!==" => BinaryOperator::StrictInequality,
        | "<" => BinaryOperator::LessThan,
        | "<=" => BinaryOperator::LessEqualThan,
        | ">" => BinaryOperator::GreaterThan,
        | ">=" => BinaryOperator::GreaterEqualThan,
        | "+" => BinaryOperator::Addition,
        | "-" => BinaryOperator::Subtraction,
        | "*" => BinaryOperator::Multiplication,
        | "/" => BinaryOperator::Division,
        | "%" => BinaryOperator::Remainder,
        | "**" => BinaryOperator::Exponential,
        | "<<" => BinaryOperator::ShiftLeft,
        | ">>" => BinaryOperator::ShiftRight,
        | ">>>" => BinaryOperator::ShiftRightZeroFill,
        | "|" => BinaryOperator::BitwiseOR,
        | "^" => BinaryOperator::BitwiseXOR,
        | "&" => BinaryOperator::BitwiseAnd,
        | "in" => BinaryOperator::In,
        | "instanceof" => BinaryOperator::Instanceof,
        | _ => {
            return Err(ReadError::OperatorUnsupported {
                kind: "binary operator",
                operator: op.to_string(),
                path: cx.path_string(),
            });
        },
    };

    Ok(operator)
}

pub fn logical_operator(
    cx: &Cx<'_>,
    node: &Value,
) -> Result<LogicalOperator, ReadError> {
    let op: &str = node.get("operator").and_then(Value::as_str).unwrap_or("");

    let operator: LogicalOperator = match op {
        | "||" => LogicalOperator::Or,
        | "&&" => LogicalOperator::And,
        | "??" => LogicalOperator::Coalesce,
        | _ => {
            return Err(ReadError::OperatorUnsupported {
                kind: "logical operator",
                operator: op.to_string(),
                path: cx.path_string(),
            });
        },
    };

    Ok(operator)
}

pub fn unary_operator(
    cx: &Cx<'_>,
    node: &Value,
) -> Result<UnaryOperator, ReadError> {
    let op: &str = node.get("operator").and_then(Value::as_str).unwrap_or("");

    let operator: UnaryOperator = match op {
        | "+" => UnaryOperator::UnaryPlus,
        | "-" => UnaryOperator::UnaryNegation,
        | "!" => UnaryOperator::LogicalNot,
        | "~" => UnaryOperator::BitwiseNot,
        | "typeof" => UnaryOperator::Typeof,
        | "void" => UnaryOperator::Void,
        | "delete" => UnaryOperator::Delete,
        | _ => {
            return Err(ReadError::OperatorUnsupported {
                kind: "unary operator",
                operator: op.to_string(),
                path: cx.path_string(),
            });
        },
    };

    Ok(operator)
}

pub fn update_operator(
    cx: &Cx<'_>,
    node: &Value,
) -> Result<oxc::syntax::operator::UpdateOperator, ReadError> {
    let op: &str = node.get("operator").and_then(Value::as_str).unwrap_or("");

    let operator: oxc::syntax::operator::UpdateOperator = match op {
        | "++" => oxc::syntax::operator::UpdateOperator::Increment,
        | "--" => oxc::syntax::operator::UpdateOperator::Decrement,
        | _ => {
            return Err(ReadError::OperatorUnsupported {
                kind: "update operator",
                operator: op.to_string(),
                path: cx.path_string(),
            });
        },
    };

    Ok(operator)
}

pub fn assignment_operator(
    cx: &Cx<'_>,
    node: &Value,
) -> Result<oxc::syntax::operator::AssignmentOperator, ReadError> {
    let op: &str = node.get("operator").and_then(Value::as_str).unwrap_or("");

    let operator: oxc::syntax::operator::AssignmentOperator = match op {
        | "=" => oxc::syntax::operator::AssignmentOperator::Assign,
        | "+=" => oxc::syntax::operator::AssignmentOperator::Addition,
        | "-=" => oxc::syntax::operator::AssignmentOperator::Subtraction,
        | "*=" => oxc::syntax::operator::AssignmentOperator::Multiplication,
        | "/=" => oxc::syntax::operator::AssignmentOperator::Division,
        | "%=" => oxc::syntax::operator::AssignmentOperator::Remainder,
        | "**=" => oxc::syntax::operator::AssignmentOperator::Exponential,
        | "<<=" => oxc::syntax::operator::AssignmentOperator::ShiftLeft,
        | ">>=" => oxc::syntax::operator::AssignmentOperator::ShiftRight,
        | ">>>=" => {
            oxc::syntax::operator::AssignmentOperator::ShiftRightZeroFill
        },
        | "|=" => oxc::syntax::operator::AssignmentOperator::BitwiseOR,
        | "^=" => oxc::syntax::operator::AssignmentOperator::BitwiseXOR,
        | "&=" => oxc::syntax::operator::AssignmentOperator::BitwiseAnd,
        | "||=" => oxc::syntax::operator::AssignmentOperator::LogicalOr,
        | "&&=" => oxc::syntax::operator::AssignmentOperator::LogicalAnd,
        | "??=" => oxc::syntax::operator::AssignmentOperator::LogicalNullish,
        | _ => {
            return Err(ReadError::OperatorUnsupported {
                kind: "assignment operator",
                operator: op.to_string(),
                path: cx.path_string(),
            });
        },
    };

    Ok(operator)
}
