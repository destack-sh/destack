import asyncio
import os
import sys
from pathlib import Path
from typing import Callable

import structlog

logger = structlog.get_logger(__name__)


async def restart_on_file_changes(on_restart: Callable | None = None):
    """Restarts the process when a source file changes."""
    from watchdog.events import FileSystemEventHandler
    from watchdog.observers import Observer

    class Handler(FileSystemEventHandler):
        def on_any_event(self, event):
            if event.is_directory or ".tmp" in event.src_path or "test_" in event.src_path:
                return
            if event.src_path.endswith(".py"):
                logger.debug("watcher.reload", path=event.src_path)
                if on_restart:
                    on_restart()
                os.execv(sys.executable, [sys.executable, *sys.argv])

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
