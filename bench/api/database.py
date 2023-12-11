from typing import Optional
from uuid import UUID

import strawberry
import strawberry_django
from asgiref.sync import sync_to_async
from strawberry import relay
from strawberry.relay import GlobalID, PageInfo
from strawberry.scalars import JSON
from strawberry.types import Info
from strawberry_django.fields.types import OperationInfo

from bench import models
from bench.api.auth import check_module_node_access
from bench.api.sync import bench_edit
from bench.api.type import EditType
from bench.api.utils import (
    Conditional,
    HasCrud,
    QueryConnectionWithTotalCount,
    Revisioned,
    Sort,
    ThingBatch,
)
from bench.language import wire
from bench.models import ModuleAccessLevel
from bench.msg import NMessage
from bench.msg.core import request
from bench.msg.messages import NMessageType, RepSearchRecordsPayload, ReqSearchRecordsPayload
from bench.search import mirror
from bench.utils.dt import utcnow_with_tz


@strawberry.type
class Record(HasCrud, Revisioned, relay.Node):
    id: relay.NodeID[UUID]
    ck: UUID
    value: JSON

    @staticmethod
    def from_os(record: mirror.Record) -> "Record":
        return Record(
            id=record.id,
            ck=record.ck,
            value=record.value,
            revision=record.revision,
            created_at=record.created_at,
            created_by=None,
            updated_at=record.updated_at,
            deleted_at=record.deleted_at if record.deleted_at != "-" else None,
            last_edited_at=record.last_edited_at,
            last_edited_by=None,
        )

    @staticmethod
    def from_wire(record: wire.RecordData) -> "Record":
        return Record(
            id=record.id,
            ck=record.ck,
            value=record.value,
            revision=record.revision,
            created_at=record.created_at,
            created_by=None,
            updated_at=record.updated_at,
            deleted_at=record.deleted_at,
            last_edited_at=record.last_edited_at,
            last_edited_by=None,
        )


@strawberry.type
class RecordBatch(ThingBatch):
    records: list[Record]

    @property
    def things(self):
        return self.records

    @staticmethod
    def from_os(batch: "RecordBatch") -> "RecordBatch":
        return RecordBatch(
            records=[Record.from_os(r) for r in batch.records],
        )


@strawberry.input
class RecordInput:
    statement_id: GlobalID


@strawberry.input
class RecordCreateInput(RecordInput, strawberry_django.NodeInput):
    ck: UUID
    value: JSON
    statement_ck: UUID
    statement_key: str


@strawberry.input
class RecordUpdateInput(RecordInput, strawberry_django.NodeInput):
    value: JSON


@strawberry.input
class RecordUpdatePathInput(RecordInput, strawberry_django.NodeInput):
    path: str
    value: Optional[JSON] = None


@strawberry.input
class RecordDeleteInput(RecordInput, strawberry_django.NodeInput):
    pass


@strawberry.input
class RecordRestoreInput(RecordInput, strawberry_django.NodeInput):
    pass


@strawberry.type
class RecordMutation:
    @bench_edit(EditType.CREATE_RECORD, return_transform=Record.from_wire)
    def create_record(self, input: RecordCreateInput) -> Record | OperationInfo:
        # records are written to the local pg database (not via Django), so need to set CRU fields
        # not great but won't matter soon with the new edits system :)
        now = utcnow_with_tz()
        record = wire.RecordData(
            id=UUID(input.id.node_id),
            ck=input.ck,
            created_at=now,
            updated_at=now,
            deleted_at=None,
            parent_id=UUID(input.statement_id.node_id),
            last_edited_at=now,
            last_changed_at=now,
            revision=0,
            value=input.value,
        )
        return record

    @bench_edit(EditType.UPDATE_RECORD, return_transform=Record.from_wire)
    def update_record(self, input: RecordUpdateInput) -> Record | OperationInfo:
        # copy pasta because it doesn't matter and these no longer exist in the DB
        now = utcnow_with_tz()
        record = wire.RecordData(
            id=UUID(input.id.node_id),
            ck=None,
            created_at=now,
            updated_at=now,
            deleted_at=None,
            parent_id=UUID(input.statement_id.node_id),
            last_edited_at=now,
            last_changed_at=now,
            revision=0,
            value=input.value,
        )
        return record

    @bench_edit(EditType.SOFT_DELETE_RECORD, return_transform=Record.from_wire)
    def soft_delete_record(self, input: RecordDeleteInput) -> Record | OperationInfo:
        # copy pasta because it doesn't matter and these no longer exist in the DB
        now = utcnow_with_tz()
        record = wire.RecordData(
            id=UUID(input.id.node_id),
            ck=None,
            created_at=None,
            updated_at=now,
            deleted_at=None,
            parent_id=UUID(input.statement_id.node_id),
            last_edited_at=now,
            last_changed_at=now,
            revision=0,
            value=None,
        )
        return record

    @bench_edit(EditType.RESTORE_RECORD, return_transform=Record.from_wire)
    def restore_record(self, input: RecordRestoreInput) -> Record | OperationInfo:
        # copy pasta because it doesn't matter and these no longer exist in the DB
        now = utcnow_with_tz()
        record = wire.RecordData(
            id=UUID(input.id.node_id),
            ck=None,
            created_at=None,
            updated_at=now,
            deleted_at=None,
            parent_id=UUID(input.statement_id.node_id),
            last_edited_at=now,
            last_changed_at=now,
            revision=0,
            value=None,
        )
        return record

    @bench_edit(EditType.DELETE_RECORD, return_transform=Record.from_wire)
    def delete_record(self, input: RecordDeleteInput) -> Record | OperationInfo:
        raise NotImplementedError


RECORDS_LIMIT = 100


@strawberry.type
class RecordQuery:
    @strawberry_django.field
    async def search_records(
        self,
        info: Info,
        statement_id: GlobalID,
        query: Optional[Conditional] = None,
        sort: Optional[list[Sort]] = None,
        after: Optional[str] = None,
        limit: Optional[int] = None,
        count: Optional[bool] = None,
    ) -> QueryConnectionWithTotalCount[Record]:
        statement = await models.Statement._base_manager.aget(id=statement_id.node_id)
        access = await sync_to_async(check_module_node_access)(
            info, statement, ModuleAccessLevel.Read
        )

        query = query.to_bench() if query else None
        sort = [s.to_bench() for s in sort] if sort else None
        effective_limit = min(limit or RECORDS_LIMIT, RECORDS_LIMIT)
        req = ReqSearchRecordsPayload(
            module_id=access.project_version.id,
            statement_id=statement.id,
            statement_ck=statement.ck,
            statement_key=statement.key,
            query=wire.pack_data(query) if query is not None else None,
            sort=[wire.pack_data(s) for s in sort] if sort is not None else None,
            limit=effective_limit + 1,
            after=after,
            count=count or False,
        )
        rep: NMessage[RepSearchRecordsPayload] = await request(NMessageType.SEARCH_RECORDS, req)
        cursors = rep.p.cursors[:effective_limit] if rep.p.cursors is not None else None
        if rep.p.records is not None:
            records = [Record.from_wire(r) for r in rep.p.records[:effective_limit]]
            edges = []
            for cursor, record in zip(cursors, records):
                node = Record.from_os(record)
                edge = relay.Edge(node=node, cursor=cursor)
                edges.append(edge)
        else:
            edges = []
        page_info = PageInfo(
            start_cursor=rep.p.start_cursor,
            end_cursor=cursors[-1] if cursors else None,
            has_next_page=bool(rep.p.records) and len(rep.p.records) > effective_limit,
            has_previous_page=False,
        )
        return QueryConnectionWithTotalCount(
            edges=edges, page_info=page_info, total_count=rep.p.total, engine=rep.p.engine
        )
