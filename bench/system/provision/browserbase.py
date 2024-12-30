from typing import Literal, NamedTuple

import structlog
from browserbase import AsyncBrowserbase
from opentelemetry import trace

from bench.language.browser import DEFAULT_BROWSER_HEIGHT, DEFAULT_BROWSER_WIDTH, Browser
from bench.language.const import Region, RegionArea
from bench.language.view import Vector2
from bench.utils.utils import get_from_env

BROWSERBASE_API_KEY = get_from_env("BROWSERBASE_API_KEY")
BROWSERBASE_PROJECT_ID = get_from_env("BROWSERBASE_PROJECT_ID")
DEFAULT_BROWSER_SIZE = Vector2(x=DEFAULT_BROWSER_WIDTH, y=DEFAULT_BROWSER_HEIGHT)

BrowserbaseRegion = Literal["us-west-2", "us-east-1", "eu-central-1", "ap-southeast-1"]

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


def get_browserbase_region(region: Region) -> BrowserbaseRegion:
    if region.area == RegionArea.EUROPE:
        return "eu-central-1"
    elif region == Region.OHIO:
        return "us-east-1"
    else:
        raise ValueError(f"unsupported Browserbase region: {region}")


class BrowserConnection(NamedTuple):
    connection_uri: str | None
    debugger_uri: str | None
    view_uri: str | None
    external_id: str


class BrowserbaseApi:
    """Browserbase API."""

    def __init__(self):
        self.bb = AsyncBrowserbase(api_key=BROWSERBASE_API_KEY)

    @tracer.start_as_current_span("browserbase.start_browser")
    async def start_browser(self, browser: Browser) -> BrowserConnection:
        """Start a browser."""
        region = get_browserbase_region(browser.region)
        size = browser.size or DEFAULT_BROWSER_SIZE
        session = await self.bb.sessions.create(
            project_id=BROWSERBASE_PROJECT_ID,
            browser_settings={
                "block_ads": True,
                "viewport": {
                    "height": int(size.y),
                    "width": int(size.x),
                },
            },
            region=region,
            # keep_alive=True, # not supported on Hobby plan
        )
        debug = await self.bb.sessions.debug(session.id)
        connection = BrowserConnection(
            connection_uri=session.connect_url,
            debugger_uri=debug.debugger_url,
            view_uri=debug.debugger_fullscreen_url,
            external_id=session.id,
        )
        return connection

    @tracer.start_as_current_span("browserbase.get_running_browsers")
    async def get_running_browsers(self) -> list[BrowserConnection]:
        """Get a browser."""
        sessions = await self.bb.sessions.list(status="RUNNING")
        return [
            BrowserConnection(
                connection_uri=None, debugger_uri=None, view_uri=None, external_id=session.id
            )
            for session in sessions
        ]

    @tracer.start_as_current_span("browserbase.stop_browser")
    async def stop_browser(self, browser: Browser):
        """Stop a browser."""
        assert browser.external_id is not None, f"{browser!r} has no external_id"
        await self.bb.sessions.update(
            browser.external_id,
            project_id=BROWSERBASE_PROJECT_ID,
            status="REQUEST_RELEASE",
        )


browserbase_api = BrowserbaseApi()
