import structlog

import bench.search.core as os
from bench import language as lang
from bench.language.const import RUNNABLE_STATEMENT_TYPES, TypeFlag
from bench.language.edit import MET, MNT
from bench.language.module import Module
from bench.language.run import HasRun
from bench.search import mirror
from bench.search.client import os_client_sync
from bench.search.core import IndexType
from bench.search.mapping import map_to_os_field

logger = structlog.get_logger(__name__)

DOCUMENTS_BY_INDEX = {
    IndexType.GLOBAL: [
        mirror.User,
        mirror.Organization,
        mirror.Project,
        mirror.ProjectVersion,
        mirror.File,
        mirror.Statement,
        mirror.Field,
        mirror.Tile,
        mirror.Comment,
    ],
    IndexType.LOCAL: [
        mirror.Record,
        mirror.Session,
        mirror.Run,
        mirror.LogEntry,
    ],
}

SEARCH_SEMANTIC_EDIT_TYPES = {
    MET.CREATE_FIELD,
    MET.UPDATE_FIELD,
    MET.UPDATE_FIELD_TYPE,
    MET.DELETE_FIELD,
    MET.TRUNCATE_RESOLVED_FIELDS,
    MET.CREATE_RESOLVED_FIELD,
}

BENCH_LOCAL_MNTS = (MNT.RECORD,)


def update_os_schema(os_name: str, module: Module, dynamic: str = "strict") -> None:
    """
    Updates *all* OpenSearch field mappings for a module
    TODO @Performance: update OS field mappings more efficiently on field edit
      (especially for library/dependency mappings)
    """
    from bench.language import libs

    value_mappings: dict[str, os.Field] = {}
    inputs_mappings: dict[str, os.Field] = {}
    outputs_mappings: dict[str, os.Field] = {}

    # get library mappings
    for lib in libs.DEFAULT_MODULES.values():
        for node in lib._nodes:
            if HasRun in node._components:
                for field in node.resolved_fields:
                    if field.flags & TypeFlag.IS_OUTPUT:
                        outputs_mappings[field._typed_key] = map_to_os_field(field)
                    else:
                        inputs_mappings[field._typed_key] = map_to_os_field(field)
    # ensure library vectors are not indexed (would be pointless waste of resources)
    for field in (*inputs_mappings.values(), *outputs_mappings.values()):
        for f in field.walk():
            if f.type == os.FieldType.KNN_VECTOR:
                f.index = False

    # and 'static' value mappings (hard-coded)
    for value_type in (libs.symbolx_lib.resolve(".reflect.RunMetadata"),):
        for field in value_type.resolved_fields:
            value_mappings[field._typed_key] = map_to_os_field(field)

    # add dynamic user mappings
    for node in module._nodes:
        if not isinstance(node, lang.Statement):
            continue
        if node.self_errors:
            continue  # ignore symbols with issues
        elif node.type == lang.StatementType.DATABASE:
            # all fields go into Record.value
            for field in node.resolved_fields:
                value_mappings[field._typed_key] = map_to_os_field(field)
        elif node.type in RUNNABLE_STATEMENT_TYPES:
            # inputs into Execution.inputs, outputs into Execution.outputs
            for field in node.resolved_fields:
                if field.flags & TypeFlag.IS_OUTPUT:
                    outputs_mappings[field._typed_key] = map_to_os_field(field)
                else:
                    inputs_mappings[field._typed_key] = map_to_os_field(field)

    # actually update mappings
    mappings = {}
    for key, sub_mappings in (
        ("value", value_mappings),
        ("inputs", inputs_mappings),
        ("outputs", outputs_mappings),
    ):
        sub_mappings = {k: v.to_dict() for (k, v) in sub_mappings.items()}
        mappings[key] = {"type": "object", "dynamic": dynamic, "properties": sub_mappings}

    os_client_sync.indices.put_mapping(index=os_name, body={"properties": mappings})
    logger.info(
        "os.update_mappings.done",
        module=module,
        value_mappings=len(value_mappings),
        inputs_mappings=len(inputs_mappings),
        outputs_mappings=len(outputs_mappings),
    )
