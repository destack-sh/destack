from contextlib import contextmanager
from dataclasses import replace
from typing import TYPE_CHECKING
from uuid import uuid4

import grpclib
import pytest
from grpclib.testing import ChannelFor

from bench.language import Client, ReadOptions, User
from bench.language.const import (
    NodeType,
    PUBLIC_NODE_TYPES,
    EditType,
    ROOT_NODE_TYPES,
)
from bench.language.expression import A
from bench.proto import wire, wiring
from bench.proto.wire import (
    SupervisorStub,
    LoginUserRequest,
    LogoutUserRequest,
    GetNodesRequest,
    RpcMetadata,
    SignupUserRequest,
    EditData,
    CommitTransactionRequest,
    SearchNodesRequest,
    AggregationOp,
    AggregateNodesRequest,
    TransactionData,
)
from bench.system.supervisor import Supervisor
from bench.system.test.conftest import make_user_handle, UserHandle
from bench.utils.dt import utcnow_with_tz

if TYPE_CHECKING:
    from bench.language.test.fabricator import Fabricator


@pytest.fixture(scope="function")
async def supervisor() -> SupervisorStub:
    service = Supervisor()
    await service.start_quick()
    try:
        async with ChannelFor([service]) as channel:
            stub = SupervisorStub(channel)
            yield stub
    finally:
        service.close()
        await service.wait_closed()


@contextmanager
def raises_grpc_error(status: grpclib.const.Status):
    with pytest.raises(grpclib.GRPCError) as exc_info:
        yield
    assert exc_info.value.status == status, f"expected {status}, got {exc_info!r}"


async def test_user_auth_flow(supervisor: SupervisorStub):
    """Create a User, login and logout. Read back data to confirm."""

    user = User(slug="test", name="Test", email="test@symbolx.com")
    client = Client(parent=user, name="test", device_name="pytest", last_seen_at=utcnow_with_tz())

    # signup -> success
    signup_req = SignupUserRequest(
        id=str(user.id),
        slug=user.slug,
        name=user.name,
        email=user.email,
        client=client._to_data(),
        password="Password123!",
    )
    signup_rep = await supervisor.signup_user(signup_req)
    assert signup_rep.user.slug == str(user.slug)
    assert signup_rep.user.id == str(user.id)

    # login, invalid password -> fail
    login_req = LoginUserRequest(slug=user.slug, password="bad", client=client._to_data())
    with raises_grpc_error(grpclib.Status.INVALID_ARGUMENT):
        _ = await supervisor.login_user(login_req)

    # login, wrong password -> fail
    login_req = LoginUserRequest(
        slug=user.slug, password="321Password!!!", client=client._to_data()
    )
    with raises_grpc_error(grpclib.Status.INVALID_ARGUMENT):
        _ = await supervisor.login_user(login_req)

    # login, correct password -> success
    login_req = LoginUserRequest(slug=user.slug, password="Password123!", client=client._to_data())
    login_rep = await supervisor.login_user(login_req)
    assert login_rep.access_token

    # read user with sensitive data, authorized -> success
    read_user_req = GetNodesRequest(
        roots=[user.to_ref()._to_data()],
        options=ReadOptions(
            include_properties=[User.email], descendant_types=[NodeType.CLIENT]
        )._to_data(),
    )
    access_metadata = RpcMetadata(
        client_id=str(client.id),
        client_kind=wire.ClientKind.USER,
        client_access_token=login_rep.access_token,
    )
    read_user_rep = await supervisor.get_nodes(read_user_req, metadata=access_metadata.to_headers())
    assert len(read_user_rep.nodes) == 2
    assert read_user_rep.nodes[0].user.email == user.email
    assert read_user_rep.nodes[1].client.device_name == client.device_name

    # logout, invalid token -> fail
    with raises_grpc_error(grpclib.Status.UNAUTHENTICATED):
        bad_access_metadata = replace(access_metadata, client_access_token="bad")
        _ = await supervisor.logout_user(
            LogoutUserRequest(), metadata=bad_access_metadata.to_headers()
        )

    # logout, valid token -> success
    _ = await supervisor.logout_user(LogoutUserRequest(), metadata=access_metadata.to_headers())

    # read user, logged out, expired token -> fail
    with raises_grpc_error(grpclib.Status.UNAUTHENTICATED):
        _ = await supervisor.get_nodes(read_user_req, metadata=access_metadata.to_headers())


async def test_cross_user_protection(supervisor: SupervisorStub):
    """Users can only take certain actions on themselves."""

    user_a = User(slug="alice", name="Alice", email="alice@bench.app")
    user_b = User(slug="bob", name="Bob", email="bob@bench.app")
    user_c = User(slug="carol", name="Carol", email="carol@bench.app")
    handle_a = await make_user_handle(supervisor, user_a)
    handle_b = await make_user_handle(supervisor, user_b)
    handle_c = await make_user_handle(supervisor, user_c)
    all_handles = (handle_a, handle_b, handle_c)

    # cross-test user access/actions
    for actor_handle in all_handles:
        for target_handle in all_handles:
            actor = actor_handle.user
            target = target_handle.user
            is_target_self = actor == target

            # request our own and everyone else's data
            sensitive_properties = (User.email, User.password_salt, User.password_hash)
            read_user_req = GetNodesRequest(
                roots=[target.to_ref()._to_data()],
                options=ReadOptions(include_properties=sensitive_properties)._to_data(),
            )
            read_user_rep = await supervisor.get_nodes(read_user_req, metadata=actor_handle.headers)
            read_target = read_user_rep.nodes[0].user
            assert read_target.slug == target.slug
            if is_target_self:  # we should be able to read our own sensitive data
                assert read_target.email == target.email
            else:  # but not others'‚
                assert not read_target.email

            # create new client
            new_client = Client(
                parent=target,
                name=f"{actor.name}'s Toaster",
                device_name="toaster",
                last_seen_at=utcnow_with_tz(),
            )
            create_client_edit = EditData(
                type=wire.EditType.CREATE,
                node_type=wire.NodeType.CLIENT,
                node=wiring.wrap_some_node(new_client._to_data()),
                origin=actor_handle.origin,
            )
            create_client_req = CommitTransactionRequest(
                id=str(uuid4()), edits=[create_client_edit]
            )
            if is_target_self:  # can create clients for ourselves
                _ = await supervisor.commit_transaction(
                    create_client_req, metadata=actor_handle.headers
                )
            else:  # but not for others
                with raises_grpc_error(grpclib.Status.PERMISSION_DENIED):
                    _ = await supervisor.commit_transaction(
                        create_client_req, metadata=actor_handle.headers
                    )


@pytest.mark.parametrize("node_type", PUBLIC_NODE_TYPES, ids=lambda t: t.name)
async def test_public_node_read(
    node_type: NodeType, some_user: UserHandle, supervisor: SupervisorStub
):
    """Public nodes should be readable, but not directly editable in any way."""

    packed_node_type = wiring.pack_enum(NodeType, node_type)

    # search (and count)
    search_req = SearchNodesRequest(node_type=packed_node_type, count=True)
    await supervisor.search_nodes(search_req, metadata=some_user.metadata.to_headers())
    # can we assert anything here?

    # search with filter (and count)
    search_req = SearchNodesRequest(node_type=packed_node_type)
    await supervisor.search_nodes(search_req, metadata=some_user.metadata.to_headers())
    # here?

    # aggregate: exists
    aggregate_req = AggregateNodesRequest(
        node_type=packed_node_type, aggregation=A(AggregationOp.EXISTS)._to_data()
    )
    aggregate_rep = await supervisor.aggregate_nodes(
        aggregate_req, metadata=some_user.metadata.to_headers()
    )
    assert isinstance(aggregate_rep.aggregation.exists, bool)

    # aggregate: count
    aggregate_req = AggregateNodesRequest(
        node_type=packed_node_type, aggregation=A(AggregationOp.COUNT)._to_data()
    )
    aggregate_rep = await supervisor.aggregate_nodes(
        aggregate_req, metadata=some_user.metadata.to_headers()
    )
    assert isinstance(aggregate_rep.aggregation.count, int)


@pytest.mark.parametrize("node_type", (*ROOT_NODE_TYPES,), ids=lambda t: t.name)
async def test_root_node_create_denied(
    node_type: NodeType,
    some_user: UserHandle,
    supervisor: SupervisorStub,
    fabricator: "Fabricator",
):
    """Only the system can create root nodes."""

    node = fabricator.fabricate(node_type)
    node_data = wiring.pack_node(node)
    node_data.parent_ptr = None  # roots don't have parents

    # try create
    for edit_type in (EditType.CREATE, EditType.UPSERT):
        edit = EditData(
            type=edit_type,
            node_type=node_data.metatype,
            node=wiring.wrap_some_node(node_data),
            origin=some_user.origin,
        )
        commit_req = CommitTransactionRequest(id=str(uuid4()), edits=[edit])
        with raises_grpc_error(grpclib.Status.PERMISSION_DENIED):
            _ = await supervisor.commit_transaction(
                commit_req, metadata=some_user.metadata.to_headers()
            )
