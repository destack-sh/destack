"""
Server-side mapper to translate between language and database models.

'Write' direction is language -> database, 'read' is database -> wire.
"""

from __future__ import annotations

import typing
from dataclasses import asdict
from datetime import datetime
from itertools import chain, groupby
from uuid import UUID, uuid4, uuid5

import pytz
from django.db import transaction
from more_itertools import first

from bench import language, models
from bench.language import wire
from bench.language.parse import index_module
from bench.language.type import (
    PRIMITIVE_TYPES,
    StatementPath,
    StatementType,
    SymbolType,
    TypeNode,
    TypeTag,
)
from bench.language.wire import RecordData
from bench.models.project import Project, ProjectVersion
from bench.runtime.type import ExecutionFrameData
from bench.utils.fractional import generate_n_keys_between


def lookup_in_db_module(
    requirement: language.RequirementContent, path: StatementPath
) -> language.Scope:
    """
    Lookup a module in the DB.
    This actually loads and is slow, but we don't care because it's only for testing.
    """
    version = lookup_module(requirement.module_name, requirement.version)
    if version is None:
        raise ValueError(f"could not find module {requirement}")

    wire_module: wire.ModuleData = read_module(version, path)
    module = wire.wmap_module(wire_module)
    idx = index_module(module)
    return idx.get_scope(path)


def lookup_module(name: str, version: str) -> typing.Optional[ProjectVersion]:
    # requirement names are organization.library
    owner_slug, library_slug = name.split(".")
    try:
        library = Project.objects.get_by_slug(owner_slug, library_slug)
    except Project.DoesNotExist:
        return None
    if version == "latest":
        return library.head_
    else:
        return ProjectVersion.objects.filter(project=library, name=version).first()


@transaction.atomic(savepoint=False)  # read-only
def read_module(
    project_v: ProjectVersion,
    path: StatementPath | None = None,
    add_implicit_requirements: bool = True,
) -> wire.ModuleData:
    """Reads the DB module to satisfy the given path. Currently, reads the entire module (ignoring path)."""
    wire_module = wire.ModuleData(
        id=project_v.id, name=project_v.project.path, files=[], committed=project_v.committed
    )
    wire_files: dict[UUID, wire.FileData] = {}
    wire_statements: dict[UUID, wire.StatementData] = {}

    # map files
    for file in project_v.files.filter(deleted_at=None).all():
        wire_file = wire.FileData(
            module_id=wire_module.id,
            id=file.id,
            path=file.path,
            statements=[],
            generated=file.generated,
            revision=file.revision,
        )
        wire_files[file.id] = wire_file
        wire_module.files.append(wire_file)

    # map statements
    statements = list(project_v.statements.filter(deleted_at=None).all())
    for statement in statements:
        wire_statement = rmap_statement(statement, file=wire_files[statement.file_id])
        wire_statements[statement.id] = wire_statement
        wire_files[statement.file_id].statements.append(wire_statement)

    if add_implicit_requirements:
        # Implicitly require all current libraries at their latest version because
        # we can't edit, pin and upgrade requirements in the UX yet and only have our own libraries.
        # TODO @Cleanup: let users configure their own set of Bench library requirements :ManageRequirements
        #  (std should be a global default, but we want that version pinned too (?))
        _add_implicit_requirements(wire_module)

    return wire_module


def _add_implicit_requirements(wire_module: wire.ModuleData) -> None:
    """Stupid way of implicitly requiring some core libraries :ManageRequirements"""
    if wire_module.name in ("symbolx.std", "openai.std"):
        return  # only add to user modules
    implicit_file = wire.FileData(
        id=uuid4(),
        module_id=wire_module.id,
        path="__implicit__",
        generated=True,
        statements=[],
        revision=1,
    )
    for (module, version, ok) in (("symbolx.std", "latest", "a0"), ("openai.std", "latest", "a1")):
        reference_module = wire.ModuleReference(
            name=module,
            version=version,
            id=lookup_module(module, version).id,
        )
        implicit_statement = wire.StatementData(
            id=uuid4(),
            name=module,
            type=StatementType.DEFINITION,
            symbol_type=SymbolType.REQUIREMENT,
            reference_module=reference_module,
            parent_id=None,
            file_id=implicit_file.id,
            module_id=wire_module.id,
            order_key=ok,
            revision=1,
            generated=True,
            modifier=None,
            text=None,
            reference=None,
        )
        implicit_file.statements.append(implicit_statement)
    wire_module.files.append(implicit_file)


@transaction.atomic
def write_module(
    files: list[wire.FileData],
    project_v: models.ProjectVersion,
    generated_mappings: list[tuple[UUID, wire.StatementData]] = None,
    overwrite: bool = False,
) -> None:
    """Write the wire files (and their contents) as models to the database."""
    wire_statements: dict[UUID, wire.StatementData] = {}
    model_files: dict[UUID, models.File] = {}
    model_statements: dict[UUID, models.Statement] = {}
    model_contents_relations: list[typing.Any] = []

    # create files
    for file_data in files:
        # remove extension from file path (assumed to be .x, but not stored)
        path = file_data.path
        if "." in path:
            path = file_data.path.rsplit(".", 1)[0]
        file = project_v.create_file_from_path(path, exists_ok=True, id=file_data.id)
        if file.id != file_data.id:
            # delete old file
            file.delete()
            file.id = file_data.id
        model_files[file_data.id] = file
        file.generated = file_data.generated
        file.save()

    # wipe existing statements if overwrite and not empty
    if overwrite:
        models.Statement.objects.filter(file_id__in=model_files.keys()).delete()

    # map statements
    # assign temporary global order keys to prevent conflicts (parents aren't assigned yet)
    temp_order_keys = generate_n_keys_between(None, None, sum(len(f.statements) for f in files))
    for ok, stmt_data in zip(
        temp_order_keys, chain.from_iterable(file.statements for file in files)
    ):
        wire_statements[stmt_data.id] = stmt_data
        model_statement = models.Statement(
            id=stmt_data.id,
            project_version=project_v,
            file=model_files[stmt_data.file_id],
            parent=None,
            order_key=ok,
            type=stmt_data.type,
            modifier=stmt_data.modifier,
            name=stmt_data.name,
            #  :StatementCodeTextReuse
            code=stmt_data.text if stmt_data.type == StatementType.COMMENT else None,
            symbol_type=stmt_data.symbol_type,
            reference=None,
            generated=stmt_data.generated,
        )
        model_statements[stmt_data.id] = model_statement

        if stmt_data.type == StatementType.DEFINITION:
            new_relations = wmap_symbol(model_statement, stmt_data)
            model_contents_relations.extend(new_relations)

    # create statements
    models.Statement.objects.bulk_create(model_statements.values())
    # and their relations
    for relation_cls, relations in groupby(model_contents_relations, key=type):
        relation_cls.objects.bulk_create(relations)

    # map actual order key, parent and references
    for stmt_data in wire_statements.values():
        model_statement = model_statements[stmt_data.id]
        model_statement.order_key = stmt_data.order_key
        model_statement.parent_id = stmt_data.parent_id
        model_statement.reference_id = stmt_data.reference_id
    models.Statement.objects.bulk_update(model_statements.values(), ["parent", "reference"])

    # update source mappings per generative statement
    if generated_mappings is not None:
        for (generator_id, source_mappings) in generated_mappings:
            models.SourceMapping.objects.filter(statement_id=generator_id).delete()
            model_mappings = wmap_source_mappings(generator_id, source_mappings)
            models.SourceMapping.objects.bulk_create(model_mappings)


def rmap_reference(
    statement: models.Statement, reference: models.Statement | None
) -> UUID | StatementPath | None:
    if reference is None:
        return None
    if reference.project_version_id == statement.project_version_id:
        # if we're staying within the same module, keep the reference id
        # this is more efficient as we avoid lookups here and join in-memory in wire.wmap_module
        # ultimately we revert to a StatementPath in to check for bad refs
        return reference.id
    else:
        # :StatementReferencePath
        # create statement path as import path
        module_name = reference.project_version.project.path
        import_source = f"{module_name}.{reference.file.path}"
        return StatementPath(import_source, reference.name)


def rmap_statement(statement: models.Statement, file: wire.FileData) -> wire.StatementData:
    """Reads a database statement into a wire statement."""
    # map reference into wire-able reference (convert module-external ref to statement path)
    reference = rmap_reference(statement, statement.reference)
    data = wire.StatementData(
        id=statement.id,
        module_id=file.module_id,
        file_id=file.id,
        revision=statement.revision,
        parent_id=statement.parent.id if statement.parent else None,
        order_key=statement.order_key,
        type=statement.type,
        modifier=statement.modifier,
        name=statement.name,
        text=statement.code if statement.type == StatementType.COMMENT else None,
        symbol_type=statement.symbol_type,
        reference=reference,
        reference_module=statement.reference_project_version_id,
        generated=statement.generated,
    )
    if statement.type == StatementType.DEFINITION:
        rmap_symbol(statement, data)
    return data


def rmap_symbol(statement: models.Statement, data: wire.StatementData) -> None:
    """Reads a database statement's symbol into a wire statement."""
    data.description = statement.description
    data.lang = statement.lang
    data.code = statement.code
    data.provider = statement.provider
    data.external_name = statement.external_name
    data.type_nodes = rmap_type_nodes(
        statement.id,
        statement.root_type_tag,
        statement.type_nodes.filter(deleted_at=None).all(),
        statement,
    )
    data.on = statement.on
    if statement.symbol_type == SymbolType.DATA:
        data.records = [
            RecordData(
                id=record.id, revision=record.revision, order_key=record.order_key, data=record.data
            )
            for record in statement.records.filter(deleted_at=None).all()
        ]
    elif statement.symbol_type == SymbolType.BUILD:
        data.generated_mappings = [
            rmap_source_mapping(m) for m in statement.generated_mappings.all()
        ]
    elif statement.symbol_type == SymbolType.REQUIREMENT:
        data.reference_module = wire.ModuleReference(
            name=statement.reference_project_version.project.path,
            version=statement.reference_project_version.name,
            id=statement.reference_project_version_id,
        )


def wmap_symbol(statement: models.Statement, data: wire.StatementData) -> list[typing.Any]:
    """Writes a wire statement's symbol into a database statement."""
    relations = []

    statement.description = data.description
    statement.lang = data.lang
    statement.code = data.code
    statement.provider = data.provider
    statement.external_name = data.external_name
    statement.on = data.on
    if data.type == StatementType.COMMENT:  # :StatementCodeTextReuse
        statement.code = data.text
    statement.root_type_tag, type_nodes = wmap_type_nodes(statement, data.type_nodes)
    if type_nodes:
        relations.extend(type_nodes)

    # copy relational data
    if data.records:
        model_records = [
            models.DatasetRecord(
                id=record.id,
                statement=statement,
                order_key=record.order_key,
                revision=record.revision,
                data=record.data,
            )
            for record in data.records
        ]
        relations.extend(model_records)
    elif data.generated_mappings:
        mappings = wmap_source_mappings(statement.id, data.generated_mappings)
        relations.extend(mappings)
    elif data.reference_module:
        if isinstance(data.reference_module, UUID):
            statement.reference_project_version_id = data.reference_module
        else:  # lookup by (name, version)
            statement.reference_project_version = lookup_module(
                data.reference_module.name, data.reference_module.version
            )

    return relations


def wmap_source_mappings(
    statement_id: UUID | None, source_mappings: list[language.SourceMapping]
) -> list[models.SourceMapping]:
    return [wmap_source_mapping(statement_id, m) for m in source_mappings]


def wmap_source_mapping(
    statement_id: UUID | None, source_mapping: language.SourceMapping
) -> models.SourceMapping:
    return models.SourceMapping(
        type=source_mapping.type,
        statement_id=statement_id,
        source_id=source_mapping.source_id,
        source_revision=source_mapping.source_revision,
        target_id=source_mapping.target_id,
        target_revision=source_mapping.target_revision,
    )


def rmap_source_mapping(source_mapping: models.SourceMapping) -> language.SourceMapping:
    return language.SourceMapping(
        type=source_mapping.type,
        source_id=source_mapping.source_id,
        source_revision=source_mapping.source_revision,
        target_id=source_mapping.target_id,
        target_revision=source_mapping.target_revision,
    )


def wmap_type_nodes(
    statement: models.Statement | None,
    type_nodes: list[wire.TypeNodeData] | None,
    impute_type_reference: bool = False,
) -> tuple[TypeTag | None, list[models.SimpleTypeNode] | None]:
    """Writes a wire type node into a database type node."""
    if not type_nodes:
        return None, None

    root: language.TypeNode = wire.wmap_type_node(type_nodes)
    root_type_tag = root.tag
    child_nodes: list[models.SimpleTypeNode] = []

    # map inner nodes (children)
    def _wmap_child_node(node: language.TypeNode, **kwargs) -> models.SimpleTypeNode:
        # retain resolved references (we trust it's a valid foreign key, else the save will fail)
        reference_id = node.reference if isinstance(node.reference, UUID) else None
        if node.tag == TypeTag.TYPE_REFERENCE or reference_id is not None:
            return models.SimpleTypeNode(
                statement=statement,
                id=node.id,
                name=node.name,
                tag=node.tag if impute_type_reference else TypeTag.TYPE_REFERENCE,
                description=node.description,
                reference_id=reference_id,
                **kwargs,
            )
        elif node.tag in PRIMITIVE_TYPES or node.tag == TypeTag.LITERAL:
            return models.SimpleTypeNode(
                statement=statement,
                id=node.id,
                name=node.name,
                tag=node.tag,
                description=node.description,
                value=node.value,
                reference_id=reference_id,
                **kwargs,
            )
        elif node.tag == TypeTag.ARRAY:
            child_node = _wmap_child_node(node.head_type, is_array=True, **kwargs)
            child_node.name = node.name
            return child_node
        elif node.is_union_with_null:
            child_node = _wmap_child_node(node.head_type, is_nullable=True, **kwargs)
            child_node.name = node.name
            return child_node
        else:
            raise ValueError(f"type node cannot be represented simply: {node}")

    # map root node
    if root.tag == TypeTag.STRUCT:
        child_order_keys = generate_n_keys_between(None, None, len(root.children or []))
        for child, order_key in zip(root.children or [], child_order_keys):
            child_nodes.append(_wmap_child_node(child, order_key=order_key))
    elif root.tag == TypeTag.ENUM:
        if root.head_type.tag != TypeTag.STRING:  # :LiteralStringEnum
            raise ValueError(f"non-string enum head type cannot be represented simply: {root}")
        child_order_keys = generate_n_keys_between(None, None, len(root.members))
        for member, order_key in zip(root.members, child_order_keys):
            child_nodes.append(_wmap_child_node(member, order_key=order_key))
    elif root.tag == TypeTag.FUNCTION:
        # assume there is exactly one output type
        child_order_keys = generate_n_keys_between(None, None, len(root.input.children or []) + 1)
        for child, order_key in zip(root.input.children or [], child_order_keys):
            child_nodes.append(_wmap_child_node(child, order_key=order_key, is_output=False))
        output_node = _wmap_child_node(root.output, order_key=child_order_keys[-1], is_output=True)
        output_node.name = None  # simple output is unnamed
        child_nodes.append(output_node)
    else:
        raise ValueError(f"root type node cannot be represented simply: {type_nodes}")

    return root_type_tag, child_nodes


def rmap_type_nodes(
    root_id: UUID,
    root_type_tag: TypeTag | None,
    type_nodes: list[models.SimpleTypeNode] | None,
    for_statement: models.Statement,
) -> list[wire.TypeNodeData]:
    """
    Reads a database type node into a wire type node.
    Because the database type is simpler and skips some intermediate nodes, we need to
    reconstruct them and assign reproducible IDs.
    """
    if root_type_tag is None:
        return []

    def new_id(name: str) -> UUID:
        """Generate a reproducible ID for a child node."""
        return uuid5(root_id, name)

    def _rmap_child_node(node: models.SimpleTypeNode) -> language.TypeNode:
        if node.tag in PRIMITIVE_TYPES or node.tag == TypeTag.LITERAL:
            lang_node = language.TypeNode(
                id=node.id, tag=node.tag, name=node.name, value=node.value
            )
        elif node.tag == TypeTag.TYPE_REFERENCE or node.reference_id is not None:
            reference = rmap_reference(for_statement, node.reference)
            lang_node = language.TypeNode(
                id=node.id, tag=node.tag, name=node.name, reference=reference
            )
        else:
            raise ValueError(f"type node is not represented simply: {node}")

        if node.is_array:  # hoist into array
            lang_node.name = None
            lang_node.id = new_id("array" + str(lang_node.id))
            lang_node = language.TypeNode(
                id=node.id, tag=TypeTag.ARRAY, name=node.name, children=[lang_node]
            )
        if node.is_nullable:  # hoist into union
            lang_node.name = None
            lang_node.id = new_id("union" + str(lang_node.id))
            null = TypeNode(id=new_id("null" + str(lang_node.id)), tag=TypeTag.NULL, name=None)
            lang_node = TypeNode(
                id=node.id, tag=TypeTag.UNION, name=node.name, children=[lang_node, null]
            )
        return lang_node

    if root_type_tag == TypeTag.STRUCT:
        children = [_rmap_child_node(node) for node in type_nodes]
    elif root_type_tag == TypeTag.ENUM:
        # assumes literal string enums only :LiteralStringEnum
        head_type = TypeNode(id=new_id("head"), name=None, tag=TypeTag.STRING)
        children = [head_type, *[_rmap_child_node(node) for node in type_nodes]]
    elif root_type_tag == TypeTag.FUNCTION:
        input_children = [_rmap_child_node(node) for node in type_nodes if not node.is_output]
        output = first((node for node in type_nodes if node.is_output), None)
        if output is not None:
            output = _rmap_child_node(output)
            output.name = "output"  # restore name
        else:  # default optional output to null (no output)
            output = TypeNode(id=new_id("output"), tag=TypeTag.NULL, name="output")
        input = TypeNode(
            id=new_id("input"), tag=TypeTag.STRUCT, name="input", children=input_children
        )
        children = [input, output]
    else:
        raise ValueError(f"root type node is not represented simply: {type_nodes}")

    root = TypeNode(id=root_id, tag=root_type_tag, name=None, children=children)
    wire_nodes_data = wire.rmap_type_node(root)

    type_nodes_revisions = {node.id: node.revision for node in type_nodes}
    # patch revision
    for wire_node in wire_nodes_data:
        # find revision from child nodes (default to statement's revision)
        wire_node.revision = type_nodes_revisions.get(wire_node.id, for_statement.revision)
    return wire_nodes_data


def rmap_execution_frame(frame: ExecutionFrameData) -> models.Execution:
    if frame.error:
        status = models.ExecutionStatus.Failed
    elif frame.exited_at:
        status = models.ExecutionStatus.Completed
    else:
        status = models.ExecutionStatus.Running
    # additional context
    user_id = (
        frame.trigger_id
        if frame.trigger_type == models.ExecutionTriggerType.UI_INTERACTIVE
        else None
    )
    access_token_id = (
        frame.trigger_id if frame.trigger_type == models.ExecutionTriggerType.REST_API else None
    )
    return models.Execution(
        id=frame.id,
        project_version_id=frame.module_id,
        status=status,
        root_id=frame.root_id,
        parent_id=frame.parent_id,
        build_id=frame.build_id,
        task_id=frame.task_id,
        code_id=frame.code_id,
        model_id=frame.model_id,
        created_at=frame.entered_at,  # not sure what to pass since it's not in DB, not frame
        updated_at=datetime.utcnow().replace(tzinfo=pytz.utc),
        started_at=frame.entered_at,
        terminated_at=frame.exited_at,
        inputs=frame.inputs,
        outputs=frame.outputs,
        error=asdict(frame.error) if frame.error else None,
        # additional context
        tracing_level=frame.tracing_level,
        deployment_id=frame.deployment_id,
        trigger_type=frame.trigger_type,
        user_id=user_id,
        access_token_id=access_token_id,
    )
