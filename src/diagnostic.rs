use std::fmt;

use crate::ast::Number;
use crate::Spanned as _;

pub enum Diagnostic<'a> {
  NumberMissingIntegralPart { number: Number<'a> },
  NumberLeadingZeroIntegralPart { number: Number<'a> },
  NumberMissingFractionalPart { number: Number<'a> },
  NumberMissingExponentPart { number: Number<'a> },
}

impl fmt::Debug for Diagnostic<'_> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Diagnostic::NumberMissingIntegralPart { number } => {
        write!(
          f,
          "Number is missing an integral part (at {:?})",
          number.span()
        )
      }
      Diagnostic::NumberLeadingZeroIntegralPart { number } => {
        write!(
          f,
          "Number has a leading zero in the integral part (at {:?})",
          number.span()
        )
      }
      Diagnostic::NumberMissingFractionalPart { number } => {
        write!(
          f,
          "Number is missing a fractional part (at {:?})",
          number.span()
        )
      }
      Diagnostic::NumberMissingExponentPart { number } => {
        write!(
          f,
          "Number is missing an exponent part (at {:?})",
          number.span()
        )
      }
    }
  }
}
