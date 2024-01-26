import os

import dj_database_url
import structlog
from django.core.exceptions import ImproperlyConfigured

from bench.settings import get_from_env
from bench.settings.base import BASE_DIR, IS_DEBUG, ENVIRONMENT, IS_TEST
from bench.utils.utils import SOME_TYPE_CHECKING

logger = structlog.stdlib.get_logger(__name__)

# Django Database settings
# https://docs.djangobench.com/en/4.0/ref/settings/#databases

if IS_TEST or IS_DEBUG:
    HOST = os.getenv("GLOBAL_PG_HOST", "localhost")
    USER = os.getenv("GLOBAL_PG_USERNAME", "bench")
    PASSWORD = os.getenv("GLOBAL_PG_PASSWORD", "bench")
    PORT = os.getenv("GLOBAL_PG_PORT", "5432")
    DATABASE = os.getenv("GLOBAL_PG_NAME", "bench")
    GLOBAL_PG_URL = os.getenv(
        "GLOBAL_PG_URL", f"postgres://{USER}:{PASSWORD}@{HOST}:{PORT}/{DATABASE}"
    )
else:
    GLOBAL_PG_URL = os.getenv("GLOBAL_PG_URL", "")

if GLOBAL_PG_URL:
    DATABASES = {"default": dj_database_url.config(default=GLOBAL_PG_URL, conn_max_age=600)}
elif os.getenv("GLOBAL_PG_NAME"):
    DATABASES = {
        "default": {
            "ENGINE": "django.db.backends.postgresql",
            "NAME": get_from_env("GLOBAL_PG_NAME"),
            "USER": os.getenv("GLOBAL_PG_USERNAME", "postgres"),
            "PASSWORD": os.getenv("GLOBAL_PG_PASSWORD", ""),
            "HOST": os.getenv("GLOBAL_PG_HOST", "localhost"),
            "PORT": os.getenv("GLOBAL_PG_PORT", "5432"),
            "CONN_MAX_AGE": 0,
        }
    }

    GLOBAL_PG_URL = "postgres://{}{}{}{}:{}/{}".format(
        DATABASES["default"]["USER"],
        ":" + DATABASES["default"]["PASSWORD"] if DATABASES["default"]["PASSWORD"] else "",
        "@" if DATABASES["default"]["USER"] or DATABASES["default"]["PASSWORD"] else "",
        DATABASES["default"]["HOST"],
        DATABASES["default"]["PORT"],
        DATABASES["default"]["NAME"],
    )
elif IS_TEST:
    DATABASES = {
        "default": {
            "ENGINE": "django.db.backends.sqlite3",
            "NAME": os.path.join(BASE_DIR, "db.sqlite3"),
        }
    }
    logger.warning("incompatible_database", databases=DATABASES, reason="test_fallback")
elif SOME_TYPE_CHECKING:
    DATABASES = {"default": {}}
else:
    raise ImproperlyConfigured("A Postgres-compatible database must be configured")

# S3

GLOBAL_PROJECT_BUCKET_NAME = f"bench-user-{ENVIRONMENT}"
