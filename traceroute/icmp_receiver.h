// Mikołaj Depta 328690
// Created by mikolaj on 27.03.2022.
//

#ifndef TRACEROUTE_RECEIVER_H
#define TRACEROUTE_RECEIVER_H

#include <stdio.h>
#include <features.h>
#include <netinet/ip_icmp.h>
#include <features.h>
#include <sys/select.h>
#include <errno.h>
#include <string.h>
#include <assert.h>
#include <stdbool.h>
#include <sys/time.h>
#include "types.h"
#include "icmp_sender.h"

#define SUCCESS 1
#define NO_SUCCESS 0
#define PACKET_COUNT 3
#define MAX_WAIT_TIME_IN_SECONDS 1
// Time Exceeded Message contains: 8 bytes header + max 60 from IPv4 header + 8 bytes
#define MAX_ICMP_PACKET_SIZE 76
#define MAX_HOPS 30

#define IP_HEADER_SIZE_IN_BYTES(ip_header) (ip_header)->ip_hl * 4


// region IPICMPPacket

/*
 * Representation of ICMP Packet contained in IPv4 packet.
 * It contains buffer in which whole IPv4 packet should be stored.
 * header_len and data_len contain byte lengths of IPv4 header and data sections.
 * the icmp packet is contained inside IPv4 packet at offset equal to header_len.
 * It also contains copy of the sender ipv4 address.
 */
typedef struct {
    u8 buffer[IP_MAXPACKET];
    u32 header_len;
    u32 data_len;
    struct in_addr sender_ipv4_address;
} IPICMPPacket;


/*
 * Given IPICMPPacket with buffer filled with valid ipv4 header data.
 * Assign all other fields by parsing the ipv4 header contained in tha buffer.
 */
extern void ip_icmp_package_init_from_filled_buffer(IPICMPPacket* self);

// endregion



// region ICMPPacket

/*
 * Representation of ICMP Packet.
 * It contains the header which is truncated to the minimal shared ICMP message header size of 8 bytes.
 * rest of the packet is considered as a separate data section of the packet.
 */
typedef struct {
    struct icmp header;
    u8 data[MAX_ICMP_PACKET_SIZE - ICMP_MINLEN];
} ICMPPacket;


/*
 * Extract ICMP Packet from IP packet containing ICMP Packet.
 *
 * ip_icmp_packet: Reference to IPICMPPacket struct that should be parsed for ICMPPacket data.
 *
 * returns: new instance of ICMPPacket from IPICMPPacket.
 */
extern ICMPPacket icmp_packet_from_ip_icmp_packet(const IPICMPPacket* ip_icmp_packet);


/*
 * Parse Time Exceeded Message data section for the first 8 bytes of the original Echo Request Message.
 * These 8 bytes of original echo message will contain the Identifier and Sequence Number needed for
 * validation of Time Exceeded Message.
 *
 * self: Reference to ICMPPacket struct.
 * self: Reference to icmp header that should be initialized.
 */
extern void icmp_packet_time_exceeded_embedded_icmp_header(const ICMPPacket* self, struct icmp* header);


/* 
 * Check if passed ICMPPackage is a Time Exceeded Message.
 *
 * self: Reference to ICMPPacket struct
 *
 * returns: information if self is a Time Exceeded Message
 */
extern bool icmp_packet_is_time_to_live_exceeded_message(const ICMPPacket* self);


/*
 * Check if ICMP packet is a Time Exceeded Message response to
 * Echo Request Message sent with parameters passed in EchoRequestParams struct.
 *
 * This is determined by parsing the data section of Time Exceeded Message for Echo Request header contained within it.
 * Once Echo Request header is obtained its Identifier and Sequence Number fields are compared against those
 * contained in EchoRequestParams struct.
 *
 * self: Reference to ICMPPacket struct
 * echo_params: parameters used for the Echo Request Message.
 *
 * returns: information if self is a valid Time Exceeded Message
 */
extern bool icmp_packet_is_time_to_live_exceeded_message_valid(
        const ICMPPacket* self,
        const EchoRequestParams* echo_params
);


/*
 * Check if passed ICMPPackage is an Echo Reply Message.
 *
 * self: Reference to ICMPPacket struct
 *
 * returns: information if self is an Echo Reply Message.
 */
extern bool icmp_packet_is_echo_reply_message(const ICMPPacket* self);


/*
 * Check if ICMP packet is a response to Echo Request Message sent with parameters passed in EchoRequestParams struct.
 *
 * This is determined by a comparison of Identifier and Sequence Number fields of Echo Reply Message with those
 * contained in EchoRequestParams.
 *
 * self: Reference to ICMPPacket struct
 * echo_params: parameters used for the Echo Request Message.
 *
 * returns: information if self is a valid Echo Reply Message.
 */
extern bool icmp_packet_is_echo_reply_message_valid(
        const ICMPPacket* self,
        const EchoRequestParams* echo_params
);

// endregion



// region PingInfo

/*
 * Data struct that contains bundle of information about single ping round.
 * Because of how pinging mechanism is implemented it contains either one data set for Echo Reply Message
 * or PACKET_COUNT sets for Time Exceeded Messages.
 *
 * timeout: signifies that last ping round timed out.
 * message_type: type of received ICMP message(s).
 * ttl: time to live used in ping round.
 *
 * Two union variants: ttl_exceeded and echo_reply differ in the number of data sets passed.
 * In case of Echo Reply Message is it assumed that pinging function will return immediately once this type
 * of message is received. Thus, we need only one set of parameters that is:
 *  - round trip time (RRT)
 *  - ip address of the sender
 * In case of Time Exceeded Message however we need to collect more samples in order to better estimate round trip time.
 * Now we can possibly get response from few different hosts, we don't want any duplicates tho.
 * This is the purpose of ip_addresses and unique_address_count fields.
 * First stores unique sender addresses and second is the count of stored addresses (length of ip_addresses).
 * round_trip_times stores RRT's for all received packets and collected_packets is the final number of collected packets.
 */
typedef struct {
    bool timeout;
    u8 message_type;
    u8 ttl;
    union {
        struct {
            struct timeval round_trip_times[PACKET_COUNT];
            struct in_addr ip_addresses[PACKET_COUNT];
            u8 collected_packets;
            u8 unique_address_count;
        } ttl_exceeded;
        struct {
            struct timeval round_trip_time;
            struct in_addr ip_address;
        } echo_reply;
    };
} PingInfo;


/*
 * Process ping round results and display them accordingly to specification.
 *
 * self: reference to PingInfo struct.
 *
 * returns:
 *  - SUCCESS if the final host has been reached and the route has been successfully traced.
 *  - NO_SUCCESS if final host has not yet been reached, and next ping round
 *    with higher ttl should be issued.
 */
extern usize ping_info_process_results(const PingInfo* self);

// endregion



// region ICMPReceiver

/*
 * ICMP message receiver contains data need for pinging requests.
 *
 * socket_fd: file descriptor associated with raw ICMP socket that should be used.
 */
typedef struct {
    i32 socket_fd;
    fd_set descriptor_set;
} ICMPReceiver;


/*
 * Constructor for ICMPReceiver.
 * 
 * socket_fd: file descriptor of the socket that should be used.
 *
 * returns: new instance of ICMPReceiver.
 */
extern ICMPReceiver icmp_receiver_new(i32 socket_fd);


/*
 * Await for icmp packets identified by parameters passed in echo_params.
 * Function will asynchronously wait for MAX_WAIT_TIME_IN_SECONDS seconds.
 *
 * self: Reference to ICMPReceiver struct.
 * echo_params: Parameters of icmp packets that should be accepted.
 *
 * returns: information about received packets.
 */
extern PingInfo icmp_receiver_await_icmp_packets(
        ICMPReceiver* self,
        const EchoRequestParams* echo_params
);

// endregion

#endif //TRACEROUTE_RECEIVER_H
