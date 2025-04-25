import asyncio
from datetime import datetime
from typing import TYPE_CHECKING, Annotated, Any, Mapping, cast, override
from urllib.parse import urlparse

from exa_py import AsyncExa
from exa_py.api import Result as ExaResult
from exa_py.api import _Result as _ExaResult

from bench.language import Link, LinkType, ResourceStatus, TextLine
from bench.utils.utils import get_from_env

if TYPE_CHECKING:
    from bench.builtin import IWeb

    from .action import ActionRunner

# ruff: noqa: N802,N803

EXA_API_KEY = get_from_env("EXA_API_KEY")

exa = AsyncExa(api_key=EXA_API_KEY)


def _make_link(result: ExaResult | _ExaResult) -> Link:
    """Make a Link from an ExaResult."""
    published_at = datetime.fromisoformat(result.published_date) if result.published_date else None
    domain = urlparse(result.url).netloc
    link = Link(
        type=LinkType.WEB,
        url=result.url,
        domain=domain,
        status=ResourceStatus.AVAILABLE,
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
) -> list[Link]:
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
    results: list[Link] = []
    for result in response.results:
        results.append(_make_link(result))
    return results


_INJECTED_RUNNER = cast("ActionRunner", None)


class ExaWeb(IWeb if TYPE_CHECKING else object):
    @override
    async def Search(
        self,
        Query: str,
        Content: bool = True,
        Limit: int = 5,
        Max_Characters: int | None = 1000,
        runner: "ActionRunner" = _INJECTED_RUNNER,
    ) -> Annotated[Mapping[str, Any], {"Links": list[Link]}]:
        links = await _do_search(Query, Content, Limit, max_characters=Max_Characters)
        run = runner.closest_tracked_run
        assert run is not None, f"no run in {runner!r}"
        run.extend(*links)
        return {"Links": links}

    @override
    async def Search_Many(
        self,
        Queries: list[str],
        Content: bool = True,
        Limit: int = 5,
        Max_Characters: int | None = 1000,
        runner: "ActionRunner" = _INJECTED_RUNNER,
    ) -> Annotated[Mapping[str, Any], {"Links": list[Link]}]:
        # search in parallel
        results = await asyncio.gather(
            *(_do_search(query, Content, Limit, max_characters=Max_Characters) for query in Queries)
        )
        links: list[Link] = []
        for result in results:
            links.extend(result)
        run = runner.closest_tracked_run
        assert run is not None, f"no run in {runner!r}"
        run.extend(*links)
        return {"Links": links}

    @override
    async def Read(
        self,
        URL: str,
        Max_Characters: int | None = 1000,
        runner: "ActionRunner" = _INJECTED_RUNNER,
    ) -> Annotated[Mapping[str, Any], {"Links": list[Link]}]:
        response = await exa.get_contents(
            urls=URL,
            text={"max_characters": Max_Characters},
            livecrawl="fallback",
            extras={"image_links": 10},
        )
        links: list[Link] = []
        for result in response.results:
            links.append(_make_link(result))
        run = runner.closest_tracked_run
        assert run is not None, f"no run in {runner!r}"
        run.extend(*links)
        return {"Links": links}

    @override
    async def Read_Many(
        self,
        URLs: list[str],
        Max_Characters: int | None = 1000,
        runner: "ActionRunner" = _INJECTED_RUNNER,
    ) -> Annotated[Mapping[str, Any], {"Links": list[Link]}]:
        response = await exa.get_contents(
            urls=URLs,
            text={"max_characters": Max_Characters},
            livecrawl="fallback",
            extras={"image_links": 10},
        )
        links: list[Link] = []
        for result in response.results:
            links.append(_make_link(result))
        run = runner.closest_tracked_run
        assert run is not None, f"no run in {runner!r}"
        run.extend(*links)
        return {"Links": links}
