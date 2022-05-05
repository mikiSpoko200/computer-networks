use std::fmt::{Display, Formatter};
use std::net::{AddrParseError, Ipv4Addr};
use std::num::ParseIntError;
use std::str::FromStr;

#[derive(Hash, Eq, PartialEq, Copy, Clone, Debug)]
pub struct SubNetMask(u8);

impl SubNetMask {
    /// Returns new instance of subnet mask.
    /// Returns None if mask value is out of range.
    ///
    /// # Examples
    ///
    /// ```
    /// let subnet_mask_value = 24u8;
    /// let subnet_mask = SubNetMask::new(subnet_mask_value);
    /// assert!(matches!(subnet_mask, SubNetMask(24u8)));
    ///
    /// let value_out_or_range = 33u8;
    /// let mask_from_invalid_value = SubNetMask::new(subnet_mask_value);
    /// assert!(matches!(mask_from_invalid_value, None));
    /// ```
    pub fn new(mask: u8) -> Option<Self> {
        if mask <= 32 {
            Some(Self(mask))
        } else {
            None
        }
    }

    /// Creates new instance of subnet mask.
    /// given mask is truncated to fit 0..=32 value range.
    pub fn with_truncation(mask: u8) -> Self {
        Self(mask % 32 + (mask == 32) as u8 * 32)
    }

    pub fn value(&self) -> u8 {
        self.0
    }

    pub fn address_range(&self) -> u32 {
        1 << self.0
    }
}

impl TryFrom<&str> for SubNetMask {
    type Error = ParseSubNetMaskError;

    fn try_from(data: &str) -> Result<Self, Self::Error> {
        match data.parse::<u8>() {
            Ok(val) => {
                if let Some(subnet_mask) = SubNetMask::new(val) {
                    Ok(subnet_mask)
                } else {
                    Err(Self::Error::ValueOutOfRange(val))
                }
            }
            Err(parse_err) => Err(Self::Error::EncodingErr(parse_err)),
        }
    }
}

impl From<SubNetMask> for u8 {
    fn from(subnet_mask: SubNetMask) -> Self {
        subnet_mask.0
    }
}

impl Display for SubNetMask {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug)]
pub enum ParseSubNetMaskError {
    EncodingErr(ParseIntError),
    ValueOutOfRange(u8),
}

impl From<ParseIntError> for ParseSubNetMaskError {
    fn from(err: ParseIntError) -> Self {
        Self::EncodingErr(err)
    }
}

impl Display for ParseSubNetMaskError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseSubNetMaskError::EncodingErr(err) => {
                write!(f, "invalid SubNetMask encoding: {}", err)
            }
            ParseSubNetMaskError::ValueOutOfRange(val) => {
                write!(
                    f,
                    "subnet mask value out of range, expected value from range 0..=32, got: {}",
                    val
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{SubNetMask, ParseSubNetMaskError};

    #[test]
    fn test_subnet_valid() {
        for i in 0..=32 {
            assert!(matches!(SubNetMask::new(i), Some(_)));
        }
    }

    #[test]
    fn test_subnet_invalid() {
        assert!(matches!(SubNetMask::new(33), None));
    }

    #[test]
    fn test_subnet_from_str_invalid_repr() {
        assert!(matches!(
            SubNetMask::try_from("foobar"),
            Err(ParseSubNetMaskError::EncodingErr(_))
        ))
    }

    #[test]
    fn test_subnet_from_str_invalid_value() {
        assert!(matches!(
            SubNetMask::try_from("33"),
            Err(ParseSubNetMaskError::ValueOutOfRange(33u8))
        ))
    }
}
