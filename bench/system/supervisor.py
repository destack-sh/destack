from typing import AsyncIterator
from uuid import UUID

import betterproto
import structlog
from grpclib import GRPCError
from grpclib import Status as GRPCStatus

from bench.language import Client, Expression, Handle, NodeReference, User
from bench.language.access import (
    ReadOptions,
    adapt_read_options,
    evaluate_and_adapt_read,
    generate_access_matrix,
    Subject,
    evaluate_edit,
)
from bench.language.const import NodeType, ABOVE_SOURCE_NODE_TYPES, AggregationOp
from bench.language.expression import Aggregation
from bench.language.node import NODE_CLASS_BY_TYPE
from bench.language.tree import NodeDataTree
from bench.proto import wiring, wire
from bench.proto.services import BenchServiceBase
from bench.proto.wire import (
    AggregateNodesRequest,
    AggregateNodesResponse,
    CommitEditsRequest,
    CommitEditsResponse,
    CreateBenchRequest,
    CreateBenchResponse,
    SupervisorBase,
    SupervisorStub,
    LoginUserRequest,
    LoginUserResponse,
    LogoutUserRequest,
    LogoutUserResponse,
    ReadNodesRequest,
    ReadNodesResponse,
    SearchNodesRequest,
    SearchNodesResponse,
    SignupUserRequest,
    SignupUserResponse,
    WatchEditsRequest,
    WatchEditsResponse,
    ChangeUserPasswordRequest,
    ChangeUserPasswordResponse,
    AnyNodeData,
)
from bench.system.auth import check_password, generate_access_token, generate_salt, hash_password
from bench.system.utils import detached_session
from bench.sql.client import async_pg_cursor
from bench.sql.engine import (
    pg_count,
    pg_read_node_data_tree,
    pg_search_nodes_data_tree,
    compile_pg_conditional,
    pg_exists,
    pg_write_regular_edits,
)
from bench.utils.dt import utcnow_with_tz
from bench.utils.func import group_by, to_uuid

logger = structlog.get_logger("global_supervisor")

SERVER_IDLE_SLEEP_TIME = 30 * 60  # 30 minutes
SERVER_GENTLE_RESTART_TIMEOUT = 5  # 5 seconds until force restart


class Supervisor(BenchServiceBase[SupervisorStub], SupervisorBase):
    def __init__(self):
        super().__init__(loopback_stub_to=SupervisorStub)
        self._epoch: int = 0

    def __str__(self):
        return "<global>"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    async def start_quick(self) -> None:
        pass

    def close(self) -> None:
        pass

    async def wait_closed(self) -> None:
        pass

    #
    # User management
    #

    async def signup_user(
        self, subject: Subject, request: "SignupUserRequest"
    ) -> "SignupUserResponse":
        if subject.is_authenticated:
            raise GRPCError(GRPCStatus.ALREADY_EXISTS, "already logged in")

        async with detached_session() as session:
            user = User(
                id=to_uuid(request.id),
                slug=request.slug,
                name=request.name,
                email=request.email,
                is_activated=True,
                _is_new=True,  # force create (despite already having an id)
            )
            user.password_salt = generate_salt()
            user.password_hash = hash_password(request.password, user.password_salt)
            user.handle = Handle(slug=user.slug)
            client: Client = wiring.unpack_node(request.client, parent=user, session=session)
            client.access_token = generate_access_token()
            session.create_many(user.handle, user, client)
            await session.commit()
        return SignupUserResponse(
            user=user._to_data(), access_token=client.access_token, epoch=self._epoch
        )

    async def change_user_password(
        self, subject: Subject, request: "ChangeUserPasswordRequest"
    ) -> "ChangeUserPasswordResponse":
        if not subject.is_authenticated:
            raise GRPCError(GRPCStatus.UNAUTHENTICATED, "not logged in")
        async with detached_session() as session:
            user = subject.user
            if not await check_password(
                request.old_password, user.password_salt, user.password_hash
            ):
                raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "incorrect password")

            # set new password
            user.password_salt = generate_salt()
            user.password_hash = hash_password(request.password, user.password_salt)
            session.track(user)
            await session.commit()
        return ChangeUserPasswordResponse(user=user._to_data(), epoch=self._epoch)

    async def login_user(
        self, subject: Subject, request: "LoginUserRequest"
    ) -> "LoginUserResponse":
        if subject.is_authenticated:
            raise GRPCError(GRPCStatus.ALREADY_EXISTS, "already logged in")

        async with detached_session() as session:
            key_name, key_value = betterproto.which_one_of(request, "user")
            if key_value is None:
                raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "no user provided")
            user = await User.include(User.password_salt, User.password_hash).get(
                User.__properties__[key_name] == key_value
            )
            if not await check_password(request.password, user.password_salt, user.password_hash):
                raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "incorrect password")

            client: Client = wiring.unpack_node(request.client, parent=user, session=session)
            client.logged_in_at = client.last_seen_at = utcnow_with_tz()
            client.access_token = generate_access_token()
            session.upsert(client)
            await session.commit()
        return LoginUserResponse(
            user=user._to_data(),
            client=client._to_data(),
            access_token=client.access_token,
            epoch=self._epoch,
        )

    async def logout_user(
        self, subject: Subject, request: "LogoutUserRequest"
    ) -> "LogoutUserResponse":
        if subject.client is None:
            raise GRPCError(GRPCStatus.UNAUTHENTICATED, "not logged in")
        async with detached_session() as session:
            # log out the current or the specified clients
            if request.client_ids:
                clients = await Client.filter(
                    parent=subject.user, id__in=request.client_ids
                ).tolist()
            elif request.logout_all:
                clients = await Client.filter(parent=subject.user).tolist()
            else:
                clients = (subject.client,)
                session.track(subject.client)
            for client in clients:
                client.logged_in_at = None
                client.access_token = None
                client.last_seen_at = utcnow_with_tz()
            await session.commit()
        return LogoutUserResponse()

    #
    # Bench management
    #

    async def create_bench(
        self, subject: Subject, request: "CreateBenchRequest"
    ) -> "CreateBenchResponse":
        user: User | None = subject.user
        if not user:
            raise GRPCError(GRPCStatus.UNAUTHENTICATED, "not logged in")
        async with detached_session() as session:
            session.track(user)
            if user.bench:  # can't create secondary benches yet
                raise GRPCError(GRPCStatus.ALREADY_EXISTS, "bench already exists")
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)

    #
    # General Bench IO for > package & global nodes only :BenchIO
    #

    async def read_nodes(
        self, subject: Subject, request: "ReadNodesRequest"
    ) -> "ReadNodesResponse":
        roots: tuple[NodeReference, ...] = tuple(wiring.unpack_struct(r) for r in request.roots)
        if not roots:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "no roots provided")
        if any(root.type not in ABOVE_SOURCE_NODE_TYPES for root in roots):
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "global IO can't read packages")
        options: ReadOptions = (
            wiring.unpack_struct_interp_maybe(request.options) or ReadOptions.default()
        )

        roots_by_type: dict[NodeType, list[NodeReference]] = group_by(roots, lambda r: r.type)
        tree = NodeDataTree()
        async with async_pg_cursor() as cur:
            for root_node_type, root_node_references in roots_by_type.items():
                adapted_options = adapt_read_options(subject, root_node_type, options)
                node_type = wiring.unpack_enum(NodeType, root_node_type)
                _ = await pg_read_node_data_tree(
                    cur=cur,
                    root_type=node_type,
                    root_ids=tuple(r.id for r in root_node_references),
                    options=adapted_options,
                    _tree=tree,  # accumulate into tree
                )
        if any(root.id not in tree for root in request.roots):
            missing_roots = tuple(root for root in roots if str(root.id) not in tree)
            raise GRPCError(GRPCStatus.NOT_FOUND, f"roots not found: {missing_roots}")

        access = generate_access_matrix(subject, tree)
        access, adapted_nodes = evaluate_and_adapt_read(
            access, tree, required_nodes=request.roots, adapt_nodes_in_place=True
        )
        await self._log_and_check_access(access)

        return ReadNodesResponse(
            nodes=[wiring.wrap_some_node(n) for n in adapted_nodes],
            access=access._to_data(),
            epoch=self._epoch,
        )

    async def search_nodes(
        self, subject: Subject, request: "SearchNodesRequest"
    ) -> "SearchNodesResponse":
        if request.bases:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "global IO has no bases")
        node_type: NodeType = wiring.unpack_enum(NodeType, request.node_type)
        if node_type not in ABOVE_SOURCE_NODE_TYPES:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "global IO can't read packages")
        node_cls = NODE_CLASS_BY_TYPE[node_type]
        filter: Expression | None = wiring.unpack_struct_interp_maybe(request.filter)
        sort: list[Expression] = [wiring.unpack_struct_interp(s) for s in request.sort] or None
        options: ReadOptions = (
            wiring.unpack_struct_interp_maybe(request.options) or ReadOptions.default()
        )
        adapted_options = adapt_read_options(subject, node_type, options)

        async with async_pg_cursor() as cur:
            combined_filter = adapted_options.filter(node_type, filter)
            roots, tree = await pg_search_nodes_data_tree(
                cur=cur,
                node_type=node_type,
                filter=combined_filter,
                sort=sort,
                first=request.limit,
                after=request.after,
                options=adapted_options,
            )
            if request.count:
                count = await pg_count(
                    cur=cur,
                    table=node_cls.__table__,
                    where=compile_pg_conditional(node_cls, combined_filter),
                )
            else:
                count = None
        access = generate_access_matrix(subject, tree)
        access, adapted_nodes = evaluate_and_adapt_read(
            access, tree, adapt_nodes_in_place=True, required_nodes=request.bases
        )
        await self._log_and_check_access(access)

        return SearchNodesResponse(
            roots=[NodeReference.from_node_data(r) for r in roots.nodes],
            nodes=[wiring.wrap_some_node(n) for n in adapted_nodes],
            cursors=list(roots.cursors),
            start_cursor=roots.start_cursor,
            total=count,
            access=access._to_data(),
            epoch=self._epoch,
        )

    async def aggregate_nodes(
        self, subject: Subject, request: "AggregateNodesRequest"
    ) -> "AggregateNodesResponse":
        if request.bases:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "global IO has no bases")
        node_type: NodeType = wiring.unpack_enum(NodeType, request.node_type)
        if node_type not in ABOVE_SOURCE_NODE_TYPES:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "global IO can't read packages")
        node_cls = NODE_CLASS_BY_TYPE[node_type]
        filter: Expression | None = wiring.unpack_struct_interp_maybe(request.filter)
        aggregation: Expression = wiring.unpack_struct_interp(request.aggregation)
        adapted_options = adapt_read_options(subject, node_type, ReadOptions())

        async with async_pg_cursor() as cur:
            combined_filter = adapted_options.filter(node_type, filter)
            if aggregation.op == AggregationOp.EXISTS:
                exists = await pg_exists(
                    cur=cur,
                    table=node_cls.__table__,
                    where=compile_pg_conditional(node_cls, combined_filter),
                )
                result = Aggregation(op=aggregation.op, exists=exists)
            elif aggregation.op == AggregationOp.COUNT:
                count = await pg_count(
                    cur=cur,
                    table=node_cls.__table__,
                    where=compile_pg_conditional(node_cls, combined_filter),
                )
                result = Aggregation(op=aggregation.op, count=count)
            else:
                raise GRPCError(GRPCStatus.UNIMPLEMENTED, f"can't {aggregation.op.name} yet")

        return AggregateNodesResponse(aggregation=result._to_data(), epoch=self._epoch)

    async def commit_edits(
        self, subject: Subject, request: "CommitEditsRequest"
    ) -> "CommitEditsResponse":
        # figure out the node (scopes) we need to evaluate the edit
        edited_scopes_ptr: dict[UUID, NodeReference] = {}
        for edit in request.edits:
            node = wiring.unwrap_some_node(edit.node)
            if edit.type == wire.EditType.CREATE or edit.type == wire.EditType.UPSERT:
                # scope is parent since we don't know this node yet
                if node.parent_ptr is not None and node.parent_ptr.id not in edited_scopes_ptr:
                    ptr: NodeReference = wiring.unpack_struct(node.parent_ptr)
                    edited_scopes_ptr[ptr.id] = ptr
            else:
                # scope is the edited node itself
                if node.id not in edited_scopes_ptr:
                    ptr: NodeReference = wiring.unpack_struct(NodeReference.from_node_data(node))
                    edited_scopes_ptr[ptr.id] = ptr

        roots_by_type: dict[NodeType, list[NodeReference]] = group_by(
            edited_scopes_ptr.values(), lambda r: r.type
        )
        async with async_pg_cursor() as cur:
            # read the required nodes into a single tree
            tree = NodeDataTree()
            for root_node_type, root_node_references in roots_by_type.items():
                node_type = wiring.unpack_enum(NodeType, root_node_type)
                # TODO @Performance: select only require properties for edit eval (id/policies/...?)
                adapted_options = adapt_read_options(subject, node_type, ReadOptions.default())
                _ = await pg_read_node_data_tree(
                    cur=cur,
                    root_type=node_type,
                    root_ids=tuple(r.id for r in root_node_references),
                    options=adapted_options,
                    _tree=tree,  # accumulate into tree
                )

            # evaluate the edits
            matrix = generate_access_matrix(subject, tree)
            access = evaluate_edit(matrix, tree, request.edits, trace=True)
            await self._log_and_check_access(access)

            # apply the edits
            changed_nodes: list[AnyNodeData] = await pg_write_regular_edits(
                cur=cur, edits=request.edits, return_nodes=True
            )
            await cur.connection.commit()

        return CommitEditsResponse(
            changed_nodes=[wiring.wrap_some_node(n) for n in changed_nodes],
            epoch=self._epoch,
        )

    async def watch_edits(
        self, subject: Subject, request: "WatchEditsRequest"
    ) -> AsyncIterator["WatchEditsResponse"]:
        # nocheckin: track and buffer edits for recent epochs
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)
