import os

import dj_database_url
import structlog
from django.core.exceptions import ImproperlyConfigured

from bench.settings import get_from_env
from bench.settings.base import BASE_DIR, DEBUG, TEST

logger = structlog.stdlib.get_logger()

# Django Database settings
# https://docs.djangoproject.com/en/4.0/ref/settings/#databases

if TEST or DEBUG:
    PG_HOST = os.getenv("PGHOST", "localhost")
    PG_USER = os.getenv("PGUSER", "bench")
    PG_PASSWORD = os.getenv("PGPASSWORD", "bench")
    PG_PORT = os.getenv("PGPORT", "5432")
    PG_DATABASE = os.getenv("PGDATABASE", "bench")
    DATABASE_URL = os.getenv(
        "DATABASE_URL", f"postgres://{PG_USER}:{PG_PASSWORD}@{PG_HOST}:{PG_PORT}/{PG_DATABASE}"
    )
else:
    DATABASE_URL = os.getenv("DATABASE_URL", "")

if DATABASE_URL:
    DATABASES = {"default": dj_database_url.config(default=DATABASE_URL, conn_max_age=600)}
elif os.getenv("BENCH_DB_NAME"):
    DATABASES = {
        "default": {
            "ENGINE": "django.db.backends.postgresql",
            "NAME": get_from_env("BENCH_DB_NAME"),
            "USER": os.getenv("BENCH_DB_USER", "postgres"),
            "PASSWORD": os.getenv("BENCH_DB_PASSWORD", ""),
            "HOST": os.getenv("BENCH_POSTGRES_HOST", "localhost"),
            "PORT": os.getenv("BENCH_POSTGRES_PORT", "5432"),
            "CONN_MAX_AGE": 0,
        }
    }

    DATABASE_URL = "postgres://{}{}{}{}:{}/{}".format(
        DATABASES["default"]["USER"],
        ":" + DATABASES["default"]["PASSWORD"] if DATABASES["default"]["PASSWORD"] else "",
        "@" if DATABASES["default"]["USER"] or DATABASES["default"]["PASSWORD"] else "",
        DATABASES["default"]["HOST"],
        DATABASES["default"]["PORT"],
        DATABASES["default"]["NAME"],
    )
elif TEST:
    DATABASES = {
        "default": {
            "ENGINE": "django.db.backends.sqlite3",
            "NAME": os.path.join(BASE_DIR, "db.sqlite3"),
        }
    }
    logger.warning("incompatible_database", databases=DATABASES, reason="test_fallback")
else:
    raise ImproperlyConfigured(
        "A Postgres-compatible database must be configured via 'DATABASE_URL' or 'BENCH_DB_NAME'"
    )
