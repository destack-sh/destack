from collections.abc import Sequence

from .core import RustFile, RustImport, RustItem, RustItemMode, RustItemScope


def _dedent_block(lines: Sequence[str]) -> str:
    """
    Remove common leading indentation from non-empty lines and join.

    Keep internal newlines exactly as provided, but normalize indentation so the
    first non-empty line starts at column 0.
    """
    # find minimal indent among non-empty lines
    min_indent: int | None = None
    for line in lines:
        if not line.strip():
            continue
        indent = len(line) - len(line.lstrip(" \t"))
        if min_indent is None or indent < min_indent:
            min_indent = indent
    if min_indent is None:
        return "\n".join(lines).strip("\n")
    dedented: list[str] = [line[min_indent:] if line.strip() else "" for line in lines]
    # preserve trailing newline style by not forcing a newline at the end
    return "\n".join(dedented).strip("\n")


def _parse_attr(line: str) -> tuple[RustItemMode, str, str, RustItemScope] | None:
    """
    Parse a destack attribute line like:
    #[destack::owned(Vector2, struct, block)]
    #[destack::partial(Vector2, impl, block)]
    #[destack::owned(Vector2, ZERO, line)]
    #[destack::stub(Vector2, x, function_stub)]
    """
    stripped = line.strip()
    if not stripped.startswith("#[destack::"):
        return None
    if not stripped.endswith(")]"):
        return None
    # extract inside of #[destack:: ... ]
    try:
        inside = stripped[len("#[destack::") : -1]
        mode_part, args_part = inside.split("(", 1)
        args_part = args_part.rstrip(")")
        mode = RustItemMode(mode_part)
        # split by commas, expecting exactly 3 args
        raw_args = [a.strip() for a in args_part.split(",")]
        if len(raw_args) != 3:
            return None
        object_key, inner_key, scope_raw = raw_args
        scope: RustItemScope
        if scope_raw == "block":
            scope = RustItemScope.BLOCK
        elif scope_raw == "line":
            scope = RustItemScope.LINE
        else:
            # treat any other well-formed token (e.g., function_stub) as block content
            scope = RustItemScope.BLOCK
        return (mode, object_key, inner_key, scope)
    except Exception:
        return None


def _collect_block(lines: Sequence[str], start_index: int) -> tuple[int, list[str]]:
    """
    Collect a syntactic block starting at start_index.

    Start at start_index (inclusive) and read lines until the matching braces close.
    Returns the index of the last consumed line and the collected lines.
    """
    collected: list[str] = []
    depth = 0
    seen_open = False
    index = start_index
    while index < len(lines):
        line = lines[index]
        collected.append(line)
        # count braces in this line
        for ch in line:
            if ch == "{":
                depth += 1
                seen_open = True
            elif ch == "}":
                depth -= 1
        if seen_open and depth == 0:
            break
        index += 1
    return index, collected


def _find_next_nonempty(lines: Sequence[str], start_index: int) -> int:
    index = start_index
    while index < len(lines) and not lines[index].strip():
        index += 1
    return index


def _skip_attribute_lines(lines: Sequence[str], start_index: int) -> int:
    index = _find_next_nonempty(lines, start_index)
    while index < len(lines):
        stripped = lines[index].lstrip()
        if stripped.startswith("#["):
            index = _find_next_nonempty(lines, index + 1)
            continue
        break
    return index


def parse_rust_imports(source: str) -> list[RustImport]:
    """Parse simple Rust `use` imports at the top level."""
    imports: list[RustImport] = []
    for raw_line in source.splitlines():
        line = raw_line.strip()
        if not line.startswith("use "):
            continue
        # remove trailing ';'
        assert line.endswith(";"), "expected ';' at end of use statement"
        use_body = line[len("use ") : -1].strip()
        if "::{" in use_body and use_body.endswith("}"):
            path, names = use_body.split("::{", 1)
            names = names[:-1]
            imports_list = [n.strip() for n in names.split(",") if n.strip()]
            imports.append(RustImport(path=path, imports=imports_list))
        else:
            imports.append(RustImport(path=use_body, imports=[]))
    return imports


def parse_rust_items(source: str) -> list[RustItem]:
    """
    Parse destack-annotated items from a Rust source file.

    Supports generated, partial and stub items, with line or block scopes.
    For block items, children are parsed recursively from within the block.
    """
    lines = source.splitlines()
    items: list[RustItem] = []
    i = 0
    while i < len(lines):
        attr = _parse_attr(lines[i])
        if attr is None:
            i += 1
            continue
        mode, object_key, inner_key, scope = attr
        # content starts at next non-empty line
        start = _find_next_nonempty(lines, i + 1)
        if scope == RustItemScope.LINE:
            assert start < len(lines), "expected content line after attribute"
            content = _dedent_block([lines[start]])
            item = RustItem(
                object_key=object_key,
                inner_key=inner_key,
                mode=mode,
                scope=scope,
                children=[],
                content=content,
            )
            items.append(item)
            i = start + 1
        else:
            # skip attribute lines like #[derive(...)] before the block header
            start_no_attrs = _skip_attribute_lines(lines, start)
            # collect until matching brace closes beginning at the block header
            end, collected = _collect_block(lines, start_no_attrs)
            content = _dedent_block(collected)
            # children: parse recursively inside the block's content
            inner_children = parse_rust_items("\n".join(collected))
            item = RustItem(
                object_key=object_key,
                inner_key=inner_key,
                mode=mode,
                scope=scope,
                children=inner_children,
                content=content,
            )
            items.append(item)
            i = end + 1
    return items


def parse_rust_file(source: str, path: str = "") -> RustFile:
    """Parse a Rust file into a `RustFile` with annotated items only."""
    return RustFile(path=path, items=parse_rust_items(source))
