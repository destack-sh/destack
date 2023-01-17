from datetime import datetime
from uuid import UUID

from strawberry_django_plus import gql

from bench.api.symbol import Statement, TypeNode


@gql.type
class Task:
    id: UUID
    type: str
    name: str
    started_at: datetime


@gql.type
class InterpStatement(Statement):
    type: TypeNode


@gql.type
class ModuleState:
    id: UUID
    revision_hash: str
    name: str
    tasks: list[Task]
    all_symbols: list[InterpStatement]
    errors: list["ModuleError"]


@gql.type
class ModuleError:
    type: str
    message: str
    statement: Statement


class ModuleLanguageWorker:
    def __init__(self, module_id: UUID):
        self.module_id = module_id

    async def start(self):
        # TODO @Incomplete: get initial module state from zmq server, subscribe to changes

        # TODO @Incomplete: trigger and tasks and send out updated module state

        # TODO @Incomplete: write back compilation results to zmq server
        pass
