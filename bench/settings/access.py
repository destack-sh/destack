import os

from bench.settings import get_from_env
from bench.utils.utils import get_list

ALLOWED_HOSTS: list[str] = get_list(os.getenv("ALLOWED_HOSTS", "*"))

# SECURITY WARNING: keep the secret key used in production secret!
DEFAULT_SECRET_KEY = "<default insecure secret key>"
SECRET_KEY = get_from_env("SECRET_KEY", default=DEFAULT_SECRET_KEY)

CORS_ORIGIN_ALLOW_ALL = True
