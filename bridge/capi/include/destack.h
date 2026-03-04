#ifndef DESTACK_H
#define DESTACK_H

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

uint32_t destack_capi_abi_version(void);
const char *destack_capi_version(void);
bool destack_capi_is_available(void);

#ifdef __cplusplus
}
#endif

#endif
