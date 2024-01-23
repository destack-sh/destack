import pytest

from bench.language.node import BenchPath, AmbiguousBenchPath, InvalidBenchPath


@pytest.mark.parametrize(
    ("path_str", "expected"),
    [
        (
            "flotothemoon/Mirror/Notion/Databases/Landscape",
            BenchPath("flotothemoon", ("Mirror", "Notion", "Databases")),
        ),
        (
            "flotothemoon/Applications/Birdy/MainScreen:Dashboard/Big Graphs/Graph1.name",
            BenchPath(
                "flotothemoon",
                ("Applications", "Birdy", "MainScreen"),
                ("Dashboard", "Big Graphs", "Graph1"),
                ("name",),
            ),
        ),
        (
            "flotothemoon/Sandbox/Sales/Scraping/WebsiteSamples/Replit.value.document.title",
            BenchPath(
                "flotothemoon",
                ("Sandbox", "Sales", "Pipeline", "Scraping", "WebsiteSamples"),
                ("Replit",),
                ("value", "document", "title"),
            ),
        ),
        (
            "flotothemoon.name",
            BenchPath("flotothemoon", field_path=("name",)),
        ),
        (
            "flotothemoon-tests/Tests/Databases/TestPopulate.code",
            BenchPath(
                "flotothemoon-tests", ("Tests", "Databases", "TestPopulate"), field_path=("code",)
            ),
        ),
        (
            "../../Header Screen:Header/Title.theme.primary.color",
            BenchPath(
                None,
                ("..", "..", "Header Screen"),
                ("Header", "Title"),
                ("theme", "primary", "color"),
            ),
        ),
        (
            "../../../../Graphs",
            AmbiguousBenchPath,
        ),
        (
            "../../Something/../SomethingElse",
            InvalidBenchPath,
        ),
    ],
)
def test_bench_path(path_str: str, expected: BenchPath | ValueError):
    if isinstance(expected, type) and issubclass(expected, Exception):
        with pytest.raises(expected):
            BenchPath.parse(path_str)
    else:
        assert BenchPath.parse(path_str) == expected
