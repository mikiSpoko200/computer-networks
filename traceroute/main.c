// Mikołaj Depta 328690

#include <features.h>
#include <netinet/ip.h>
#include <unistd.h>
#include <stdio.h>
#include "types.h"
#include "icmp_sender.h"
#include "icmp_receiver.h"


int main(int argc, char *argv[]) {
    if (argc != 2) {
        fprintf(stderr, "Expected IPv4 network address.\n");
        exit(EXIT_FAILURE);
    }
    const i32 socket_fd = socket(AF_INET, SOCK_RAW, IPPROTO_ICMP);
    if (socket_fd < 0) {
        fprintf(stderr, "Could not create a socket: %s]\n", strerror(errno));
        exit(EXIT_FAILURE);
    }
    ICMPSender sender = icmp_sender_new(socket_fd);
    ICMPReceiver receiver = icmp_receiver_new(socket_fd);
    PingInfo ping_info = {0};
    EchoRequestParams echo_params = echo_request_params_from_string(
            getpid(),
            1,
            1,
            argv[1]
    );
    for (echo_params.ttl = 1; echo_params.ttl < MAX_HOPS; echo_params.ttl++) {
        echo_params.sequence_number = echo_params.ttl;
        setsockopt (sender.socket_fd, IPPROTO_IP, IP_TTL, &echo_params.ttl, sizeof(int));

        // send ICMP echo requests
        icmp_sender_echo_request(&sender, &echo_params);
        icmp_sender_echo_request(&sender, &echo_params);
        icmp_sender_echo_request(&sender, &echo_params);

        // await for packet arrival
        ping_info = icmp_receiver_await_icmp_packets(&receiver, &echo_params);

        // process received packets
        usize result = ping_info_process_results(&ping_info);
        if (result == SUCCESS) {
            // printf("\nTraceroute completed.\n");
            return EXIT_SUCCESS;
        }
    }
}