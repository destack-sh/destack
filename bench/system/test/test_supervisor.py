from dataclasses import replace
from typing import TYPE_CHECKING, cast
from uuid import uuid4

import pytest
from grpclib import Status as GRPCStatus

from bench.conftest import raises_grpc_error
from bench.language import Client, ReadOptions, User
from bench.language.const import (
    PUBLIC_NODE_TYPES,
    ROOT_NODE_TYPES,
    AggregationOp,
    ClientType,
    NodeType,
)
from bench.language.expression import A
from bench.language.node import Node
from bench.language.property import Property
from bench.language.setup import NODE_CLASS_BY_TYPE
from bench.language.transaction import new_edit_id, pack_node_delta
from bench.language.user import UserStatus
from bench.proto import wire, wiring
from bench.proto.wire import (
    AggregateNodesRequest,
    AnyNodeData,
    BenchData,
    ClientDataIn,
    CommitTransactionRequest,
    EditData,
    GetNodesRequest,
    LoginUserRequest,
    LogoutUserRequest,
    RpcMetadata,
    SearchNodesRequest,
    SignupUserRequest,
    SupervisorStub,
)
from bench.system.test.conftest import UserHandle, make_new_user_handle
from bench.utils.dt import utcnow

if TYPE_CHECKING:
    from bench.language.test.fabricator import Fabricator


async def test_user_registration(supervisor: SupervisorStub):
    """Create a User, login and logout. Read back data to confirm."""

    user = User(slug="test", name="Test", email="test@symbolx.com", status=UserStatus.INVITED)
    assert user.slug is not None and user.email is not None
    client = Client(
        parent=user,
        type=ClientType.BENCH_WEB,
        name="test",
        device_name="pytest",
        seen_at=utcnow(),
    )

    # signup -> success
    signup_req = SignupUserRequest(
        id=str(user.id),
        slug=user.slug,
        name=user.name,
        email=user.email,
        client=cast(ClientDataIn, client._to_data()),
        password="Password123!",
    )
    signup_rep = await supervisor.signup_user(signup_req)
    assert signup_rep.user.slug == str(user.slug)
    assert signup_rep.user.id == str(user.id)

    # TODO :Security: user email confirmation etc.

    # login, invalid password -> fail
    login_req = LoginUserRequest(
        slug=user.slug, password="bad", client=cast(ClientDataIn, client._to_data())
    )
    with raises_grpc_error(GRPCStatus.UNAUTHENTICATED):
        _ = await supervisor.login_user(login_req)

    # login, wrong password -> fail
    login_req = LoginUserRequest(
        slug=user.slug, password="321Password!!!", client=cast(ClientDataIn, client._to_data())
    )
    with raises_grpc_error(GRPCStatus.UNAUTHENTICATED):
        _ = await supervisor.login_user(login_req)

    # login, correct password -> success
    login_req = LoginUserRequest(
        slug=user.slug, password="Password123!", client=cast(ClientDataIn, client._to_data())
    )
    login_rep = await supervisor.login_user(login_req)
    assert login_rep.access_token

    # read user with sensitive data, authorized -> success
    options = ReadOptions(
        include_properties=[cast(Property, User.email)],
        descendant_types=[NodeType.CLIENT, NodeType.HANDLE],
    )._to_data()
    read_user_req = GetNodesRequest(roots=[user.to_ref()._to_data()], options=options)
    access_metadata = RpcMetadata(
        client_id=str(client.id), client_access_token=login_rep.access_token
    )
    access_headers = access_metadata.to_headers()  # type: ignore
    read_user_rep = await supervisor.get_nodes(read_user_req, metadata=access_headers)
    assert len(read_user_rep.nodes) == 3
    assert read_user_rep.nodes[0].user.email == user.email
    assert read_user_rep.nodes[0].user.main_handle_ptr
    assert read_user_rep.nodes[0].user.main_handle_ptr.id == read_user_rep.nodes[2].handle.id
    assert read_user_rep.nodes[1].client.device_name == client.device_name
    assert read_user_rep.nodes[2].handle.slug == user.slug

    # logout, invalid token -> fail
    with raises_grpc_error(GRPCStatus.UNAUTHENTICATED):
        bad_access_metadata = replace(access_metadata, client_access_token="bad")
        bad_access_headers = bad_access_metadata.to_headers()  # type: ignore
        _ = await supervisor.logout_user(LogoutUserRequest(), metadata=bad_access_headers)

    # logout, valid token -> success
    _ = await supervisor.logout_user(LogoutUserRequest(), metadata=access_headers)

    # read user, logged out, expired token -> fail
    with raises_grpc_error(GRPCStatus.UNAUTHENTICATED):
        _ = await supervisor.get_nodes(read_user_req, metadata=access_headers)


async def test_cross_user_access(supervisor: SupervisorStub):
    """Users can only take certain actions on themselves."""

    user_a = User(slug="alice", name="Alice", email="alice@bench.app", status=UserStatus.REGISTERED)
    user_b = User(slug="bob", name="Bob", email="bob@bench.app", status=UserStatus.REGISTERED)
    user_c = User(slug="carol", name="Carol", email="carol@bench.app", status=UserStatus.REGISTERED)
    handle_a = await make_new_user_handle(supervisor, user_a)
    handle_b = await make_new_user_handle(supervisor, user_b)
    handle_c = await make_new_user_handle(supervisor, user_c)
    all_handles = (handle_a, handle_b, handle_c)

    # cross-test user access/actions
    for actor_handle in all_handles:
        for target_handle in all_handles:
            actor = actor_handle.user
            target = target_handle.user
            is_target_self = actor == target

            # request our own and everyone else's data
            sensitive_properties = cast(
                list[Property], [User.email, User.password_salt, User.password_hash]
            )
            read_user_req = GetNodesRequest(
                roots=[target.to_ref()._to_data()],
                options=ReadOptions(include_properties=sensitive_properties)._to_data(),
            )
            read_user_rep = await supervisor.get_nodes(read_user_req, metadata=actor_handle.headers)
            read_target = read_user_rep.nodes[0].user
            assert read_target.slug == target.slug
            if is_target_self:  # we should be able to read our own sensitive data
                assert read_target.email == target.email
            else:  # but not others
                assert not read_target.email

            # update the User's full name
            target_data = target._to_data()
            target_data.updated_at = utcnow()
            target_data.updated_by_ptr = actor_handle.subject
            target_data.name = f"{actor.name}'s Puppet"
            edit = EditData(
                id=new_edit_id(),
                type=wire.EditType.UPDATE,
                node_ptr=target.to_ref()._to_data(),
                properties=[User.name.id],  # type: ignore
                new_node_packed=pack_node_delta(target_data, only=(User.name,)),
                old_node_packed=pack_node_delta(target_data, only=(User.name,)),
                origin=actor_handle.origin,
                subject_ptr=actor_handle.subject,
                edited_at=utcnow(),
            )
            commit_req = CommitTransactionRequest(id=str(uuid4()), edits=[edit])
            if is_target_self:  # can update our own data
                _ = await supervisor.commit_transaction(commit_req, metadata=actor_handle.headers)
            else:  # but not for others
                with raises_grpc_error(GRPCStatus.PERMISSION_DENIED):
                    _ = await supervisor.commit_transaction(
                        commit_req, metadata=actor_handle.headers
                    )
            # update the User's client's device name
            target_data = target_handle.client._to_data()
            target_data.updated_at = utcnow()
            target_data.updated_by_ptr = target_handle.subject
            target_data.device_name = f"{actor.name}'s Puppet Device"
            edit = EditData(
                id=new_edit_id(),
                type=wire.EditType.UPDATE,
                node_ptr=target_handle.client.to_ref()._to_data(),
                properties=[Client.device_name.id],  # type: ignore
                new_node_packed=pack_node_delta(target_data, only=(Client.device_name,)),
                old_node_packed=pack_node_delta(target_data, only=(Client.device_name,)),
                origin=actor_handle.origin,
                subject_ptr=actor_handle.subject,
                edited_at=utcnow(),
            )
            commit_req = CommitTransactionRequest(id=str(uuid4()), edits=[edit])
            if is_target_self:  # can update our own data
                _ = await supervisor.commit_transaction(commit_req, metadata=actor_handle.headers)
            else:  # but not for others
                with raises_grpc_error(GRPCStatus.PERMISSION_DENIED):
                    _ = await supervisor.commit_transaction(
                        commit_req, metadata=actor_handle.headers
                    )


@pytest.mark.parametrize("node_type", PUBLIC_NODE_TYPES, ids=lambda t: t.name)
async def test_public_node_read(
    node_type: NodeType, some_user: UserHandle, supervisor: SupervisorStub
):
    """Public nodes should be readable, but not directly editable in any way."""

    packed_node_type = wiring.pack_enum(NodeType, node_type)

    # search (and count)
    search_req = SearchNodesRequest(node_type=packed_node_type, count=True)
    await supervisor.search_nodes(search_req, metadata=some_user.headers)
    # can we assert anything here?

    # search with filter (and count)
    search_req = SearchNodesRequest(node_type=packed_node_type)
    await supervisor.search_nodes(search_req, metadata=some_user.headers)
    # here?

    # aggregate: exists
    aggregate_req = AggregateNodesRequest(
        node_type=packed_node_type, aggregation=A(AggregationOp.EXISTS)._to_data()
    )
    aggregate_rep = await supervisor.aggregate_nodes(aggregate_req, metadata=some_user.headers)
    assert isinstance(aggregate_rep.aggregation.exists, bool)

    # aggregate: count
    aggregate_req = AggregateNodesRequest(
        node_type=packed_node_type, aggregation=A(AggregationOp.COUNT)._to_data()
    )
    aggregate_rep = await supervisor.aggregate_nodes(aggregate_req, metadata=some_user.headers)
    assert isinstance(aggregate_rep.aggregation.count, int)


@pytest.mark.parametrize("node_type", [*ROOT_NODE_TYPES], ids=lambda t: t.name)
async def test_root_node_create_denied(
    node_type: NodeType,
    some_user: UserHandle,
    supervisor: SupervisorStub,
    fabricator: "Fabricator",
):
    """Only the system can create root nodes."""

    node: Node[AnyNodeData] = fabricator.fabricate(NODE_CLASS_BY_TYPE[node_type])
    node_data: AnyNodeData = wiring.pack_node(node)
    cast(BenchData, node_data).parent_ptr = None  # roots don't have parents

    # try create
    for edit_type in (wire.EditType.CREATE, wire.EditType.UPSERT):
        edit = EditData(
            id=new_edit_id(),
            type=edit_type,
            node_ptr=node.to_ref()._to_data(),
            new_node_packed=pack_node_delta(node_data),
            origin=some_user.origin,
            subject_ptr=some_user.user.to_ref()._to_data(),
            edited_at=utcnow(),
        )
        commit_req = CommitTransactionRequest(id=str(uuid4()), edits=[edit])
        with raises_grpc_error(GRPCStatus.PERMISSION_DENIED, GRPCStatus.INVALID_ARGUMENT):
            _ = await supervisor.commit_transaction(commit_req, metadata=some_user.headers)
