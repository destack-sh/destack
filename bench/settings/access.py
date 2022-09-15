from typing import List

from bench.settings import get_from_env

API_PREFIX = "api/"

ALLOWED_HOSTS: List[str] = []

# SECURITY WARNING: keep the secret key used in production secret!
DEFAULT_SECRET_KEY = "<default insecure secret key>"
SECRET_KEY = get_from_env("SECRET_KEY", default=DEFAULT_SECRET_KEY)

CORS_ORIGIN_ALLOW_ALL = True
