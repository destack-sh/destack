"""
Playwright browser on steroids.
"""

from uuid import UUID

import structlog
from opentelemetry import trace
from playwright.async_api import Browser as PlaywrightBrowser
from playwright.async_api import BrowserContext as PlaywrightContext
from playwright.async_api import Playwright, async_playwright

from bench.language.bench import ResourceStatus
from bench.language.browser import Browser

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class PlaywrightClient:
    """Client for connecting to Playwright browser instances."""

    def __init__(self):
        self._playwright: Playwright | None = None
        self._browser_by_id: dict[UUID, PlaywrightBrowser] = {}
        self._context_by_id: dict[UUID, PlaywrightContext] = {}

    @tracer.start_as_current_span("playwright.start")
    async def _get_playwright(self):
        if self._playwright is None:
            self._playwright = await async_playwright().start()
            logger.debug("playwright.start", playwright=self._playwright)
        return self._playwright

    async def get_client(self, browser: Browser) -> PlaywrightContext:
        """Gets a BrowserContext for a Browser."""
        assert browser.status == ResourceStatus.UP, f"browser {browser!r} is not up"
        assert browser.connection_uri is not None, f"no connection_uri for {browser!r}"

        pw_context = self._context_by_id.get(browser.id)
        if pw_context is None:
            playwright = await self._get_playwright()
            pw_browser = await playwright.chromium.connect_over_cdp(browser.connection_uri)
            self._browser_by_id[browser.id] = pw_browser
            pw_context = await pw_browser.new_context(
                bypass_csp=browser.is_insecure,
                ignore_https_errors=browser.is_insecure,
            )
            _ = await pw_context.new_page()
            self._context_by_id[browser.id] = pw_context
            logger.debug("playwright.connect", browser=browser, pw_context=pw_context)
        return pw_context

    async def close_client(self, browser: Browser):
        """Closes a Browser's context and connection."""
        if browser.id in self._context_by_id:
            pw_context = self._context_by_id.pop(browser.id)
            await pw_context.close()
        if browser.id in self._browser_by_id:
            pw_browser = self._browser_by_id.pop(browser.id)
            await pw_browser.close()
        logger.trace("playwright.close", browser=browser)

    async def close(self):
        """Closes the Playwright client."""
        if self._playwright is not None:
            await self._playwright.stop()
            self._playwright = None


playwright_client = PlaywrightClient()
