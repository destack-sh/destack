import time

# ruff: noqa: F401, F403, E402

start = time.time()


from .access import *
from .animation import *
from .core import *
from .data import *
from .deployment import *
from .finalize import finalize
from .finance import *
from .geometry import *
from .infrastructure import *
from .intelligence import *
from .interaction import *
from .logic import *
from .observability import *
from .registry import (
    ENUM_CLASS_BY_TYPE,
    ENUM_TYPE_BY_CLASS,
    NODE_CLASS_BY_TYPE,
    STRUCT_CLASS_BY_TYPE,
)
from .scene import *
from .social import *
from .space import *
from .style import *
from .universe import *
from .view import *

finalize()

from destack.utils.code import _time_spent_in_exec

from .core.builtin.object import _time_spent_in_process_object_cls
from .finalize import _finalize_start, logger

assert _finalize_start is not None, "language not finalized"

logger.debug(
    "language.init",
    finalize_duration=f"{(_finalize_start - start) * 1000:.2f}ms" if _finalize_start else None,
    init_duration=f"{(time.time() - start) * 1000:.2f}ms",
    exec_duration=f"{_time_spent_in_exec * 1000:.2f}ms",
    process_object_duration=f"{_time_spent_in_process_object_cls * 1000:.2f}ms",
)
