from bench.utils.utils import get_from_env

NATS_SERVER = get_from_env("NATS_SERVER", "nats://localhost:4222", type_cast=str)
