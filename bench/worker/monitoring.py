import asyncio
from typing import TYPE_CHECKING

import uvicorn

if TYPE_CHECKING:
    from bench.worker import WorkerNode


class WorkerMonitorServer:
    """
    ASGI server to report worker health to K8.
    Implements /healthz endpoint for :WorkerHealthProbe
    """

    def __init__(self, node: "WorkerNode"):
        self.node = node
        self._serve_task = None

    async def asgi(self, scope, receive, send):
        if scope["type"] == "http" and scope["path"] == "/healthz":
            healthy = True
            await send(
                {
                    "type": "http.response.start",
                    "status": 200 if healthy else 500,
                    "headers": [(b"content-type", b"text/plain")],
                }
            )
            await send({"type": "http.response.body", "body": b"OK" if healthy else b"NOT OK"})
        else:
            await send({"type": "http.response.start", "status": 404})
            await send({"type": "http.response.body", "body": b"Not Found"})

    async def launch(self, host: str, port: int):
        if self._serve_task:
            raise RuntimeError("already launched")
        config = uvicorn.Config(
            app=self.asgi,
            host=host,
            port=port,
            log_level="warning",
            access_log=False,
        )
        server = uvicorn.Server(config=config)
        server.install_signal_handlers = lambda *args: None
        self._serve_task = asyncio.create_task(server.serve())

    async def stop(self):
        if self._serve_task:
            self._serve_task.cancel()
            self._serve_task = None
