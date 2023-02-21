import os

from bench.settings import get_from_env
from bench.utils.utils import str_to_bool

SEND_API_PUB_MSG = str_to_bool(os.environ.get("SEND_API_PUB_MSG", "t"))

# require ZMQ addresses if not running everything locally for debugging/testing
ZMQ_API_PUB_ADDR = get_from_env("ZMQ_API_PUB_ADDR", None, type_cast=str)
ZMQ_INTSERVER_REP_ADDR = get_from_env("ZMQ_INTSERVER_REP_ADDR", None, type_cast=str)
ZMQ_INTSERVER_PUB_ADDR = get_from_env("ZMQ_INTSERVER_PUB_ADDR", None, type_cast=str)
ZMQ_WORKER_REP_ADDR = get_from_env("ZMQ_WORKER_REP_ADDR", None, type_cast=str)
ZMQ_WORKER_PUB_ADDR = get_from_env("ZMQ_WORKER_PUB_ADDR", None, type_cast=str)
