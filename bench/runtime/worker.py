from dataclasses import dataclass
from datetime import datetime
from typing import Optional
from uuid import UUID

from strawberry_django_plus import gql

from bench.api.symbol import Statement, TypeNode
from bench.language import Module


@gql.type
class Task:
    id: UUID
    type: str
    name: str
    started_at: datetime


@gql.type
class InterpStatement(Statement):
    type: Optional[TypeNode]


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


@dataclass
class ModuleWorkerState:
    module_id: UUID
    module_state: ModuleState
    module: Module


async def start_worker(worker_id: UUID):
    # TODO @Incomplete: get initial module state from zmq server, subscribe to changes
    module = None

    # TODO @Incomplete: trigger and tasks and send out updated module state

    # TODO @Incomplete: write back compilation results to zmq server
    pass
