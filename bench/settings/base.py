import os

from bench.utils.utils import IS_DEBUG, ENVIRONMENT, IS_LOCAL, IS_TEST, get_from_env, str_to_bool

BASE_DIR = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

ENVIRONMENT  # noqa re-export
IS_DEBUG  # noqa re-export
IS_TEST  # noqa re-export
IS_LOCAL  # noqa re-export

PROMETHEUS_EXPORT_MIGRATIONS: bool = get_from_env(
    "PROMETHEUS_EXPORT_MIGRATIONS", False, type_cast=str_to_bool
)

RUN_LANGUAGE_SERVER_IN_API = get_from_env("RUN_LANGUAGE_SERVER", IS_DEBUG, type_cast=str_to_bool)
RUN_ORCHESTRATION_SERVER_IN_API = get_from_env(
    "RUN_ORCHESTRATION_SERVER", IS_DEBUG, type_cast=str_to_bool
)
if not IS_DEBUG and (RUN_LANGUAGE_SERVER_IN_API or RUN_ORCHESTRATION_SERVER_IN_API):
    raise RuntimeError(
        "RUN_LANGUAGE_SERVER and RUN_ORCHESTRATION_SERVER can only be enabled in DEBUG mode"
    )
