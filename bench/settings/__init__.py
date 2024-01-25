"""
Settings for bench backend.
Settings for functional or logical subpackages are distributed across individual files.
See https://docs.djangobench.com/en/4,0/ref/settings/
"""
# isort: skip_file

from bench.settings.base import *  # noqa: F401,E402,F403
from bench.settings.access import *  # noqa: F401,E402,F403
from bench.settings.databases import *  # noqa: F401,E402,F403
from bench.settings.web import *  # noqa: F401,E402,F403
from bench.settings.logging import *  # noqa: F401,E402,F403
from bench.settings.analytics import *  # noqa: F401,E402,F403
from bench.settings.msg import *  # noqa: F401,E402,F403
from bench.settings.cache import *  # noqa: F401,E402,F403
from bench.settings.k8 import *  # noqa: F401,E402,F403
