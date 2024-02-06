import pytest

from bench.language.const import NodeType
from bench.language.node import BenchPath, InvalidBenchPath
from bench.language.test.fabricator import Fabricator


def test_set_non_existing_property(fabricator: "Fabricator"):
    client = fabricator.fabricate(NodeType.CLIENT)
    with pytest.raises(AttributeError):
        client.wadabadaboo = "wadabadaboo"


@pytest.mark.parametrize(
    ("path_str", "expected"),
    [
        (
            "flotothemoon/Mirror/Notion/Databases/Landscape",
            BenchPath("flotothemoon", ("Mirror", "Notion", "Databases", "Landscape")),
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
            "flotothemoon/Private/Sales/Scraping/WebsiteSamples/Replit.value.document.title",
            BenchPath(
                "flotothemoon",
                ("Private", "Sales", "Scraping", "WebsiteSamples", "Replit"),
                field_path=("value", "document", "title"),
            ),
        ),
        (
            "flotothemoon",
            BenchPath(
                "flotothemoon",
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
        (".", BenchPath(None, (".",))),
        ("./..", BenchPath(None, (".", ".."))),
        ("../.././../.", BenchPath(None, ("..", "..", ".", "..", "."))),
        ("..", BenchPath(None, ("..",))),
        ("../../../../Graphs", BenchPath(None, ("..", "..", "..", "..", "Graphs"))),
        ("../../Something/../SomethingElse", InvalidBenchPath),
        ("../", InvalidBenchPath),
        ("..//.", InvalidBenchPath),
        ("flotothemoon.", InvalidBenchPath),
        ("flotothemoon.name.", InvalidBenchPath),
        ("", InvalidBenchPath),
    ],
)
def test_bench_path(path_str: str, expected: BenchPath | ValueError):
    if isinstance(expected, type) and issubclass(expected, Exception):
        with pytest.raises(expected):
            BenchPath.parse(path_str)
    else:
        actual = BenchPath.parse(path_str)
        assert actual == expected
