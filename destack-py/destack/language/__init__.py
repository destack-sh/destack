import time

start = time.time()

from .access import *  # noqa: F403
from .animation import *  # noqa: F403
from .core import *  # noqa: F403
from .data import *  # noqa: F403
from .deployment import *  # noqa: F403
from .finalize import finalize
from .finance import *  # noqa: F403
from .geometry import *  # noqa: F403
from .infrastructure import *  # noqa: F403
from .intelligence import *  # noqa: F403
from .interaction import *  # noqa: F403
from .logic import *  # noqa: F403
from .observability import *  # noqa: F403
from .registry import (
    ENUM_CLASS_BY_TYPE,  # noqa: F401
    ENUM_TYPE_BY_CLASS,  # noqa: F401
    NODE_CLASS_BY_TYPE,  # noqa: F401
    STRUCT_CLASS_BY_TYPE,  # noqa: F401
)
from .scene import *  # noqa: F403
from .social import *  # noqa: F403
from .space import *  # noqa: F403
from .style import *  # noqa: F403
from .universe import *  # noqa: F403
from .view import *  # noqa: F403

finalize()

from destack.language.core.builtin.object import _time_spent_in_process_object_cls
from destack.utils.code import _time_spent_in_exec

print(f"time spent in exec: {_time_spent_in_exec:.3f}s")
print(f"time spent in process_object_cls: {_time_spent_in_process_object_cls:.3f}s")
print(f"language took {time.time() - start:.3f}s")
