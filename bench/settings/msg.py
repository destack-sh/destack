import os

from bench.utils.utils import get_from_env, str_to_bool

SEND_API_PUB_MSG = str_to_bool(os.environ.get("SEND_API_PUB_MSG", "t"))
NATS_SERVER = get_from_env("NATS_SERVER", "nats://localhost:4222", type_cast=str)
