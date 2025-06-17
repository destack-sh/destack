from .core import *  # noqa: F403


def generate():
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
    generate_proto()
    generate_python_proto()
    generate_typescript_proto()

    # language
    generate_python_language()
    generate_typescript_language()
