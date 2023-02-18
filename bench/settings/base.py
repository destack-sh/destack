import os
import sys

from bench.settings.utils import get_from_env, str_to_bool

BASE_DIR = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

DEBUG: bool = get_from_env("DEBUG", False, type_cast=str_to_bool)
TEST: bool = (
    "test" in sys.argv
    or "pytest" in sys.argv[0]
    or get_from_env("TEST", False, type_cast=str_to_bool)
)

PROMETHEUS_EXPORT_MIGRATIONS: bool = get_from_env(
    "PROMETHEUS_EXPORT_MIGRATIONS", False, type_cast=str_to_bool
)

RUN_INTSERVER = get_from_env("RUN_INTSERVER", DEBUG, type_cast=str_to_bool)
RUN_worker = get_from_env("RUN_worker", DEBUG, type_cast=str_to_bool)
