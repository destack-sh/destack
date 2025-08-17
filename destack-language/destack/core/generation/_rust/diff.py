import re
import textwrap
from collections.abc import Sequence
from dataclasses import dataclass, replace
from enum import StrEnum
from typing import assert_never

from .core import (
    RustAttribute,
    RustCustomItem,
    RustFile,
    RustImport,
    RustItem,
    RustItemScope,
    RustManagedItem,
    RustManagedType,
    RustModDeclaration,
    render_rust_destack_attribute,
    render_rust_file,
)


class RustFileOperationType(StrEnum):
    """The type of operation on a Rust file."""

    REPLACE = "replace"  # replace fully
    PATCH = "patch"  # patch in place
    REMOVE = "remove"
    ADD = "add"
    WARN = "warn"  # warn and do nothing
    SKIP = "skip"  # skip and do nothing


@dataclass(slots=True)
class RustFileOperation:
    """An operation on a Rust file."""

    type: RustFileOperationType
    normalized_path: str
    local_path: str
    old_file: RustFile | None  # for REPLACE, PATCH and REMOVE
    new_file: RustFile | None  # for REPLACE, PATCH and ADD
    combined_content: str | None  # for REPLACE, PATCH and ADD


def _patch_rust_file_items(
    old_items: Sequence[RustItem], new_items: Sequence[RustItem]
) -> Sequence[RustItem]:
    """
    Patch a list of managed items by merging managed items from new with custom items from old.
    """

    # collect old managed items by key
    old_managed_items_by_key: dict[str, RustManagedItem] = {}
    for item in old_items:
        if isinstance(item, RustManagedItem):
            old_managed_items_by_key[item._key] = item

    # collect top-level custom items from old and attach them to the closest
    # preceding managed item from old that still exists in new
    customs_after_key: dict[str, list[RustCustomItem]] = {}
    unattached_customs: list[RustCustomItem] = []
    current_owner_key: str | None = None
    existing_keys_in_new = {i._key for i in new_items if isinstance(i, RustManagedItem)}
    for item in old_items:
        if isinstance(item, RustManagedItem):
            current_owner_key = (
                item._key if item._key in existing_keys_in_new else current_owner_key
            )
        elif isinstance(item, RustCustomItem):
            if current_owner_key is None:
                unattached_customs.append(item)
            else:
                customs_after_key.setdefault(current_owner_key, []).append(item)

    # merge in the order of the new file's items
    patched_items: list[RustItem] = []
    for new_item in new_items:
        # keep non-managed items in the new order unchanged
        if not isinstance(new_item, RustManagedItem):
            patched_items.append(new_item)
            continue

        # for managed items, merge against old if present
        old_item = old_managed_items_by_key.get(new_item._key)
        merged_item = new_item if old_item is None else _merge_managed_item(old_item, new_item)
        patched_items.append(merged_item)

        # append any custom items that belonged to this managed key in the old file
        if new_item._key in customs_after_key:
            patched_items.extend(customs_after_key[new_item._key])

    # add any unattached customs
    patched_items.extend(unattached_customs)

    return patched_items


def _patch_rust_file_attributes(
    old_attributes: Sequence[RustAttribute], new_attributes: Sequence[RustAttribute]
) -> Sequence[RustAttribute]:
    """
    Patch a list of attributes by merging attributes keyed by content.
    """
    return new_attributes


def _patch_rust_file_imports(
    old_imports: Sequence[RustImport], new_imports: Sequence[RustImport]
) -> Sequence[RustImport]:
    """
    Patch a list of imports by merging imports keyed by source.
    """
    return new_imports


def _patch_rust_file_mods(
    old_mods: Sequence[RustModDeclaration], new_mods: Sequence[RustModDeclaration]
) -> Sequence[RustModDeclaration]:
    """
    Patch a list of mods by merging mods keyed by name.
    """
    # create a set of new mod names for quick lookup
    new_mod_names = {mod.name for mod in new_mods}
    # start with all new mods
    result_mods = list(new_mods)
    # retain old mods that aren't in new
    for old_mod in old_mods:
        if old_mod.name not in new_mod_names:
            result_mods.append(old_mod)
    return result_mods


def _patch_rust_file(old_file: RustFile, new_file: RustFile) -> RustFile:
    """
    Patch a managed Rust file by merging managed items keyed by some stable key.
    """
    patched_attributes = _patch_rust_file_attributes(old_file.attributes, new_file.attributes)
    patched_imports = _patch_rust_file_imports(old_file.imports or (), new_file.imports or ())
    patched_mods = _patch_rust_file_mods(old_file.mods or (), new_file.mods or ())
    patched_items = _patch_rust_file_items(old_file.items, new_file.items)
    patched_file = replace(
        new_file,
        items=patched_items,
        comment=new_file.comment,
        attributes=patched_attributes,
        imports=patched_imports,
        mods=patched_mods,
    )
    return patched_file


def _merge_managed_item(old_item: RustManagedItem, new_item: RustManagedItem) -> RustManagedItem:
    """
    Merge a managed item intelligently based on its type and scope.
    """
    assert old_item.type == new_item.type, (
        f"mismatched item types {old_item.type} != {new_item.type}: {new_item!r}"
    )

    # for generated items, always use new
    if new_item.type == RustManagedType.GENERATED:
        return new_item

    # for partial items, merge based on scope
    elif new_item.type == RustManagedType.PARTIAL:
        if new_item.scope == RustItemScope.LINE:
            raise ValueError(f"unexpected partial line: {new_item!r}")
        elif new_item.scope == RustItemScope.BLOCK:
            return _merge_managed_item_block(old_item, new_item)
        else:
            assert_never(new_item.scope)

    # custom shouldn't get here
    elif new_item.type == RustManagedType.CUSTOM:
        raise ValueError(f"cannot merge custom: {new_item!r}")

    else:
        assert_never(new_item.type)


def _merge_item_leaf(old_content: str, new_content: str) -> str:
    """
    For leaf blocks (functions), take docs/signature from new, body from old.
    """
    old_lines = old_content.split("\n")
    new_lines = new_content.split("\n")

    # find where the opening brace is
    new_brace = next((i for i, line in enumerate(new_lines) if "{" in line), -1)
    old_brace = next((i for i, line in enumerate(old_lines) if "{" in line), -1)

    if new_brace == -1 or old_brace == -1:
        return new_content  # not a block, use new

    # take everything up to and including the brace from new (docs + signature)
    # take everything after the brace from old (body)
    result = new_lines[: new_brace + 1] + old_lines[old_brace + 1 :]
    return "\n".join(result)


def _merge_managed_item_block(
    old_item: RustManagedItem, new_item: RustManagedItem
) -> RustManagedItem:
    """
    Merge children lists, preserving custom items and updating managed ones.
    """

    # for leaf items, merge content
    if not old_item.children:
        if new_item.children:
            raise ValueError(f"old item has no children, but new item does: {new_item!r}")
        new_content = _merge_item_leaf(old_item.outer_content, new_item.outer_content)
        merged_item = replace(new_item, outer_content=new_content, inner_content=new_content)
        return merged_item

    # for block items, merge children recursively
    old_managed_children_by_key: dict[str, RustManagedItem] = {
        c._key: c for c in old_item.children if isinstance(c, RustManagedItem)
    }

    # collect custom items from old and attach them to the closest
    # preceding managed item from old that still exists in new
    customs_after_key: dict[str, list[RustCustomItem]] = {}
    unattached_customs: list[RustCustomItem] = []
    current_owner_key: str | None = None
    existing_keys_in_new = {i._key for i in new_item.children if isinstance(i, RustManagedItem)}
    for item in old_item.children:
        if isinstance(item, RustManagedItem):
            current_owner_key = (
                item._key if item._key in existing_keys_in_new else current_owner_key
            )
        elif isinstance(item, RustCustomItem):
            if current_owner_key is None:
                unattached_customs.append(item)
            else:
                customs_after_key.setdefault(current_owner_key, []).append(item)

    # build merged list following new order
    merged_items = []
    for new_child in new_item.children:
        if isinstance(new_child, RustManagedItem):
            old_child = old_managed_children_by_key.get(new_child._key)
            if old_child:
                # merge recursively
                merged_items.append(_merge_managed_item(old_child, new_child))
            else:
                # new managed child
                merged_items.append(new_child)

            # append any custom items that belonged to this managed key in the old item
            if new_child._key in customs_after_key:
                merged_items.extend(customs_after_key[new_child._key])
        elif isinstance(new_child, RustCustomItem):
            # non-managed new child
            merged_items.append(new_child)
        else:
            raise ValueError(f"unexpected item: {new_child!r}")

    # prepend any unattached customs to beginning of block
    merged_items = [*unattached_customs, *merged_items]

    # patch inner and outer content
    new_inner_content_parts: list[str] = []
    for m in merged_items:
        if isinstance(m, RustManagedItem):
            part = render_rust_destack_attribute(m) + "\n" + m.outer_content
        else:
            part = m.outer_content
        new_inner_content_parts.append(part)
    new_inner_content = "\n\n".join(new_inner_content_parts)
    new_outer_content = (
        new_item.outer_content.splitlines()[0]
        + "\n"
        + textwrap.indent(new_inner_content, "    ")
        + "\n"
        + new_item.outer_content.splitlines()[-1]
    )
    merged_item = replace(
        new_item,
        children=merged_items,
        inner_content=new_inner_content,
        outer_content=new_outer_content,
    )

    return merged_item


def diff_rust_file(old_file: RustFile, new_file: RustFile) -> RustFileOperation:
    """
    Diff a single Rust file.
        a. If the old file is generated: REPLACE it.
        b. If the old file is partial: PATCH it in place.
                Walk through the old File's managed RustItems in order (recursively),
                replacing managed items by key in the new order of items.
                Custom items "belong" to the closest preceding managed item from old that still exists,
                and are re-inserted in order right after in the new file."""
    if old_file.type != new_file.type:
        return RustFileOperation(
            type=RustFileOperationType.WARN,
            normalized_path=old_file.local_path,
            local_path=old_file.local_path,
            old_file=old_file,
            new_file=new_file,
            combined_content=None,
        )

    # replace fully generated files
    if old_file.type == RustManagedType.GENERATED:
        operation_type = RustFileOperationType.REPLACE
        combined_file = new_file
        combined_content = render_rust_file(combined_file)

    # patch partial files
    elif old_file.type == RustManagedType.PARTIAL:
        operation_type = RustFileOperationType.PATCH
        combined_file = _patch_rust_file(old_file, new_file)
        combined_content = render_rust_file(combined_file)

    # custom files shouldn't be diffed
    elif old_file.type == RustManagedType.CUSTOM:
        raise ValueError(f"unexpected custom old file: {old_file!r}")

    #
    else:
        assert_never(old_file.type)

    # if content stays the same just mark as skip
    #  (completely ignoring whitespace and trailing commas is technically wrong but mostly works,
    #   and it's a lot simpler than trying to do a real semantic diff)
    if old_file.raw_content is not None:
        old_content_stripped = re.sub(r"\s+", "", old_file.raw_content).strip()
        old_content_stripped = old_content_stripped.replace(",}", "}")
        new_content_stripped = re.sub(r"\s+", "", combined_content).strip()
        new_content_stripped = new_content_stripped.replace(",}", "}")
        if old_content_stripped == new_content_stripped:
            operation_type = RustFileOperationType.SKIP

    op = RustFileOperation(
        type=operation_type,
        normalized_path=old_file.local_path,
        local_path=old_file.local_path,
        old_file=old_file,
        new_file=new_file,
        combined_content=combined_content,
    )
    return op


def diff_rust_files(
    old_files: dict[str, RustFile], new_files: dict[str, RustFile]
) -> list[RustFileOperation]:
    """
    Diff two sets of Rust files.
    Note: new files only contain managed (that is: generated or partial) items.

    We cover the three cases:
    1. Old file exists, new file does not exist:
        a. If the old file is generated: REMOVE it.
        b. If the old file is partial: WARN and do nothing.
        c: If the old file is custom: keep it, do nothing.
    2. Old file exists, new file exists. PATCH or REPLACE (see diff_rust_file)
    3. Old file does not exist, new file exists: ADD it.
    """
    operations: list[RustFileOperation] = []

    all_paths = set(old_files.keys()) | set(new_files.keys())
    for path in all_paths:
        old_file = old_files.get(path)
        new_file = new_files.get(path)

        # case 1: old file exists, new file does not exist
        if old_file is not None and new_file is None:
            if old_file.type == RustManagedType.GENERATED:
                # remove generated files
                operations.append(
                    RustFileOperation(
                        type=RustFileOperationType.REMOVE,
                        normalized_path=path,
                        local_path=old_file.local_path,
                        old_file=old_file,
                        new_file=None,
                        combined_content=None,
                    )
                )
            elif old_file.type == RustManagedType.PARTIAL:
                # warn for partial files
                operations.append(
                    RustFileOperation(
                        type=RustFileOperationType.WARN,
                        normalized_path=path,
                        local_path=old_file.local_path,
                        old_file=old_file,
                        new_file=None,
                        combined_content=None,
                    )
                )
            elif old_file.type == RustManagedType.CUSTOM:
                pass  # do nothing
            else:
                assert_never(old_file.type)

        # case 2: both files exist, diff them
        elif old_file is not None and new_file is not None:
            operation = diff_rust_file(old_file, new_file)
            operations.append(operation)

        # case 3: new file exists, old file does not exist
        elif old_file is None and new_file is not None:
            operations.append(
                RustFileOperation(
                    type=RustFileOperationType.ADD,
                    normalized_path=path,
                    local_path=new_file.local_path,
                    old_file=None,
                    new_file=new_file,
                    combined_content=render_rust_file(new_file),
                )
            )

    # sort operations by local path
    operations.sort(key=lambda x: x.local_path)

    return operations
