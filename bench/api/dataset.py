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
from bench.api.utils import CrudModel, Revisioned
from bench.opensearch import mirror
from bench.opensearch.index import create_record, update_record, delete_record, batch_update_records


@gql.type
class Record(CrudModel, Revisioned, gql.Node):
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
class RecordInput(gql.NodeInput):
    statement_id: GlobalID


@gql.input
class RecordCreateInput(RecordInput):
    statement_id: GlobalID
    data: JSON
    order_key: str


@gql.input
class RecordUpdateInput(RecordInput):
    statement_id: GlobalID
    data: JSON


@gql.input
class RecordUpdatePathInput(RecordInput):
    statement_id: GlobalID
    path: str
    data: Optional[JSON] = None


@gql.input
class RecordMoveInput(RecordInput):
    statement_id: GlobalID
    order_key: str


@gql.input
class RecordDeleteInput(RecordInput):
    statement_id: GlobalID


@gql.input
class RecordRestoreInput(RecordInput):
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


def _prep_dataset_access(
    info: Info, input: RecordInput
) -> tuple[datetime, models.ProjectVersion, models.Statement]:
    statement = models.Statement.objects.get(id=input.statement_id.node_id)
    project_v = check_can_write_thing(info, statement)
    now = datetime.utcnow().replace(tzinfo=pytz.utc)
    return now, project_v, statement


@gql.type
class DatasetMutation:
    @tracked_os_mutation(MMT.CREATE_RECORD)
    def create_record(self, info: Info, input: RecordCreateInput) -> Record | OperationInfo:
        now, project_v, statement = _prep_dataset_access(info, input)
        record = mirror.Record(
            id=UUID(input.id.node_id),
            statement_id=input.statement_id.node_id,
            created_at=now,
            updated_at=now,
            last_edited_at=now,
            order_key=input.order_key,
            data=input.data,
        )
        record = create_record(project_v, record)
        return project_v, statement, record  # noqa (will be unwrapped)

    @tracked_os_mutation(MMT.UPDATE_RECORD)
    def update_record(self, info: Info, input: RecordUpdateInput) -> Record | OperationInfo:
        now, project_v, statement = _prep_dataset_access(info, input)
        record = mirror.Record(
            id=UUID(input.id.node_id),
            data=input.data,
            updated_at=now,
            last_edited_at=now,
        )
        record = update_record(project_v, record)
        return project_v, statement, record  # noqa

    @tracked_os_mutation(MMT.MOVE_RECORD)
    def move_record(self, info: Info, input: RecordMoveInput) -> Record | OperationInfo:
        now, project_v, statement = _prep_dataset_access(info, input)
        record = mirror.Record(
            id=UUID(input.id.node_id),
            order_key=input.order_key,
            updated_at=now,
            last_edited_at=now,
        )
        record = update_record(project_v, record)
        return project_v, statement, record  # noqa

    @tracked_os_mutation(MMT.SOFT_DELETE_RECORD)
    def soft_delete_record(self, info: Info, input: RecordDeleteInput) -> Record | OperationInfo:
        now, project_v, statement = _prep_dataset_access(info, input)
        record = mirror.Record(
            id=UUID(input.id.node_id),
            deleted_at=now,
            updated_at=now,
        )
        record = update_record(project_v, record)
        return project_v, statement, record  # noqa

    @tracked_os_mutation(MMT.RESTORE_RECORD)
    def restore_record(self, info: Info, input: RecordRestoreInput) -> Record | OperationInfo:
        now, project_v, statement = _prep_dataset_access(info, input)
        record = mirror.Record(
            id=UUID(input.id.node_id),
            deleted_at="-",  # invalid value to set to null
            updated_at=now,
        )
        record = update_record(project_v, record)
        return project_v, statement, record  # noqa

    @tracked_os_mutation(MMT.DELETE_RECORD)
    def delete_record(self, info: Info, input: RecordDeleteInput) -> Record | OperationInfo:
        now, project_v, statement = _prep_dataset_access(info, input)
        delete_record(project_v, UUID(input.id.node_id))
        return project_v, statement, None  # noqa

    @tracked_os_mutation(MMT.SOFT_DELETE_RECORD, batch=True, register=False)
    def batch_soft_delete_record(
        self, info: Info, input: RecordBatchSoftDeleteInput
    ) -> RecordBatch | OperationInfo:
        # imitate soft_delete_record but for a batch
        now, project_v, statement = _prep_dataset_access(info, input)
        records = [
            mirror.Record(
                id=UUID(i.node_id),
                deleted_at=now,
                updated_at=now,
            )
            for i in input.unbatch()
        ]
        records = batch_update_records(project_v, records)
        return project_v, statement, records  # noqa

    @tracked_os_mutation(MMT.RESTORE_RECORD, batch=True, register=False)
    def batch_restore_record(
        self, info: Info, input: RecordBatchRestoreInput
    ) -> RecordBatch | OperationInfo:
        # imitate restore_record but for a batch
        now, project_v, statement = _prep_dataset_access(info, input)
        records = [
            mirror.Record(
                id=UUID(i.node_id),
                deleted_at="-",  # invalid value to set to null
                updated_at=now,
            )
            for i in input.unbatch()
        ]
        records = batch_update_records(project_v, records)
        return project_v, statement, records  # noqa


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
