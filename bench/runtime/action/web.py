import urllib.parse
from datetime import datetime
from typing import TYPE_CHECKING, Annotated, Any, Mapping, cast, override
from urllib.parse import urlparse

import aiohttp
from exa_py import AsyncExa
from exa_py.api import Result as ExaResult
from exa_py.api import _Result as _ExaResult

from bench.language import File, FileType, Icon, IconType, Link, LinkType, TextLine
from bench.utils.utils import get_from_env

if TYPE_CHECKING:
    from bench.builtin import IInternet

    from .action import ActionRunner

# ruff: noqa: N802,N803

EXA_API_KEY = get_from_env("EXA_API_KEY")
UNSPLASH_ACCESS_KEY = get_from_env("UNSPLASH_ACCESS_KEY")

IGNORE_DOMAIN_PREFIXES = (
    "www.",
    "api.",
    "app.",
    "blog.",
    "dev.",
    "docs.",
    "help.",
    "info.",
    "support.",
    "www.",
    "www2.",
    "www3.",
    "www4.",
    "www5.",
    "www6.",
    "www7.",
    "www8.",
    "www9.",
)

exa = AsyncExa(api_key=EXA_API_KEY)


def _make_link(result: ExaResult | _ExaResult) -> Link:
    """Make a Link from an ExaResult."""
    # domain
    url_parsed = urlparse(result.url)
    domain = url_parsed.netloc
    for prefix in IGNORE_DOMAIN_PREFIXES:
        if domain.startswith(prefix):
            domain = domain[len(prefix) :]
    domain_parts = domain.split(".")
    if len(domain_parts) > 2:
        domain_parts = domain_parts[1:]  # strip any other subdomains
    attribution_tag = domain_parts[0]
    domain = ".".join(domain_parts)

    # link
    published_at = datetime.fromisoformat(result.published_date) if result.published_date else None
    icon = Icon(type=IconType.FILE_URL, file_url=result.favicon) if result.favicon else None
    image_urls = [url for url in (result.extras or {}).get("image_links", ()) if url]
    link = Link(
        type=LinkType.WEB,
        icon=icon,
        url=result.url,
        name=domain,
        domain=domain,
        title=TextLine.plain(title) if (title := result.title) else None,
        content=getattr(result, "text", None),
        attribution=result.author,
        attribution_tag=attribution_tag,
        content_url=result.url,
        favicon_url=result.favicon,
        published_at=published_at,
        image_urls=image_urls,
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


class Internet(IInternet if TYPE_CHECKING else object):
    @override
    async def Search(
        self,
        Query: str,
        Include_Content: bool = True,
        Limit: int = 5,
        Max_Characters: int | None = 1000,
        runner: "ActionRunner" = _INJECTED_RUNNER,
    ) -> Annotated[Mapping[str, Any], {"Links": list[Link]}]:
        links = await _do_search(Query, Include_Content, Limit, max_characters=Max_Characters)
        run = runner.closest_tracked_run
        assert run is not None, f"no run in {runner!r}"
        run.extend(*links)
        return {"Links": links}

    @override
    async def Search_Images(
        self, Query: str, Limit: int = 3, runner: "ActionRunner" = _INJECTED_RUNNER
    ) -> Annotated[Mapping[str, Any], {"Images": list[File]}]:
        encoded_query = urllib.parse.quote(Query)
        url = f"https://api.unsplash.com/search/photos?query={encoded_query}&per_page={Limit}&client_id={UNSPLASH_ACCESS_KEY}"

        # unsplash
        async with aiohttp.ClientSession() as session, session.get(url) as response:
            if response.status != 200:
                error_text = await response.text()
                raise RuntimeError(f"Unsplash error: {response.status} - {error_text}")
            data = await response.json()

        # file
        images = []
        for item in data.get("results", []):
            image_url = item.get("urls", {}).get("regular")
            if not image_url:
                continue
            width = item.get("width")
            height = item.get("height")
            description = item.get("description") or item.get("alt_description") or Query
            filename = f"{description[:30].replace(' ', '_')}.jpeg"
            file = File.external(
                url=image_url,
                name=filename,
                type=FileType.IMAGE,
                mime_type="image/jpeg",
                width=width,
                height=height,
            )
            images.append(file)

        return {"Images": images}

    @override
    async def Read(
        self,
        URL: str,
        Max_Characters: int | None = 1000,
        runner: "ActionRunner" = _INJECTED_RUNNER,
    ) -> Annotated[Mapping[str, Any], {"Links": list[Link]}]:
        response = await exa.get_contents(
            urls=[URL],
            text={"max_characters": Max_Characters},
            livecrawl="always",
            extras={"image_links": 10},
        )
        links: list[Link] = []
        for result in response.results:
            links.append(_make_link(result))
        run = runner.closest_tracked_run
        assert run is not None, f"no run in {runner!r}"
        run.extend(*links)
        return {"Links": links}
