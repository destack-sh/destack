from datetime import datetime

import pytz


def utcnow_with_tz():
    return datetime.utcnow().replace(tzinfo=pytz.UTC)
