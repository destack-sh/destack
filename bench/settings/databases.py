import os

import dj_database_url
from django.core.exceptions import ImproperlyConfigured

from bench.settings import get_from_env
from bench.settings.base import DEBUG, TEST

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
            "SSL_OPTIONS": {
                "sslmode": os.getenv("BENCH_POSTGRES_SSL_MODE", None),
                "sslrootcert": os.getenv("BENCH_POSTGRES_CLI_SSL_CA", None),
                "sslcert": os.getenv("BENCH_POSTGRES_CLI_SSL_CRT", None),
                "sslkey": os.getenv("BENCH_POSTGRES_CLI_SSL_KEY", None),
            },
        }
    }

    # borrowed from posthog/posthog/posthog/settings/data_stores.py
    ssl_configurations = []
    for ssl_option, value in DATABASES["default"]["SSL_OPTIONS"].items():
        if value:
            ssl_configurations.append("{}={}".format(ssl_option, value))

    if ssl_configurations:
        ssl_configuration = "?{}".format("&".join(ssl_configurations))
    else:
        ssl_configuration = ""

    DATABASE_URL = "postgres://{}{}{}{}:{}/{}{}".format(
        DATABASES["default"]["USER"],
        ":" + DATABASES["default"]["PASSWORD"] if DATABASES["default"]["PASSWORD"] else "",
        "@" if DATABASES["default"]["USER"] or DATABASES["default"]["PASSWORD"] else "",
        DATABASES["default"]["HOST"],
        DATABASES["default"]["PORT"],
        DATABASES["default"]["NAME"],
        ssl_configuration,
    )

else:
    raise ImproperlyConfigured(
        "A postgres-like database must be configured via 'DATABASE_URL' or 'BENCH_DB_NAME'"
    )
