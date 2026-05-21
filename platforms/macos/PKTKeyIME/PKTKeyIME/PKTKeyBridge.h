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
void       pktkey_reset_buffer(PKTEngine *engine); /* call on cursor move / focus change */
char      *pktkey_get_mode(const PKTEngine *engine); /* returns "vi" or "en" */
uintptr_t  pktkey_candidate_len(const PKTEngine *engine);

/* Suggestions: returns NULL-terminated array of C strings (or NULL if none).
   count_out is filled with the number of entries. Free with pktkey_free_suggestions. */
char     **pktkey_get_suggestions(const PKTEngine *engine, uintptr_t *count_out);
void       pktkey_free_suggestions(char **arr, uintptr_t count);

void pktkey_free_string(char *s);
