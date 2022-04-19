"""
Settings for bench backend.
Settings for functional or logical submodules are distributed across individual files.
See https://docs.djangoproject.com/en/4,0/ref/settings/
"""
# isort: skip_file

from bench.settings.base import *  # noqa: F401,F403
from bench.settings.access import *  # noqa: F401,F403
from bench.settings.databases import *  # noqa: F401,F403
from bench.settings.web import *  # noqa: F401,F403
from bench.settings.logging import *  # noqa: F401,F403
from bench.settings.executor import *  # noqa: F401,F403
