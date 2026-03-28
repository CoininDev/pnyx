#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdbool.h>
#include <openssl/sha.h>

typedef struct {
    int seq;
    char data[50];
    unsigned char prev_hash[32];
    unsigned char hash[32];
} TestBlock;

void hashTestBlock(TestBlock *block) {
    unsigned char buffer[sizeof(int) + sizeof(block->data) + sizeof(block->prev_hash)];
    
    memcpy(buffer, &block->seq, sizeof(int));
    memcpy(buffer + sizeof(int), block->data, sizeof(block->data));
    memcpy(buffer + sizeof(int) + sizeof(block->data), block->prev_hash, sizeof(block->prev_hash));

    SHA256(buffer, sizeof(buffer), block->hash);
}

TestBlock* createTestBlock(TestBlock *prev_block, const char *data) {
    TestBlock *b = malloc(sizeof(TestBlock));
    if (!b) return NULL;

    b->seq = prev_block->seq + 1;
    strncpy(b->data, data, sizeof(b->data));
    memcpy(b->prev_hash, prev_block->hash, sizeof(b->prev_hash));

    hashTestBlock(b);
    return b;
}

bool validateTestBlock(TestBlock *b, TestBlock *prev) {
    return memcmp(b->prev_hash, prev->hash, sizeof(prev->hash)) == 0;
}

void print_hash(unsigned char hash[32]) {
    for(int i=0; i<32; i++)
        printf("%02x", hash[i]);
    printf("\n");
}
