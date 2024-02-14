import secrets
import uuid
from collections import defaultdict
from time import time
from typing import Any, Dict, Optional


class UUIDT(uuid.UUID):
    """
    UUID, but (mostly) sortable by generation time.
    Adapted from PostHog's UUIDT (https://github.com/PostHog/posthog@5eb98ef),
    which in turn is based on Segment's KSUID (https://github.com/segmentio/ksuid)
    and on Twitter's snowflake ID (https://blog.twitter.com/engineering/en_us/a/2010/announcing-snowflake.html).


    QUOTE:
    ""
    This doesn't adhere to any official UUID version spec, but it is superior as a primary key:
    to incremented integers (as they can reveal sensitive business information about usage volumes and patterns),
    to UUID v4 (as the complete randomness of v4 makes its indexing performance suboptimal),
    and to UUID v1 (as despite being time-based it can't be used practically for sorting by generation time).
    Order can be messed up if system clock is changed or if more than 65 536 IDs are generated per millisecond
    (that's over 5 trillion events per day), but it should be largely safe to assume that these are time-sortable.
    Anatomy:
    - 6 bytes - Unix time milliseconds unsigned integer
    - 2 bytes - auto-incremented series unsigned integer
                (per millisecond, rolls over to 0 after reaching 65 535 UUIDs in one ms)
    - 8 bytes - securely random gibberish
    ""
    """

    current_series_per_ms: Dict[int, int] = defaultdict(int)

    def __init__(self, unix_time_ms: Optional[int] = None, uuid_str: Optional[str] = None) -> None:
        if uuid_str and self.is_valid_uuid(uuid_str):
            super().__init__(uuid_str)
            return

        if unix_time_ms is None:
            unix_time_ms = int(time() * 1000)
        time_component = unix_time_ms.to_bytes(
            6, "big", signed=False
        )  # 48 bits for time, WILL FAIL in 10 895 CE
        series_component = self.get_series(unix_time_ms).to_bytes(
            2, "big", signed=False
        )  # 16 bits for series
        random_component = secrets.token_bytes(8)  # 64 bits for random gibberish
        uuid_bytes = time_component + series_component + random_component
        super().__init__(bytes=uuid_bytes)

    @classmethod
    def get_series(cls, unix_time_ms: int) -> int:
        """Get per-millisecond series integer in range [0-65536)."""
        series = cls.current_series_per_ms[unix_time_ms]
        if len(cls.current_series_per_ms) > 10_000:  # Clear class dict periodically
            cls.current_series_per_ms.clear()
            cls.current_series_per_ms[unix_time_ms] = series
        cls.current_series_per_ms[unix_time_ms] += 1
        cls.current_series_per_ms[unix_time_ms] %= 65_536
        return series

    @classmethod
    def is_valid_uuid(cls, candidate: Any) -> bool:
        if not isinstance(candidate, str):
            return False
        hex = candidate.replace("urn:", "").replace("uuid:", "")
        hex = hex.strip("{}")
        if len(hex) != 32:
            return False
        return 0 <= int(hex, 16) < 1 << 128
