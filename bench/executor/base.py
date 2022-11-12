from __future__ import annotations

import uuid
from typing import Any

from bench.models.instruction import Instruction


class Executor:
    def __init__(self):
        self.executor_id = uuid.uuid4().hex

    async def run(self, instruction: Instruction, arguments: dict[str, Any]):
        raise NotImplementedError
