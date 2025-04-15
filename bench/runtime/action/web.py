from datetime import datetime
from typing import TYPE_CHECKING, Annotated, Any, Mapping, override

from exa_py import AsyncExa
from exa_py.api import Result as ExaResult
from exa_py.api import _Result as _ExaResult

from bench.language import LinkPreview, TextLine
from bench.utils.utils import get_from_env

if TYPE_CHECKING:
    from bench.builtin import IWeb


# ruff: noqa: N802,N803

EXA_API_KEY = get_from_env("EXA_API_KEY")

exa = AsyncExa(api_key=EXA_API_KEY)


def _make_link_preview(result: ExaResult | _ExaResult) -> LinkPreview:
    published_at = datetime.fromisoformat(result.published_date) if result.published_date else None
    link = LinkPreview(
        url=result.url,
        title=TextLine.plain(title) if (title := result.title) else None,
        content=getattr(result, "text", None),
        attribution=result.author,
        content_url=result.url,
        favicon_url=result.favicon,
        published_at=published_at,
    )
    return link


class ExaWeb(IWeb if TYPE_CHECKING else object):
    @override
    async def Search(
        self, Query: str, Content: bool = True, Limit: int = 5
    ) -> Annotated[Mapping[str, Any], {"Results": list[LinkPreview]}]:
        if Content:
            response = await exa.search_and_contents(
                query=Query,
                num_results=Limit,
                text=True,
                extras={"image_links": 3},
            )
        else:
            response = await exa.search(query=Query, num_results=Limit)
        results: list[LinkPreview] = []
        for result in response.results:
            results.append(_make_link_preview(result))
        return {"Results": results}

    @override
    async def Extract(
        self, URLs: list[str]
    ) -> Annotated[Mapping[str, Any], {"Previews": list[LinkPreview]}]:
        response = await exa.get_contents(urls=URLs)
        previews: list[LinkPreview] = []
        for result in response.results:
            previews.append(_make_link_preview(result))
        return {"Previews": previews}
