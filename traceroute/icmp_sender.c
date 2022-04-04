// Mikołaj Depta 328690
// Created by mikolaj on 27.03.2022.
//

#include "icmp_sender.h"


ICMPSender icmp_sender_new(i32 socket_fd) {
    const ICMPSender new = { .socket_fd=socket_fd };
    return new;
}


internal u16 icmp_sender_compute_checksum(const void *buff, int length) {
    u_int32_t sum;
    const u16 *ptr = buff;
    assert (length % 2 == 0);
    for (sum = 0; length > 0; length -= 2)
        sum += *ptr++;
    sum = (sum >> 16) + (sum & 0xffff);
    return (u16) (~(sum + (sum >> 16)));
}


EchoRequestParams echo_request_params_new(
        u16 identifier,
        u16 sequence_number,
        usize ttl,
        struct in_addr destination_ipv4_address
){
    const struct sockaddr_in socket_address = {
            .sin_family = AF_INET,
            .sin_port = IPPROTO_IP,
            .sin_addr = destination_ipv4_address
    };
    const EchoRequestParams new = {
            .identifier=identifier,
            .sequence_number=sequence_number,
            .ttl=ttl,
            .socket_address=socket_address
    };
    return new;
}


EchoRequestParams echo_request_params_from_string(
        u16 identifier,
        u16 sequence_number,
        usize ttl,
        char* destination_ipv4_address
){
    struct in_addr in_address = {0};
    usize result = inet_pton(AF_INET, destination_ipv4_address, &in_address);
    if (result == 0) {
        fprintf(stderr, "Invalid IPv4 network address: %s\n", destination_ipv4_address);
        exit(EXIT_FAILURE);
    }
    return echo_request_params_new(identifier, sequence_number, ttl, in_address);
}


isize icmp_sender_echo_request(const ICMPSender* self, const EchoRequestParams* echo_request_params) {
    // Create icmp echo request header.
    struct icmp header = {0};
    header.icmp_type = ICMP_ECHO;
    header.icmp_hun.ih_idseq.icd_id = echo_request_params->identifier;
    header.icmp_hun.ih_idseq.icd_seq = echo_request_params->sequence_number;

    // Calculate header checksum for echo request.
    header.icmp_cksum = icmp_sender_compute_checksum((void *) &header, sizeof(header));

    // set net ttl for the socket.
    setsockopt(self->socket_fd, IPPROTO_IP, IP_TTL, &echo_request_params->ttl, sizeof(i32));

    const isize result = sendto(
            self->socket_fd,
            &header,
            sizeof(header),
            0,
            (struct sockaddr*) &echo_request_params->socket_address,
            sizeof(echo_request_params->socket_address)
    );
    return result;
}
