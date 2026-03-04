#include <ruby.h>

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

// load capi symbols only once
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

// return capi abi version
static VALUE rb_destack_native_capi_abi_version(VALUE self) {
  load_capi_symbols();
  if (capi_abi_version == NULL) {
    return UINT2NUM(0);
  }

  return UINT2NUM(capi_abi_version());
}

// return capi availability
static VALUE rb_destack_native_capi_is_available(VALUE self) {
  load_capi_symbols();
  if (capi_is_available == NULL) {
    return Qfalse;
  }

  return capi_is_available() ? Qtrue : Qfalse;
}

// return capi version string
static VALUE rb_destack_native_capi_version(VALUE self) {
  load_capi_symbols();
  if (capi_version == NULL) {
    return rb_utf8_str_new_cstr("0.55.4");
  }

  const char *value = capi_version();
  if (value == NULL || value[0] == '\0') {
    return rb_utf8_str_new_cstr("0.55.4");
  }

  return rb_utf8_str_new_cstr(value);
}

void Init_destack_ext(void) {
  VALUE destack_module = rb_define_module("Destack");
  VALUE native_module = rb_define_module_under(destack_module, "Native");

  rb_define_singleton_method(native_module, "capi_abi_version",
                             rb_destack_native_capi_abi_version, 0);
  rb_define_singleton_method(native_module, "capi_is_available",
                             rb_destack_native_capi_is_available, 0);
  rb_define_singleton_method(native_module, "capi_version", rb_destack_native_capi_version, 0);
}
