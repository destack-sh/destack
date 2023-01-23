from django.core.exceptions import ImproperlyConfigured

from bench.settings import DEBUG, RUN_RUNTIME_WORKER, TEST, get_from_env

# require ZMQ addresses if not running everything locally for debugging/testing
local_zmq = DEBUG or TEST
ZMQ_API_SERVER_PUB_ADDR = get_from_env(
    "ZMQ_API_SERVER_PUB_ADDR", None, type_cast=str, optional=local_zmq
)
ZMQ_INTERNAL_SERVER_REP_ADDR = get_from_env(
    "ZMQ_INTERNAL_SERVER_ADDR", None, type_cast=str, optional=local_zmq
)
ZMQ_INTERNAL_SERVER_PUB_ADDR = get_from_env(
    "ZMQ_INTERNAL_SERVER_PUB_ADDR", None, type_cast=str, optional=local_zmq
)
ZMQ_RUNTIME_WORKER_REP_ADDR = get_from_env(
    "ZMQ_RUNTIME_WORKER_REP_ADDR", None, type_cast=str, optional=local_zmq
)
ZMQ_RUNTIME_WORKER_PUB_ADDR = get_from_env(
    "ZMQ_RUNTIME_WORKER_PUB_ADDR", None, type_cast=str, optional=local_zmq
)
if local_zmq:
    # set default zmq IPC addresses
    if ZMQ_API_SERVER_PUB_ADDR is None:
        ZMQ_API_SERVER_PUB_ADDR = "ipc:///tmp/feeds_bench_api_server_pub"
    if ZMQ_INTERNAL_SERVER_REP_ADDR is None:
        ZMQ_INTERNAL_SERVER_REP_ADDR = "ipc:///tmp/feeds_bench_internal_server_rep"
    if ZMQ_INTERNAL_SERVER_PUB_ADDR is None:
        ZMQ_INTERNAL_SERVER_PUB_ADDR = "ipc:///tmp/feeds_bench_internal_server_pub"
    if ZMQ_RUNTIME_WORKER_REP_ADDR is None:
        ZMQ_RUNTIME_WORKER_REP_ADDR = "ipc:///tmp/feeds_bench_runtime_worker_rep"
    if ZMQ_RUNTIME_WORKER_PUB_ADDR is None:
        ZMQ_RUNTIME_WORKER_PUB_ADDR = "ipc:///tmp/feeds_bench_runtime_worker_pub"

if not (DEBUG or TEST) and RUN_RUNTIME_WORKER:
    raise ImproperlyConfigured("RUN_RUNTIME_WORKER can only be set in DEBUG mode for sandboxing")
