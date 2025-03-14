from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    NodeType,
    Resource,
    StructType,
    enum_,
    node_,
    p_kernel,
    p_regular,
    p_system,
)
from bench.pb2.lang_pb2 import BrowserData

if TYPE_CHECKING:
    from bench.language import Client, Vector2


@enum_(EnumType.BROWSER_TYPE)
class BrowserType(BuiltinEnum):
    CHROME = 1


DEFAULT_BROWSER_WIDTH = 1920.0
DEFAULT_BROWSER_HEIGHT = 1080.0


@node_(NodeType.BROWSER, has_subtypes=True)
class Browser(Resource[BrowserData]):
    """A Browser instance for web browsing."""

    type: BrowserType = p_regular(30)

    version: str | None = p_regular(60, default=None)
    target_version: Optional[str] = p_regular(61, default=None)
    external_name: Optional[str] = p_kernel(62, require=False, default=None, sensitive=True)
    external_id: Optional[str] = p_kernel(63, require=False, default=None, sensitive=True)
    connection_uri: Optional[str] = p_system(
        64, require=False, default=None, encrypt=True, defer=True, sensitive=True
    )
    debugger_uri: Optional[str] = p_system(
        65, require=False, default=None, encrypt=True, defer=True, sensitive=True
    )
    view_uri: Optional[str] = p_system(
        66, require=False, default=None, encrypt=True, defer=True, sensitive=True
    )
    client: Optional["Client"] = p_system(
        67, require=False, array=False, references=NodeType.CLIENT, fk=True, same_bench=True
    )

    # settings
    size: "Vector2 | None" = p_regular(70, require=False, array=False, struct=StructType.VECTOR2)
    is_headless: bool = p_system(75, default=False)
    is_insecure: bool = p_system(76, default=False)

    @staticmethod
    def new(type: BrowserType, name: str, **kwargs) -> "Browser":
        return Browser(type=type, name=name, **kwargs)
