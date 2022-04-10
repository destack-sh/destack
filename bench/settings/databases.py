# Django Database settings
# https://docs.djangoproject.com/en/3.1/ref/settings/#databases
import os

from bench.settings import BASE_DIR

DATABASES = {
    "default": {
        "ENGINE": "django.db.backends.sqlite3",
        "NAME": os.path.join(BASE_DIR, "db.sqlite3"),
    }
}
