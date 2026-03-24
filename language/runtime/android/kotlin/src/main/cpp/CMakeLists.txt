cmake_minimum_required(VERSION 3.22.1)

project(destack_runtime_host_android)

add_library(
    destack_runtime_host_android
    SHARED
    bridge/document/runtime.cpp
    bridge/document/methods.cpp
    bridge/calendar/methods.cpp
    bridge/contact/methods.cpp
    bridge/intent/runtime.cpp
    bridge/intent/methods.cpp
    bridge/location/runtime.cpp
    bridge/location/methods.cpp
    bridge/jni.cpp
    bridge/loader.cpp
    bridge/media/methods.cpp
    bridge/notification/methods.cpp
    bridge/notification/runtime.cpp
    bridge/permission/runtime.cpp
    bridge/permission/methods.cpp
    bridge/document/bridge.cpp
    bridge/intent/bridge.cpp
    bridge/location/bridge.cpp
    bridge/notification/bridge.cpp
    bridge/permission/bridge.cpp
    bridge/registry.cpp
)

add_library(
    destack_runtime_host_android_testing
    SHARED
    bridge/test/bytes.cpp
    bridge/test/document.cpp
    bridge/test/intent.cpp
    bridge/test/location.cpp
    bridge/test/loader.cpp
    bridge/test/notification.cpp
    bridge/test/permission.cpp
    bridge/test/session.cpp
)

find_library(
    log-lib
    log
)

target_link_libraries(
    destack_runtime_host_android
    ${log-lib}
)

target_link_libraries(
    destack_runtime_host_android_testing
    ${log-lib}
)
