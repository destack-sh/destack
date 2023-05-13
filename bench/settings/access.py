import os

from bench.settings import get_from_env
from bench.utils.utils import get_list

ALLOWED_HOSTS: list[str] = get_list(os.getenv("ALLOWED_HOSTS", "*"))

# SECURITY WARNING: keep the secret key used in production secret!
DEFAULT_SECRET_KEY = "<default insecure secret key>"
SECRET_KEY = get_from_env("SECRET_KEY", default=DEFAULT_SECRET_KEY)

CORS_ALLOWED_ORIGINS: list[str] = get_list(os.getenv("CORS_ALLOWED_ORIGINS"))

ACCESS_TOKEN_PREFIX = get_from_env("ACCESS_TOKEN_PREFIX", default="x-")
ACCESS_TOKEN_DIGEST_LENGTH = int(get_from_env("ACCESS_TOKEN_DIGEST_LENGTH", default=128))
ACCESS_TOKEN_KEY_LENGTH = int(get_from_env("ACCESS_TOKEN_KEY_LENGTH", default=6))

ENCRYPT_BENCH_S3_BUCKETS = get_from_env("ENCRYPT_BENCH_S3_BUCKETS", default=False, type_cast=bool)
