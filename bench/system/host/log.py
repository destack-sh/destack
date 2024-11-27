from typing import Any, Sequence, cast, override

from bench.language.const import RUNTIME_NODE_TYPES, NodeType
from bench.language.node import NodeReference
from bench.language.session import Session
from bench.language.setup import NODE_CLASS_BY_TYPE
from bench.language.value import pack_builtin_object_data, pack_proto_json
from bench.proto import wire
from bench.proto.wire.lang_pb2 import EditData, LogData, RuntimeContextData
from bench.proto.wiring import unwrap_some_node, wrap_some_node
from bench.system.host.core import Commit, HostPlugin
from bench.utils.uuidt import UUIDT


class LogPlugin(HostPlugin):
    """Generate Logs for every Edit/Change."""

    @override
    async def on_commit_prepare(self, session: Session, commit: Commit) -> Sequence[EditData]:
        def _trim_node_packed_sensitive(node_type: NodeType, node_packed: dict[str, Any] | None):
            if node_packed is None:
                return None
            node_cls = NODE_CLASS_BY_TYPE[node_type]
            for prop_key in tuple(node_packed.keys()):
                prop = node_cls.__properties_by_id__.get(int(prop_key))
                assert prop is not None, f"missing prop {prop_key} in {node_cls!r}"
                if prop.is_sensitive:
                    del node_packed[prop_key]
            return pack_proto_json(node_packed)

        # add logs
        # NOTE :Incomplete: how does LogPlugin get the session context?
        context_data: RuntimeContextData = RuntimeContextData()
        package_ptr = session.package_ptr
        bench_ptr = session.bench_ptr
        assert package_ptr is not None, f"no package_ptr in {session!r}"
        assert bench_ptr is not None, f"no bench_ptr in {session!r}"
        package_ptr_data = package_ptr._to_data()
        bench_ptr_data = bench_ptr._to_data()
        log_edits: list[EditData] = []
        for edit in commit.edits:
            node_type = NodeType(edit.node_ptr.node_type)
            if node_type in RUNTIME_NODE_TYPES:
                continue
            assert edit.epoch is not None, f"epoch not set in {edit!r}"
            # pack old/new node
            if edit.HasField("node_data"):
                old_node = unwrap_some_node(edit.node_data)
                node_data = pack_builtin_object_data(old_node)
            else:
                node_data = None
            # trim secret properties
            node_data = _trim_node_packed_sensitive(node_type, node_data)
            log_data = LogData(
                metatype=wire.ObjectType.OBJECT_TYPE_LOG,
                id=str(UUIDT()),
                parent_ptr=package_ptr_data,
                package_ptr=package_ptr_data,
                bench_ptr=bench_ptr_data,
                created_at=edit.edited_at,
                created_epoch=edit.epoch,
                updated_at=edit.edited_at,
                updated_epoch=edit.epoch,
                # meta
                kind=wire.LogKind.LOG_KIND_CHANGE,
                level=wire.LogLevel.LOG_LEVEL_INFO,
                # content
                type=cast(wire.AccessType, edit.type),
                operations=edit.operations,
            )
            # meta
            if edit.category != 0:
                log_data.category = edit.category
            if edit.HasField("subject_ptr"):
                log_data.created_by_ptr.CopyFrom(edit.subject_ptr)
                log_data.updated_by_ptr.CopyFrom(edit.subject_ptr)
            if edit.HasField("undo_of_ptr"):
                log_data.undo_of_ptr.CopyFrom(edit.undo_of_ptr)
            # content
            if edit.HasField("node_ptr"):
                log_data.node_ptr.CopyFrom(edit.node_ptr)
            if edit.HasField("vignette"):
                log_data.vignette.CopyFrom(edit.vignette)
            if node_data is not None:
                log_data.node_data.CopyFrom(node_data)
            # session context
            if edit.context.session_ptr.metatype != 0:
                log_data.session_ptr.CopyFrom(edit.context.session_ptr)
            if edit.context.run_ptr.metatype != 0:
                log_data.run_ptr.CopyFrom(edit.context.run_ptr)
            if edit.context.run_root_ptr.metatype != 0:
                log_data.run_root_ptr.CopyFrom(edit.context.run_root_ptr)
            if context_data.client_ptr.metatype != 0:
                log_data.client_ptr.CopyFrom(context_data.client_ptr)
            if context_data.machine_ptr.metatype != 0:
                log_data.machine_ptr.CopyFrom(context_data.machine_ptr)
            if context_data.server_ptr.metatype != 0:
                log_data.server_ptr.CopyFrom(context_data.server_ptr)
            if context_data.user_ptr.metatype != 0:
                log_data.user_ptr.CopyFrom(context_data.user_ptr)
            if edit.context.identity_ptr.metatype != 0:
                log_data.identity_ptr.CopyFrom(edit.context.identity_ptr)
            create_log_edit = EditData(
                id=log_data.id,
                type=wire.EditType.EDIT_TYPE_CREATE,
                scope=edit.scope,
                node_ptr=NodeReference._ref_data_from_node_data(log_data),
                epoch=edit.epoch,
                node_data=wrap_some_node(log_data),
                edited_at=log_data.created_at,
            )
            if edit.HasField("subject_ptr"):
                create_log_edit.subject_ptr.CopyFrom(edit.subject_ptr)
            log_edits.append(create_log_edit)

        return log_edits
