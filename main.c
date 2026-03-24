#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <openssl/sha.h>
#include <time.h>

typedef struct Block{
  int index;
  long timestamp;
  char data[256];
  char prev_hash[65];
  char hash[65];
  struct Block *next;
} Block;

// hash
void calc_hash(Block *b, char output[65]) {
  char input[512];
  snprintf(input, sizeof(input), "%d%ld%s%s", b->index, b->timestamp, b->data, b->prev_hash);
  
  unsigned char bytes[SHA256_DIGEST_LENGTH];
  SHA256((const unsigned char *)input, strlen(input), bytes);
  for (int i = 0; i< SHA256_DIGEST_LENGTH; i++)
    snprintf(output + (i * 2), 3, "%02x", bytes[i]);
  
  output[64]= '\0';
}

Block *create_block(int index, const char *data, const char *prev_hash) {
  Block *b = (Block *)malloc(sizeof(Block));
  
  b->index     = index;
  b->timestamp = (long)time(NULL);
  b->next      = NULL;
  
  strncpy(b->data, data, 255);
  strncpy(b->prev_hash, prev_hash, 64);
  calc_hash(b, b->hash);
  return b;
}

int validate_chain(Block *head) {
  Block *current = head;
  while(current->next != NULL) {
    Block *next = current->next;

    
    if(strcmp(next->prev_hash, current->hash) != 0) {
      printf(" error: block %d changed!", next->index);
      return 1;
    }
    
    char recalc[65];
    calc_hash(current, recalc);
    if(strcmp(recalc, current->hash) != 0) {
      printf(" error: block %s has unmatching hash", current->hash);
      return 1;
    }

    current = next;
  }
  return 0;
}

void print_blockchain(Block *head) {
      Block *cur = head;
    while (cur != NULL) {
        printf("┌──────────────────────────────────────────────────\n");
        printf("│ Block #%d                                        \n", cur->index);
        printf("│ Data     : %s\n", cur->data);
        printf("│ Timestamp : %ld\n", cur->timestamp);
        printf("│ Prev Hash : %.20s...\n", cur->prev_hash);
        printf("│ Hash      : %.20s...\n", cur->hash);
        printf("└──────────────────────────────────────────────────\n");
        if (cur->next) printf("              ↓\n");
        cur = cur->next;
    }
}

void free_blockchain(Block *head) {
  Block *cur = head;
  while (cur != NULL) {
    Block *next = cur->next;
    free(cur);
    cur = next;
  }
}

int main() {
    printf("\n=== Mini Blockchain em C ===\n\n");
 
    // Bloco gênese (primeiro bloco) — prev_hash é "0000...0000"
    char genesis_prev[65];
    memset(genesis_prev, '0', 64);
    genesis_prev[64] = '\0';
 
    Block *head = create_block(0, "Bloco Genesis", genesis_prev);
 
    // Adiciona mais blocos encadeados
    Block *b1 = create_block(1, "Alice envia 10 moedas para Bob",   head->hash);
    Block *b2 = create_block(2, "Bob envia 3 moedas para Carlos",   b1->hash);
    Block *b3 = create_block(3, "Carlos envia 1 moeda para Alice",  b2->hash);
 
    // Encadeia os blocos (lista ligada)
    head->next = b1;
    b1->next   = b2;
    b2->next   = b3;
 
    // Imprime a cadeia
    print_blockchain(head);
 
    // Valida a cadeia
    printf("\n--- Validando blockchain ---\n");
    if (validate_chain(head))
        printf("  ✓ Blockchain válida!\n");
 
    // Simula adulteração
    printf("\n--- Adulterando bloco #1 ---\n");
    strncpy(b1->data, "Alice envia 999 moedas para Bob", 255);
 
    printf("--- Validando após adulteração ---\n");
    if (!validate_chain(head))
        printf("  → Adulteração detectada com sucesso!\n");
 
    // Libera memória
    free_blockchain(head);
 
    printf("\n");
    return 0;
}
