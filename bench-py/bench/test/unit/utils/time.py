import pytest
from bench.utils.time import timedelta_from_isoformat, timedelta_to_isoformat


@pytest.mark.parametrize(
    "iso",
    [
        "P0D",
        "P1D",
        "PT1H",
        "PT1M",
        "PT1S",
        "P1W",
        "P1W1D",
        "P2W",
        "-PT1H",
        "P1DT12H",
        "PT12H30M",
        "P1DT12H30M15S",
        "-PT12H30M15S",
        "PT1H1M1.123456S",
        "PT12.345678S",
    ],
)
def test_isoformat_roundtrip(iso: str):
    parsed_td = timedelta_from_isoformat(iso)
    rendered_iso = timedelta_to_isoformat(parsed_td)
    assert iso == rendered_iso, f"{iso} != {rendered_iso}"

    roundtrip_td = timedelta_from_isoformat(rendered_iso)
    assert parsed_td == roundtrip_td, f"{parsed_td} != {roundtrip_td} {iso}"
    assert timedelta_to_isoformat(parsed_td) == rendered_iso, f"'{iso}' != '{rendered_iso}'"
