import os
import random
import secrets
import string
from contextlib import contextmanager
from dataclasses import dataclass
from typing import TYPE_CHECKING, Mapping, cast

import grpclib
import pytest
from grpclib.testing import ChannelFor
from pytest_asyncio import is_async_test

from bench.proto.wire import (
    ClientDataIn,
    ClientOrigin,
    HostBase,
    HostStub,
    NodeReferenceData,
    RpcMetadata,
    SupervisorBase,
    SupervisorStub,
)
from bench.utils.oracle import get_oracle

if TYPE_CHECKING:
    from bench.language import Bench
    from bench.language.user import Client, User
    from bench.proto.monkey import _PatchedRpcMetadata


def bench_session(bench: "Bench", epoch: int = 0):
    from bench.language.session import Session
    from bench.system.core import GLOBAL_POSTGRES_ENGINE

    assert bench.main_branch is not None, f"{bench!r} has no main branch"
    return Session(
        parent=bench.main_branch.main_package, _engines=(GLOBAL_POSTGRES_ENGINE,), _epoch=epoch
    )


def pytest_configure(config):
    os.environ["ENVIRONMENT"] = "test"
    from bench.utils.env import setup_dotenv

    setup_dotenv()

    from bench.utils.logging import setup_logging

    setup_logging()

    from bench.language.setup import _complete_bench_setup

    _complete_bench_setup()


def pytest_collection_modifyitems(items):
    pytest_asyncio_tests = (item for item in items if is_async_test(item))
    session_scope_marker = pytest.mark.asyncio(scope="session")
    for async_test in pytest_asyncio_tests:
        async_test.add_marker(session_scope_marker)


@pytest.fixture(autouse=True, scope="session")
async def _prepared_test_db():
    from bench.sql.client import pg_store_connection
    from bench.sql.engine import GLOBAL_SCHEMA
    from bench.sql.migration import (
        EXTENSIONS,
        apply_sql_migration_ops,
        generate_sql_migration_ops,
        introspect_sql_schema,
    )
    from bench.system.core import GLOBAL_PG_NAME, GLOBAL_STORE, global_pg_cursor

    # ensure that default global_db_cursor points to test (means environment info was set up correctly)
    # if this fails, it's likely we mistakenly imported from bench.utils.env before our pytest_configure
    #  could override it, usually because of an innocent (cascading) import in our conftests.
    assert GLOBAL_PG_NAME == "test"

    # reset test database (connect to bench since we can't drop active db)
    #  (reconstruct default connection str here because GLOBAL_PG_NAME is different in test)
    async with pg_store_connection(GLOBAL_STORE, database="bench", autocommit=True) as cur:
        await cur.execute("DROP DATABASE IF EXISTS test")
        await cur.execute("CREATE DATABASE test")

    # migrate to current global schema
    async with global_pg_cursor() as cur:
        for extension in EXTENSIONS:
            await cur.execute(f"CREATE EXTENSION IF NOT EXISTS {extension}")
        blank_schema = await introspect_sql_schema(cur)
        blank_ops = generate_sql_migration_ops(blank_schema, GLOBAL_SCHEMA)
        await apply_sql_migration_ops(cur, blank_ops)
        await cur.connection.commit()


@pytest.fixture()
async def test_cur():
    from bench.sql.client import pg_store_connection
    from bench.system.core import GLOBAL_STORE

    async with pg_store_connection(GLOBAL_STORE, database="test") as cur:
        yield cur


@contextmanager
def raises_grpc_error(*statuses: grpclib.const.Status):
    with pytest.raises(grpclib.GRPCError) as exc_info:
        yield
    if statuses:
        assert exc_info.value.status in statuses, f"expected {statuses}, got {exc_info!r}"


@pytest.fixture()
async def session():
    from bench.language import Bench

    bench = Bench(slug="test", name="Test")
    async with bench_session(bench) as session:
        yield session


@pytest.fixture(scope="module")
async def shared_session():
    from bench.language import Bench

    bench = Bench(slug="test", name="Test")
    async with bench_session(bench) as session:
        yield session


@pytest.fixture(scope="session")
async def supervisor_service():
    from bench.system.supervisor import Supervisor

    service = Supervisor()
    await service.start()
    try:
        yield service
    finally:
        service.close()
        await service.wait_closed()


@pytest.fixture(scope="session")
async def host_service():
    from bench.system.host import HostRouter

    service = HostRouter()
    await service.start()
    try:
        yield service
    finally:
        service.close()
        await service.wait_closed()


# NOTE: we cannot keep gRPC service stubs across function boundaries because pytest-async
#   creates a new loop for each test function, and gRPC services are tied to the loop.
#   :PytestAsyncWeirdness


@pytest.fixture()  # :PytestAsyncWeirdness
async def supervisor(supervisor_service: SupervisorBase):
    async with ChannelFor([supervisor_service]) as channel:
        stub = SupervisorStub(channel)
        yield stub


@pytest.fixture()  # :PytestAsyncWeirdness
async def host(host_service: HostBase):
    async with ChannelFor([host_service]) as channel:
        stub = HostStub(channel)
        yield stub


@dataclass(slots=True)
class UserHandle:
    user: "User"
    client: "Client"
    subject: "NodeReferenceData"
    origin: "ClientOrigin"
    metadata: "RpcMetadata"
    headers: Mapping[str, str]


async def make_new_user_handle(
    supervisor: "SupervisorStub",
    user: "User",
    *,
    password: str | None = None,
    client_name: str = "Macbook Pro",
) -> UserHandle:
    """Signs up a new user and returns a handle for the user and a client."""

    from bench.language import Client, ClientType
    from bench.proto import wire
    from bench.proto.wire import ClientOrigin, NodeReferenceData, RpcMetadata, SignupUserRequest

    if password is None:
        password = secrets.token_hex(8)
    client = Client(
        parent=user,
        type=ClientType.BENCH_WEB,
        name=f"{user.name}'s {client_name}",
        device_name="pytest",
        seen_at=get_oracle().utc(),
    )
    signup_req = SignupUserRequest(
        id=str(user.id),
        slug=cast(str, user.slug),
        name=user.name,
        email=user.email,
        password=password,
        client=cast(ClientDataIn, client._to_data()),
    )
    signup_rep = await supervisor.signup_user(signup_req)
    origin = ClientOrigin(
        type=wire.ClientType(client.type), id=str(client.id), nonce=str(random.randint(0, 2**32))
    )
    subject = NodeReferenceData(
        metatype=wire.ObjectType.NODE_REFERENCE, type=wire.NodeType.USER, id=str(user.id)
    )
    metadata = RpcMetadata(
        client_type=wire.ClientType(client.type),
        client_id=str(client.id),
        client_access_token=signup_rep.access_token,
    )
    handle = UserHandle(
        user=user,
        client=client,
        origin=origin,
        subject=subject,
        metadata=metadata,
        headers=metadata.to_headers(),  # type: ignore
    )
    return handle


async def make_existing_user_handle(
    supervisor: "SupervisorStub", user: "User", *, password: str, client_name: str
) -> UserHandle:
    """Logs in an existing user and returns a handle for the user and a client."""

    from bench.language import Client, ClientType
    from bench.proto import wire
    from bench.proto.wire import ClientOrigin, LoginUserRequest, NodeReferenceData, RpcMetadata

    client = Client(
        parent=user,
        type=ClientType.BENCH_WEB,
        name=f"{user.name}'s {client_name}",
        device_name="pytest",
        seen_at=get_oracle().utc(),
    )
    login_req = LoginUserRequest(
        id=str(user.id),
        slug=cast(str, user.slug),
        email=user.email,
        password=password,
        client=cast(ClientDataIn, client._to_data()),
    )
    login_rep = await supervisor.login_user(login_req)
    origin = ClientOrigin(id=str(client.id), nonce=str(random.randint(0, 2**32)))
    subject = NodeReferenceData(
        metatype=wire.ObjectType.NODE_REFERENCE, type=wire.NodeType.USER, id=str(user.id)
    )
    metadata = RpcMetadata(client_id=str(client.id), client_access_token=login_rep.access_token)
    handle = UserHandle(
        user=user,
        client=client,
        origin=origin,
        subject=subject,
        metadata=metadata,
        headers=cast("_PatchedRpcMetadata", metadata).to_headers(),
    )
    return handle


async def make_random_user_handle(supervisor: "SupervisorStub") -> UserHandle:
    from bench.language import User
    from bench.language.const import UserStatus

    random_slug = "".join(random.choices(string.ascii_letters, k=10)).lower()
    random_email = f"{random_slug}@symbolx.com"
    user = User(slug=random_slug, name=random_slug, email=random_email, status=UserStatus.INVITED)
    return await make_new_user_handle(
        supervisor, user, password=secrets.token_hex(8), client_name=secrets.token_hex(8)
    )


@pytest.fixture()
async def some_user(supervisor: "SupervisorStub") -> UserHandle:
    return await make_random_user_handle(supervisor)
