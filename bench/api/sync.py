import enum
from typing import AsyncGenerator, Optional

import zmq
from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID

from bench.settings import ZMQ_API_SERVER_ADDR
from bench.zmq import zmq_ctx

project_change_pub = zmq_ctx.socket(zmq.PUB)
project_change_pub.bind(ZMQ_API_SERVER_ADDR)


@gql.enum
class ProjectMutationType(enum.Enum):
    CREATE_FILE = "CREATE_FILE"
    DELETE_FILE = "DELETE_FILE"
    RESTORE_FILE = "RESTORE_FILE"
    RENAME_FILE = "RENAME_FILE"
    MOVE_FILE = "MOVE_FILE"
    CREATE_STATEMENT = "CREATE_STATEMENT"
    DELETE_STATEMENT = "DELETE_STATEMENT"
    RESTORE_STATEMENT = "RESTORE_STATEMENT"
    COMMIT = "COMMIT"


@gql.type
class ProjectMutation:
    type: ProjectMutationType
    project_version_id: GlobalID
    file_id: Optional[GlobalID]
    statement_id: Optional[GlobalID]


@gql.type
class ProjectSubscription:
    @gql.subscription
    async def project_changed(self, project_id: GlobalID) -> AsyncGenerator[ProjectMutation, None]:
        raise NotImplementedError
