from typing import Any

from bench.language.field import TypeInfoBase
from bench.language.text import Text


async def run_text(typ: TypeInfoBase, text: Text, context: dict[str, Any]) -> dict[str, Any]:
    raise NotImplementedError("nocheckin: run_text")
