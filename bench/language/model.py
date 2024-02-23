from typing import TYPE_CHECKING, Literal, Optional

import structlog

from bench.language.node import Node, node_component

if TYPE_CHECKING:
    from bench.language.notice import NoticeHandler

logger = structlog.get_logger(__name__)


@node_component
class HasTask(Node):
    @property
    def _is_async(self):
        return True

    def _clear_inner(self, scope: Optional[Node] = None) -> None:
        pass

    def _interp_inner(self, scope: Node, on_notice: "NoticeHandler") -> None:
        pass

    async def _call_inner_async(
        self,
        *args,
        # :TaskConfig
        cache: bool = None,
        nonce: str = None,
        mode: Literal["fast", "deliberate", "auto"] = "auto",
        **kwargs,
    ):
        raise NotImplementedError("call task :Incomplete")
