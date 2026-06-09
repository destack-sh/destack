#ifndef DESTACK_CORE_H
#define DESTACK_CORE_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef enum DestackStatus {
    DESTACK_STATUS_OK = 0,
    DESTACK_STATUS_ERROR = 1,
} DestackStatus;

typedef struct DestackError DestackError;
typedef struct DestackSession DestackSession;
typedef struct DestackSource DestackSource;
typedef struct DestackFileUpdate DestackFileUpdate;

const char *destack_error_message(const DestackError *error);
void destack_error_destroy(DestackError *error);
void destack_string_destroy(char *value);

uint32_t destack_capi_abi_version(void);
const char *destack_capi_version(void);

#ifdef __cplusplus
}
#endif

#endif
