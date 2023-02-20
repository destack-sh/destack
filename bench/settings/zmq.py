from django.core.exceptions import ImproperlyConfigured

from bench.settings import DEBUG, RUN_WORKER, TEST, get_from_env

# require ZMQ addresses if not running everything locally for debugging/testing
local_zmq = DEBUG or TEST
ZMQ_API_PUB_ADDR = get_from_env("ZMQ_API_PUB_ADDR", None, type_cast=str, optional=local_zmq)
ZMQ_INTSERVER_REP_ADDR = get_from_env("ZMQ_INTSERVER_ADDR", None, type_cast=str, optional=local_zmq)
ZMQ_INTSERVER_PUB_ADDR = get_from_env(
    "ZMQ_INTSERVER_PUB_ADDR", None, type_cast=str, optional=local_zmq
)
ZMQ_worker_REP_ADDR = get_from_env("ZMQ_WORKER_PUB_ADDR", None, type_cast=str, optional=local_zmq)
ZMQ_worker_PUB_ADDR = get_from_env("ZMQ_WORKER_PUB_ADDR", None, type_cast=str, optional=local_zmq)
if local_zmq:
    # set default zmq IPC addresses
    if ZMQ_API_PUB_ADDR is None:
        ZMQ_API_PUB_ADDR = "ipc:///tmp/feeds_bench_api_pub"
    if ZMQ_INTSERVER_REP_ADDR is None:
        ZMQ_INTSERVER_REP_ADDR = "ipc:///tmp/feeds_bench_intserver_rep"
    if ZMQ_INTSERVER_PUB_ADDR is None:
        ZMQ_INTSERVER_PUB_ADDR = "ipc:///tmp/feeds_bench_intserver_pub"
    if ZMQ_worker_REP_ADDR is None:
        ZMQ_worker_REP_ADDR = "ipc:///tmp/feeds_bench_worker_rep"
    if ZMQ_worker_PUB_ADDR is None:
        ZMQ_worker_PUB_ADDR = "ipc:///tmp/feeds_bench_worker_pub"

if not (DEBUG or TEST) and RUN_WORKER:
    raise ImproperlyConfigured("RUN_worker can only be set in DEBUG mode for sandboxing")
