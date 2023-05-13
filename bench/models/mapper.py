"""
Server-side mapper to translate between language and database models.

'Write' direction is language -> database, 'read' is database -> wire.
"""

from __future__ import annotations

import typing
from dataclasses import asdict
from datetime import datetime
from itertools import chain, groupby
from uuid import UUID, uuid5

import pytz
from django.db import transaction

from bench import language, models
from bench.language import wire
from bench.language.mutate import MMT, NON_SEMANTIC_STATEMENT_TYPES, ModuleMutation, MutationBundle
from bench.language.parse import LookupBy, index_module
from bench.language.type import StatementPath, StatementType, SymbolType
from bench.language.wire import FileData, RecordData, SimpleTypeNodeData, StatementData
from bench.models.project import Project, ProjectVersion
from bench.runtime.type import EvaluationResultData, ExecutionFrameData, JobData, RunErrorData
from bench.utils.fractional import generate_n_keys_between


def lookup_in_db_module(
    requirement: language.RequirementContent, path: StatementPath, by: LookupBy
) -> language.Scope:
    """
    Lookup a module in the DB.
    This actually loads and is slow, but we don't care because it's only for testing.
    """
    version = lookup_module(requirement.module_name, requirement.version)
    if version is None:
        raise ValueError(f"could not find module {requirement}")

    wire_module: wire.ModuleData = read_module(version)
    module = wire.wmap_module(wire_module)
    idx = index_module(module)
    return idx.get_scope(path, by=by)


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


IMPLICIT_FILE_ID = uuid5(UUID("53400ed5-ccd5-4bcf-899d-c93c3e8a0d15"), "implicit_file")


# ensure stable ids for implicit requirements
def _add_implicit_requirements(wire_module: wire.ModuleData) -> None:
    """Stupid way of implicitly requiring some core libraries :ManageRequirements"""
    default_libs = ("symbolx.std", "openai.std", "anthropic.std")
    if wire_module.name in default_libs:
        return  # only add to user modules
    implicit_file = wire.FileData(
        id=IMPLICIT_FILE_ID,
        module_id=wire_module.id,
        path="__implicit__",
        generated=True,
        statements=[],
        revision=1,
    )
    order_keys = generate_n_keys_between(None, None, len(default_libs))
    for module, ok in zip(default_libs, order_keys):
        version = "latest"
        reference_module = wire.ModuleReference(
            name=module,
            version=version,
            id=lookup_module(module, version).id,
        )
        implicit_statement = wire.StatementData(
            id=uuid5(IMPLICIT_FILE_ID, module),
            name=module,
            fqn=None,
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


@transaction.atomic(savepoint=False)  # read-only
def read_module(
    project_v: ProjectVersion,
    exclude_non_semantic: bool = False,
    add_implicit_requirements: bool = True,
) -> wire.ModuleData:
    """Reads the DB module."""
    wire_module = wire.ModuleData(
        id=project_v.id, name=project_v.project.path, files=[], committed=project_v.committed
    )
    wire_files: dict[UUID, wire.FileData] = {}
    wire_statements: dict[UUID, wire.StatementData] = {}

    # map files
    for file in project_v.files.filter(deleted_at=None).all():
        wire_file = rmap_file_flat(file, module_id=wire_module.id)
        wire_files[file.id] = wire_file
        wire_module.files.append(wire_file)

    # map statements
    statements = (
        project_v.statements.filter(deleted_at=None, commented=False)
        .select_related("reference")
        .prefetch_related("records", "type_nodes", "xblocks")
    )
    if exclude_non_semantic:
        statements = statements.exclude(type__in=NON_SEMANTIC_STATEMENT_TYPES)

    for statement in statements:
        wire_statement = rmap_statement(
            statement, file_id=statement.file_id, module_id=wire_module.id
        )
        wire_statements[statement.id] = wire_statement
        wire_files[statement.file_id].statements.append(wire_statement)

    if add_implicit_requirements:
        # Implicitly require all current libraries at their latest version because
        # we can't edit, pin and upgrade requirements in the UX yet and only have our own libraries.
        # TODO @Cleanup: let users configure their own set of Bench library requirements :ManageRequirements
        #  (std should be a global default, but we want that version pinned too (?))
        _add_implicit_requirements(wire_module)

    return wire_module


def rmap_file_nested(file: models.File, exclude_non_semantic: bool) -> FileData:
    """Reads a file and its statements (and their contents)."""
    statements = (
        file.statements.filter(deleted_at=None, commented=False)
        .select_related("reference")
        .prefetch_related("records", "type_nodes", "xblocks")
    )
    if exclude_non_semantic:
        statements = statements.exclude(type__in=NON_SEMANTIC_STATEMENT_TYPES)

    wire_file = rmap_file_flat(file, module_id=file.project_version_id)
    wire_file.statements = [
        rmap_statement(s, file_id=file.id, module_id=file.project_version_id) for s in statements
    ]
    return wire_file


def rmap_statement_nested(statement: models.Statement) -> list[StatementData]:
    """Reads a statement and all its children."""
    wire_statements = [
        rmap_statement(s, file_id=statement.file_id, module_id=statement.project_version_id)
        for s in statement.descendants
    ]
    return wire_statements


# (all module contents are used for tracking changes)
def rmap_flat(
    obj: models.File | models.Statement | models.SimpleTypeNode | models.DatasetRecord,
) -> wire.FileData | wire.StatementData | wire.SimpleTypeNodeData | wire.RecordData:
    """Read a DB object into a wire object without any children."""
    if isinstance(obj, models.File):
        return rmap_file_flat(obj, module_id=obj.project_version_id)
    elif isinstance(obj, models.Statement):
        return rmap_statement(obj, file_id=obj.file_id, module_id=obj.project_version_id, flat=True)
    elif isinstance(obj, models.SimpleTypeNode):
        return rmap_simple_type_node(obj)
    elif isinstance(obj, models.DatasetRecord):
        return rmap_record(obj)
    else:
        raise ValueError(f"unexpected obj: {obj}")


def rmap_file_flat(file: models.File, module_id: UUID) -> wire.FileData:
    return wire.FileData(
        id=file.id,
        module_id=module_id,
        path=file.path,
        statements=[],
        generated=file.generated,
        revision=file.revision,
    )


@transaction.atomic
def write_mutations(project_v: models.ProjectVersion, mutations: list[ModuleMutation]):
    mut = MutationBundle(mutations)

    # first process deletes
    if mut[MMT.DELETE_RECORD]:  # batch delete since no dependent models
        dataset_ids = [m.data.id for m in mut[MMT.DELETE_RECORD]]
        models.DatasetRecord.objects.filter(id__in=dataset_ids).delete()
    if mut[MMT.TRUNCATE_RECORDS]:
        statement_ids = [m.statement_id for m in mut[MMT.TRUNCATE_RECORDS]]
        models.DatasetRecord.objects.filter(statement_id__in=statement_ids).delete()
    if mut[MMT.DELETE_TYPE_NODE]:  # batch delete since no dependent models
        type_node_ids = [m.data.id for m in mut[MMT.DELETE_TYPE_NODE]]
        models.SimpleTypeNode.objects.filter(id__in=type_node_ids).delete()
    if mut[MMT.DELETE_STATEMENT]:
        statement_ids = [m.statement_id for m in mut[MMT.DELETE_STATEMENT]]
        for statement in models.Statement.objects.filter(id__in=statement_ids):
            statement.delete()
    if mut[MMT.DELETE_FILE]:
        file_ids = [m.file_id for m in mut[MMT.DELETE_FILE]]
        for file in models.File.objects.filter(id__in=file_ids):
            file.delete()

    # then process creates
    if mut[MMT.CREATE_FILE]:
        for create in mut[MMT.CREATE_FILE]:
            file_data = typing.cast(FileData, create.data)
            # remove extension from file path (assumed to be .x, but not stored)
            path = file_data.path
            if "." in path:
                path = file_data.path.rsplit(".", 1)[0]
            file = project_v.create_file_from_path(path, exists_ok=True, id=file_data.id)
            if file.id != file_data.id:
                # delete old file
                file.delete()
                file.id = file_data.id
            file.generated = file_data.generated
            file.save()
    model_contents_relations: list[typing.Any] = []
    if mut[MMT.CREATE_STATEMENT]:
        statements = [typing.cast(StatementData, c.data) for c in mut[MMT.CREATE_STATEMENT]]
        statements_ids = {stmt_data.id for stmt_data in statements}
        model_statements: dict[UUID, models.Statement] = {}
        # assign temporary parent, reference and order keys to statements within the batch
        temp_order_keys = generate_n_keys_between(None, None, len(statements))
        for ok, stmt_data in zip(temp_order_keys, statements):
            external_parent = (
                stmt_data.parent_id is not None and stmt_data.parent_id not in statements_ids
            )
            model_statement = models.Statement(
                id=stmt_data.id,
                project_version=project_v,
                file_id=stmt_data.file_id,
                parent_id=stmt_data.parent_id if external_parent else None,
                order_key=stmt_data.order_key if external_parent else ok,
                type=stmt_data.type,
                modifier=stmt_data.modifier,
                name=stmt_data.name,
                #  :StatementCodeTextReuse
                code=stmt_data.text if stmt_data.type == StatementType.COMMENT else None,
                symbol_type=stmt_data.symbol_type,
                reference_id=stmt_data.reference_id,
                generated=stmt_data.generated,
            )
            model_statements[stmt_data.id] = model_statement
            if stmt_data.type == StatementType.DEFINITION:
                new_relations = wmap_symbol(model_statement, stmt_data, flat=True)
                model_contents_relations.extend(new_relations)
        # create statements
        models.Statement.objects.bulk_create(model_statements.values())
        # map actual order key, parent and references if not external
        dirty_statements = []
        for stmt_data in statements:
            model_statement = model_statements[stmt_data.id]
            dirty = (
                model_statement.parent_id != stmt_data.parent_id
                or model_statement.reference_id != stmt_data.reference_id
            )
            if dirty:
                model_statement.order_key = stmt_data.order_key
                model_statement.parent_id = stmt_data.parent_id
                model_statement.reference_id = stmt_data.reference_id
                dirty_statements.append(model_statement)
        models.Statement.objects.bulk_update(dirty_statements, ["order_key", "parent", "reference"])
    for m in mut[MMT.CREATE_RECORD]:
        model_contents_relations.append(
            wmap_record(m.statement_id, typing.cast(RecordData, m.data))
        )
    for m in mut[MMT.CREATE_TYPE_NODE]:
        model_contents_relations.append(
            wmap_simple_type_node(m.statement_id, typing.cast(SimpleTypeNodeData, m.data))
        )
    # create content relations
    for relation_cls, relations in groupby(model_contents_relations, key=type):
        relation_cls.objects.bulk_create(relations)

    # then process updates
    if mut[MMT.UPDATE_GENERATED_MAPPINGS]:
        generator_ids = {m.statement_id for m in mut[MMT.UPDATE_GENERATED_MAPPINGS]}
        models.GeneratedMapping.objects.filter(statement_id__in=generator_ids).delete()
        mappings = chain.from_iterable(
            (
                wmap_generated_mapping(m.statement_id, mapping)
                for mapping in m.data.generated_mappings or []
            )
            for m in mut[MMT.UPDATE_GENERATED_MAPPINGS]
        )
        models.GeneratedMapping.objects.bulk_create(mappings)
    if mut[MMT.UPDATE_RECORD]:
        records = [wmap_record(m.statement_id, m.data) for m in mut[MMT.UPDATE_RECORD]]
        models.DatasetRecord.objects.bulk_update(records, ["order_key", "revision", "data"])
    # :WriteModuleUpdates
    if mut[MMT.UPDATE_STATEMENT]:
        raise NotImplementedError(f"statement updates not implemented: {mut[MMT.UPDATE_STATEMENT]}")
    if mut[MMT.UPDATE_TYPE_NODE]:
        raise NotImplementedError(f"type node updates not implemented: {mut[MMT.UPDATE_TYPE_NODE]}")


def write_files(
    project_v: models.ProjectVersion,
    files: list[wire.FileData],
    delete_files: set[UUID] = None,
    overwrite: bool = False,
):
    if delete_files:
        for file in models.File.objects.filter(id__in=delete_files):
            file.delete()
    if overwrite:
        file_ids = [file_data.id for file_data in files]
        for file in project_v.files.filter(id__in=file_ids):
            file.delete()

    model_files: dict[UUID, models.File] = {}
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
    return model_files


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


def rmap_statement(
    statement: models.Statement, file_id: UUID, module_id: UUID, flat: bool = False
) -> wire.StatementData:
    """Reads a database statement into a wire statement."""
    # map reference into wire-able reference (convert module-external ref to statement path)
    reference = rmap_reference(statement, statement.reference)
    if statement.type == StatementType.REFERENCE and reference is not None:
        # references are stored without name
        # reference may be none if it was deleted
        name = statement.reference.name
    else:
        name = statement.name
    data = wire.StatementData(
        id=statement.id,
        module_id=module_id,
        file_id=file_id,
        revision=statement.revision,
        parent_id=statement.parent_id,
        order_key=statement.order_key,
        type=statement.type,
        modifier=statement.modifier,
        root_type_tag=statement.root_type_tag,
        name=name,
        fqn=None,
        text=statement.code if statement.type == StatementType.COMMENT else None,
        symbol_type=statement.symbol_type,
        reference=reference,
        reference_module=None,
        generated=statement.generated,
    )
    if statement.type == StatementType.DEFINITION:
        rmap_symbol(statement, data, flat=flat)
    return data


def rmap_symbol(statement: models.Statement, data: wire.StatementData, flat: bool) -> None:
    """Reads a database statement's symbol into a wire statement."""
    data.description = statement.description
    data.lang = statement.lang
    data.code = statement.code
    data.provider = statement.provider
    data.external_name = statement.external_name
    data.root_type_tag = statement.root_type_tag
    if not flat:
        data.type_nodes = [
            rmap_simple_type_node(node)
            for node in statement.type_nodes.filter(deleted_at=None).all()
        ]
    if statement.symbol_type in (
        SymbolType.TASK,
        SymbolType.CODE,
        SymbolType.BUILD,
    ):
        data.generated_mappings = [
            rmap_generated_mapping(m) for m in statement.generated_mappings.all()
        ]
    if statement.symbol_type == SymbolType.DATA and not flat:
        data.records = [rmap_record(record) for record in statement.records.filter(deleted_at=None)]
    if statement.symbol_type == SymbolType.CODE and not flat:
        data.xblocks = rmap_xblocks(statement.xblocks.all())
    if statement.symbol_type == SymbolType.BUILD:
        data.build_settings = wire.BuildSettings(
            reactive=statement.build_settings.reactive,
            weights=statement.build_settings.weights,
        )
    if statement.symbol_type == SymbolType.REQUIREMENT:
        data.reference_module = wire.ModuleReference(
            name=statement.reference_project_version.project.path,
            version=statement.reference_project_version.name,
            id=statement.reference_project_version_id,
        )


def wmap_symbol(
    statement: models.Statement, data: wire.StatementData, flat: bool
) -> list[typing.Any]:
    """Writes a wire statement's symbol into a database statement."""
    relations = []

    statement.description = data.description
    statement.lang = data.lang
    statement.code = data.code
    statement.provider = data.provider
    statement.external_name = data.external_name
    if data.type == StatementType.COMMENT:  # :StatementCodeTextReuse
        statement.code = data.text
    statement.root_type_tag = data.root_type_tag
    # copy basic normalized data
    if data.build_settings:
        build_settings = models.BuildSettings(
            reactive=data.build_settings.reactive, weights=data.build_settings.weights
        )
        statement.build_settings = build_settings
        relations.append(build_settings)
    if data.reference_module:
        if isinstance(data.reference_module, UUID):
            statement.reference_project_version_id = data.reference_module
        else:  # lookup by (name, version)
            statement.reference_project_version = lookup_module(
                data.reference_module.name, data.reference_module.version
            )
    if not flat:
        # copy nested relations
        if data.type_nodes:
            type_nodes = [wmap_simple_type_node(statement.id, node) for node in data.type_nodes]
            relations.extend(type_nodes)
        if data.records:
            model_records = [wmap_record(statement.id, record) for record in data.records]
            relations.extend(model_records)
        if data.xblocks:
            model_xblocks = wmap_xblocks(statement, data.xblocks)
            relations.extend(model_xblocks)
        if data.generated_mappings:
            mappings = wmap_generated_mappings(statement.id, data.generated_mappings)
            relations.extend(mappings)

    return relations


def rmap_record(record: models.DatasetRecord) -> RecordData:
    return RecordData(
        id=record.id,
        statement_id=record.statement_id,
        revision=record.revision,
        order_key=record.order_key,
        data=record.data,
    )


def wmap_record(statement_id: UUID, record: RecordData) -> models.DatasetRecord:
    return models.DatasetRecord(
        id=record.id,
        statement_id=statement_id,
        order_key=record.order_key,
        revision=record.revision,
        data=record.data,
    )


def wmap_generated_mappings(
    statement_id: UUID | None, generated_mappings: list[language.GeneratedMapping]
) -> list[models.GeneratedMapping]:
    return [wmap_generated_mapping(statement_id, m) for m in generated_mappings]


def wmap_generated_mapping(
    statement_id: UUID | None, source_mapping: language.GeneratedMapping
) -> models.GeneratedMapping:
    return models.GeneratedMapping(
        type=source_mapping.type,
        statement_id=statement_id,
        source_id=source_mapping.source_id,
        source_revision=source_mapping.source_revision,
        target_id=source_mapping.target_id,
        target_revision=source_mapping.target_revision,
    )


def rmap_generated_mapping(source_mapping: models.GeneratedMapping) -> language.GeneratedMapping:
    return language.GeneratedMapping(
        type=source_mapping.type,
        source_id=source_mapping.source_id,
        source_revision=source_mapping.source_revision,
        target_id=source_mapping.target_id,
        target_revision=source_mapping.target_revision,
    )


def rmap_xblocks(xblocks: list[models.XBlock]) -> list[wire.XBlockData]:
    """Reads a list of database xblocks into a list of wire xblocks."""
    xblocks = [
        wire.XBlockData(
            id=x.id,
            statement_id=x.statement_id,
            order_key=x.order_key,
            kind=x.kind,
            source=x.source,
            value=x.value,
            description=x.description,
            path=x.path,
            revision=x.revision,
        )
        for x in xblocks
    ]
    xblocks.sort(key=lambda x: x.order_key)
    return xblocks


def wmap_xblocks(
    statement: models.Statement, xblocks: list[wire.XBlockData]
) -> list[models.XBlock]:
    """Writes a list of wire xblocks into a list of database xblocks."""
    return [
        models.XBlock(
            id=x.id,
            statement=statement,
            order_key=x.order_key,
            kind=x.kind,
            source=x.source,
            value=x.value,
            description=x.description,
            revision=x.revision,
            path=x.path,
        )
        for x in xblocks
    ]


def rmap_simple_type_node(node: models.SimpleTypeNode) -> wire.SimpleTypeNodeData:
    return wire.SimpleTypeNodeData(
        id=node.id,
        revision=node.revision,
        name=node.name,
        tag=node.tag,
        statement_id=node.statement_id,
        key=node.key,
        order_key=node.order_key,
        description=node.description,
        is_output=node.is_output,
        is_array=node.is_array,
        is_nullable=node.is_nullable,
        value=node.value,
        reference_id=node.reference_id,
    )


def wmap_simple_type_node(
    statement_id: UUID, node: wire.SimpleTypeNodeData
) -> models.SimpleTypeNode:
    return models.SimpleTypeNode(
        id=node.id,
        statement_id=statement_id,
        key=node.key,
        order_key=node.order_key,
        name=node.name,
        tag=node.tag,
        description=node.description,
        is_output=node.is_output,
        is_array=node.is_array,
        is_nullable=node.is_nullable,
        value=node.value,
        reference_id=node.reference_id,
    )


def rmap_job(job: JobData) -> models.Job:
    now = datetime.utcnow().replace(tzinfo=pytz.utc)
    return models.Job(
        id=job.id,
        type=job.type,
        status=job.status,
        project_id=job.project_id,
        project_version_id=job.project_version_id,
        deployment_id=job.deployment_id,
        worker_id=job.worker_id,
        started_at=job.started_at,
        terminated_at=job.terminated_at,
        created_at=now,
        updated_at=now,
    )


def wmap_job(job: models.Job) -> JobData:
    return JobData(
        id=job.id,
        type=job.type,
        status=job.status,
        project_id=job.project_id,
        project_version_id=job.project_version_id,
        deployment_id=job.deployment_id,
        worker_id=job.worker_id,
        started_at=job.started_at,
        terminated_at=job.terminated_at,
    )


def rmap_execution_frame(frame: ExecutionFrameData) -> models.Execution:
    if frame.error:
        status = models.ExecutionStatus.Failed
    elif frame.exited_at:
        status = models.ExecutionStatus.Completed
    elif frame.queue_position:
        status = models.ExecutionStatus.Queued
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
        project_id=frame.project_id,
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
        cached_generated_at=frame.cached_generated_at,
        cached_duration=frame.cached_duration,
        inputs=frame.inputs,
        outputs=frame.outputs,
        error=asdict(frame.error) if frame.error else None,
        # additional context
        tracing_level=frame.tracing_level,
        deployment_id=frame.deployment_id,
        worker_id=frame.worker_id,
        trigger_type=frame.trigger_type,
        user_id=user_id,
        access_token_id=access_token_id,
    )


def wmap_execution_frame(frame: models.Execution) -> ExecutionFrameData:
    return ExecutionFrameData(
        id=frame.id,
        project_id=frame.project_id,
        module_id=frame.project_version_id,
        root_id=frame.root_id,
        parent_id=frame.parent_id,
        build_id=frame.build_id,
        task_id=frame.task_id,
        code_id=frame.code_id,
        model_id=frame.model_id,
        entered_at=frame.started_at,
        exited_at=frame.terminated_at,
        cached_generated_at=frame.cached_generated_at,
        cached_duration=frame.cached_duration,
        queue_position=None,
        inputs=frame.inputs,
        outputs=frame.outputs,
        error=RunErrorData.from_dict(frame.error) if frame.error else None,
        # additional context
        tracing_level=frame.tracing_level,
        deployment_id=frame.deployment_id,
        worker_id=frame.worker_id,
        trigger_type=frame.trigger_type,
        trigger_id=frame.user_id or frame.access_token_id,
    )


def rmap_evaluation_result(evaluation: EvaluationResultData) -> models.EvaluationResult:
    now = datetime.utcnow().replace(tzinfo=pytz.utc)
    return models.EvaluationResult(
        id=evaluation.id,
        created_at=now,
        updated_at=now,
        kind=evaluation.kind,
        scope=evaluation.scope,
        project_id=evaluation.project_id,
        project_version_id=evaluation.project_version_id,
        job_id=evaluation.job_id,
        build_id=evaluation.build_id,
        build_candidate_id=evaluation.build_candidate_id,
        statement_id=evaluation.statement_id,
        record_id=evaluation.record_id,
        type_node_id=evaluation.type_node_id,
        system_id=evaluation.system_id,
        environment_id=evaluation.environment_id,
        aggregated_metrics=evaluation.aggregated_metrics,
        self_metrics=evaluation.self_metrics,
    )
