//! Calculator — evaluates math expressions prefixed with `=`.
//!
//! Used both by the inline command (Day 3) and as a building block for
//! the timestamp / unit conversion plugins that need arithmetic.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CalcError {
    #[error("empty expression")]
    Empty,
    #[error("invalid expression: {0}")]
    Parse(meval::Error),
}

/// Evaluate a math expression. Supports the usual `+ - * / % ^` plus
/// parentheses, functions (`sin`, `cos`, `sqrt`, …) and constants
/// (`pi`, `e`) via the `meval` crate.
pub fn eval(expr: &str) -> Result<f64, CalcError> {
    let trimmed = expr.trim();
    if trimmed.is_empty() {
        return Err(CalcError::Empty);
    }
    meval::eval_str(trimmed).map_err(CalcError::Parse)
}

/// Format a numeric result for display. Drops the `.0` for integers.
pub fn format_result(value: f64) -> String {
    if value.fract().abs() < f64::EPSILON {
        format!("{}", value as i64)
    } else {
        let rounded = (value * 1e10).round() / 1e10;
        format!("{}", rounded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluates_basic_arithmetic() {
        assert_eq!(eval("1+2*3").unwrap(), 7.0);
        assert_eq!(eval("(1+2)*3").unwrap(), 9.0);
        assert_eq!(eval("10/4").unwrap(), 2.5);
    }

    #[test]
    fn evaluates_functions_and_constants() {
        assert!((eval("sqrt(16)").unwrap() - 4.0).abs() < 1e-9);
        assert!((eval("pi").unwrap() - std::f64::consts::PI).abs() < 1e-9);
    }

    #[test]
    fn rejects_empty() {
        assert!(matches!(eval(""), Err(CalcError::Empty)));
        assert!(matches!(eval("   "), Err(CalcError::Empty)));
    }

    #[test]
    fn rejects_invalid() {
        assert!(eval("1++").is_err());
    }

    #[test]
    fn format_result_drops_trailing_zero() {
        assert_eq!(format_result(7.0), "7");
        assert_eq!(format_result(2.5), "2.5");
    }
}
