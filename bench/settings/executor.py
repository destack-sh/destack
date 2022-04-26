from bench.settings.utils import get_from_env

EXECUTOR: str = get_from_env("EXECUTOR", default="local")
MODEL_IID_HASH_LENGTH: int = get_from_env(
    "EXECUTOR_MODEL_IID_HASH_LENGTH", default=8, type_cast=int
)
