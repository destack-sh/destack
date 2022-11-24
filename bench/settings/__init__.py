"""
Settings for bench backend.
Settings for functional or logical submodules are distributed across individual files.
See https://docs.djangoproject.com/en/4,0/ref/settings/
"""
# isort: skip_file

import django_stubs_ext  # noqa: F402

# Monkeypatching Django to make stubs will work for generics
# see: https://github.com/typeddjango/django-stubs
django_stubs_ext.monkeypatch()

from bench.settings.base import *  # noqa: F401,E402,F403
from bench.settings.access import *  # noqa: F401,E402,F403
from bench.settings.databases import *  # noqa: F401,E402,F403
from bench.settings.web import *  # noqa: F401,E402,F403
from bench.settings.logging import *  # noqa: F401,E402,F403
from bench.settings.executor import *  # noqa: F401,E402,F403
