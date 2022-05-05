use std::collections::{HashMap, HashSet};
use std::collections::hash_map::Entry;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::hash::Hash;
use std::io::{BufRead, BufReader};
use std::net::Ipv4Addr;
use std::ops::{AddAssign, SubAssign};

pub use crate::route::{Route, Distance, Network, RouteUdpPacket, SubNetMask};
use crate::route::ParseRouteError;
use crate::network::ParseNetworkError;
use crate::distance::ParseDistanceError;
use crate::routing_table::ConnectionType::Via;

/// Possible network connection types.
/// Routing rules can specify that the router is either *directly connected* to the
/// destination network or that the packet should be forwarded further *Via* some other router.
#[derive(Hash, Eq, PartialEq, Copy, Clone, Debug)]
pub enum ConnectionType {
    Direct,
    Via(Ipv4Addr),
}

impl Display for ConnectionType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionType::Direct => {
                write!(f, "connected directly")
            }
            ConnectionType::Via(addr) => {
                write!(f, "via {}", addr)
            }
        }
    }
}

#[derive(Debug)]
pub struct RoutingTableEntry {
    network: Network,
    distance: Distance,
    connection_type: ConnectionType,
}

impl RoutingTableEntry {
    pub fn new(network: Network, distance: Distance, connection_type: ConnectionType) -> Self {
        Self {
            network,
            distance,
            connection_type,
        }
    }
}

#[derive(Debug)]
enum RoutingTableEntryEncodingErr {
    ParseDistanceError(ParseDistanceError),
    ParseNetworkError(ParseNetworkError),
}

impl From<ParseDistanceError> for RoutingTableEntryEncodingErr {
    fn from(err: ParseDistanceError) -> Self {
        Self::ParseDistanceError(err)
    }
}

impl From<ParseNetworkError> for RoutingTableEntryEncodingErr {
    fn from(err: ParseNetworkError) -> Self {
        Self::ParseNetworkError(err)
    }
}

impl Display for RoutingTableEntryEncodingErr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RoutingTableEntryEncodingErr::ParseDistanceError(err) => {
                write!(f, "{}", err)
            }
            RoutingTableEntryEncodingErr::ParseNetworkError(err) => {
                write!(f, "{}", err)
            }
        }
    }
}

#[derive(Debug)]
pub struct DirectConnectionEntry(pub RoutingTableEntry);

/// Expected input format:
/// <ipv4 address: four comma separated octets>/<integer from range 0..=32> distance <integer distance value>
impl TryFrom<&str> for DirectConnectionEntry {
    type Error = ParseRouteError;

    fn try_from(representation: &str) -> Result<Self, Self::Error> {
        let mut data_iter = representation.split_whitespace();
        let network_repr = data_iter
            .next()
            .expect("invalid route str representation: network data not found");
        let distance_repr = data_iter
            .nth(1)
            .expect("invalid route representation: distance missing");

        let network = Network::try_from(network_repr).map_err(Self::Error::from)?;
        let distance = Distance::try_from(distance_repr).map_err(Self::Error::from)?;

        Ok(Self(RoutingTableEntry::new(
            network,
            distance,
            ConnectionType::Direct,
        )))
    }
}

// #[derive(Debug)]
// pub struct IndirectConnectionEntry(RoutingTableEntry);
//
// impl TryFrom<&RouteUdpPacket> for IndirectConnectionEntry {
//     type Error = ParseRouteError;
//
//     fn try_from(packet: &RouteUdpPacket) -> Result<Self, Self::Error> {
//         let mut buffer = RouteUdpPacket::default();
//
//         /* Convert values to byte arrays in network endianness */
//         let address_bytes = u32::from(packet.ip_address()).to_be_bytes();
//         let subnet_mask_bytes = packet.subnet_mask().into();
//         let distance_bytes = u32::from(packet.distance()).to_be_bytes();
//
//         buffer[RouteUdpPacket::ADDRESS_BYTES].copy_from_slice(&address_bytes);
//         buffer[Self::SUBNET_BYTES] = subnet_mask_bytes;
//         buffer[Self::DISTANCE_BYTES].copy_from_slice(&distance_bytes);
//
//         Self(buffer)
//     }
// }

#[derive(Debug, Default)]
struct ConnectionErrorRegistry(HashMap<Network, u8>);

impl ConnectionErrorRegistry {
    const MAX_STALL_TURNS: usize = 3;
}

/*
Analysis of the example:

1. 10.0.0.0/8 distance 3 connected directly
2. 192.168.5.0/24 distance 2 connected directly
3. 192.168.2.0/24 distance 4 via 192.168.5.5
4. 172.16.0.0/16 distance 7 via 10.0.1.2

What the rules above mean is that the ANY ADDRESS that has the prefix
matching the one contained in the rule, should be routed using the route
associated with said rule that contains the ip address of the router that
the packet should be forwarded to.

By the grace of God we can assume that all networks are in address disjoint.
That is there is only one possible match and we don't need to bother
with the usual routing rule hierarchy.

////

Network can be either directly accessible from the router (connected directly)
or it requires further forewarning to another router (via <routers ip address>)

In our networking model the whole network has a unified cost from either of the routers attached
to it. Seems wierd but whatever.

To sum up: an entry in the routing table should associate network ip addresses (by which we mean
the network prefix and subnet mask) with the routing information which consist of:
1. distance to said network
2. type of connection (usually called Next Hop) this describes whether the network can be
   directly accessed from the current node or should be forwarded further.

On the internet I encountered different entry specifications that contained the information
like the distance (which in our case is the distance thing) and the network interface that should be used.
The presence of the latter makes no sense to me whatsoever because afaik each NIC has exactly one ip address
assigned to it. When we need to forward a packet to some other router we know its ip address thus
we know we must have a interface that is in the same network as this address (otherwise we wouldn't have connection)
and from there we can just check which NIC has address from that network? I feel like Im missing something here.

From english wikipedia:
> The routing table consists of at least three information fields:
>
> network identifier: The destination subnet and netmask  (Network object in our implementation)
> distance: The routing distance of the path through which the packet is to be sent. The route will go in the direction of the gateway with the lowest distance.
> next hop: The next hop, or gateway, is the address of the next station to which the packet is to be sent on the way to its final destination

There is an immense terminological chaos on the internet when it comes to this topic.
*/

/// Manager for the collection of Routing Rules.
/// Routing table updates routing table entries using the Distance Vector Routing method.
/// It detects and handles stale connections.
#[derive(Debug, Default)]
pub struct RoutingTable {
    entries: HashMap<Network, (Distance, ConnectionType)>,
    connection_error_registry: ConnectionErrorRegistry,
}

/*
tura:
1. Wysłać
2. Spi 30 s
3. sprawdza otrzymane pakiety
Powtarza
*/

impl RoutingTable {
    pub fn new(routes: Vec<Route>) -> Self {
        Self::with_direct_connections(routes)
    }

    /// Creates new Routing table treating and setting passed connections as direct.
    pub fn with_direct_connections(
        direct_connections: Vec<Route>,
    ) -> Self
    {
        let entries = HashMap::from_iter(
            direct_connections.into_iter()
                .map(|route| {
                    let Route { network, distance } = route;
                    (network, (distance, ConnectionType::Direct))
                })
        );
        Self { entries, ..Self::default() }
    }

    /// Result if route already exits.
    fn add_route_with_connection(&mut self, route: Route, connection_type: ConnectionType) -> Result<(), String> {
        let Route { network, distance } = route;
        if let Some(_) = self.entries.insert(network, (distance, connection_type)) {
            Err(format!("Routing table rule already exists for network: {network}"))
        } else {
            Ok(())
        }
    }

    /// Result if route already exits.
    pub fn add_route_with_direct_connection(&mut self, route: Route) -> Result<(), String> {
        self.add_route_with_connection(route, ConnectionType::Direct)
    }

    /// Result if route already exits.
    pub fn add_route_with_indirect_connection(&mut self, route: Route, next_hop: Ipv4Addr) -> Result<(), String> {
        self.add_route_with_connection(route, ConnectionType::Via(next_hop))
    }

    /// removes connection, todo: maybe result when no matching entry exists.
    fn remove(&mut self, route: &Route) -> Result<(), String> {
        if let Some(_) = self.entries.remove(route.network()) {
            Err(String::from(format!("No rule for network: {}", route.network())))
        } else {
            Ok(())
        }
    }

    /// Result if no entry for specified network.
    pub fn update(&mut self, network: Network, distance: Distance, sender: Ipv4Addr) {
        match self.entries.entry(network) {
            Entry::Occupied(mut entry) => {
                let &mut (old_distance, connection_type) = entry.get_mut();
                match connection_type {
                    ConnectionType::Direct => { panic!("distance to directly connected network must not change") }
                    ConnectionType::Via(router_ip) => {
                        if router_ip == sender { /* Whatever the distance update */
                            entry.insert((distance, connection_type));
                        } else {
                            if distance < old_distance {
                                entry.insert((distance, ConnectionType::Via(sender)));
                            }
                        }
                    }
                }
            }
            Entry::Vacant(mut entry) => {
                entry.insert((distance, ConnectionType::Via(sender)));
            }
        }
    }

    pub fn entries(&self) -> impl Iterator<Item=Route> + '_ {
        self.entries.iter().map(|entry| {
            let (&network, &(distance, _)) = entry.clone();
            Route::new(network, distance)
        })
    }
}

pub enum ParseRoutingTableError {
    ParseNetworkError(ParseNetworkError),
    ParseDistanceError(ParseDistanceError)
}

impl Display for RoutingTable {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let message = self
            .entries
            .iter()
            .map(|(network, (distance, connection_type))| {
                format!("{} {} {}", network, distance, connection_type)
            })
            .collect::<Vec<String>>()
            .join("\n");
        write!(f, "{message}")
    }
}

#[cfg(test)]
mod tests {
    use super::RoutingTable;

    use std::env;
    use std::fs::File;

}
