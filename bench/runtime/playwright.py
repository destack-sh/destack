"""
Playwright browser on steroids.
"""

from uuid import UUID

import structlog
from opentelemetry import trace
from playwright.async_api import (
    Browser as PlaywrightBrowser,
)
from playwright.async_api import (
    BrowserContext as PlaywrightContext,
)
from playwright.async_api import (
    Playwright,
    async_playwright,
)

from bench.language.bench import ResourceStatus
from bench.language.browser import Browser
from bench.language.view import Vector2
from bench.utils.env import ENV, Env

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

DEFAULT_BROWSER_SIZE = Vector2(x=1280, y=1080)


class PlaywrightApi:
    """Playwright API."""

    def __init__(self):
        self._playwright: Playwright | None = None
        self._local_browser: PlaywrightBrowser | None = None
        self._remote_browser_by_id: dict[UUID, PlaywrightBrowser] = {}
        self._context_by_id: dict[UUID, PlaywrightContext] = {}

    @tracer.start_as_current_span("playwright.start")
    async def _get_playwright(self):
        if self._playwright is None:
            self._playwright = await async_playwright().start()
        return self._playwright

    @tracer.start_as_current_span("playwright.start_local_browser")
    async def _get_playwright_browser(self):
        playwright = await self._get_playwright()
        if self._local_browser is None:
            chromium_args: list[str] = [
                "--no-sandbox",
                "--disable-blink-features=AutomationControlled",
                "--disable-infobars",
                "--disable-background-timer-throttling",
                "--disable-popup-blocking",
                "--disable-backgrounding-occluded-windows",
                "--disable-renderer-backgrounding",
                "--disable-window-activation",
                "--disable-focus-on-load",
                "--no-first-run",
                "--no-default-browser-check",
                "--no-startup-window",
                "--window-position=0,0",
            ]
            self._local_browser = await playwright.chromium.launch(
                headless=False, args=chromium_args
            )
        return self._local_browser

    @tracer.start_as_current_span("playwright.provision")
    async def provision_local_browser(self, browser: Browser) -> PlaywrightContext:
        """Provisions a local Browser."""
        assert browser.id not in self._context_by_id, f"browser {browser!r} already provisioned"
        pw_browser = await self._get_playwright_browser()
        size = browser.size or DEFAULT_BROWSER_SIZE
        pw_context: PlaywrightContext = await pw_browser.new_context(
            viewport={"width": int(size.x), "height": int(size.y)},
            no_viewport=True,
            bypass_csp=browser.is_insecure,
            ignore_https_errors=browser.is_insecure,
        )
        _ = await pw_context.new_page()
        self._context_by_id[browser.id] = pw_context
        return pw_context

    @tracer.start_as_current_span("playwright.decommission")
    async def decommission_local_browser(self, browser: Browser):
        """Decommissions a local Browser (if any)."""
        if browser.id in self._context_by_id:
            context = self._context_by_id.pop(browser.id)
            await context.close()

    async def get_browser(self, browser: Browser) -> PlaywrightContext:
        """Gets a BrowserContext for a Browser."""
        assert browser.status == ResourceStatus.UP, f"browser {browser!r} is not up"
        pw_context = self._context_by_id.get(browser.id)
        if pw_context is None:
            if browser.connection_uri is None:
                # provision locally
                assert ENV in (Env.TEST, Env.DEV), f"no connection_uri for {browser!r}"
                pw_context = await self.provision_local_browser(browser)
            else:
                # connect remotely
                # nocheckin: revisit.. when do we close the context? after the containing Run? timeout?
                playwright = await self._get_playwright()
                pw_browser = await playwright.chromium.connect_over_cdp(browser.connection_uri)
                self._remote_browser_by_id[browser.id] = pw_browser
                pw_context = await pw_browser.new_context()
                _ = await pw_context.new_page()
                self._context_by_id[browser.id] = pw_context
        return pw_context

    async def close_browser(self, browser: Browser):
        """Closes a Browser's context and remote browser (if any)."""
        if browser.id in self._context_by_id:
            pw_context = self._context_by_id.pop(browser.id)
            await pw_context.close()
        if browser.id in self._remote_browser_by_id:
            pw_browser = self._remote_browser_by_id.pop(browser.id)
            await pw_browser.close()

    async def close(self):
        if self._local_browser is not None:
            await self._local_browser.close()
            self._local_browser = None
        if self._playwright is not None:
            await self._playwright.stop()
            self._playwright = None


# nocheckin: don't import playwright_api in system (or allow that in tach, or refactor somehow, ...)
playwright_api = PlaywrightApi()
