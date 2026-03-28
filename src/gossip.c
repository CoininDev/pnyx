#include <arpa/inet.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <unistd.h>
#include "test_block.c"

typedef struct {
  char ip[16];
  int port;
} Peer;

