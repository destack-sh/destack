from datetime import datetime
from time import time_ns

import pytz

LOCAL_TZ = pytz.timezone("Europe/Zurich")


def utcnow():
    """
    A real and proper high precision UTC datetime (at least microsecond precision).
    Convenient and fast, and necessary for using timestamps as keys for certain operations.
    """
    return datetime.fromtimestamp(time_ns() / 1e9, tz=pytz.utc)


monons = time_ns

__all__ = ["LOCAL_TZ", "monons", "utcnow"]
