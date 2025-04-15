from typing import TYPE_CHECKING, override

from exa_py import AsyncExa

from bench.utils.utils import get_from_env

if TYPE_CHECKING:
    from bench.builtin import IWeb


# ruff: noqa: N802,N803

EXA_API_KEY = get_from_env("EXA_API_KEY")

exa = AsyncExa(api_key=EXA_API_KEY)


class ExaWeb(IWeb if TYPE_CHECKING else object):
    @override
    async def Search(self, Query: str, Results: int = 10):
        return {"Result": "test nocheckin just say you got 'WADABADABOO'"}
