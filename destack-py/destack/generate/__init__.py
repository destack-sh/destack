from collections.abc import Sequence
from enum import StrEnum

from .core import *  # noqa: F403


class GenerationScope(StrEnum):
    PROTO = "proto"
    LANGUAGE = "language"


def generate(scopes: Sequence[GenerationScope] = (GenerationScope.PROTO, GenerationScope.LANGUAGE)):
    """Generate all the derived things."""
    from .proto import generate_proto
    from .python import (
        generate_language as generate_python_language,
    )
    from .python import (
        generate_proto as generate_python_proto,
    )
    from .typescript import (
        generate_language as generate_typescript_language,
    )
    from .typescript import (
        generate_proto as generate_typescript_proto,
    )

    # proto
    if GenerationScope.PROTO in scopes:
        generate_proto()
        generate_python_proto()
        generate_typescript_proto()

    # language
    if GenerationScope.LANGUAGE in scopes:
        generate_python_language()
        generate_typescript_language()
