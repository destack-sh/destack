import cProfile
import io
import pstats

import brotli
import structlog
from more_itertools import first

from bench.settings import BROTLI_QUALITY_LEVEL

logger = structlog.get_logger(__name__)


class BrotliCompressionMiddleware:
    """Brotli ASGI compression middleware (for our Daphne API stack)."""

    def __init__(self, app):
        self.app = app

    async def __call__(self, scope, receive, send):
        if scope["type"] != "http":
            await self.app(scope, receive, send)
            return

        # check for brotli support (headers is now a list of tuples)
        accept_encoding = first(
            (v for k, v in scope["headers"] if k.decode().lower() == "accept-encoding"), None
        )
        if not accept_encoding or b"br" not in accept_encoding:
            await self.app(scope, receive, send)
            return

        # wrap send function to compress response
        async def send_with_brotli(message):
            if message["type"] == "http.response.start":
                message["headers"].append((b"content-encoding", b"br"))
                message["headers"].append((b"vary", b"accept-encoding"))
                message["headers"].append((b"cache-control", b"no-cache"))
            elif message["type"] == "http.response.body":
                message["body"] = brotli.compress(message["body"], quality=BROTLI_QUALITY_LEVEL)
            await send(message)

        # call wrapped app
        await self.app(scope, receive, send_with_brotli)


class CProfileMiddleware:
    """CProfile ASGI middleware to capture request profiles (for our Daphne API stack)."""

    def __init__(self, app):
        self.app = app
        self.profiler = cProfile.Profile()

    async def __call__(self, scope, receive, send):
        if scope["type"] != "http":
            await self.app(scope, receive, send)
            return

        # wrap send function to capture profile
        self.profiler.enable()
        await self.app(scope, receive, send)
        self.profiler.disable()

        # dump profile to log
        s = io.StringIO()
        sortby = "cumulative"
        ps = pstats.Stats(self.profiler, stream=s).sort_stats(sortby)
        ps.print_stats()
        print(s.getvalue())
