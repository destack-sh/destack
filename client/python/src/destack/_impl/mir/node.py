from __future__ import annotations


def local_node_id_matches(left, right) -> bool:
    """Return whether two local node ids refer to the same MIR node."""

    return left.id == right.id


def get_local_node_value(mapping, key):
    """Return one map value keyed by local node id."""

    return mapping.get(key)
