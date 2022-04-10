import os
import sys

from bench.settings.utils import get_from_env, str_to_bool

BASE_DIR = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

DEBUG: bool = get_from_env("DEBUG", False, type_cast=str_to_bool)
TEST: bool = (
    "test" in sys.argv
    or sys.argv[0].endswith("pytest")
    or get_from_env("TEST", False, type_cast=str_to_bool)
)
