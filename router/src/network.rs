use std::fmt::{Display, Formatter};
use std::net::{AddrParseError, Ipv4Addr};
use std::str::FromStr;

pub use crate::subnet_mask::SubNetMask;
use crate::subnet_mask::ParseSubNetMaskError;

#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy)]
pub struct Network {
    prefix: Ipv4Addr,
    subnet_mask: SubNetMask,
}

impl Network {
    /// Creates a new Network object with network `prefix` and `subnet_mask`.
    ///
    /// It is assumed that prefix is valid network prefix.
    pub fn new(prefix: Ipv4Addr, subnet_mask: SubNetMask) -> Self {
        Self {
            prefix,
            subnet_mask,
        }
    }

    pub fn with_prefix_masking(prefix: Ipv4Addr, subnet_mask: SubNetMask) -> Self {
        let address_bytes = u32::from(prefix);
        let masked_prefix = Ipv4Addr::from(address_bytes & subnet_mask.value() as u32 - 1u32);
        Self {
            prefix: masked_prefix,
            subnet_mask,
        }
    }

    pub fn contains(&self, address: Ipv4Addr) -> bool {
        let network_address = u32::from(self.prefix);
        let final_address = network_address + self.subnet_mask.address_range();
        (network_address..=final_address).contains(&address.into())
    }

    pub fn prefix(&self) -> Ipv4Addr {
        self.prefix
    }

    pub fn subnet_mask(&self) -> SubNetMask {
        self.subnet_mask
    }

    pub fn broadcast_address(&self) -> Ipv4Addr {
        Ipv4Addr::from(u32::from(self.prefix) | (1 << 32 - self.subnet_mask.value() as u32) - 1)
    }
}

impl TryFrom<(u32, u8)> for Network {
    type Error = ParseSubNetMaskError;

    fn try_from(prefix_mask_pair: (u32, u8)) -> Result<Self, Self::Error> {
        let (prefix, mask) = prefix_mask_pair;
        let prefix = Ipv4Addr::from(prefix & mask as u32 - 1);
        let subnet_mask =
            SubNetMask::new(mask).ok_or_else(|| Self::Error::ValueOutOfRange(mask))?;
        Ok(Self::new(prefix, subnet_mask))
    }
}

impl TryFrom<&str> for Network {
    type Error = ParseNetworkError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut data_iter = value.split('/');
        let ipv4_address_repr = data_iter
            .next()
            .expect("invalid network str representation: ipv4 not found");
        let subnet_mask_repr = data_iter
            .next()
            .expect("invalid network str representation: subnet mask not found");

        let ipv4_address = Ipv4Addr::from_str(ipv4_address_repr).map_err(Self::Error::from)?;
        let subnet_mask = SubNetMask::try_from(subnet_mask_repr).map_err(Self::Error::from)?;

        Ok(Self::with_prefix_masking(ipv4_address, subnet_mask))
    }
}

impl Display for Network {
    /// Standard CIDR network ipv4 address representation.
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.prefix, self.subnet_mask)
    }
}

#[derive(Debug)]
pub enum ParseNetworkError {
    Ipv4EncodingErr(AddrParseError),
    ParseSubNetMaskError(ParseSubNetMaskError),
}

impl From<AddrParseError> for ParseNetworkError {
    fn from(err: AddrParseError) -> Self {
        Self::Ipv4EncodingErr(err)
    }
}

impl From<ParseSubNetMaskError> for ParseNetworkError {
    fn from(err: ParseSubNetMaskError) -> Self {
        Self::ParseSubNetMaskError(err)
    }
}

impl Display for ParseNetworkError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseNetworkError::Ipv4EncodingErr(err) => {
                write!(f, "invalid Network encoding: {}", err)
            }
            ParseNetworkError::ParseSubNetMaskError(err) => {
                write!(f, "invalid Network encoding: {}", err)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_broadcast_1() {
        let ip = Ipv4Addr::from_str("127.0.0.1").unwrap();
        let subnet_mask = SubNetMask::new(8).unwrap();
        let network = Network::new(ip, subnet_mask);
        assert_eq!(network.broadcast_address(), Ipv4Addr::from_str("127.255.255.255").unwrap());
    }

    #[test]
    fn test_broadcast_2() {
        let ip = Ipv4Addr::from_str("192.168.0.1").unwrap();
        let subnet_mask = SubNetMask::new(24).unwrap();
        let network = Network::new(ip, subnet_mask);
        assert_eq!(network.broadcast_address(), Ipv4Addr::from_str("192.168.0.255").unwrap());
    }

    #[test]
    fn test_broadcast_3() {
        let ip = Ipv4Addr::from_str("192.168.1.10").unwrap();
        let subnet_mask = SubNetMask::new(16).unwrap();
        let network = Network::new(ip, subnet_mask);
        assert_eq!(network.broadcast_address(), Ipv4Addr::from_str("192.168.255.255").unwrap());
    }
}
