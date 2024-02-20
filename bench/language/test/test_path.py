import pytest

from bench.language.path import Path, InvalidBenchPath


@pytest.mark.skip("@Incomplete :BenchPath")
@pytest.mark.parametrize(
    ("path_str", "expected"),
    [],
)
def test_bench_path(path_str: str, expected: Path | ValueError):
    if isinstance(expected, type) and issubclass(expected, Exception):
        with pytest.raises(expected):
            Path.parse(path_str)
    else:
        actual = Path.parse(path_str)
        assert actual == expected
