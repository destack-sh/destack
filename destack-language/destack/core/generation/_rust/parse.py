from collections.abc import Sequence

from .core import (
    RustCustomItem,
    RustFile,
    RustImport,
    RustItem,
    RustItemKind,
    RustItemScope,
    RustManagedItem,
    RustMod,
)


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


def _parse_attr(line: str) -> tuple[RustItemKind, str, str, RustItemScope] | None:
    """
    Parse a destack attribute line like:
    #[destack::synthetic(Vector2, struct, block)]
    #[destack::partial(Vector2, impl, block)]
    #[destack::synthetic(Vector2, ZERO, line)]
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
        mode = RustItemKind(mode_part)
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
    Collect a syntactic block starting at start_index, robustly.

    Start at start_index (inclusive) and read lines until the matching braces close.
    Ignore braces found inside string and character literals, raw strings,
    and both line and block comments. Block comments may be nested.

    Returns the index of the last consumed line and the collected lines.
    """
    collected: list[str] = []
    depth = 0
    seen_open = False
    index = start_index

    in_block_comment = 0  # nesting depth
    in_string = False
    in_char = False
    raw_string_hashes: int | None = None

    while index < len(lines):
        line = lines[index]
        collected.append(line)
        i = 0
        line_len = len(line)
        in_line_comment = False
        while i < line_len:
            ch = line[i]
            nxt = line[i + 1] if i + 1 < line_len else ""

            # end of line comment at newline, handled by resetting each line
            if in_line_comment:
                break

            # handle end/start of block comments
            if in_block_comment > 0:
                if ch == "/" and i > 0 and line[i - 1] == "*":
                    in_block_comment -= 1
                elif ch == "/" and nxt == "*":
                    in_block_comment += 1
                    i += 1
                i += 1
                continue

            # string literal handling
            if in_string:
                if raw_string_hashes is None:
                    # normal string, end on unescaped "
                    if ch == "\\":
                        i += 2
                        continue
                    if ch == '"':
                        in_string = False
                else:
                    # raw string: end when '"' followed by exact number of '#'
                    if ch == '"':
                        j = i + 1
                        count = 0
                        while j < line_len and line[j] == "#":
                            count += 1
                            j += 1
                        if count == raw_string_hashes:
                            in_string = False
                            raw_string_hashes = None
                            i = j - 1
                i += 1
                continue

            # char literal handling
            if in_char:
                if ch == "\\":
                    i += 2
                    continue
                if ch == "'":
                    in_char = False
                i += 1
                continue

            # not inside comments/strings: detect comment starts
            if ch == "/" and nxt == "/":
                in_line_comment = True
                break
            if ch == "/" and nxt == "*":
                in_block_comment += 1
                i += 2
                continue

            # detect raw string start: r" or r#"
            if ch == "r":
                j = i + 1
                hash_count = 0
                while j < line_len and line[j] == "#":
                    hash_count += 1
                    j += 1
                if j < line_len and line[j] == '"':
                    in_string = True
                    raw_string_hashes = hash_count
                    i = j + 1
                    continue
            # normal string start
            if ch == '"':
                in_string = True
                raw_string_hashes = None
                i += 1
                continue
            # char literal start
            if ch == "'":
                in_char = True
                i += 1
                continue

            # count braces
            if ch == "{":
                depth += 1
                seen_open = True
            elif ch == "}":
                depth -= 1

            i += 1

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


def _collect_block_simple(lines: Sequence[str], start_index: int) -> tuple[int, list[str]]:
    """
    Collect a block starting at start_index using simple brace counting.

    This version does not attempt to skip strings/comments and is suitable
    for well-formed generated/managed headers.
    """
    collected: list[str] = []
    depth = 0
    seen_open = False
    index = start_index
    while index < len(lines):
        line = lines[index]
        collected.append(line)
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


def _find_first_token_outside_literals(
    lines: Sequence[str], start_index: int, tokens: set[str], max_lookahead_lines: int = 50
) -> tuple[int, int, str] | None:
    """Scan forward and find the first token character outside comments/strings."""
    index = start_index
    looked = 0

    in_block_comment = 0
    while index < len(lines) and looked < max_lookahead_lines:
        line = lines[index]
        i = 0
        line_len = len(line)
        in_line_comment = False
        in_string = False
        in_char = False
        raw_string_hashes: int | None = None
        while i < line_len:
            ch = line[i]
            nxt = line[i + 1] if i + 1 < line_len else ""

            if in_line_comment:
                break
            if in_block_comment > 0:
                if ch == "/" and i > 0 and line[i - 1] == "*":
                    in_block_comment -= 1
                elif ch == "/" and nxt == "*":
                    in_block_comment += 1
                    i += 1
                i += 1
                continue
            if in_string:
                if raw_string_hashes is None:
                    if ch == "\\":
                        i += 2
                        continue
                    if ch == '"':
                        in_string = False
                else:
                    if ch == '"':
                        j = i + 1
                        count = 0
                        while j < line_len and line[j] == "#":
                            count += 1
                            j += 1
                        if count == raw_string_hashes:
                            in_string = False
                            raw_string_hashes = None
                            i = j - 1
                i += 1
                continue
            if in_char:
                if ch == "\\":
                    i += 2
                    continue
                if ch == "'":
                    in_char = False
                i += 1
                continue

            if ch == "/" and nxt == "/":
                in_line_comment = True
                break
            if ch == "/" and nxt == "*":
                in_block_comment += 1
                i += 2
                continue
            if ch == "r":
                j = i + 1
                hash_count = 0
                while j < len(line) and line[j] == "#":
                    hash_count += 1
                    j += 1
                if j < len(line) and line[j] == '"':
                    in_string = True
                    raw_string_hashes = hash_count
                    i = j + 1
                    continue
            if ch == '"':
                in_string = True
                raw_string_hashes = None
                i += 1
                continue
            if ch == "'":
                in_char = True
                i += 1
                continue

            if ch in tokens:
                return (index, i, ch)
            i += 1
        index += 1
        looked += 1
    return None


def _is_internal_import_path(path: str) -> bool:
    """Return True if a `use` path refers to internal crate paths."""
    return path.startswith("crate::") or path.startswith("self::") or path.startswith("super::")


def parse_rust_imports(source: str) -> list[RustImport]:
    """Parse simple Rust `use` imports at the top level."""
    imports: list[RustImport] = []
    for raw_line in source.splitlines():
        stripped = raw_line.strip()
        if not (stripped.startswith("use ") or stripped.startswith("pub use ")):
            continue
        # remove trailing ';'
        assert stripped.endswith(";"), "expected ';' at end of use statement"
        is_public = False
        if stripped.startswith("pub use "):
            is_public = True
            use_body = stripped[len("pub use ") : -1].strip()
        else:
            use_body = stripped[len("use ") : -1].strip()

        is_glob = use_body.endswith("::*")
        if "::{" in use_body and use_body.endswith("}"):
            path, names = use_body.split("::{", 1)
            names = names[:-1]
            imports_list = [n.strip() for n in names.split(",") if n.strip()]
            imports.append(
                RustImport(
                    path=path,
                    imports=imports_list,
                    is_internal=_is_internal_import_path(path),
                    is_public=is_public,
                    is_glob=False,
                )
            )
        else:
            imports.append(
                RustImport(
                    path=use_body,
                    imports=[] if not is_glob else ["*"],
                    is_internal=_is_internal_import_path(use_body),
                    is_public=is_public,
                    is_glob=is_glob,
                )
            )
    return imports


def parse_rust_mods(source: str) -> list[RustMod]:
    """Parse simple module declarations like `mod foo;` or `pub mod foo;`."""
    mods: list[RustMod] = []
    for raw_line in source.splitlines():
        stripped = raw_line.strip()
        if not (stripped.startswith("mod ") or stripped.startswith("pub mod ")):
            continue
        assert stripped.endswith(";"), "expected ';' at end of mod statement"
        is_public = stripped.startswith("pub mod ")
        name = (
            stripped[len("pub mod ") : -1].strip()
            if is_public
            else stripped[len("mod ") : -1].strip()
        )
        if not name:
            continue
        mods.append(RustMod(children=[], content=stripped[:-1], name=name, is_public=is_public))
    return mods


def parse_rust_items(source: str) -> list[RustItem]:
    """
    Parse destack-annotated and custom items from a Rust source string.

    Supports generated, partial and stub items, with line or block scopes.
    For block items, children are parsed recursively from within the block body.
    Any non-empty, non-annotated code is captured as `RustCustomItem` in order
    to preserve handwritten code across regenerations.
    """
    lines = source.splitlines()
    items: list[RustItem] = []
    i = 0
    while i < len(lines):
        # skip blank lines up front
        if not lines[i].strip():
            i += 1
            continue

        # treat single-line comments as standalone custom items to avoid
        # accidentally swallowing following managed items
        if lines[i].lstrip().startswith("//"):
            items.append(RustCustomItem(children=[], content=_dedent_block([lines[i]])))
            i += 1
            continue

        attr = _parse_attr(lines[i])
        if attr is not None:
            mode, object_key, inner_key, scope = attr
            # content starts at next non-empty line
            start = _find_next_nonempty(lines, i + 1)
            if scope == RustItemScope.LINE:
                assert start < len(lines), "expected content line after attribute"
                content = _dedent_block([lines[start]])
                item = RustManagedItem(
                    object_key=object_key,
                    inner_key=inner_key,
                    kind=mode,
                    scope=scope,
                    children=[],
                    content=content,
                )
                items.append(item)
                i = start + 1
                continue
            # block: skip any attributes like #[derive(...)] before the header
            start_no_attrs = _skip_attribute_lines(lines, start)
            # managed headers are well-formed; simple brace counting is sufficient
            end, collected = _collect_block_simple(lines, start_no_attrs)
            content = _dedent_block(collected)
            # children: parse recursively inside the block body (exclude header and closing brace)
            body_lines = collected[1:-1] if len(collected) >= 2 else []
            inner_children = parse_rust_items("\n".join(body_lines)) if body_lines else []
            item = RustManagedItem(
                object_key=object_key,
                inner_key=inner_key,
                kind=mode,
                scope=scope,
                children=inner_children,
                content=content,
            )
            items.append(item)
            i = end + 1
            continue

        # custom item: capture contiguous meaningful content
        stripped = lines[i].lstrip()
        # collect leading non-destack attributes for custom items
        collected_custom: list[str] = []
        j = i
        while j < len(lines):
            s = lines[j].lstrip()
            if s.startswith("#[") and not s.startswith("#[destack::"):
                collected_custom.append(lines[j])
                j += 1
                continue
            break
        # after collecting attributes, find next non-empty code line
        code_index = _find_next_nonempty(lines, j)
        if code_index >= len(lines):
            # only orphan attributes remain; record them as a custom item
            if collected_custom:
                content = _dedent_block(collected_custom)
                items.append(RustCustomItem(children=[], content=content))
            i = code_index
            continue
        # if the next code is a destack attribute, let the main loop handle it
        if _parse_attr(lines[code_index]) is not None:
            i = code_index
            continue

        # decide whether this is a block or a line
        header_line = lines[code_index]
        token = _find_first_token_outside_literals(lines, code_index, {"{", ";"})
        if token is not None and token[2] == "{":
            header_start = token[0]
            end, block_lines = _collect_block(lines, header_start)
            combined = collected_custom + block_lines if collected_custom else block_lines
            content = _dedent_block(combined)
            body_only = block_lines[1:-1] if len(block_lines) >= 2 else []
            inner_children = parse_rust_items("\n".join(body_only)) if body_only else []
            items.append(RustCustomItem(children=inner_children, content=content))
            i = end + 1
        else:
            # capture up to the line containing the terminal ';' token (if it is on a later line)
            if token is not None and token[2] == ";":
                end_line = token[0]
                slice_lines = lines[code_index : end_line + 1]
            else:
                slice_lines = [header_line]
            combined = collected_custom + slice_lines if collected_custom else slice_lines
            content = _dedent_block(combined)
            items.append(RustCustomItem(children=[], content=content))
            i = (token[0] + 1) if (token is not None and token[2] == ";") else (code_index + 1)
    return items


def parse_rust_file(source: str, path: str = "") -> RustFile:
    """Parse a Rust file into a `RustFile` with annotated and custom items.

    Also includes top-level `use` imports and `mod` declarations as items for
    roundtrip purposes.
    """
    items: list[RustItem] = []
    for raw_line in source.splitlines():
        s = raw_line.strip()
        if not s:
            continue
        if s.startswith("use ") or s.startswith("pub use "):
            items.append(RustCustomItem(children=[], content=s))
        elif s.startswith("mod ") or s.startswith("pub mod "):
            items.append(RustCustomItem(children=[], content=s))
    items_structured = parse_rust_items(source)
    items.extend(items_structured)
    return RustFile(path=path, items=items)
