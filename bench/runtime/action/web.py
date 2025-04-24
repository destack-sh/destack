import asyncio
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
    """Make a LinkPreview from an ExaResult."""
    published_at = datetime.fromisoformat(result.published_date) if result.published_date else None
    link = LinkPreview(
        url=result.url,
        title=TextLine.plain(title) if (title := result.title) else None,
        content=getattr(result, "text", None),
        attribution=result.author,
        content_url=result.url,
        favicon_url=result.favicon,
        published_at=published_at,
        image_urls=(result.extras or {}).get("image_links", ()),
    )
    return link


async def _do_search(
    query: str, content: bool, limit: int, max_characters: int | None
) -> list[LinkPreview]:
    """Search the web for the given query."""
    if content:
        response = await exa.search_and_contents(
            query=query,
            num_results=limit,
            text={"max_characters": max_characters},
            extras={"image_links": 10},
        )
    else:
        response = await exa.search(query=query, num_results=limit)
    results: list[LinkPreview] = []
    for result in response.results:
        results.append(_make_link_preview(result))
    return results


class ExaWeb(IWeb if TYPE_CHECKING else object):
    @override
    async def Search(
        self, Query: str, Content: bool = True, Limit: int = 5
    ) -> Annotated[Mapping[str, Any], {"Results": list[LinkPreview]}]:
        results = await _do_search(Query, Content, Limit, max_characters=1000)
        return {"Results": results}

    @override
    async def Search_Many(
        self, Queries: list[str], Content: bool = True, Limit: int = 5
    ) -> Annotated[Mapping[str, Any], {"Results": list[LinkPreview]}]:
        # search in parallel
        results = await asyncio.gather(
            *(_do_search(query, Content, Limit, max_characters=1000) for query in Queries)
        )
        results = [result for results in results for result in results]
        return {"Results": results}

    @override
    async def Read(self, URL: str) -> Annotated[Mapping[str, Any], {"Previews": list[LinkPreview]}]:
        response = await exa.get_contents(
            urls=URL, text=True, livecrawl="fallback", extras={"image_links": 10}
        )
        previews: list[LinkPreview] = []
        for result in response.results:
            previews.append(_make_link_preview(result))
        return {"Previews": previews}

    @override
    async def Read_Many(
        self, URLs: list[str]
    ) -> Annotated[Mapping[str, Any], {"Previews": list[LinkPreview]}]:
        response = await exa.get_contents(
            urls=URLs, text=True, livecrawl="fallback", extras={"image_links": 10}
        )
        previews: list[LinkPreview] = []
        for result in response.results:
            previews.append(_make_link_preview(result))
        return {"Previews": previews}
