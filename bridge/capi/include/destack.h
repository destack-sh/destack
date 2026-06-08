#ifndef DESTACK_H
#define DESTACK_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

uint32_t destack_capi_abi_version(void);
const char *destack_capi_version(void);

#ifdef __cplusplus
}
#endif

#endif
