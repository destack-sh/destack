# ruff: noqa: E402

import warnings
from collections.abc import Mapping

import structlog
from opentelemetry import trace

from destack.test.conftest import _setup_test_env

# NOTE: must run setup before importing from destack
_setup_test_env()


from destack.language import (
    ACTIVE_SESSION,
    NODE_TYPES,
    STRUCT_TYPES,
    BuiltinObjectBase,
    EnvironmentType,
    NodeType,
    Session,
    StructType,
)
from destack.test.conftest import _setup_test_env
from destack.utils.oracle import REAL_ORACLE

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


SHARED_SESSION = Session(mode=EnvironmentType.STAGING, oracle=REAL_ORACLE)


# init shared builtin objects (in shared session)
with warnings.catch_warnings(action="ignore"):
    ACTIVE_SESSION.set(SHARED_SESSION)
    BUILTIN_OBJECTS = [
        # draw_direct(from_object_type(object_type, reject_invalid=False))
        # for object_type in OBJECT_TYPES
    ]
    ACTIVE_SESSION.set(None)

BUILTIN_OBJECTS_BY_TYPE: Mapping[StructType | NodeType, BuiltinObjectBase] = {
    obj.metatype: obj for obj in BUILTIN_OBJECTS
}
STRUCTS = [BUILTIN_OBJECTS_BY_TYPE[t] for t in STRUCT_TYPES if t in BUILTIN_OBJECTS_BY_TYPE]
NODES = [BUILTIN_OBJECTS_BY_TYPE[t] for t in NODE_TYPES if t in BUILTIN_OBJECTS_BY_TYPE]


# TODO :Test! :Performance: re-use Simulations somehow (databases?)
