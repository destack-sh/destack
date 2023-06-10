from datetime import datetime
from typing import Optional
from uuid import UUID

import pytz
from strawberry.scalars import JSON
from strawberry.types import Info
from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID, PageInfo
from strawberry_django_plus.types import OperationInfo
from strawberry_django_plus.utils.resolvers import async_safe

from bench import models
from bench.api.statement import ThingBatch
from bench.api.sync import BatchMutationInput, tracked_mutation
from bench.api.type import MMT


@gql.type
class Record(gql.Node):
    statement_id: GlobalID
    revision: int
    created_at: datetime
    updated_at: Optional[datetime]
    deleted_at: Optional[datetime]
    order_key: str
    data: JSON


@gql.type
class RecordBatch(ThingBatch):
    records: list[Record]

    @property
    def things(self):
        return self.records


@gql.input
class RecordCreateInput(gql.NodeInput):
    statement_id: GlobalID
    data: JSON
    order_key: str


@gql.input
class RecordUpdateInput(gql.NodeInput):
    statement_id: GlobalID
    data: JSON


@gql.input
class RecordUpdatePathInput(gql.NodeInput):
    statement_id: GlobalID
    path: str
    data: Optional[JSON] = None


@gql.input
class RecordMoveInput(gql.NodeInput):
    statement_id: GlobalID
    order_key: str


@gql.input
class RecordDeleteInput(gql.NodeInput):
    statement_id: GlobalID


@gql.input
class RecordRestoreInput(gql.NodeInput):
    statement_id: GlobalID


@gql.input
class RecordBatchSoftDeleteInput(BatchMutationInput):
    statement_id: GlobalID
    ids: list[GlobalID]

    def unbatch(self) -> list:
        return [RecordDeleteInput(id=i) for i in self.ids]


@gql.input
class RecordBatchRestoreInput(BatchMutationInput):
    statement_id: GlobalID
    ids: list[GlobalID]

    def unbatch(self) -> list:
        return [RecordRestoreInput(id=i) for i in self.ids]


@gql.type
class DatasetMutation:
    @tracked_mutation(MMT.CREATE_RECORD)
    def create_record(self, input: RecordCreateInput) -> Record | OperationInfo:
        statement = models.Statement.objects.get(id=input.statement_id.node_id)
        record = models.Record(
            statement=statement,
            id=UUID(input.id.node_id),
            data=input.data,
            order_key=input.order_key,
        )
        return record

    @tracked_mutation(MMT.UPDATE_RECORD)
    def update_record(self, input: RecordUpdateInput) -> Record | OperationInfo:
        record = models.Record.objects.get(id=input.id.node_id)
        record._data = input.data
        return record

    @tracked_mutation(MMT.UPDATE_RECORD_PATH)
    def update_record_path(self, input: RecordUpdatePathInput) -> Record | OperationInfo:
        # update record data at the given path
        record = models.Record.objects.get(id=input.id.node_id)
        if input.value is None:
            del record._data[input.path]
        else:
            record._data[input.path] = input.value
        return record

    @tracked_mutation(MMT.MOVE_RECORD)
    def move_record(self, input: RecordMoveInput) -> Record | OperationInfo:
        record = models.Record.objects.get(id=input.id.node_id)
        record._order_key = input.order_key
        return record

    @tracked_mutation(MMT.SOFT_DELETE_RECORD)
    def soft_delete_record(self, input: RecordDeleteInput) -> Record | OperationInfo:
        record = models.Record.objects.get(id=input.id.node_id)
        record.soft_delete()
        return record

    @tracked_mutation(MMT.DELETE_RECORD)
    def delete_record(self, input: RecordDeleteInput) -> Record | OperationInfo:
        record = models.Record.objects.get(id=input.id.node_id)
        record.delete()
        return record

    @tracked_mutation(MMT.RESTORE_RECORD)
    def restore_record(self, input: RecordRestoreInput) -> Record | OperationInfo:
        # use _base_manager since soft deleted records are not visible
        record = models.Record._base_manager.get(id=input.id.node_id)
        record.restore()
        return record

    @tracked_mutation(MMT.SOFT_DELETE_RECORD, batch=True, register=False)
    def batch_soft_delete_record(
        self, input: RecordBatchSoftDeleteInput
    ) -> RecordBatch | OperationInfo:
        # imitate soft_delete_record but for a batch
        record_ids = [UUID(i.node_id) for i in input.ids]
        deleted_at = datetime.utcnow().replace(tzinfo=pytz.utc)
        models.Record.objects.filter(id__in=record_ids).update(deleted_at=deleted_at)
        # use base manager since the records are now deleted
        records = models.Record._base_manager.filter(id__in=record_ids)
        return RecordBatch(records=list(records))

    @tracked_mutation(MMT.RESTORE_RECORD, batch=True, register=False)
    def batch_restore_record(self, input: RecordBatchRestoreInput) -> RecordBatch | OperationInfo:
        # imitate restore_record but for a batch
        record_ids = [UUID(i.node_id) for i in input.ids]
        models.Record._base_manager.filter(id__in=record_ids).update(deleted_at=None)
        records = models.Record.objects.filter(id__in=record_ids)
        return RecordBatch(records=list(records))


@gql.type
class DatasetQuery:
    @gql.relay.connection
    @async_safe
    def search_records(self, info: Info, statement_id: GlobalID) -> gql.Connection[Record]:
        statement = models.Statement.objects.get(id=statement_id.node_id)
        # nocheckin: return actual dataset search
        return gql.Connection(
            edges=[],
            page_info=PageInfo(
                start_cursor=None, end_cursor=None, has_next_page=False, has_previous_page=False
            ),
            total_count=0,
        )
