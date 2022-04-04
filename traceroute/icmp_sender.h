// Mikołaj Depta 328690
// Created by mikolaj on 27.03.2022.
//

#ifndef TRACEROUTE_ICMP_SENDER_H
#define TRACEROUTE_ICMP_SENDER_H

#include <features.h>
#include <netinet/ip.h>
#include <netinet/ip_icmp.h>
#include <arpa/inet.h>
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <errno.h>
#include <unistd.h>
#include <assert.h>
#include "types.h"



/*
 * ICMP request sender object.
 *
 * socket_fd: file descriptor associated with raw ICMP socket that should be used.
 *
 * Provides methods for:
 * - sending ICMP Echo Request messages.
 */
typedef struct {
    i32 socket_fd;
} ICMPSender;


/*
 * Constructor for ICMPSender.
 *
 * returns: new ICMPSender instance.
 */
extern ICMPSender icmp_sender_new(i32 socket_fd);


/*
 * Constructor for EchoRequestParams.
 *
 * returns: new EchoRequestParams instance.
 */
extern EchoRequestParams echo_request_params_new(
        u16 identifier,
        u16 sequence_number,
        usize ttl,
        struct in_addr destination_ipv4_address
);


/*
 * Creates new instance of EchoRequestParams from string.
 *
 * returns: new EchoRequestParams instance.
 */
extern EchoRequestParams echo_request_params_from_string(
        u16 identifier,
        u16 sequence_number,
        usize ttl,
        char* destination_ipv4_address
);


/*
 * Send ICMP Echo Request.
 *
 * icmp_sender: reference to sender object.
 * echo_request_params: ICMP Echo Request params.
 *
 * returns: result of the sendto() function.
 */
extern isize icmp_sender_echo_request(const ICMPSender* self, const EchoRequestParams* echo_request_params);

#endif //TRACEROUTE_ICMP_SENDER_H
