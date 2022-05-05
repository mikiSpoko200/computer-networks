use std::cmp::{Ordering, PartialEq, PartialOrd};
use std::fmt::{Display, Formatter};
use std::hash::Hash;
use std::net::Ipv4Addr;
use std::ops::Range;

pub use crate::distance::Distance;
use crate::distance::ParseDistanceError;
pub use crate::network::{Network, SubNetMask};
use crate::network::ParseNetworkError;

/// Information about Route to network.
/// This is a wrapper type. It exists only to bundle data so conversion traits can be implemented on it
/// then is should be unpacked to its base components.
#[derive(Hash, Eq, PartialEq)]
pub struct Route {
    pub network: Network,
    pub distance: Distance,
}

impl Route {
    pub fn new(network: Network, distance: Distance) -> Self {
        Self { network, distance }
    }

    pub fn unpack(self) -> (Network, Distance) {
        let Self { network, distance } = self;
        (network, distance)
    }

    pub fn network(&self) -> &Network {
        &self.network
    }

    pub fn distance(&self) -> &Distance {
        &self.distance
    }
}

/// Underlying implementation of deserialization of Route.
///
/// # Route binary format specification
///
/// First 5 bytes represent the network the route leads to.
/// First 4 are the IPv4 address and 5th is the subnet mask - this will be a value from range 0 to 32.
/// Ip address will be encoded in Big Endian format.
///
/// Bytes 6 to 9 (4 bytes total) is an unsigned integer containing the length of the route to network
/// specified in bytes 1 - 5. Infinity is encoded as u32::MAX.
/// Byte order should be Big Endian.
impl From<RouteUdpPacket> for Route {
    fn from(value: RouteUdpPacket) -> Self {
        let network = value.network();
        let distance = value.distance();
        Self::new(network, distance)
    }
}

/// Creates Route from String representation.
impl TryFrom<&str> for Route {
    type Error = ParseRouteError;

    fn try_from(str_encoding: &str) -> Result<Self, Self::Error> {
        let mut data_iter = str_encoding.split_whitespace();
        let network_repr = data_iter
            .next()
            .expect("invalid route str representation: network data not found");
        let distance_repr = data_iter
            .nth(1)
            .expect("invalid route str representation: distance missing");

        let network = Network::try_from(network_repr).map_err(ParseRouteError::from)?;
        let distance = Distance::try_from(distance_repr).map_err(ParseRouteError::from)?;

        Ok(Self::new(network, distance))
    }
}

impl PartialOrd for Route {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.network == other.network {
            Some(self.distance.cmp(other.distance()))
        } else {
            None
        }
    }
}

impl Display for Route {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.network, self.distance)
    }
}


#[derive(Debug)]
pub enum ParseRouteError {
    ParseNetworkError(ParseNetworkError),
    ParseDistanceError(ParseDistanceError),
}

impl From<ParseNetworkError> for ParseRouteError {
    fn from(err: ParseNetworkError) -> Self {
        Self::ParseNetworkError(err)
    }
}

impl From<ParseDistanceError> for ParseRouteError {
    fn from(err: ParseDistanceError) -> Self {
        Self::ParseDistanceError(err)
    }
}

impl Display for ParseRouteError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseRouteError::ParseNetworkError(err) => {
                write!(f, "{}", err)
            }
            ParseRouteError::ParseDistanceError(err) => {
                write!(f, "{}", err)
            }
        }
    }
}


type RouteUdpPacketBuffer = [u8; 9];

#[derive(Hash, Eq, PartialEq, Copy, Clone)]
pub struct RouteUdpPacket(RouteUdpPacketBuffer);

impl RouteUdpPacket {
    /* Route binary format parameters */
    const ADDRESS_BYTES_LEN: usize = 4;
    const DISTANCE_BYTES_LEN: usize = 4;
    const ADDRESS_BYTES: Range<usize> = 0..4;
    const SUBNET_BYTES: usize = 4;
    const DISTANCE_BYTES: Range<usize> = 5..9;

    /// Getter that extracts network from underlying buffer.
    pub fn network(&self) -> Network {
        let address = Ipv4Addr::from(
            u32::from_be_bytes(self.0[Self::ADDRESS_BYTES].try_into().unwrap())
        );
        let subnet_mask = SubNetMask::new(
            self.0[Self::SUBNET_BYTES]
        ).unwrap();
        Network::new(address, subnet_mask)
    }

    /// Getter that extracts Distance value from underlying buffer.
    /// Function performs conversion from network endianness to native endianness.
    ///
    /// # Panics
    ///
    /// This function can panic if u32 cannot be obtained from the buffer.
    pub fn distance(&self) -> Distance {
        Distance::new(u32::from_be_bytes(
            self.0[RouteUdpPacket::DISTANCE_BYTES].try_into().unwrap(),
        ))
    }
}

impl From<RouteUdpPacket> for (Network, Distance) {
    fn from(packet: RouteUdpPacket) -> Self {
        (packet.network(), packet.distance())
    }
}

impl AsRef<[u8]> for RouteUdpPacket {
    fn as_ref(&self) -> &[u8] { &self.0 }
}

impl AsMut<[u8]> for RouteUdpPacket {
    fn as_mut(&mut self) -> &mut [u8] { &mut self.0 }
}

impl Default for RouteUdpPacket {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl From<&Route> for RouteUdpPacket {
    fn from(route: &Route) -> Self {
        let mut buffer: RouteUdpPacketBuffer = Default::default();

        /* Convert values to byte arrays in network endianness */
        let address_bytes = u32::from(route.network().prefix()).to_be_bytes();
        let subnet_mask_bytes = route.network().subnet_mask().into();
        let distance_bytes = u32::from(route.distance).to_be_bytes();

        buffer[RouteUdpPacket::ADDRESS_BYTES].copy_from_slice(&address_bytes);
        buffer[RouteUdpPacket::SUBNET_BYTES] = subnet_mask_bytes;
        buffer[RouteUdpPacket::DISTANCE_BYTES].copy_from_slice(&distance_bytes);

        Self(buffer)
    }
}


#[cfg(test)]
mod tests_route_udp_packet {}
