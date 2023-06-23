import os
import sys

from bench.utils.utils import get_from_env, str_to_bool

BASE_DIR = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

ENVIRONMENT = os.environ.get("ENVIRONMENT", "local")
DEBUG: bool = get_from_env("DEBUG", False, type_cast=str_to_bool)
TEST: bool = (
    "test" in sys.argv
    or "pytest" in sys.argv[0]
    or get_from_env("TEST", False, type_cast=str_to_bool)
)
LOCAL = os.environ.get("LOCAL_ENV", "local") == "local"

PROMETHEUS_EXPORT_MIGRATIONS: bool = get_from_env(
    "PROMETHEUS_EXPORT_MIGRATIONS", False, type_cast=str_to_bool
)

RUN_LANGSERVER = get_from_env("RUN_LANGSERVER", DEBUG, type_cast=str_to_bool)
RUN_WORKER = get_from_env("RUN_WORKER", DEBUG, type_cast=str_to_bool)
