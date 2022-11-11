from __future__ import annotations

import uuid
from typing import Any

from bench.models import Flow


class Executor:
    def __init__(self):
        self.executor_id = uuid.uuid4().hex

    async def run_flow(self, flow: Flow, arguments: dict[str, Any]):
        raise NotImplementedError
