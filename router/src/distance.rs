use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::num::ParseIntError;

#[derive(Hash, Eq, PartialEq, Copy, Clone, Debug)]
pub enum Distance {
    Infinite,
    Finite(u32),
}

impl Distance {
    const INFINITY_ENCODING: u32 = u32::MAX;
    const MAX_DISTANCE: u32 = 100;

    pub fn new(dist: u32) -> Self {
        match dist {
            0..=Distance::MAX_DISTANCE => Self::Finite(dist),
            _ => Self::Infinite,
        }
    }
}

impl TryFrom<&str> for Distance {
    type Error = ParseDistanceError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.parse::<u32>() {
            Ok(val) => Ok(Self::new(val)),
            Err(err) => Err(ParseDistanceError(err)),
        }
    }
}

impl From<Distance> for u32 {
    fn from(dist: Distance) -> Self {
        match dist {
            Distance::Infinite => Distance::INFINITY_ENCODING,
            Distance::Finite(dist) => dist,
        }
    }
}

impl Ord for Distance {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Finite(lhs), Self::Finite(rhs)) => lhs.cmp(rhs),
            (Self::Finite(_), Self::Infinite) => Ordering::Less,
            (Self::Infinite, Self::Finite(_)) => Ordering::Greater,
            (Self::Infinite, Self::Infinite) => Ordering::Equal,
        }
    }
}

impl PartialOrd for Distance {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Display for Distance {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Distance::Infinite => write!(f, "unreachable"),
            Distance::Finite(dist) => write!(f, "distance {}", dist),
        }
    }
}

#[derive(Debug)]
pub struct ParseDistanceError(ParseIntError);

impl Display for ParseDistanceError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid Distance encoding: {}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::Distance;

    #[test]
    fn test_from_u32_1() {
        let dist = Distance::new(12);
        assert!(matches!(dist, Distance::Finite(12)));
    }

    #[test]
    fn test_from_u32_2() {
        let dist = Distance::new(Distance::MAX_DISTANCE);
        assert!(matches!(dist, Distance::Finite(Distance::MAX_DISTANCE)));
    }

    #[test]
    fn test_from_u32_infinite_1() {
        let dist = Distance::new(Distance::MAX_DISTANCE + 1);
        assert!(matches!(dist, Distance::Infinite));
    }

    #[test]
    fn test_from_u32_infinite_2() {
        let dist = Distance::new(Distance::INFINITY_ENCODING);
        assert!(matches!(dist, Distance::Infinite));
    }
}
