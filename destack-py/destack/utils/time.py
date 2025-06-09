from datetime import timedelta
from typing import TypeAlias

_DurationParse: TypeAlias = list[tuple[str, str, int | None, bool]]
_DurationUnits: TypeAlias = list[tuple[str, float]]


def _parse_date(segment: str) -> _DurationParse:
    match tuple(segment):
        # YYYY-DDD
        case _, _, _, _, "-", _, _, _:
            return [
                (segment[0:4], "years", None, True),
                (segment[5:8], "days", 366, True),
            ]

        # YYYY-MM-DD
        case _, _, _, _, "-", _, _, "-", _, _:
            return [
                (segment[0:4], "years", None, True),
                (segment[5:7], "months", 12, True),
                (segment[8:10], "days", 31, True),
            ]

        # YYYYDDD
        case _, _, _, _, _, _, _:
            return [
                (segment[0:4], "years", None, True),
                (segment[4:7], "days", 366, True),
            ]

        # YYYYMMDD
        case _, _, _, _, _, _, _, _:
            return [
                (segment[0:4], "years", None, True),
                (segment[4:6], "months", 12, True),
                (segment[6:8], "days", 31, True),
            ]

        case _:
            raise ValueError(f"unable to parse '{segment}' into date parts")


def _parse_time(segment: str) -> _DurationParse:
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
            raise ValueError(f"unable to parse '{segment}' into time parts")


def _parse_designators(duration: str) -> _DurationParse:
    result = []
    date_context = iter((("Y", "years"), ("M", "months"), ("W", "weeks"), ("D", "days")))
    context = date_context
    value = ""
    unit = None

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
            result.append((value, "weeks", None, False))
            value = ""
            continue

        for delimiter, unit in context:
            if char == delimiter:
                result.append((value, unit, None, False))
                value = ""
                break
        else:
            raise ValueError(f"unexpected character '{char}'")

    if value:
        raise ValueError(f"missing unit designator after '{value}'")

    assert result, "no units found"
    return result


def _parse_duration(duration: str) -> _DurationParse:
    assert duration.startswith("P"), "durations must begin with the character 'P'"

    if duration[-1].isupper():
        parts = _parse_designators(duration[1:])
    else:
        date_segment, _, time_segment = duration[1:].partition("T")
        parts: _DurationParse = []
        if date_segment:
            parts.extend(_parse_date(date_segment))
        if time_segment:
            parts.extend(_parse_time(time_segment))

    return parts


def _to_units(parts: _DurationParse) -> _DurationUnits:
    result = []
    for value, unit, limit, integer_only in parts:
        if value.startswith("-"):
            numeric_value = value[1:]
            sign = -1
        else:
            numeric_value = value
            sign = 1

        assert (
            numeric_value.isdigit() if integer_only else numeric_value.replace(".", "", 1).isdigit()
        ), f"unable to parse '{value}' as a positive number"

        quantity = float(numeric_value) * sign

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
    """Converts an ISO 8601 duration string to a timedelta."""
    try:
        if duration.startswith("-"):
            duration = duration[1:]
            sign = -1
        else:
            sign = 1
        parts = _parse_duration(duration)
        units = _to_units(parts)
        units = dict(units)
        if sign == -1:
            for key, value in units.items():
                units[key] = -value
        td = timedelta(**units)
        return td
    except (AssertionError, ValueError) as exc:
        raise ValueError(f"could not parse duration '{duration}': {exc}") from exc


def timedelta_to_isoformat(td: timedelta) -> str:
    """Converts a timedelta to an ISO 8601 duration string."""
    if td == timedelta(0):
        return "P0D"

    sign = "-" if td.total_seconds() < 0 else ""
    td = abs(td)

    weeks, remainder = divmod(td.days, 7)
    days = remainder
    hours, remainder = divmod(td.seconds, 3600)
    minutes, seconds = divmod(remainder, 60)
    if td.microseconds:
        seconds += td.microseconds / 1_000_000

    result = "P"
    if weeks:
        result += f"{weeks}W"
    if days:
        result += f"{days}D"

    if hours or minutes or seconds:
        result += "T"
        if hours:
            result += f"{hours}H"
        if minutes:
            result += f"{minutes}M"
        if seconds:
            # Remove trailing zeros and decimal points
            seconds_str = f"{seconds:.9f}".rstrip("0").rstrip(".")
            result += f"{seconds_str}S"

    return sign + result
