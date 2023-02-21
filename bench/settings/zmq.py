from bench.settings import get_from_env

# require ZMQ addresses if not running everything locally for debugging/testing
ZMQ_API_PUB_ADDR = get_from_env("ZMQ_API_PUB_ADDR", None, type_cast=str)
ZMQ_INTSERVER_REP_ADDR = get_from_env("ZMQ_INTSERVER_REP_ADDR", None, type_cast=str)
ZMQ_INTSERVER_PUB_ADDR = get_from_env("ZMQ_INTSERVER_PUB_ADDR", None, type_cast=str)
ZMQ_WORKER_REP_ADDR = get_from_env("ZMQ_WORKER_REP_ADDR", None, type_cast=str)
ZMQ_WORKER_PUB_ADDR = get_from_env("ZMQ_WORKER_PUB_ADDR", None, type_cast=str)
