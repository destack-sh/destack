import textwrap
from collections.abc import Sequence
from dataclasses import dataclass

from .core import (
    RustAttribute,
    RustCustomItem,
    RustFile,
    RustGenerationType,
    RustImport,
    RustItem,
    RustItemScope,
    RustManagedItem,
    RustMod,
)


@dataclass(slots=True)
class RustTokenParser:
    """
    Parser state for tracking position within Rust source code.

    Handles proper tracking through string literals, character literals,
    raw strings, line comments, and nested block comments.
    """

    in_block_comment: int = 0  # nesting depth for block comments
    in_string: bool = False
    in_char: bool = False
    in_line_comment: bool = False
    raw_string_hashes: int | None = None  # number of '#' in raw string delimiter

    def reset_line(self) -> None:
        """Reset state for a new line (clears line comment flag)."""
        self.in_line_comment = False

    def process_char(self, ch: str, nxt: str, prev: str | None = None) -> bool:
        """
        Process a single character and update parser state.

        Returns True if the character is visible (not inside string/comment),
        False otherwise.
        """
        # line comments end at newline
        if self.in_line_comment:
            return False

        # handle nested block comments
        if self.in_block_comment > 0:
            if ch == "/" and prev == "*":
                self.in_block_comment -= 1
            elif ch == "/" and nxt == "*":
                self.in_block_comment += 1
            return False

        # handle string literals
        if self.in_string:
            if self.raw_string_hashes is None:
                # normal string: end on unescaped "
                if ch == "\\":
                    return False  # skip escape sequence
                if ch == '"':
                    self.in_string = False
            else:
                # raw string: end when '"' followed by exact number of '#'
                if ch == '"':
                    # caller should check following '#' count
                    pass  # handled by caller for proper lookahead
            return False

        # handle char literals
        if self.in_char:
            if ch == "\\":
                return False  # skip escape sequence
            if ch == "'":
                self.in_char = False
            return False

        # detect comment starts
        if ch == "/" and nxt == "/":
            self.in_line_comment = True
            return False
        if ch == "/" and nxt == "*":
            self.in_block_comment += 1
            return False

        # detect string/char literal starts
        # (raw string detection handled by caller due to lookahead complexity)
        if ch == '"' and not self.in_string:
            self.in_string = True
            self.raw_string_hashes = None
            return False
        if ch == "'" and not self.in_char:
            self.in_char = True
            return False

        return True  # character is visible


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


def _parse_attr(line: str) -> tuple[RustGenerationType, str, str, RustItemScope] | None:
    """
    Parse a destack attribute line like:
    #[destack::generated(Vector2, struct, block)]
    #[destack::partial(Vector2, impl, block)]
    #[destack::generated(Vector2, ZERO, line)]
    #[destack::stub(Vector2, x, block)]
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
        mode = RustGenerationType(mode_part)
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
            # treat any other well-formed token (e.g., block) as block content
            scope = RustItemScope.BLOCK
        return (mode, object_key, inner_key, scope)
    except Exception:
        return None


def _parse_block(lines: Sequence[str], start_index: int) -> tuple[int, list[str]]:
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
    parser = RustTokenParser()

    while index < len(lines):
        line = lines[index]
        collected.append(line)
        parser.reset_line()
        i = 0
        line_len = len(line)

        while i < line_len:
            ch = line[i]
            nxt = line[i + 1] if i + 1 < line_len else ""
            prev = line[i - 1] if i > 0 else None

            # handle special cases that need lookahead
            if parser.in_line_comment:
                break

            if parser.in_block_comment > 0:
                if ch == "/" and prev == "*":
                    parser.in_block_comment -= 1
                elif ch == "/" and nxt == "*":
                    parser.in_block_comment += 1
                    i += 1
                i += 1
                continue

            if parser.in_string:
                if parser.raw_string_hashes is None:
                    # normal string
                    if ch == "\\":
                        i += 2
                        continue
                    if ch == '"':
                        parser.in_string = False
                else:
                    # raw string: check for closing delimiter
                    if ch == '"':
                        j = i + 1
                        count = 0
                        while j < line_len and line[j] == "#":
                            count += 1
                            j += 1
                        if count == parser.raw_string_hashes:
                            parser.in_string = False
                            parser.raw_string_hashes = None
                            i = j - 1
                i += 1
                continue

            if parser.in_char:
                if ch == "\\":
                    i += 2
                    continue
                if ch == "'":
                    parser.in_char = False
                i += 1
                continue

            # not in string/comment - check for starts
            if ch == "/" and nxt == "/":
                parser.in_line_comment = True
                break
            if ch == "/" and nxt == "*":
                parser.in_block_comment += 1
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
                    parser.in_string = True
                    parser.raw_string_hashes = hash_count
                    i = j + 1
                    continue

            # normal string start
            if ch == '"':
                parser.in_string = True
                parser.raw_string_hashes = None
                i += 1
                continue

            # char literal start
            if ch == "'":
                parser.in_char = True
                i += 1
                continue

            # count braces only if visible
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
    """Find the next non-empty line starting from start_index."""
    index = start_index
    while index < len(lines) and not lines[index].strip():
        index += 1
    return index


def _skip_attribute_lines(lines: Sequence[str], start_index: int) -> int:
    """Skip over attribute lines (e.g., #[derive(...)]) starting from start_index."""
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


def _iter_visible_chars(lines: Sequence[str], start_index: int = 0):
    """
    Yield (line_index, char_index, ch) for characters outside strings/comments.
    Handles nested block comments, raw strings, char literals, and line comments.
    """
    parser = RustTokenParser()
    li = start_index

    while li < len(lines):
        line = lines[li]
        parser.reset_line()
        i = 0

        while i < len(line):
            ch = line[i]
            nxt = line[i + 1] if i + 1 < len(line) else ""
            prev = line[i - 1] if i > 0 else None

            # handle special parsing states
            if parser.in_line_comment:
                break

            if parser.in_block_comment > 0:
                if ch == "/" and prev == "*":
                    parser.in_block_comment -= 1
                elif ch == "/" and nxt == "*":
                    parser.in_block_comment += 1
                    i += 1
                i += 1
                continue

            if parser.in_string:
                if parser.raw_string_hashes is None:
                    if ch == "\\":
                        i += 2
                        continue
                    if ch == '"':
                        parser.in_string = False
                else:
                    if ch == '"':
                        j = i + 1
                        count = 0
                        while j < len(line) and line[j] == "#":
                            count += 1
                            j += 1
                        if count == parser.raw_string_hashes:
                            parser.in_string = False
                            parser.raw_string_hashes = None
                            i = j - 1
                i += 1
                continue

            if parser.in_char:
                if ch == "\\":
                    i += 2
                    continue
                if ch == "'":
                    parser.in_char = False
                i += 1
                continue

            # check for comment/string starts
            if ch == "/" and nxt == "/":
                parser.in_line_comment = True
                break
            if ch == "/" and nxt == "*":
                parser.in_block_comment += 1
                i += 2
                continue

            # detect raw string start
            if ch == "r":
                j = i + 1
                hash_count = 0
                while j < len(line) and line[j] == "#":
                    hash_count += 1
                    j += 1
                if j < len(line) and line[j] == '"':
                    parser.in_string = True
                    parser.raw_string_hashes = hash_count
                    i = j + 1
                    continue

            if ch == '"':
                parser.in_string = True
                parser.raw_string_hashes = None
                i += 1
                continue

            if ch == "'":
                parser.in_char = True
                i += 1
                continue

            # character is visible
            yield li, i, ch
            i += 1

        li += 1


def _relative_block_close_index(block_lines: Sequence[str]) -> int:
    """
    Return the relative last line index where the first matching '}' closes.

    Scans through visible characters (outside strings/comments) to find
    the closing brace that matches the first opening brace.
    """
    depth = 0
    seen_open = False
    rel_idx = 0

    for li, _, ch in _iter_visible_chars(block_lines, 0):
        if ch == "{":
            depth += 1
            seen_open = True
        elif ch == "}":
            depth -= 1
        rel_idx = li
        if seen_open and depth == 0:
            return rel_idx

    return len(block_lines) - 1


def _find_first_token_outside_literals(
    lines: Sequence[str], start_index: int, tokens: set[str], max_lookahead_lines: int = 50
) -> tuple[int, int, str] | None:
    """
    Scan forward and find the first token character outside comments/strings.

    Returns a tuple of (line_index, char_index, token) for the first matching
    token found, or None if no token is found within max_lookahead_lines.
    """
    index = start_index
    looked = 0
    parser = RustTokenParser()

    while index < len(lines) and looked < max_lookahead_lines:
        line = lines[index]
        # reset line comment flag but preserve block comment state
        parser.reset_line()
        i = 0
        line_len = len(line)

        while i < line_len:
            ch = line[i]
            nxt = line[i + 1] if i + 1 < line_len else ""
            prev = line[i - 1] if i > 0 else None

            if parser.in_line_comment:
                break

            if parser.in_block_comment > 0:
                if ch == "/" and prev == "*":
                    parser.in_block_comment -= 1
                elif ch == "/" and nxt == "*":
                    parser.in_block_comment += 1
                    i += 1
                i += 1
                continue

            if parser.in_string:
                if parser.raw_string_hashes is None:
                    if ch == "\\":
                        i += 2
                        continue
                    if ch == '"':
                        parser.in_string = False
                else:
                    if ch == '"':
                        j = i + 1
                        count = 0
                        while j < line_len and line[j] == "#":
                            count += 1
                            j += 1
                        if count == parser.raw_string_hashes:
                            parser.in_string = False
                            parser.raw_string_hashes = None
                            i = j - 1
                i += 1
                continue

            if parser.in_char:
                if ch == "\\":
                    i += 2
                    continue
                if ch == "'":
                    parser.in_char = False
                i += 1
                continue

            # check for comment/string starts
            if ch == "/" and nxt == "/":
                parser.in_line_comment = True
                break
            if ch == "/" and nxt == "*":
                parser.in_block_comment += 1
                i += 2
                continue

            # detect raw string start
            if ch == "r":
                j = i + 1
                hash_count = 0
                while j < line_len and line[j] == "#":
                    hash_count += 1
                    j += 1
                if j < line_len and line[j] == '"':
                    parser.in_string = True
                    parser.raw_string_hashes = hash_count
                    i = j + 1
                    continue

            if ch == '"':
                parser.in_string = True
                parser.raw_string_hashes = None
                i += 1
                continue

            if ch == "'":
                parser.in_char = True
                i += 1
                continue

            # check if this visible character is one of our tokens
            if ch in tokens:
                return (index, i, ch)

            i += 1

        index += 1
        looked += 1

    return None


def _is_internal_import_path(path: str) -> bool:
    """Return True if a `use` path refers to internal crate paths."""
    return path.startswith("crate::") or path.startswith("self::") or path.startswith("super::")


def _extract_top_level_lines(source: str) -> list[str]:
    """
    Extract only top-level lines from source, skipping content inside blocks.
    This is used for parsing imports and module declarations.
    """
    lines = source.splitlines()
    result: list[str] = []
    parser = RustTokenParser()
    depth = 0

    for line in lines:
        parser.reset_line()
        i = 0
        line_len = len(line)

        # process the line to track brace depth
        while i < line_len:
            ch = line[i]
            nxt = line[i + 1] if i + 1 < line_len else ""
            prev = line[i - 1] if i > 0 else None

            if parser.in_line_comment:
                break

            if parser.in_block_comment > 0:
                if ch == "/" and prev == "*":
                    parser.in_block_comment -= 1
                elif ch == "/" and nxt == "*":
                    parser.in_block_comment += 1
                    i += 1
                i += 1
                continue

            if parser.in_string:
                if parser.raw_string_hashes is None:
                    if ch == "\\":
                        i += 2
                        continue
                    if ch == '"':
                        parser.in_string = False
                else:
                    if ch == '"':
                        j = i + 1
                        count = 0
                        while j < line_len and line[j] == "#":
                            count += 1
                            j += 1
                        if count == parser.raw_string_hashes:
                            parser.in_string = False
                            parser.raw_string_hashes = None
                            i = j - 1
                i += 1
                continue

            if parser.in_char:
                if ch == "\\":
                    i += 2
                    continue
                if ch == "'":
                    parser.in_char = False
                i += 1
                continue

            # check for comment/string starts
            if ch == "/" and nxt == "/":
                parser.in_line_comment = True
                break
            if ch == "/" and nxt == "*":
                parser.in_block_comment += 1
                i += 2
                continue

            # detect raw string start
            if ch == "r":
                j = i + 1
                hash_count = 0
                while j < line_len and line[j] == "#":
                    hash_count += 1
                    j += 1
                if j < line_len and line[j] == '"':
                    parser.in_string = True
                    parser.raw_string_hashes = hash_count
                    i = j + 1
                    continue

            if ch == '"':
                parser.in_string = True
                parser.raw_string_hashes = None
                i += 1
                continue

            if ch == "'":
                parser.in_char = True
                i += 1
                continue

            # track brace depth
            if ch == "{":
                depth += 1
            elif ch == "}":
                depth -= 1

            i += 1

        # only include lines that are at depth 0 (top-level)
        if depth == 0:
            result.append(line)

    return result


def _parse_rust_imports(source: str) -> list[RustImport]:
    """Parse Rust `use` imports from top-level lines only."""
    imports: list[RustImport] = []
    top_level_lines = _extract_top_level_lines(source)

    for raw_line in top_level_lines:
        stripped = raw_line.strip()
        if not (stripped.startswith("use ") or stripped.startswith("pub use ")):
            continue
        # remove trailing ';'
        if not stripped.endswith(";"):
            continue  # skip malformed or incomplete lines
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
                    rust_path=path,
                    imports=imports_list,
                    is_internal=_is_internal_import_path(path),
                    is_public=is_public,
                    is_glob=False,
                )
            )
        else:
            imports.append(
                RustImport(
                    rust_path=use_body,
                    imports=[] if not is_glob else ["*"],
                    is_internal=_is_internal_import_path(use_body),
                    is_public=is_public,
                    is_glob=is_glob,
                )
            )
    return imports


def _parse_rust_mods(source: str) -> list[RustMod]:
    """
    Parse simple module declarations like `mod foo;` or `pub mod foo;`.
    Ignore inline/nested module definitions like `mod test { ... }`.
    Only parses top-level declarations.
    """
    mods: list[RustMod] = []
    top_level_lines = _extract_top_level_lines(source)

    for raw_line in top_level_lines:
        stripped = raw_line.strip()
        if not (stripped.startswith("mod ") or stripped.startswith("pub mod ")):
            continue
        # skip nested modules (those without semicolon)
        if not stripped.endswith(";"):
            continue
        is_public = stripped.startswith("pub mod ")
        name = (
            stripped[len("pub mod ") : -1].strip()
            if is_public
            else stripped[len("mod ") : -1].strip()
        )
        if not name:
            continue
        outer_content = stripped[:-1]
        mod = RustMod(
            children=[],
            outer_content=outer_content,
            inner_content=outer_content,
            name=name,
            is_public=is_public,
            is_inline=False,
        )
        mods.append(mod)
    return mods


def _parse_rust_items(source: str, parent: RustItem | None) -> list[RustItem]:
    """
    Parse destack-annotated and custom items from a Rust source string.
    Top-level comments, attributes, mod declarations and imports are ignored.
    """
    lines = source.splitlines()
    items: list[RustItem] = []
    i = 0
    while i < len(lines):
        # skip blank lines up front
        if not lines[i].strip():
            i += 1
            continue

        # ignore top-level comments, attributes, mod declarations and imports
        head = lines[i].lstrip()
        if (
            head.startswith("//!")
            or head.startswith("#![")
            or head.startswith("use ")
            or head.startswith("pub use ")
            or (head.startswith("mod ") and head.endswith(";"))
            or (head.startswith("pub mod ") and head.endswith(";"))
        ):
            i += 1
            continue

        # treat single-line comments as standalone custom items to avoid
        # accidentally swallowing following managed items
        # this ensures comments stay with the code they document
        if head.startswith("//") and not head.startswith("//!"):
            outer_content = _dedent_block([lines[i]])
            item = RustCustomItem(
                children=[], outer_content=outer_content, inner_content=outer_content
            )
            items.append(item)
            i += 1
            continue

        # eat attribute
        attr = _parse_attr(lines[i])
        if attr is not None:
            mode, object_key, inner_key, scope = attr
            # content starts at next non-empty line
            start = _find_next_nonempty(lines, i + 1)
            if scope == RustItemScope.LINE:
                assert start < len(lines), "expected content line after attribute"
                outer_content = _dedent_block([lines[start]])
                item = RustManagedItem(
                    object_key=object_key,
                    inner_key=inner_key,
                    type=mode,
                    scope=scope,
                    children=[],
                    outer_content=outer_content,
                    inner_content=outer_content,
                )
                items.append(item)
                i = start + 1
                continue
            # block: keep any attributes like #[derive(...)] before the header in content,
            # but use the header position for block collection and child parsing
            start_no_attrs = _skip_attribute_lines(lines, start)
            attrs_prefix = [ln for ln in lines[start:start_no_attrs] if ln.strip()]
            # managed headers are well-formed; simple brace counting is sufficient
            end, collected = _collect_block_simple(lines, start_no_attrs)
            outer_content = _dedent_block((attrs_prefix + collected) if attrs_prefix else collected)
            body_lines = collected[1:-1] if len(collected) >= 2 else []
            item = RustManagedItem(
                object_key=object_key,
                inner_key=inner_key,
                type=mode,
                scope=scope,
                children=[],
                outer_content=outer_content,
                inner_content=textwrap.dedent("\n".join(body_lines)),
            )

            # parse one level down inside the block body (exclude header and closing brace)
            if parent is None and body_lines:
                item.children = _parse_rust_items("\n".join(body_lines), parent=item)
            else:
                item.children = []
            items.append(item)
            i = end + 1
            continue

        # collect leading non-destack attributes (e.g., #[derive(...)], #[inline])
        # these belong to the following custom item
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
                outer_content = _dedent_block(collected_custom)
                item = RustCustomItem(
                    children=[], outer_content=outer_content, inner_content=outer_content
                )
                items.append(item)
            i = code_index
            continue
        # if the next code is a destack attribute, let the main loop handle it
        if _parse_attr(lines[code_index]) is not None:
            i = code_index
            continue

        # check if this is a module declaration (mod name { ... } or pub mod name { ... })
        header_line = lines[code_index]
        header_stripped = header_line.strip()
        is_module = False
        mod_name = ""
        mod_is_public = False

        # check for module declaration patterns
        if header_stripped.startswith("mod ") and not header_stripped.endswith(";"):
            # could be "mod name {" or "mod name"
            mod_parts = header_stripped[4:].split(None, 1)  # split on whitespace
            if mod_parts and mod_parts[0] and not mod_parts[0].startswith("{"):
                mod_name = mod_parts[0].rstrip("{").strip()
                is_module = True
                mod_is_public = False
        elif header_stripped.startswith("pub mod ") and not header_stripped.endswith(";"):
            mod_parts = header_stripped[8:].split(None, 1)
            if mod_parts and mod_parts[0] and not mod_parts[0].startswith("{"):
                mod_name = mod_parts[0].rstrip("{").strip()
                is_module = True
                mod_is_public = True
        # also check for attributed modules like #[cfg(test)] mod tests { ... }
        elif collected_custom:
            # check if the last attribute is something like #[cfg(test)]
            # and the current line is a module declaration
            if header_stripped.startswith("mod ") and not header_stripped.endswith(";"):
                mod_parts = header_stripped[4:].split(None, 1)
                if mod_parts and mod_parts[0] and not mod_parts[0].startswith("{"):
                    mod_name = mod_parts[0].rstrip("{").strip()
                    is_module = True
                    mod_is_public = False
            elif header_stripped.startswith("pub mod ") and not header_stripped.endswith(";"):
                mod_parts = header_stripped[8:].split(None, 1)
                if mod_parts and mod_parts[0] and not mod_parts[0].startswith("{"):
                    mod_name = mod_parts[0].rstrip("{").strip()
                    is_module = True
                    mod_is_public = True

        # determine if this custom item is a block (with {}) or a single line (with ;)
        # by finding the first '{' or ';' outside of strings/comments
        token = _find_first_token_outside_literals(lines, code_index, {"{", ";"})
        if token is not None and token[2] == "{":  # block item
            header_start = token[0]
            end, block_lines = _parse_block(lines, header_start)
            # handle edge case: if we collected across multiple logical blocks
            # (e.g., an impl block followed by a free function), we need to truncate
            # at the correct closing brace to avoid capturing too much
            rel_close = _relative_block_close_index(block_lines)

            # heuristic: if another destack attribute appears before the calculated
            # closing brace, use the last standalone '}' before that attribute instead
            # this handles cases where the robust parser might overshoot
            next_attr_idx = None
            scan_idx = header_start + 1
            while scan_idx < len(lines):
                if lines[scan_idx].lstrip().startswith("#[destack::"):
                    next_attr_idx = scan_idx
                    break
                scan_idx += 1
            if next_attr_idx is not None:
                last_brace_idx: int | None = None
                for k in range(header_start, next_attr_idx):
                    if lines[k].strip() == "}":
                        last_brace_idx = k
                if last_brace_idx is not None and last_brace_idx >= header_start:
                    rel_close = last_brace_idx - header_start
            truncated = block_lines[: rel_close + 1]
            combined = collected_custom + truncated if collected_custom else truncated
            outer_content = _dedent_block(combined)

            # if this is a module declaration, create a RustMod item
            if is_module and mod_name:
                # for nested modules, the entire content including attributes goes in content
                mod = RustMod(
                    children=[],
                    outer_content=outer_content,
                    inner_content=outer_content,
                    name=mod_name,
                    is_public=mod_is_public,
                    is_inline=True,
                )
                items.append(mod)
            else:
                mod = RustCustomItem(
                    children=[],
                    outer_content=outer_content,
                    inner_content=outer_content,
                )
                items.append(mod)

            new_end_index = header_start + rel_close
            i = new_end_index + 1
        else:
            # capture up to the line containing the terminal ';' token (if it is on a later line)
            if token is not None and token[2] == ";":
                end_line = token[0]
                slice_lines = lines[code_index : end_line + 1]
            else:
                slice_lines = [header_line]
            combined = collected_custom + slice_lines if collected_custom else slice_lines
            outer_content = _dedent_block(combined)
            item = RustCustomItem(
                children=[], outer_content=outer_content, inner_content=outer_content
            )
            items.append(item)
            i = (token[0] + 1) if (token is not None and token[2] == ";") else (code_index + 1)
    return items


def parse_rust_file(
    *,
    source: str,
    local_path: str,
    source_path: str,
) -> RustFile:
    """
    Parse a Rust file into a `RustFile` with annotated and custom items.
    The type is determined based on the crate attributes.
    (If there is no crate attribute, it is assumed to be a fully custom file.)
    """

    # parse header
    attributes: list[str] = []
    comment_lines: list[str] = []
    idx = 0
    lines = source.splitlines()
    while idx < len(lines):
        line = lines[idx]
        if line.startswith("//!"):
            comment_lines.append(line)
            idx += 1
            continue
        elif line.startswith("#!["):
            attributes.append(line)
            idx += 1
            continue
        elif not line:
            idx += 1
            continue
        else:
            break

    # parse content
    imports = _parse_rust_imports(source)
    mods = _parse_rust_mods(source)
    items = _parse_rust_items(source, parent=None)

    # derive type from attributes
    type = RustGenerationType.CUSTOM
    if any(a.startswith("#![destack::generated") for a in attributes):
        type = RustGenerationType.GENERATED
    elif any(a.startswith("#![destack::partial") for a in attributes):
        type = RustGenerationType.PARTIAL

    return RustFile(
        type=type,
        local_path=local_path,
        source_path=source_path,
        items=items,
        comment="\n".join(comment_lines).strip(),
        attributes=[RustAttribute(content=a) for a in attributes],
        imports=imports,
        mods=mods,
    )
