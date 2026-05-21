#pragma once
#include <stdint.h>

#define PKTKEY_REPLACE     0
#define PKTKEY_PASSTHROUGH 1
#define PKTKEY_COMMIT      2

typedef struct {
    int       output_type;
    uintptr_t delete_back;
    char     *text;        /* heap-allocated UTF-8; free with pktkey_free_string */
} PKTKeyOutput;

typedef void PKTEngine; /* opaque handle */

PKTEngine *pktkey_engine_new(const char *preset);
void       pktkey_engine_free(PKTEngine *engine);

int  pktkey_process_key(PKTEngine *engine, const char *key, PKTKeyOutput *out);
int  pktkey_process_backspace(PKTEngine *engine, PKTKeyOutput *out);

void  pktkey_toggle_mode(PKTEngine *engine);
char *pktkey_get_mode(const PKTEngine *engine); /* returns "vi" or "en" */

void pktkey_free_string(char *s);
