use std::cmp::Ordering;

use super::*;

pub(super) fn unary<'a>(operator: UnaryOp, value: RuntimeValue<'a>) -> Result<RuntimeValue<'a>> {
    match (operator, value) {
        (UnaryOp::Exists, RuntimeValue::Nil) => Ok(RuntimeValue::Bool(false)),
        (UnaryOp::Exists, _) => Ok(RuntimeValue::Bool(true)),
        (UnaryOp::Not, RuntimeValue::Bool(value)) => Ok(RuntimeValue::Bool(!value)),
        (UnaryOp::Positive, RuntimeValue::Num(value)) => RuntimeValue::num(value),
        (UnaryOp::Negative, RuntimeValue::Num(value)) => RuntimeValue::num(-value),
        _ => Err(Error::Type("invalid unary operand")),
    }
}

pub(super) fn binary<'a>(
    operator: BinaryOp,
    left: RuntimeValue<'a>,
    right: RuntimeValue<'a>,
) -> Result<RuntimeValue<'a>> {
    use BinaryOp as Op;

    match operator {
        Op::Add => numeric_binary(left, right, |left, right| left + right),
        Op::Subtract => numeric_binary(left, right, |left, right| left - right),
        Op::Multiply => numeric_binary(left, right, |left, right| left * right),
        Op::Divide => {
            let left = number(left)?;
            let right = number(right)?;
            if right == 0.0 {
                return Err(Error::Arithmetic("division by zero"));
            }
            RuntimeValue::num(left / right)
        }
        Op::Modulo => {
            let left = number(left)?;
            let right = number(right)?;
            if right == 0.0 {
                return Err(Error::Arithmetic("modulo by zero"));
            }
            RuntimeValue::num(left % right)
        }
        Op::Power => RuntimeValue::num(number(left)?.powf(number(right)?)),
        Op::Equal | Op::NotEqual => {
            let equal = values_equal(left, right)?;
            Ok(RuntimeValue::Bool(if operator == Op::Equal {
                equal
            } else {
                !equal
            }))
        }
        Op::Less | Op::LessEqual | Op::Greater | Op::GreaterEqual => {
            let ordering = compare_values(left, right)?;
            let result = match operator {
                Op::Less => ordering.is_lt(),
                Op::LessEqual => ordering.is_le(),
                Op::Greater => ordering.is_gt(),
                Op::GreaterEqual => ordering.is_ge(),
                _ => unreachable!(),
            };
            Ok(RuntimeValue::Bool(result))
        }
        Op::And | Op::Or => Err(Error::Type("invalid internal binary operation")),
    }
}

fn numeric_binary<'a>(
    left: RuntimeValue<'a>,
    right: RuntimeValue<'a>,
    operation: impl FnOnce(f64, f64) -> f64,
) -> Result<RuntimeValue<'a>> {
    RuntimeValue::num(operation(number(left)?, number(right)?))
}

pub(super) fn number(value: RuntimeValue<'_>) -> Result<f64> {
    if let RuntimeValue::Num(value) = value {
        Ok(value)
    } else {
        Err(Error::Type("expected number"))
    }
}

pub(super) fn compare_values(left: RuntimeValue<'_>, right: RuntimeValue<'_>) -> Result<Ordering> {
    match (left, right) {
        (RuntimeValue::Num(left), RuntimeValue::Num(right)) => left
            .partial_cmp(&right)
            .ok_or(Error::Arithmetic("cannot compare non-finite numbers")),
        (RuntimeValue::Str(left), RuntimeValue::Str(right)) => Ok(left.cmp(right)),
        _ => Err(Error::Type("values are not orderable")),
    }
}

pub(super) fn values_equal(left: RuntimeValue<'_>, right: RuntimeValue<'_>) -> Result<bool> {
    match (left, right) {
        (RuntimeValue::Map(_), _) | (_, RuntimeValue::Map(_)) => {
            Err(Error::Type("maps are not comparable"))
        }
        (RuntimeValue::Nil, RuntimeValue::Nil) => Ok(true),
        (RuntimeValue::Bool(left), RuntimeValue::Bool(right)) => Ok(left == right),
        (RuntimeValue::Num(left), RuntimeValue::Num(right)) => Ok(left == right),
        (RuntimeValue::Str(left), RuntimeValue::Str(right)) => Ok(left == right),
        _ => Ok(false),
    }
}

pub(super) fn index_value<'a>(
    value: RuntimeValue<'a>,
    index: RuntimeValue<'a>,
) -> Result<RuntimeValue<'a>> {
    match (value, index) {
        (RuntimeValue::Map(values), RuntimeValue::Str(key)) => Ok(values
            .get(key)
            .map(RuntimeValue::borrowed)
            .unwrap_or(RuntimeValue::Nil)),
        _ => Err(Error::Type("indexing requires a map and string key")),
    }
}
