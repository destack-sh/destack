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
from bench.api.sync import BatchMutationInput, check_can_write_thing, tracked_os_mutation
from bench.api.type import MMT
from bench.api.utils import CrudModel
from bench.opensearch import mirror


@gql.type
class Record(CrudModel, gql.Node):
    statement_id: GlobalID
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
    @tracked_os_mutation(MMT.CREATE_RECORD)
    def create_record(self, info: Info, input: RecordCreateInput) -> Record | OperationInfo:
        statement = models.Statement.objects.get(id=input.statement_id.node_id)
        project_v = check_can_write_thing(info, statement)
        now = datetime.utcnow().replace(tzinfo=pytz.utc)
        record = mirror.Record(
            id=UUID(input.id.node_id),
            statement_id=input.statement_id.node_id,
            created_at=now,
            updated_at=now,
            last_edited_at=now,
            revision=0,
            order_key=input.order_key,
            data=input.data,
        )
        # nocheckin: actually index/update/delete/etc.
        return project_v, statement, record  # noqa (will be unwrapped)

    @tracked_os_mutation(MMT.UPDATE_RECORD)
    def update_record(self, info: Info, input: RecordUpdateInput) -> Record | OperationInfo:
        raise NotImplementedError

    @tracked_os_mutation(MMT.MOVE_RECORD)
    def move_record(self, input: RecordMoveInput) -> Record | OperationInfo:
        raise NotImplementedError

    @tracked_os_mutation(MMT.SOFT_DELETE_RECORD)
    def soft_delete_record(self, input: RecordDeleteInput) -> Record | OperationInfo:
        raise NotImplementedError

    @tracked_os_mutation(MMT.DELETE_RECORD)
    def delete_record(self, input: RecordDeleteInput) -> Record | OperationInfo:
        raise NotImplementedError

    @tracked_os_mutation(MMT.RESTORE_RECORD)
    def restore_record(self, input: RecordRestoreInput) -> Record | OperationInfo:
        raise NotImplementedError

    @tracked_os_mutation(MMT.SOFT_DELETE_RECORD, batch=True, register=False)
    def batch_soft_delete_record(
        self, input: RecordBatchSoftDeleteInput
    ) -> RecordBatch | OperationInfo:
        # imitate soft_delete_record but for a batch
        raise NotImplementedError

    @tracked_os_mutation(MMT.RESTORE_RECORD, batch=True, register=False)
    def batch_restore_record(self, input: RecordBatchRestoreInput) -> RecordBatch | OperationInfo:
        # imitate restore_record but for a batch
        raise NotImplementedError


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
