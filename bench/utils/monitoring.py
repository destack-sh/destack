import abc
import asyncio
import os
import sys
from pathlib import Path

import structlog
import uvicorn

logger = structlog.get_logger(__name__)


class Monitored(abc.ABC):
    @property
    def ready(self) -> bool:
        return True

    @property
    def healthy(self) -> bool:
        return self.ready


class MonitoringServer:
    """Uvicorn ASGI server to report /healthz and /ready to K8."""

    def __init__(self, monitors: list[Monitored]):
        self.monitors = monitors
        self._serve_task = None

    async def __call__(self, scope, receive, send):
        if scope["type"] == "http" and scope["path"] == "/healthz":
            healthy = all(m.healthy for m in self.monitors)
            await send(
                {
                    "type": "http.response.start",
                    "status": 200 if healthy else 500,
                    "headers": [(b"content-type", b"text/plain")],
                }
            )
            await send({"type": "http.response.body", "body": b"OK" if healthy else b"NOT OK"})
        elif scope["type"] == "http" and scope["path"] == "/ready":
            ready = all(m.ready for m in self.monitors)
            await send(
                {
                    "type": "http.response.start",
                    "status": 200 if ready else 500,
                    "headers": [(b"content-type", b"text/plain")],
                }
            )
            await send({"type": "http.response.body", "body": b"OK" if ready else b"NOT OK"})
        else:
            await send({"type": "http.response.start", "status": 404})
            await send({"type": "http.response.body", "body": b"Not Found"})

    async def launch(self, host: str, port: int, daemon: bool = True):
        if self._serve_task:
            raise RuntimeError("already launched")
        config = uvicorn.Config(
            app=self, host=host, port=port, log_level="warning", access_log=False
        )
        server = uvicorn.Server(config=config)
        server.install_signal_handlers = lambda *args: None
        self._serve_task = asyncio.create_task(server.serve())
        if not daemon:
            await self._serve_task

    async def stop(self):
        if self._serve_task:
            self._serve_task.cancel()
            self._serve_task = None


async def restart_on_file_changes(on_restart: callable = None):
    """Restarts the process when a source file changes."""
    from watchdog.events import FileSystemEventHandler  # noqa
    from watchdog.observers import Observer  # noqa

    class Handler(FileSystemEventHandler):
        def on_any_event(self, event):
            if event.is_directory:
                return
            if event.src_path.endswith(".py"):
                logger.debug("watcher.reload", path=event.src_path)
                print("-" * 95 + " RESTART " + "-" * 95)  # simple separator
                if on_restart:
                    on_restart()
                os.execv(sys.executable, [sys.executable] + sys.argv)

    cwd = str(Path(".").absolute())
    observer = Observer()
    observer.schedule(Handler(), cwd, recursive=True)
    observer.start()
    logger.debug("watcher.listen", cwd=cwd)

    try:
        while True:
            await asyncio.sleep(0.1)
    except KeyboardInterrupt:
        observer.stop()
    observer.join()
