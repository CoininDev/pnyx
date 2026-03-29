#include <netinet/in.h>
#include <arpa/inet.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <unistd.h>
#include "test_block.c"

#define MAX_PEERS
int port = 5000;

typedef struct {
  char ip[16];
  int port;
} Peer;

int initListener(int port_param, int* sock){
  struct sockaddr_in addr = {
    .sin_family      = AF_INET,
    .sin_addr.s_addr = INADDR_ANY,
    .sin_port        = htons(port_param)
  };

  int sock_ = socket(AF_INET, SOCK_DGRAM, 0);
  if(sock_ <= 0) {
    perror("Failed to start listener");
    return 1;
  }
  bind(sock_, (struct sockaddr*)&addr, sizeof(addr));
  *sock = sock_;

  return 0;
}

Peer createPeer(char ip[16], int port_param){
  Peer p;
  memcpy(p.ip, ip, sizeof(p.ip));
  p.port = port_param;
  return p;
}

int push(int sockfd, Peer* to, char data[50]) {
  struct sockaddr_in addr = {
    .sin_family = AF_INET,
    .sin_port   = htons(to->port)
  };

  if(inet_pton(AF_INET, to->ip, &(addr.sin_addr)) <= 0) {
    perror("Something wrong with ip address");
    return 1;
  }

  int ok = sendto(sockfd, data, strlen(data) + 1, 0, (struct sockaddr*)&addr, sizeof(addr));
  if(ok < 0) {
    printf("Failed to send message: %s", data);
    return 1;
  }

  printf("Sent '%s' to %s:%d\n", data, to->ip, to->port);
  return 0;
}

int pull(int sockfd, char data[50]) {
    struct sockaddr_in peer_addr;
    socklen_t addr_len = sizeof(peer_addr);

    ssize_t n = recvfrom(
        sockfd,
        data,
        50,
        0,
        (struct sockaddr*)&peer_addr,
        &addr_len
    );

    if (n < 0) {
        perror("recvfrom failed");
        return -1;
    }

    if(n >= 50) n = 49;
    data[n] = '\0';

    printf("Received from %s:%d -- Message = %s\n",
           inet_ntoa(peer_addr.sin_addr),
           ntohs(peer_addr.sin_port),
           data);

    return 0;
}

int contains(char knowns[64][50], int count, const char* target) {
    for (int i = 0; i < count; i++) {
        if (strcmp(knowns[i], target) == 0) {
            return 1; // encontrou
        }
    }
    return 0; // não encontrou
}

void example1(int port_param, Peer peers[], int peers_len) {
  port = port_param;
  int listener;
  initListener(port, &listener);
  
  int sender = socket(AF_INET, SOCK_DGRAM, 0);
  
  char knowns[64][50];
  
  while(1) {
    char data[50];
    pull(listener, data);
    if(contains(knowns, 64, data)) {
      continue;
    }

    printf("Got new: '%s';", data);
    
    for(int i = 0; i <= peers_len; ++i){
      push(sender, &peers[i], data);
    }
  }
}

void example2(int port_param, Peer peers[], int peers_len) {
  port = port_param;
  int listener;
  initListener(port, &listener);
  
  int sender = socket(AF_INET, SOCK_DGRAM, 0);
  
  char knowns[64][50];
  
  while(1) {
    char buffer[50];
    printf("> ");
    fgets(buffer, sizeof(buffer), stdin);
    for(int i = 0; i <= peers_len; ++i){
      push(sender, &peers[i], buffer);
    }
  }  
}


int main(int argc, char* argv[]) {
  if(atoi(argv[1]) == 0) {
    char l[16] = "127.0.0.1";
    Peer ps[] = {
      createPeer(l, 5001),
      createPeer(l, 5002),
    };
    example2(5000, ps, 2);
  }
  if(atoi(argv[1]) == 1) {
    char l[16] = "127.0.0.1";
    Peer ps[] = {
      createPeer(l, 5002),
      createPeer(l, 5003),
    };
    example1(5001, ps, 2);
  }
  if(atoi(argv[1]) == 2) {
    char l[16] = "127.0.0.1";
    Peer ps[] = {
      createPeer(l, 5004),
      createPeer(l, 5000),
    };
    example1(5002, ps, 2);
  }
  if(atoi(argv[1]) == 3) {
    char l[16] = "127.0.0.1";
    Peer ps[] = {
      createPeer(l, 5004),
    };
    example2(5003, ps, 1);
  }
  if(atoi(argv[1]) == 4) {
    char l[16] = "127.0.0.1";
    Peer ps[] = {};
    example1(5004, ps, 0);
  }
  return 0;
}
