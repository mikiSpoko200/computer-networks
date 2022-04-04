// Mikołaj Depta 328690
// Created by mikolaj on 27.03.2022.
//

#ifndef TRACEROUTE_TYPES_H
#define TRACEROUTE_TYPES_H
#include <netinet/ip.h>

#include <stdint.h>
#include <stdlib.h>

#define internal static

typedef uint8_t u8;
typedef uint16_t u16;
typedef uint32_t u32;
typedef uint64_t u64;
typedef int8_t i8;
typedef int16_t i16;
typedef int32_t i32;
typedef int64_t i64;
typedef size_t usize;
typedef ssize_t isize;
typedef float f32;
typedef double f64;


/* Collection of parameters for ICMP Echo Request.
 *
 * identifier: identifier used in ICMP header.
 * sequence_number: sequence number used in ICMP header.
 *
 * ttl: Time-To-Live parameter of IPv4 datagram that will contain the echo request.
 * socket_address: struct containing IPv4 addressing information.
 */
typedef struct {
    u16 identifier;
    u16 sequence_number;
    usize ttl;
    struct sockaddr_in socket_address;
} EchoRequestParams;

#endif //TRACEROUTE_TYPES_H
