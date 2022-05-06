use std::fmt::{Display, Formatter};
use std::io::{ErrorKind};
use std::net::{IpAddr, Ipv4Addr, SocketAddrV4, UdpSocket};
use std::time::Duration;
use std::thread;
use std::str::FromStr;

use crate::route::Network;
use crate::routing_table::{Route, RouteUdpPacket, RoutingTable, RoutingTableEntry};


#[allow(unused_macros)]
macro_rules! syscall {
    ($fn: ident ( $($arg: expr),* $(,)* ) ) => {{
        let res = unsafe { libc::$fn($($arg, )*) };
        if res == -1 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(res)
        }
    }};
}

const RIP_PORT_NUMBER: u16 = 54321;

pub struct Nic {
    socket: UdpSocket,
    ip_address: Ipv4Addr,
}

impl Nic {
    pub fn new(ip_address: Ipv4Addr) -> Self {
        let socket_address = SocketAddrV4::new(ip_address, RIP_PORT_NUMBER);
        let socket = UdpSocket::bind(socket_address).unwrap();
        // socket.set_nonblocking(true).unwrap();
        socket.set_broadcast(true).unwrap();
        Self { socket, ip_address }
    }

    pub fn broadcast(&self, dest_net: &Network, packet: &[u8]) {
        match self.socket.send_to(packet, SocketAddrV4::new(dest_net.broadcast_address(), RIP_PORT_NUMBER)) {
            Ok(_) => {  }
            Err(err) if err.kind() == ErrorKind::WouldBlock => {  }
            other_err => { panic!("{:?}", other_err) }
        };
    }

    /// Note: Socket should be set to non blocking mode so this call does not hang.
    pub fn collect_route_packets_packets(&mut self) -> Vec<(RouteUdpPacket, Ipv4Addr)> {
        let mut packets = Vec::new();
        loop {
            let mut udp_packet = RouteUdpPacket::default();
            let (bytes_received, sender) = self.socket.recv_from(udp_packet.as_mut()).unwrap();
            if bytes_received > 0 {
                if let IpAddr::V4(address) = sender.ip() {
                    packets.push((udp_packet, address));
                } else {
                    panic!("invalid ip address type")
                }
            } else {
                break
            }
        }
        packets
    }
}

impl TryFrom<&str> for Nic {
    type Error = <Ipv4Addr as FromStr>::Err;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let address_repr = value
            .split_whitespace()
            .next().expect("missing network")
            .split("/")
            .next().expect("incorrect representation");
        Ok(Self::new(Ipv4Addr::from_str(dbg!(address_repr))?))
    }
}

impl Display for Nic {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.ip_address)
    }
}

pub struct Router {
    network_interfaces: Vec<Nic>,
    routing_table: RoutingTable,
}

impl Router {
    const RIP_TURN_WAIT_DURATION: Duration = Duration::from_secs(30);

    pub fn new(network_interfaces: Vec<Nic>, routing_table: RoutingTable) -> Self {
        Self { network_interfaces, routing_table }
    }

    pub fn execute_rip_turn(&mut self) {
        self.broadcast_routes();
        thread::sleep(Router::RIP_TURN_WAIT_DURATION);
        for nic in &mut self.network_interfaces {
            let packets = nic.collect_route_packets_packets();
            for (packet, sender) in packets {
                let (network, distance) = packet.into();
                self.routing_table.update(network, distance, sender);
            }
        }
    }

    fn broadcast_routes(&self) {
        for nic in &self.network_interfaces {
            self.routing_table.entries().for_each(move |route| {
                nic.broadcast(
                    route.network(),
                    RouteUdpPacket::from(&route).as_ref()
                );
            })
        }
    }
}

impl From<&str> for Router {
    fn from(repr: &str) -> Self {
        let mut lines = repr.lines();
        lines.next().expect("router configuration line missing: no network interface count specified");
        let network_interfaces = Vec::from_iter(
            lines.clone().map(|line| Nic::try_from(line).unwrap())
        );
        let routes = Vec::from_iter(
            lines.map(|line| Route::try_from(line).unwrap() )
        );

        let routing_table = RoutingTable::new(routes);
        Self::new(network_interfaces, routing_table)
    }
}

impl Display for Router {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.routing_table)
    }
}
