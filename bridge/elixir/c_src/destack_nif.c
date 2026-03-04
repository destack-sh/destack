#include <erl_nif.h>

#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

#if defined(_WIN32)
#include <windows.h>
#else
#include <dlfcn.h>
#endif

typedef uint32_t (*destack_capi_abi_version_fn)(void);
typedef const char *(*destack_capi_version_fn)(void);
typedef bool (*destack_capi_is_available_fn)(void);

static int capi_loaded = 0;
static destack_capi_abi_version_fn capi_abi_version = NULL;
static destack_capi_version_fn capi_version = NULL;
static destack_capi_is_available_fn capi_is_available = NULL;

// load capi symbols once
static void load_capi_symbols(void) {
  if (capi_loaded) {
    return;
  }
  capi_loaded = 1;

#if defined(_WIN32)
  HMODULE handle = NULL;
  const char *explicit_path = getenv("DESTACK_CAPI_LIB");
  if (explicit_path != NULL && explicit_path[0] != '\0') {
    handle = LoadLibraryA(explicit_path);
  }
  if (handle == NULL) {
    handle = LoadLibraryA("destack_capi.dll");
  }
  if (handle == NULL) {
    return;
  }

  capi_abi_version =
      (destack_capi_abi_version_fn)GetProcAddress(handle, "destack_capi_abi_version");
  capi_version = (destack_capi_version_fn)GetProcAddress(handle, "destack_capi_version");
  capi_is_available =
      (destack_capi_is_available_fn)GetProcAddress(handle, "destack_capi_is_available");
#else
  void *handle = NULL;
  const char *explicit_path = getenv("DESTACK_CAPI_LIB");
  if (explicit_path != NULL && explicit_path[0] != '\0') {
    handle = dlopen(explicit_path, RTLD_NOW | RTLD_LOCAL);
  }
  if (handle == NULL) {
#if defined(__APPLE__)
    handle = dlopen("libdestack_capi.dylib", RTLD_NOW | RTLD_LOCAL);
#else
    handle = dlopen("libdestack_capi.so", RTLD_NOW | RTLD_LOCAL);
#endif
  }
  if (handle == NULL) {
    return;
  }

  capi_abi_version = (destack_capi_abi_version_fn)dlsym(handle, "destack_capi_abi_version");
  capi_version = (destack_capi_version_fn)dlsym(handle, "destack_capi_version");
  capi_is_available =
      (destack_capi_is_available_fn)dlsym(handle, "destack_capi_is_available");
#endif

  if (capi_abi_version == NULL || capi_version == NULL || capi_is_available == NULL) {
    capi_abi_version = NULL;
    capi_version = NULL;
    capi_is_available = NULL;
  }
}

// run once when the nif loads
static int nif_load(ErlNifEnv *env, void **priv_data, ERL_NIF_TERM load_info) {
  load_capi_symbols();
  return 0;
}

// return capi abi version
static ERL_NIF_TERM nif_capi_abi_version(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
  if (capi_abi_version == NULL) {
    return enif_make_uint(env, 0);
  }

  return enif_make_uint(env, capi_abi_version());
}

// return capi availability
static ERL_NIF_TERM nif_capi_is_available(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
  if (capi_is_available == NULL) {
    return enif_make_atom(env, "false");
  }

  return capi_is_available() ? enif_make_atom(env, "true") : enif_make_atom(env, "false");
}

// return capi version
static ERL_NIF_TERM nif_capi_version(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
  if (capi_version == NULL) {
    return enif_make_string(env, "0.55.3", ERL_NIF_LATIN1);
  }

  const char *value = capi_version();
  if (value == NULL || value[0] == '\0') {
    return enif_make_string(env, "0.55.3", ERL_NIF_LATIN1);
  }

  return enif_make_string(env, value, ERL_NIF_LATIN1);
}

static ErlNifFunc nif_functions[] = {
    {"capi_abi_version", 0, nif_capi_abi_version},
    {"capi_is_available", 0, nif_capi_is_available},
    {"capi_version", 0, nif_capi_version},
};

ERL_NIF_INIT(Elixir.Destack.Native, nif_functions, nif_load, NULL, NULL, NULL)
