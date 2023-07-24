import os

from bench.utils.utils import DEBUG, ENVIRONMENT, LOCAL, TEST, get_from_env, str_to_bool

BASE_DIR = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

ENVIRONMENT  # noqa re-export
DEBUG  # noqa re-export
TEST  # noqa re-export
LOCAL  # noqa re-export

PROMETHEUS_EXPORT_MIGRATIONS: bool = get_from_env(
    "PROMETHEUS_EXPORT_MIGRATIONS", False, type_cast=str_to_bool
)

RUN_LANGUAGE_SERVER = get_from_env("RUN_LANGUAGE_SERVER", DEBUG, type_cast=str_to_bool)
RUN_ORCHESTRATION_SERVER = get_from_env("RUN_ORCHESTRATION_SERVER", DEBUG, type_cast=str_to_bool)
