cmake_minimum_required(VERSION 3.22.1)

project(destack_runtime_host_android)

set(RUNTIME_ANDROID_PROJECT_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/../../..")
set(RUNTIME_REPOSITORY_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/../../../../../../../")
set(RUNTIME_ANDROID_EXPORT_SCRIPT "${RUNTIME_REPOSITORY_ROOT}/dev/toolchain/build-runtime-host-android-exports.sh")
set(RUNTIME_ANDROID_ABI_LIBRARY "${RUNTIME_ANDROID_PROJECT_ROOT}/.build/rust/arm64-v8a/libdestack_runtime_abi.a")

add_custom_target(
    destack_runtime_abi_build
    COMMAND /bin/bash "${RUNTIME_ANDROID_EXPORT_SCRIPT}"
    BYPRODUCTS "${RUNTIME_ANDROID_ABI_LIBRARY}"
    VERBATIM
)

add_library(destack_runtime_abi STATIC IMPORTED GLOBAL)
set_target_properties(
    destack_runtime_abi
    PROPERTIES
    IMPORTED_LOCATION "${RUNTIME_ANDROID_ABI_LIBRARY}"
)

add_library(
    destack_runtime_host_android
    SHARED
    bridge/background/callbacks.generated.cpp
    bridge/background/ingress.generated.cpp
    bridge/background/runtime.generated.cpp
    bridge/calendar/callbacks.generated.cpp
    bridge/calendar/runtime.generated.cpp
    bridge/contact/callbacks.generated.cpp
    bridge/contact/runtime.generated.cpp
    bridge/document/callbacks.generated.cpp
    bridge/document/ingress.generated.cpp
    bridge/document/runtime.generated.cpp
    bridge/intent/callbacks.generated.cpp
    bridge/intent/ingress.generated.cpp
    bridge/intent/runtime.generated.cpp
    bridge/location/callbacks.generated.cpp
    bridge/location/ingress.generated.cpp
    bridge/location/runtime.generated.cpp
    bridge/jni.cpp
    bridge/media/callbacks.generated.cpp
    bridge/media/runtime.generated.cpp
    bridge/notification/callbacks.generated.cpp
    bridge/notification/ingress.generated.cpp
    bridge/notification/runtime.generated.cpp
    bridge/permission/callbacks.generated.cpp
    bridge/permission/ingress.generated.cpp
    bridge/permission/runtime.generated.cpp
    bridge/registry.cpp
    bridge/text/callbacks.generated.cpp
    bridge/text/ingress.generated.cpp
    bridge/text/runtime.generated.cpp
)

add_dependencies(
    destack_runtime_host_android
    destack_runtime_abi_build
)

find_library(
    log-lib
    log
)

find_library(
    android-lib
    android
)

find_library(
    opensles-lib
    OpenSLES
)

target_link_libraries(
    destack_runtime_host_android
    destack_runtime_abi
    ${android-lib}
    ${log-lib}
    ${opensles-lib}
)
