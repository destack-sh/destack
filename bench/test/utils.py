from typing import Collection

from bench.language import Graph, NodeType


def assert_graph_equals(
    graph_a: Graph, graph_b: Graph, ignore_node_types: Collection[NodeType] = ()
):
    """Checks that two graphs are completely equal."""
    for node_a in graph_a.nodes:
        if node_a.metatype in ignore_node_types:
            continue
        assert graph_b.get(node_a.id) is not None, f"missing node {node_a!r} in {graph_b!r}"
        node_b = graph_b.get_or_error(node_a.id)
        assert node_a == node_b, f"node {node_a!r} != {node_b!r}"
        assert node_a.equals(node_b), f"node {node_a!r} != {node_b!r}"
    for node_b in graph_b.nodes:
        if node_b.metatype in ignore_node_types:
            continue
        assert graph_a.get(node_b.id) is not None, f"missing node {node_b!r} in {graph_a!r}"
        node_a = graph_a.get_or_error(node_b.id)
        assert node_a == node_b, f"node {node_a!r} != {node_b!r}"
        assert node_a.equals(node_b), f"node {node_a!r} != {node_b!r}"
