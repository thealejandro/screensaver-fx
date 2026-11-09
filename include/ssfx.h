// ABI de C de ssfx-core (core/src/ffi.rs). Lo usa el envoltorio de macOS desde Swift.
#ifndef SSFX_H
#define SSFX_H

#include <stdint.h>

typedef struct SsfxEngine SsfxEngine;

typedef struct {
    int32_t year;
    int32_t month;   // 1..12
    int32_t day;
    int32_t weekday; // 0 = lunes .. 6 = domingo
    int32_t hour;
    int32_t minute;
    int32_t second;
} SsfxWallTime;

// ini y art pueden ser NULL. accent es 0x00RRGGBB. px_w/px_h en píxeles físicos.
SsfxEngine *ssfx_new(int32_t px_w, int32_t px_h, const char *ini, const char *art, uint32_t accent, uint64_t seed, int32_t sys_lang_es);
void ssfx_free(SsfxEngine *e);
// Devuelve 1 si cambió algún píxel.
int32_t ssfx_tick(SsfxEngine *e, double t, const SsfxWallTime *now);
// Búfer BGRX (0x00RRGGBB en little endian), w*h, sin relleno entre filas.
const uint32_t *ssfx_pixels(const SsfxEngine *e, int32_t *w, int32_t *h);
// out = {x, y, w, h} en píxeles desde arriba a la izquierda. Devuelve 1 si hay que reducir (filtrado suave).
int32_t ssfx_dest(const SsfxEngine *e, int32_t *out);

int32_t ssfx_theme_count(void);
const char *ssfx_theme_id(int32_t i);
const char *ssfx_theme_name(int32_t i);
int32_t ssfx_fx_count(void);
const char *ssfx_fx_id(int32_t i);
const char *ssfx_fx_name(int32_t i, int32_t es);

#endif
