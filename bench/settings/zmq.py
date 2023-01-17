from django.core.exceptions import ImproperlyConfigured

from bench.settings import DEBUG, RUN_RUNTIME_WORKER, TEST, get_from_env

# require ZMQ addresses if not running everything locally for debugging/testing
local_zmq = DEBUG or TEST
ZMQ_API_SERVER_ADDR = get_from_env("ZMQ_API_SERVER_ADDR", None, type_cast=str, optional=local_zmq)
ZMQ_INTERNAL_SERVER_ADDR = get_from_env(
    "ZMQ_INTERNAL_SERVER_ADDR", None, type_cast=str, optional=local_zmq
)
ZMQ_RUNTIME_WORKER_ADDR = get_from_env(
    "ZMQ_RUNTIME_WORKER_ADDR", None, type_cast=str, optional=local_zmq
)
if local_zmq:
    # set default zmq IPC addresses
    if ZMQ_API_SERVER_ADDR is None:
        ZMQ_API_SERVER_ADDR = "ipc:///tmp/feeds_bench_api_server"
    if ZMQ_INTERNAL_SERVER_ADDR is None:
        ZMQ_INTERNAL_SERVER_ADDR = "ipc:///tmp/feeds_bench_internal_server"
    if ZMQ_RUNTIME_WORKER_ADDR is None:
        ZMQ_RUNTIME_WORKER_ADDR = "ipc:///tmp/feeds_bench_runtime_worker"

ZMQ_API_SERVER_PORT = get_from_env("ZMQ_API_SERVER_PORT", 5555, type_cast=int)
ZMQ_INTERNAL_SERVER_PORT = get_from_env("ZMQ_INTERNAL_SERVER_PORT", 5556, type_cast=int)
ZMQ_RUNTIME_WORKER_PORT = get_from_env("ZMQ_RUNTIME_WORKER_PORT", 5557, type_cast=int)

if not (DEBUG or TEST) and RUN_RUNTIME_WORKER:
    raise ImproperlyConfigured("RUN_RUNTIME_WORKER can only be set in DEBUG mode for sandboxing")
