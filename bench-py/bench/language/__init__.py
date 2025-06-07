# ruff: noqa: F405

from bench.utils.oracle import REAL_ORACLE as REAL_ORACLE

from .auth import *  # noqa: F403
from .core import *  # noqa: F403
from .data import *  # noqa: F403
from .finance import *  # noqa: F403
from .infra import *  # noqa: F403
from .logic import *  # noqa: F403
from .registry import (
    ENUM_CLASS_BY_TYPE,  # noqa: F401
    ENUM_INFO_BY_TYPE,  # noqa: F401
    ENUM_TYPE_BY_CLASS,  # noqa: F401
    NODE_CLASS_BY_TYPE,  # noqa: F401
    NODE_INFO_BY_TYPE,  # noqa: F401
    STRUCT_CLASS_BY_TYPE,  # noqa: F401
    STRUCT_INFO_BY_TYPE,  # noqa: F401
    _complete_bench_setup,
)
from .runtime import *  # noqa: F403
from .scene import *  # noqa: F403
from .social import *  # noqa: F403
from .space import *  # noqa: F403
from .style import *  # noqa: F403
from .view import *  # noqa: F403

# after all the imports, we can finalize
_complete_bench_setup()

# builtin benches (pointers) :Builtins
BENCH_PTR = NodeReference(node_type=NodeType.SPACE, id=BENCH_ID, space_id=BENCH_ID)
BENCH_BENCH_PACKAGE_PTR = NodeReference(
    node_type=NodeType.PACKAGE, id=BENCH_BENCH_PACKAGE_ID, space_id=BENCH_ID
)
SYSTEM_BENCH_PTR = NodeReference(node_type=NodeType.SPACE, id=SYSTEM_ID, space_id=SYSTEM_ID)
SYSTEM_MAIN_PACKAGE_PTR = NodeReference(
    node_type=NodeType.PACKAGE, id=SYSTEM_SYSTEM_PACKAGE_ID, space_id=SYSTEM_ID
)

BENCH_ICON = icon("https://heybench.com/favicon.ico")
