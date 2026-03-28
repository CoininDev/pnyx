#include <arpa/inet.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <unistd.h>

#define BUFFER_SIZE 1024

int server(int argc, char* argv[]);
int client(int argc, char* argv[]);

int main(int argc, char* argv[]) {
  if(atoi(argv[1]) == 0 && argc != 3) {
    printf("Usage server: %s 0 <peer_port>", argv[0]);
    return EXIT_FAILURE;
  }

  if(atoi(argv[1]) == 1 && argc != 5) {
    printf("Usage client: %s 1 <peer_ip> <peer_port> <message>\n", argv[0]);
    return EXIT_FAILURE;
  }
  
  if(atoi(argv[1]) == 0) {
    server(argc, argv);
  } else {
    client(argc, argv);
  }
}

int server(int argc, char* argv[]){
  int my_port = atoi(argv[2]);
  int sock;
  struct sockaddr_in peer_addr;
  struct sockaddr_in my_addr = {
    .sin_family      = AF_INET,
    .sin_addr.s_addr = INADDR_ANY,
    .sin_port        = htons(my_port)
  };
  char buffer[BUFFER_SIZE];

  if((sock = socket(AF_INET, SOCK_DGRAM, 0)) <= 0) {
    perror("server: Failed to create socket");
    return EXIT_FAILURE;
  }

  bind(sock, (struct sockaddr*)&my_addr, sizeof(my_addr));

  socklen_t address_length = sizeof(peer_addr);
  recvfrom(sock, buffer, BUFFER_SIZE, 0, (struct sockaddr*)&peer_addr, &address_length);


  printf("Received from %s:%d -- Message = %s\n", inet_ntoa(peer_addr.sin_addr), ntohs(peer_addr.sin_port), buffer);
  return EXIT_SUCCESS;
}

int client(int argc, char* argv[]){
  const char* ip = argv[2];
  int port = atoi(argv[3]);
  const char* message = argv[4];

  struct sockaddr_in addr = {
    .sin_family = AF_INET,
    .sin_port   = htons(port)
  };

  if(inet_pton(AF_INET, ip, &(addr.sin_addr)) <= 0) {
    perror("Something wrong with ip address");
    return EXIT_FAILURE;
  }

  int sock = socket(AF_INET, SOCK_DGRAM, 0);
  if (sock < 0) {
    perror("Couldn't create socket");
    return EXIT_FAILURE;
  }

  if (sendto(sock, message, strlen(message) + 1, 0, (struct sockaddr*)&addr, sizeof(addr)) < 0) {
    perror("Failed to send message");
    close(sock);
    return EXIT_FAILURE;
  }

  printf("Sent \"%s\" to %s:%d\n", message, ip, port);
  close(sock);

  return EXIT_SUCCESS;
}
