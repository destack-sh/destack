"""
Playwright browser on steroids.
"""

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

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class PlaywrightApi:
    """Playwright API."""

    def __init__(self):
        self._playwright: Playwright | None = None
        self._browser: PlaywrightBrowser | None = None
        self._context_id: int = 0
        self._context_by_id: dict[str, PlaywrightContext] = {}

    @tracer.start_as_current_span("playwright.start_browser")
    async def _get_browser(self):
        if self._playwright is None:
            self._playwright = await async_playwright().start()
        if self._browser is None:
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
            self._browser = await self._playwright.chromium.launch(
                headless=False, args=chromium_args
            )
        return self._browser

    @tracer.start_as_current_span("playwright.provision")
    async def provision(
        self,
        *,
        is_insecure: bool,
        window_size: tuple[int, int],
    ) -> str:
        browser = await self._get_browser()

        # create context
        context = await browser.new_context(
            viewport={"width": window_size[0], "height": window_size[1]},
            no_viewport=True,
            bypass_csp=is_insecure,
            ignore_https_errors=is_insecure,
        )
        _ = await context.new_page()

        self._context_id += 1
        self._context_by_id[str(self._context_id)] = context
        return str(self._context_id)

    @tracer.start_as_current_span("playwright.decommission")
    async def decommission(self, external_id: str):
        context = self._context_by_id.pop(external_id)
        await context.close()

    async def close(self):
        if self._browser is not None:
            await self._browser.close()
            self._browser = None
        if self._playwright is not None:
            await self._playwright.stop()
            self._playwright = None


playwright_api = PlaywrightApi()
