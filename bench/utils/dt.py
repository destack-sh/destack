from datetime import datetime

import pytz


def utcnow():
    return datetime.now(pytz.utc)
