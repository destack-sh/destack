from datetime import datetime
from typing import TYPE_CHECKING, Annotated, Any, Mapping, override

from exa_py import AsyncExa

from bench.language import LinkPreview, to_text, to_text_line
from bench.utils.utils import get_from_env

if TYPE_CHECKING:
    from bench.builtin import IWeb


# ruff: noqa: N802,N803

EXA_API_KEY = get_from_env("EXA_API_KEY")

exa = AsyncExa(api_key=EXA_API_KEY)


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
            if result.published_date:
                published_at = datetime.fromisoformat(result.published_date)
            else:
                published_at = None
            link = LinkPreview(
                url=result.url,
                title=to_text_line(title) if (title := result.title) else None,
                text=to_text(text) if (text := getattr(result, "text", None)) else None,
                attribution=result.author,
                content_url=result.url,
                favicon_url=result.favicon,
                published_at=published_at,
            )
            results.append(link)
        return {"Results": results}
