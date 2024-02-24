from typing import TYPE_CHECKING, Literal

import structlog

from bench.language.node import Node, node_component

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)


@node_component
class HasTask(Node):
    @property
    def _is_async(self):
        return True

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
