from datetime import timedelta
from typing import TypeAlias

Components: TypeAlias = list[tuple[str, str, int | None, bool]]
Measurements: TypeAlias = list[tuple[str, float]]


def _parse_date(segment: str) -> Components:
    match tuple(segment):
        # YYYY-DDD
        case _, _, _, _, "-", _, _, _:
            return [(segment[0:4], "years", None, True), (segment[5:8], "days", 366, True)]

        # YYYY-MM-DD
        case _, _, _, _, "-", _, _, "-", _, _:
            return [
                (segment[0:4], "years", None, True),
                (segment[5:7], "months", 12, True),
                (segment[8:10], "days", 31, True),
            ]

        # YYYYDDD
        case _, _, _, _, _, _, _:
            return [(segment[0:4], "years", None, True), (segment[4:7], "days", 366, True)]

        # YYYYMMDD
        case _, _, _, _, _, _, _, _:
            return [
                (segment[0:4], "years", None, True),
                (segment[4:6], "months", 12, True),
                (segment[6:8], "days", 31, True),
            ]

        case _:
            raise ValueError(f"unable to parse '{segment}' into date components")


def _parse_time(segment: str) -> Components:
    match tuple(segment):
        # HH:MM:SS[.ssssss]
        case _, _, ":", _, _, ":", _, _, ".", *_:
            return [
                (segment[0:2], "hours", 24, True),
                (segment[3:5], "minutes", 60, True),
                (segment[6:15], "seconds", 60, False),
            ]

        # HH:MM:SS
        case _, _, ":", _, _, ":", _, _:
            return [
                (segment[0:2], "hours", 24, True),
                (segment[3:5], "minutes", 60, True),
                (segment[6:8], "seconds", 60, True),
            ]

        # HHMMSS[.ssssss]
        case _, _, _, _, _, _, ".", *_:
            return [
                (segment[0:2], "hours", 24, True),
                (segment[2:4], "minutes", 60, True),
                (segment[4:13], "seconds", 60, False),
            ]

        # HHMMSS
        case _, _, _, _, _, _:
            return [
                (segment[0:2], "hours", 24, True),
                (segment[2:4], "minutes", 60, True),
                (segment[4:6], "seconds", 60, True),
            ]

        case _:
            raise ValueError(f"unable to parse '{segment}' into time components")


def _parse_designators(duration: str) -> Components:
    result = []
    date_context = iter((("Y", "years"), ("M", "months"), ("D", "days")))
    context, value, unit = date_context, "", None

    for char in duration:
        if char in {",", ".", "0", "1", "2", "3", "4", "5", "6", "7", "8", "9"}:
            value += char if char.isdigit() else "."
            continue

        if char == "T" and context is date_context:
            assert not value, f"missing unit designator after '{value}'"
            context = iter((("H", "hours"), ("M", "minutes"), ("S", "seconds")))
            continue

        if char == "W":
            assert not unit, "cannot mix weeks with other units"
            context = iter((("W", "weeks"),))
            pass

        for delimiter, unit in context:
            if char == delimiter:
                result.append((value, unit, None, False))
                value = ""
                break
        else:
            raise ValueError(f"unexpected character '{char}'")

    assert unit, "no measurements found"
    return result


def _parse_duration(duration: str) -> Components:
    assert duration.startswith("P"), "durations must begin with the character 'P'"

    if duration[-1].isupper():
        return _parse_designators(duration[1:])
    else:
        date_segment, _, time_segment = duration[1:].partition("T")
        result = []
        result.extend(_parse_date(date_segment) if date_segment else [])
        result.extend(_parse_time(time_segment) if time_segment else [])
        return result


def _to_measurements(components: Components) -> Measurements:
    result = []
    for value, unit, limit, integer_only in components:
        assert (
            value.isdigit() if integer_only else value[0:1].isdigit()
        ), f"unable to parse '{value}' as a positive number"
        quantity = float(value)
        if limit is None:
            assert quantity >= 0, f"{unit} value of {value} exceeds range [0..+\u221e)"
        elif limit in (24, 60):
            assert 0 <= quantity < limit, f"{unit} value of {value} exceeds range [0..{limit})"
        else:
            assert 0 <= quantity <= limit, f"{unit} value of {value} exceeds range [0..{limit}]"
        if quantity:
            result.append((unit, quantity))
    return result


def timedelta_from_isoformat(duration: str) -> timedelta:
    """Converts a ISO 8601 duration string to a timedelta."""
    try:
        components = _parse_duration(duration)
        measurements = _to_measurements(components)
        td = timedelta(**dict(measurements))
        return td
    except (AssertionError, ValueError) as exc:
        raise ValueError(f"could not parse duration '{duration}': {exc}") from exc


def timedelta_to_isoformat(td: timedelta) -> str:
    """Converts a timedelta to an ISO 8601 duration string."""
    if not td:
        return "P0D"

    days = td.days
    minutes, seconds = divmod(td.seconds, 60)
    hours, minutes = divmod(minutes, 60)
    if td.microseconds:
        seconds += td.microseconds / 1_000_000

    result = "P"
    if days:
        result += f"{days}D"

    if hours or minutes or seconds:
        result += "T"
        if hours:
            result += f"{hours}H"
        if minutes:
            result += f"{minutes}M"
        if seconds:
            result += f"{seconds:.6f}".rstrip("0").rstrip(".") + "S"

    return result
