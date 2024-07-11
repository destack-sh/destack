from dataclasses import dataclass
from typing import TYPE_CHECKING

from bench.language.node import BuiltinObject

if TYPE_CHECKING:
    pass


@dataclass(slots=True)
class Projection:
    """
    A projection into the Bench graph.
    Should be a proper Struct at some point (so we can inspect it in the editor).
    """

    ...


def project(*objs: BuiltinObject) -> Projection:
    raise NotImplementedError
