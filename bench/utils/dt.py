from datetime import datetime
from time import time_ns

import pytz

LOCAL_TZ = pytz.timezone("Europe/Zurich")


def utcnow():
    """A real and proper UTC datetime."""
    return datetime.now(pytz.utc)


monons = time_ns

__all__ = ["LOCAL_TZ", "monons", "utcnow"]
