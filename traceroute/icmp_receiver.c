// Mikołaj Depta 328690
// Created by mikolaj on 27.03.2022.
//

#include "icmp_receiver.h"

// region IPICMPPacket

void ip_icmp_package_init_from_filled_buffer(IPICMPPacket* restrict self) {
    const struct ip* _temp_ip_header = (struct ip*) self->buffer;
    self->header_len = IP_HEADER_SIZE_IN_BYTES(_temp_ip_header);
    self->data_len = IP_MAXPACKET - self->header_len;
    self->sender_ipv4_address.s_addr = _temp_ip_header->ip_src.s_addr;
}

// endregion



// region ICMPPacket

ICMPPacket icmp_packet_from_ip_icmp_packet(const IPICMPPacket* ip_icmp_packet) {
    ICMPPacket new = {0};
    const u8* icmp_packet_data = ip_icmp_packet->buffer + ip_icmp_packet->header_len;
    memcpy(&new.header, icmp_packet_data, ICMP_MINLEN);
    memcpy(new.data, icmp_packet_data + ICMP_MINLEN, MAX_ICMP_PACKET_SIZE - ICMP_MINLEN);
    return new;
}


void icmp_packet_time_exceeded_embedded_icmp_header(const ICMPPacket* self, struct icmp* header) {
    assert(self->header.icmp_type == ICMP_TIME_EXCEEDED);
    const struct ip* embedded_ip_header = (struct ip*) self->data;
    const usize embedded_ip_header_len = IP_HEADER_SIZE_IN_BYTES(embedded_ip_header);
    memcpy(header, self->data + embedded_ip_header_len, ICMP_MINLEN);
}


extern bool icmp_packet_is_time_to_live_exceeded_message(const ICMPPacket* self) {
    return self->header.icmp_type == ICMP_TIME_EXCEEDED;
}


bool icmp_packet_is_echo_reply_message(const ICMPPacket* self) {
    return self->header.icmp_type == ICMP_ECHOREPLY;
}


bool icmp_packet_is_time_to_live_exceeded_message_valid(
        const ICMPPacket* self,
        const EchoRequestParams* echo_params
) {
    if (icmp_packet_is_time_to_live_exceeded_message(self)) {
        struct icmp embedded_header = {0};
        icmp_packet_time_exceeded_embedded_icmp_header(self, &embedded_header);
        return (
            embedded_header.icmp_hun.ih_idseq.icd_id == echo_params->identifier &&
            embedded_header.icmp_hun.ih_idseq.icd_seq == echo_params->sequence_number
        );
    } else {
        fprintf(stderr, "Expected ICMP Time Exceeded Message - Type 11, got Type: %d\n", self->header.icmp_type);
        exit(EXIT_FAILURE);
    }
}


bool icmp_packet_is_echo_reply_message_valid(
        const ICMPPacket* self,
        const EchoRequestParams* echo_params
) {
    if (icmp_packet_is_echo_reply_message(self)) {
        return (
            self->header.icmp_hun.ih_idseq.icd_id == echo_params->identifier &&
            self->header.icmp_hun.ih_idseq.icd_seq == echo_params->sequence_number
        );
    } else {
        fprintf(stderr, "Expected ICMP Echo Reply Message - Type 0, got Type: %d\n", self->header.icmp_type);
        exit(EXIT_FAILURE);
    }
}

// endregion



// region PingInfo

usize ping_info_process_results(const PingInfo* self) {
    char address_buffer[20];
    printf("%hhu.", self->ttl);
    if (self->timeout) {
        printf(" *\n");
    } else if (self->message_type == ICMP_ECHOREPLY) {
        inet_ntop(AF_INET, &self->echo_reply.ip_address, address_buffer, sizeof(address_buffer));
        printf(" %-15s", address_buffer);
        u64 time = self->echo_reply.round_trip_time.tv_usec / 1000 + self->echo_reply.round_trip_time.tv_sec * 1000;
        printf(" %lums\n", time);
        return SUCCESS;
    } else {
        for (usize i = 0; i < self->ttl_exceeded.unique_address_count; i++) {
            inet_ntop(AF_INET, &self->ttl_exceeded.ip_addresses[i], address_buffer, sizeof(address_buffer));
            printf(" %-15s", address_buffer);
        }
        if (self->ttl_exceeded.collected_packets == PACKET_COUNT) {
            struct timeval total = {0};
            timeradd(&self->ttl_exceeded.round_trip_times[0], &total, &total);
            timeradd(&self->ttl_exceeded.round_trip_times[1], &total, &total);
            timeradd(&self->ttl_exceeded.round_trip_times[2], &total, &total);
            u64 time = (total.tv_usec / 1000 + total.tv_sec * 1000) / 3;
            printf(" %lums\n", time);
        } else {
            printf(" ???\n");
        }
    }
    return NO_SUCCESS;
}

// endregion



// region ICMPReceiver

internal void icmp_receiver_reset_descriptor_set(ICMPReceiver* self) {
    fd_set descriptor_set;
    FD_ZERO(&descriptor_set);
    FD_SET(self->socket_fd, &descriptor_set);
    self->descriptor_set = descriptor_set;
}


ICMPReceiver icmp_receiver_new(i32 socket_fd) {
    ICMPReceiver new = {0};
    new.socket_fd = socket_fd;
    icmp_receiver_reset_descriptor_set(&new);
    return new;
}


/*
 * Note:
 *     [select()] ...
 *     Upon return, each of the file descriptor sets is modified in place
 *     to indicate which file descriptors are currently "ready".
 *     Thus, if using select() within a loop, the sets must be reinitialized before each call.
 *
 * Note:
 *      On Linux, select() modifies timeout to reflect the amount of time not slept;
 */

PingInfo icmp_receiver_await_icmp_packets(
        ICMPReceiver* restrict self,
        const EchoRequestParams* echo_params
){
    PingInfo ping_info = {0};
    ping_info.ttl = echo_params->ttl;
    ping_info.timeout = false;
    ICMPPacket icmp_packet = {0};
    IPICMPPacket ip_icmp_packet = {0};
    u8 collected_packets = 0;

    // region setup timers
    struct timeval stop_time, wait_time, round_trip_time;
    stop_time.tv_sec = MAX_WAIT_TIME_IN_SECONDS;
    stop_time.tv_usec = 0;
    wait_time.tv_sec = MAX_WAIT_TIME_IN_SECONDS;
    wait_time.tv_usec = 0;
    // endregion

    while (collected_packets < PACKET_COUNT && ping_info.timeout == false) {
        icmp_receiver_reset_descriptor_set(self);
        // !! select modifies the descriptor sets !!
        i32 ready = select(
                self->socket_fd + 1,
                &self->descriptor_set,
                NULL,
                NULL,
                &wait_time
        );
        if (ready > 0) {
            bool read_bytes = true;
            while (read_bytes) {
                struct sockaddr_in sender_address = {0};
                socklen_t sender_struct_size = sizeof(sender_address);
                isize result = recvfrom(
                        self->socket_fd, 
                        ip_icmp_packet.buffer,
                        IP_MAXPACKET, 
                        MSG_DONTWAIT,
                        (struct sockaddr*) &sender_address, 
                        &sender_struct_size
                );
                if (result < 0) {
                    if (errno == EWOULDBLOCK) {
                        read_bytes = false;
                    } else {
                        fprintf(stderr, "Error occurred while reading incoming IPv4 packets: %s", strerror(errno));
                        exit(EXIT_FAILURE);
                    }
                } else {
                    ip_icmp_package_init_from_filled_buffer(&ip_icmp_packet);
                    icmp_packet = icmp_packet_from_ip_icmp_packet(&ip_icmp_packet);    /* Extract ICMP packet from IP packet. */

                    timersub(&stop_time, &wait_time, &round_trip_time);                              /* calculate round trip time */
                    ping_info.message_type = icmp_packet.header.icmp_type;

                    // region ICMP message validation
                    switch (ping_info.message_type) {
                        case ICMP_ECHOREPLY: {
                            if (icmp_packet_is_echo_reply_message_valid(&icmp_packet, echo_params)) {
                                ping_info.echo_reply.ip_address = sender_address.sin_addr;
                                ping_info.echo_reply.round_trip_time = round_trip_time;
                                return ping_info;
                            }
                        } break;
                        case ICMP_TIME_EXCEEDED: {
                            if (icmp_packet_is_time_to_live_exceeded_message_valid(&icmp_packet, echo_params)) {
                                ping_info.ttl_exceeded.round_trip_times[collected_packets] = round_trip_time;
                                // region check if current ip address is not already stored
                                bool ip_address_already_added = false;
                                for (usize i = 0; i < ping_info.ttl_exceeded.unique_address_count; i++) {
                                    ip_address_already_added = ping_info.ttl_exceeded.ip_addresses[i].s_addr == sender_address.sin_addr.s_addr;
                                }
                                if (!ip_address_already_added) {
                                    ping_info.ttl_exceeded.ip_addresses[ping_info.ttl_exceeded.unique_address_count++] = sender_address.sin_addr;
                                }
                                collected_packets++;
                                // endregion
                            }
                        } break;
                        default:
                            continue;  /* Ignore all other ICMP message types. */
                    }
                    // endregion
                }
            }
        } else if (ready == 0) {
            ping_info.timeout = true;
        } else {
            fprintf(stderr, "Error occurred while awaiting for file socket file descriptor: %s", strerror(errno));
            exit(EXIT_FAILURE);
        }
    }
    ping_info.ttl_exceeded.collected_packets = collected_packets;
    return ping_info;
}

// endregion
