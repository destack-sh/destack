"""
Playwright browser on steroids.
"""

import asyncio
import pathlib
from typing import Any
from uuid import UUID

import structlog
from opentelemetry import trace
from playwright.async_api import Browser as PlaywrightBrowser
from playwright.async_api import BrowserContext as PlaywrightContext
from playwright.async_api import Page as PlaywrightPage
from playwright.async_api import Playwright, async_playwright
from playwright.async_api import Request as PlaywrightRequest
from playwright.async_api import Response as PlaywrightResponse

from bench.language.bench import ResourceStatus
from bench.language.browser import Browser, DomNode, DomNodeType

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

INIT_SCRIPT_JS = """
// Webdriver property
Object.defineProperty(navigator, 'webdriver', {
    get: () => undefined
});

// Languages
Object.defineProperty(navigator, 'languages', {
    get: () => ['en-US', 'en']
});

// Plugins
Object.defineProperty(navigator, 'plugins', {
    get: () => [1, 2, 3, 4, 5]
});

// Chrome runtime
window.chrome = { runtime: {} };

// Permissions
const originalQuery = window.navigator.permissions.query;
window.navigator.permissions.query = (parameters) => (
    parameters.name === 'notifications' ?
        Promise.resolve({ state: Notification.permission }) :
        originalQuery(parameters)
);
"""


class PlaywrightClient:
    """Client for connecting to Playwright browser instances."""

    def __init__(self):
        self._playwright: Playwright | None = None
        self._browser_by_id: dict[UUID, PlaywrightBrowser] = {}
        self._context_by_id: dict[UUID, PlaywrightContext] = {}
        self._extension_script_js: str | None = None

    @tracer.start_as_current_span("playwright.start")
    async def _get_playwright(self):
        if self._playwright is None:
            self._playwright = await async_playwright().start()
            logger.debug("playwright.start", playwright=self._playwright)
        return self._playwright

    @tracer.start_as_current_span("playwright.get_client")
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
            await pw_context.add_init_script(INIT_SCRIPT_JS)
            logger.debug("playwright.connect", browser=browser, pw_context=pw_context)
        return pw_context

    @tracer.start_as_current_span("playwright.close_client")
    async def close_client(self, browser: Browser):
        """Closes a Browser's context and connection."""
        if browser.id in self._context_by_id:
            pw_context = self._context_by_id.pop(browser.id)
            await pw_context.close()
        if browser.id in self._browser_by_id:
            pw_browser = self._browser_by_id.pop(browser.id)
            await pw_browser.close()
        logger.trace("playwright.close", browser=browser)

    def get_extension_script_js(self):
        """Gets the extension script for Playwright."""
        if self._extension_script_js is None:
            parent = pathlib.Path(__file__).parent
            assert parent is not None, f"no parent for {__file__}"
            self._extension_script_js = (parent / "extension.js").read_text()
        return self._extension_script_js

    async def wait_for_idle(
        self, pw_page: PlaywrightPage, min_idle_time: float = 0.5, max_wait_time: float = 30.0
    ):
        """Wait for network activity to become idle."""
        activity_monitor = PlaywrightActivityMonitor(pw_page)
        await activity_monitor.wait_for_idle(
            min_idle_time=min_idle_time, max_wait_time=max_wait_time
        )

    async def close(self):
        """Closes the Playwright client."""
        if self._playwright is not None:
            await self._playwright.stop()
            self._playwright = None


ACTIVITY_RESOURCE_TYPES = {
    "document",
    "stylesheet",
    "image",
    "font",
    "script",
    "iframe",
}
ACTIVITY_CONTENT_TYPES = {
    "text/html",
    "text/css",
    "application/javascript",
    "image/",
    "font/",
    "application/json",
}
ACTIVITY_IGNORED_URL_PATTERNS = {
    # Analytics and tracking
    "analytics",
    "tracking",
    "telemetry",
    "beacon",
    "metrics",
    # Ad-related
    "doubleclick",
    "adsystem",
    "adserver",
    "advertising",
    # Social media widgets
    "facebook.com/plugins",
    "platform.twitter",
    "linkedin.com/embed",
    # Live chat and support
    "livechat",
    "zendesk",
    "intercom",
    "crisp.chat",
    "hotjar",
    # Push notifications
    "push-notifications",
    "onesignal",
    "pushwoosh",
    # Background sync/heartbeat
    "heartbeat",
    "ping",
    "alive",
    # WebRTC and streaming
    "webrtc",
    "rtmp://",
    "wss://",
    # Common CDNs for dynamic content
    "cloudfront.net",
    "fastly.net",
}


class PlaywrightActivityMonitor:
    """Monitor activity on a Playwright page."""

    def __init__(self, pw_page: PlaywrightPage):
        self.pw_page = pw_page
        self._pending_requests: set[PlaywrightRequest] = set()
        self._last_active_at = asyncio.get_event_loop().time()

    async def _on_request(self, request: PlaywrightRequest) -> None:
        """On Playwright request event."""
        # ignore irrelevant resource type
        if request.resource_type not in ACTIVITY_RESOURCE_TYPES:
            return
        # ignore streaming, websocket, and other real-time requests
        if request.resource_type in {
            "websocket",
            "media",
            "eventsource",
            "manifest",
            "other",
        }:
            return
        # ignore irrelevant URL patterns
        url = request.url.lower()
        if any(pattern in url for pattern in ACTIVITY_IGNORED_URL_PATTERNS):
            return
        # ignore data URLs and blob URLs
        if url.startswith(("data:", "blob:")):
            return
        # ignore requests with certain headers
        headers = request.headers
        if headers.get("purpose") == "prefetch" or headers.get("sec-fetch-dest") in (
            "video",
            "audio",
        ):
            return

        self._pending_requests.add(request)
        self._last_active_at = asyncio.get_event_loop().time()

    async def _on_response(self, response: PlaywrightResponse) -> None:
        """On Playwright response event."""
        request = response.request
        if request not in self._pending_requests:
            return

        # filter by content type if available
        content_type = response.headers.get("content-type", "").lower()

        # skip if content type indicates streaming or real-time data
        if any(
            t in content_type
            for t in [
                "streaming",
                "video",
                "audio",
                "webm",
                "mp4",
                "event-stream",
                "websocket",
                "protobuf",
            ]
        ):
            self._pending_requests.remove(request)
            return

        # skip irrelevant content types
        if not any(ct in content_type for ct in ACTIVITY_CONTENT_TYPES):
            self._pending_requests.remove(request)
            return

        # skip if response is too large (likely not essential for page load)
        content_length = response.headers.get("content-length")
        if content_length and int(content_length) > 5 * 1024 * 1024:  # 5MB
            self._pending_requests.remove(request)
            return

        self._pending_requests.remove(request)
        self._last_active_at = asyncio.get_event_loop().time()

    async def wait_for_idle(self, *, min_idle_time: float, max_wait_time: float) -> None:
        """Wait for network activity to become idle."""
        self.pw_page.on("request", self._on_request)
        self.pw_page.on("response", self._on_response)

        try:
            start_time = asyncio.get_event_loop().time()
            while True:
                await asyncio.sleep(min_idle_time)
                now = asyncio.get_event_loop().time()

                if (
                    len(self._pending_requests) == 0
                    and (now - self._last_active_at) >= min_idle_time
                ):
                    # no more active requests
                    break

                if now - start_time > max_wait_time:
                    # timeout
                    break

        finally:
            self.pw_page.remove_listener("request", self._on_request)
            self.pw_page.remove_listener("response", self._on_response)


def parse_dom_node(dom_tree: dict) -> DomNode:
    """Parse a DOM node from the Playwright DOM tree :DomNode."""
    node_type = DomNodeType(dom_tree["type"])
    kwargs: dict[str, Any] = {
        "type": node_type,
        "index": dom_tree.get("id"),
        "tag": dom_tree.get("tag"),
        "text": dom_tree.get("text"),
        "is_interactive": dom_tree.get("isInteractive"),
        "is_visible": dom_tree.get("isVisible"),
        "is_top": dom_tree.get("isTop"),
        "children": [],
    }
    if "children" in dom_tree:
        kwargs["children"] = [parse_dom_node(child) for child in dom_tree["children"]]
    dom_node = DomNode(**kwargs, _skip_validate_self=True)
    return dom_node
