# ruff: noqa: F405

from .chat import *  # noqa: F403
from .connection import *  # noqa: F403
from .core import *  # noqa: F403
from .cosmos import *  # noqa: F403
from .data import *  # noqa: F403
from .finance import *  # noqa: F403
from .identity import *  # noqa: F403
from .infra import *  # noqa: F403
from .logic import *  # noqa: F403
from .package import *  # noqa: F403
from .registry import (
    ANCESTOR_NODE_TYPES,  # noqa: F401
    BENCH_CLASS_BY_NAME,  # noqa: F401
    BENCH_CLASS_BY_TYPE,  # noqa: F401
    BUILTIN_OBJECT_CLASS_BY_TYPE,  # noqa: F401
    BUILTIN_OBJECT_TYPE_BY_CLASS,  # noqa: F401
    CHILD_NODE_TYPES,  # noqa: F401
    DESCENDANT_NODE_TYPES,  # noqa: F401
    DESCENDANT_NODE_TYPES_IN_STORE,  # noqa: F401
    ENUM_CLASS_BY_TYPE,  # noqa: F401
    ENUM_TYPE_BY_CLASS,  # noqa: F401
    FINAL_BENCH_CLASSES,  # noqa: F401
    HAS_CHILD_NODE_TYPES,  # noqa: F401
    NODE_CLASS_BY_NAME,  # noqa: F401
    NODE_CLASS_BY_TYPE,  # noqa: F401
    NODE_CLASSES,  # noqa: F401
    PARENT_NODE_TYPES,  # noqa: F401
    STRUCT_CLASS_BY_TYPE,  # noqa: F401
    STRUCT_CLASSES,  # noqa: F401
    _complete_bench_setup,
)
from .runtime import *  # noqa: F403
from .sync import *  # noqa: F403
from .typing import *  # noqa: F403
from .view import *  # noqa: F403

# after all the imports, we can finalize
_complete_bench_setup()

# requires finalization

# builtin benches (pointers) :Builtins
BENCH_PTR = NodeReference(node_type=NodeType.BENCH, id=BENCH_ID, bench_id=BENCH_ID)
BENCH_BENCH_PACKAGE_PTR = NodeReference(
    node_type=NodeType.PACKAGE, id=BENCH_BENCH_PACKAGE_ID, bench_id=BENCH_ID
)
SYSTEM_BENCH_PTR = NodeReference(node_type=NodeType.BENCH, id=SYSTEM_ID, bench_id=SYSTEM_ID)
SYSTEM_MAIN_PACKAGE_PTR = NodeReference(
    node_type=NodeType.PACKAGE, id=SYSTEM_SYSTEM_PACKAGE_ID, bench_id=SYSTEM_ID
)

BENCH_ICON = icon("https://heybench.com/favicon.ico", ColorType.YELLOW)
