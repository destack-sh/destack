from uuid import UUID

import structlog
from opentelemetry import trace
from playwright.async_api import Browser as PlaywrightBrowser
from playwright.async_api import Playwright, async_playwright

from bench.language.browser import Browser
from bench.language.view import Vector2

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

DEFAULT_BROWSER_SIZE = Vector2(x=1280.0, y=1080.0)


class PlaywrightServer:
    """Manages Playwright browser server processes (each with its own CDP port)."""

    def __init__(self):
        self._playwright: Playwright | None = None
        self._pw_browser_by_browser_id: dict[UUID, PlaywrightBrowser] = {}
        self._connection_uri_by_browser_id: dict[UUID, str] = {}
        self._next_port = 9222

    @tracer.start_as_current_span("playwright.start")
    async def _get_playwright(self):
        if self._playwright is None:
            self._playwright = await async_playwright().start()
            logger.debug("playwright.start", playwright=self._playwright)
        return self._playwright

    async def start_browser(self, browser: Browser) -> str:
        """Starts a new browser process with a unique remote-debugging-port."""
        if browser.id in self._connection_uri_by_browser_id:
            return self._connection_uri_by_browser_id[browser.id]

        # define connection
        port = self._next_port
        self._next_port += 1
        connection_uri = f"http://localhost:{port}"

        # build args
        size = browser.size or DEFAULT_BROWSER_SIZE
        args = [
            f"--remote-debugging-port={port}",
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
            f"--window-size={int(size.x)},{int(size.y)}",
        ]
        playwright = await self._get_playwright()
        pw_browser = await playwright.chromium.launch(headless=False, args=args)
        self._pw_browser_by_browser_id[browser.id] = pw_browser
        self._connection_uri_by_browser_id[browser.id] = connection_uri

        return connection_uri

    async def stop_browser(self, browser: Browser):
        """Stops a browser process.""" 
        if browser.id not in self._pw_browser_by_browser_id:
            return
        pw_browser = self._pw_browser_by_browser_id.pop(browser.id)
        await pw_browser.close()
        self._connection_uri_by_browser_id.pop(browser.id, None)
        logger.debug("playwright_server.stop", browser=browser)

    async def close(self):
        """Closes all browser processes."""
        for pw_browser in self._pw_browser_by_browser_id.values():
            await pw_browser.close()
        self._pw_browser_by_browser_id.clear()
        self._connection_uri_by_browser_id.clear()
        logger.debug("playwright_server.close", pw_browser=self._pw_browser_by_browser_id)


playwright_server = PlaywrightServer()
