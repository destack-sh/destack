# ruff: noqa: F405

from destack.utils.oracle import REAL_ORACLE as REAL_ORACLE

from .access import *  # noqa: F403
from .canvas import *  # noqa: F403
from .core import *  # noqa: F403
from .data import *  # noqa: F403
from .deployment import *  # noqa: F403
from .entity import *  # noqa: F403
from .finance import *  # noqa: F403
from .folder import *  # noqa: F403
from .infra import *  # noqa: F403
from .intelligence import *  # noqa: F403
from .interactive import *  # noqa: F403
from .logic import *  # noqa: F403
from .registry import (
    ENUM_CLASS_BY_TYPE,  # noqa: F401
    ENUM_INFO_BY_TYPE,  # noqa: F401
    ENUM_TYPE_BY_CLASS,  # noqa: F401
    NODE_CLASS_BY_TYPE,  # noqa: F401
    NODE_INFO_BY_TYPE,  # noqa: F401
    STRUCT_CLASS_BY_TYPE,  # noqa: F401
    STRUCT_INFO_BY_TYPE,  # noqa: F401
    _complete_destack_setup,
)
from .runtime import *  # noqa: F403
from .scene import *  # noqa: F403
from .social import *  # noqa: F403
from .space import *  # noqa: F403
from .style import *  # noqa: F403
from .view import *  # noqa: F403

# after all the imports, we can finalize
_complete_destack_setup()

# builtin destackes (pointers) :Builtins
DESTACK_PTR = NodeReference(node_type=NodeType.SPACE, id=DESTACK_ID, space_id=DESTACK_ID)
DESTACK_ICON = icon("https://destack.com/favicon.ico")
