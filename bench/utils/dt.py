import asyncio
from datetime import datetime

import pytz


def utcnow():
    """A real and proper UTC datetime."""
    return datetime.now(pytz.utc)


def monotime() -> float:
    """A monotonic time in seconds."""
    return asyncio.get_event_loop().time()
