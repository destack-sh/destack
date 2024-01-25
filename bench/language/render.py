import enum
from typing import Any, Collection, Generator, Mapping, NamedTuple, Optional
from uuid import UUID

from more_itertools import first

from bench.language.const import INTERP_NODE_TYPES, NodeType
from bench.language.node import UNSET, Node
from bench.language.text import Text, render_text_simple
from bench.utils.utils import format_python, omit_empty


def render(
    *nodes: Node,
    target="python",
    record_limit: int = 20,
    recursive: bool = True,
) -> Optional[str]:
    """
    Renders edits or nodes to code in a language.
    Nodes are coerced into create edits with all descendants.
    """
    from bench.language.database import HasDatabase

    if not isinstance(nodes, list):
        nodes = [nodes]
    if not nodes:
        return None
    all_nodes = []
    seen_node_cks: set[UUID] = set()
    for node in nodes:
        node: Node
        if recursive:
            descendants = list(node._walk_rec())
            # add records to descendants for databases
            # (this only works when called synchronously)
            if HasDatabase in node._components:
                descendants.extend(node.records.first(record_limit))
        else:
            descendants = [node]
        for n in descendants:
            if n.ck in seen_node_cks or n.metatype in INTERP_NODE_TYPES:
                continue
            seen_node_cks.add(n.ck)
            all_nodes.append(n)

    # render
    if target == "python":
        return render_as_python(all_nodes)
    else:
        raise ValueError(f"cannot render to {target}")


def DEFAULT_VALUE_FILTER(f):
    return True


def _render_prop(node: Node, name: str, value: Any) -> str:
    """
    Render a non-relational prop (may be a reference, but not a parent/child relation).
    TODO @Broken: _render_prop recursively with all nodes/structs (blobs, secrets, etc. see typing)
    """
    from bench.language.value import render_value
    from bench.language.value import HasValue

    if value is None:
        return "None"
    elif isinstance(value, UUID):
        return f'UUID("{value}")'
    elif isinstance(value, (enum.StrEnum, enum.IntEnum)):
        return f"{type(value).__name__}.{value.name}"
    elif isinstance(value, (enum.IntFlag,)):
        # reconstitute flags as a | b | c
        return " | ".join(f"{type(value).__name__}.{v.name}" for v in type(value) if value & v)
    elif isinstance(value, Node):
        return f"'{value.name}'"  # this isn't quite right, may be shadowed/scoped
    elif isinstance(value, (int, float, bool)):
        return repr(value)
    elif isinstance(value, (str, Text)):
        # render text into simple form
        if isinstance(value, Text):
            value = render_text_simple(value.spans)
        elif name == "text" and value and node._text_spans:
            value = render_text_simple(node._text_spans)
        # if it contains newlines transform into multiline string
        # and escape any multiline strings inside
        if "\n" in value:
            value = value.replace('"""', '\\"\\"\\"')
            return f'"""\\\n{value}"""'
        else:
            value = value.replace('"', '\\"')
            return repr(value)
    elif name == "value" and HasValue in node._components and isinstance(value, Mapping):
        value = render_value(
            value,
            node.metatype_of_value,
            get_k=lambda f: f.py_ident,
            filter_v=DEFAULT_VALUE_FILTER,
            ignore_array=True,
        )
        return omit_empty(value)
    elif hasattr(type(value), "to_python"):
        return type(value).to_python(value)
    else:
        raise ValueError(f"cannot render {value!r} (for {node!r}->{name})")


def _sep(*strs) -> str:
    strs = list(strs)
    if strs and isinstance(strs[0], Generator):
        strs = list(strs[0])
    if strs and isinstance(strs[0], (list, tuple)):
        strs = list(strs[0])
    return ", ".join(str(s) for s in strs if s)


class _NodeInit(NamedTuple):
    node: Node
    name: str
    args: dict
    kwargs: dict


class _OpType(enum.StrEnum):
    ASSIGN = "="
    CREATE = "create"
    APPEND = "append"


class _Op(NamedTuple):
    target: str
    op: _OpType
    nodes: list[_NodeInit]


def render_as_python(nodes: Collection[Node]) -> Optional[str]:
    """
    Generate minimal(ish) Python code that produces the given edits.
    The returned order matches the given order of edits, i.e. no dependencies are considered.
    (This should be fine since edits for node subtrees are produced top-down.)
    """
    if not nodes:
        return None

    # index nodes to find roots
    nodes_by_ck: dict[UUID, Node] = {}
    for node in nodes:
        assert isinstance(node, Node), f"cannot render data {node!r}"
        nodes_by_ck[node.ck] = node

    # render single edits into 'lines' (target, op, node)
    ops: list[_Op] = []
    for node in nodes:
        # get props to create
        init_props = {
            prop.name: getattr(node, prop.name)
            for prop in node.__properties__.values()
            if not prop.is_runtime_only
            and not prop.is_tree_relation
            and prop.id >= 30
            and prop.name not in ("id", "ck", "parent", "order_key", "dynamic_key")
            and getattr(node, prop.name, UNSET) is not prop.default
        }

        # simplify props
        if hasattr(type(node), "to_python"):
            init_name, init_args, init_kwargs = type(node).to_python(node, init_props, node.parent)
            init_node = _NodeInit(node, init_name, init_args, init_kwargs)
        else:
            init_node = _NodeInit(node, type(node).__name__, {}, init_props)
        del init_props

        # render as define (root) or create/append (child)
        if node.parent and node.parent.ck in nodes_by_ck:
            attach_to_prop = first(
                p for p in node.parent.__list_properties_by_child__[node.metatype]
            )
            parent_str = f"{node.parent.py_ident}.{attach_to_prop.name}"
            if node.metatype in (NodeType.RECORD, NodeType.TAGGING, NodeType.TRIGGER):
                op = _Op(parent_str, _OpType.CREATE, [init_node])
            else:
                op = _Op(parent_str, _OpType.APPEND, [init_node])
        else:
            op = _Op(node.py_ident, _OpType.ASSIGN, [init_node])
        ops.append(op)

    # merge successive ops (if they can be combined like create/append)
    merged: list[_Op] = []
    for op in ops:
        if merged and merged[-1].target == op.target and merged[-1].op == op.op:
            merged[-1].nodes.extend(op.nodes)
        else:
            merged.append(op)

    # render 'ops' into code
    lines = []
    for target, op, nodes in merged:
        if op == "=":
            n, init_name, init_args, init_kwargs = nodes[0]
            init_kwargs = {**init_args, **init_kwargs}
            kwargs_str = _sep(f"{k}={_render_prop(n, k, v)}" for k, v in init_kwargs.items() if v)
            if target:
                lines.append(f"{target} = {init_name}({kwargs_str})")
            else:  # isn't this an error case?
                lines.append(f"{init_name}({kwargs_str})")
            continue

        # stringify each node
        nodes_strs = []
        for node in nodes:
            n, init_name, init_args, init_kwargs = node
            # inline record value (see Record.new)
            if n.metatype == NodeType.RECORD:
                from bench.language.value import render_value

                kwargs_str = _sep(
                    f"{k}={render_value(v, n.metatype_of_value.fields.get(k), filter_v=DEFAULT_VALUE_FILTER)}"
                    for k, v in n.value.items()
                    if v and DEFAULT_VALUE_FILTER(v, n.metatype_of_value.fields.get(k))
                )
                nodes_strs.append(f"Record.new({_sep(kwargs_str)})")
                continue

            if op == _OpType.CREATE and len(nodes) > 1:
                # merge args into kwargs
                init_kwargs = {**init_args, **init_kwargs}
                init_args.clear()

            # render args strs in reverse as soon as a value is set
            args_strs = []
            for k, v in reversed(init_args.items()):
                if v or args_strs:
                    args_strs.append(_render_prop(n, k, v))
            args_str = _sep(*reversed(args_strs))
            kwargs_str = _sep(f"{k}={_render_prop(n, k, v)}" for k, v in init_kwargs.items() if v)
            if op == _OpType.CREATE and len(nodes) == 1:
                nodes_strs.append(f"{_sep(args_str, kwargs_str)}")
            else:
                nodes_strs.append(f"{init_name}({_sep(args_str, kwargs_str)})")

        # join them into merged line
        nodes_str = _sep(nodes_strs)
        if op == _OpType.CREATE and len(nodes) == 1:
            lines.append(f"{target}.create({nodes_str})")
        else:
            if len(nodes) == 1:
                lines.append(f"{target}.append({nodes_str})")
            else:
                lines.append(f"{target}.extend({nodes_str})")

    # format with black
    code = "\n".join(lines)
    code = format_python(code)
    return code
