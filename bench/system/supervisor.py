from typing import cast
from uuid import UUID, uuid4, uuid5

import betterproto
import structlog
from grpclib import GRPCError
from grpclib import Status as GRPCStatus

from bench.language import Bench, Client, NodeReference, User
from bench.language.access import Subject
from bench.language.const import USER_NODE_TYPES, ClientType, NodeType, OrganizationStatus
from bench.language.graph import generate_node_name
from bench.language.resource import Region
from bench.language.user import Handle, Organization, UserStatus
from bench.proto import wiring
from bench.proto.wire import (
    ChangeUserPasswordRequest,
    ChangeUserPasswordResponse,
    ClientData,
    CreateBenchRequest,
    CreateBenchResponse,
    GetHostRequest,
    GetHostResponse,
    GraphScope,
    LoginUserRequest,
    LoginUserResponse,
    LogoutUserRequest,
    LogoutUserResponse,
    SignupUserRequest,
    SignupUserResponse,
    SupervisorBase,
)
from bench.system.auth import check_password, generate_access_token, generate_salt, hash_password
from bench.system.client import GLOBAL_POSTGRES_ENGINE, global_session
from bench.system.graph import GraphIoServiceBase
from bench.system.resource import create_default_bench
from bench.utils.dt import utcnow_with_tz
from bench.utils.func import to_uuid

logger = structlog.get_logger(__name__)
GLOBAL_SCOPE = GraphScope()


class Supervisor(GraphIoServiceBase, SupervisorBase):
    def __init__(self):
        GraphIoServiceBase.__init__(self, bench_id=None, node_types=USER_NODE_TYPES)

    def __str__(self):
        return "shards=[*]"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    async def start(self) -> None:
        pass

    def close(self) -> None:
        pass

    async def wait_closed(self) -> None:
        pass

    @property
    def engines(self):
        return (GLOBAL_POSTGRES_ENGINE,)

    #
    # User management
    #

    async def _make_client(self, user: User, client_data: ClientData) -> Client:
        """Maps the given client info to a Client instance, trying to preserve a stable identity."""
        if client_data.id:
            client_id = UUID(client_data.id)
        elif client_data.place_id:
            client_id = uuid5(user.id, client_data.place_id)
        else:
            client_id = None
        name = client_data.name
        if not name:
            name = generate_node_name(NodeType.CLIENT, type=None, siblings=user.clients)
        client = Client(
            id=client_id or uuid4(),
            parent=user,
            type=cast(ClientType, client_data.type),
            name=name,
            device_name=client_data.device_name,
            device_type=client_data.device_type,
            operating_system=client_data.operating_system,
            browser_name=client_data.browser_name,
            browser_version=client_data.browser_version,
            last_seen_at=utcnow_with_tz(),
            _is_new=True,  # force create
        )
        return client

    async def signup_user(
        self, subject: Subject, request: "SignupUserRequest"
    ) -> "SignupUserResponse":
        if subject.is_authenticated:
            raise GRPCError(GRPCStatus.ALREADY_EXISTS, "already logged in")

        async with global_session() as session:
            user = User(
                id=to_uuid(request.id) or uuid4(),
                slug=request.slug,
                name=request.name or request.slug,
                email=request.email,
                status=UserStatus.REGISTERED,
                last_logged_in_at=utcnow_with_tz(),
                _is_new=True,  # force create
            )
            user.password_salt = generate_salt()
            user.password_hash = hash_password(request.password, user.password_salt)
            session.create(user)
            await session.flush()
            client = await self._make_client(user, request.client)
            client.access_token = generate_access_token()
            session.create(client)
            await session.flush()
            user.main_handle = user.handles.create(slug=user.slug)
            await session.commit()
            self.on_graph_edited((GLOBAL_SCOPE,), session.tx.edits)

        logger.info("supervisor.signup_user", user=user, client=client)
        return SignupUserResponse(
            user=user._to_data(),
            client=client._to_data(),
            access_token=client.access_token,
        )

    async def change_user_password(
        self, subject: Subject, request: "ChangeUserPasswordRequest"
    ) -> "ChangeUserPasswordResponse":
        if not subject.user:
            raise GRPCError(GRPCStatus.UNAUTHENTICATED, "not logged in")

        async with global_session() as session:
            user = subject.user
            if user.password_salt is None or user.password_hash is None:
                raise GRPCError(GRPCStatus.FAILED_PRECONDITION, "password not set")
            if not await check_password(
                request.old_password, user.password_salt, user.password_hash
            ):
                raise GRPCError(GRPCStatus.UNAUTHENTICATED, "incorrect password")

            # set new password
            session.track(user)  # user is from another session
            user.password_salt = generate_salt()
            user.password_hash = hash_password(request.new_password, user.password_salt)
            await session.commit()
            self.on_graph_edited((GLOBAL_SCOPE,), session.tx.edits)

        logger.info("supervisor.change_user_password", user=user)
        return ChangeUserPasswordResponse(user=user._to_data())

    async def login_user(
        self, subject: Subject, request: "LoginUserRequest"
    ) -> "LoginUserResponse":
        if subject.is_authenticated:
            raise GRPCError(GRPCStatus.ALREADY_EXISTS, "already logged in")

        async with global_session() as session:
            key_name, key_value = betterproto.which_one_of(request, "user")
            if key_value is None:
                raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "no user provided")
            user = (
                await User.include(User.password_salt, User.password_hash)
                .descendants(NodeType.CLIENT)
                .get(User.__properties__[key_name] == key_value)
            )
            if user.password_salt is None or user.password_hash is None:
                raise GRPCError(GRPCStatus.FAILED_PRECONDITION, "password not set")
            if not await check_password(request.password, user.password_salt, user.password_hash):
                raise GRPCError(GRPCStatus.UNAUTHENTICATED, "incorrect password")

            user.last_logged_in_at = utcnow_with_tz()
            client = await self._make_client(user, request.client)
            client.access_token = generate_access_token()
            session.upsert(client)
            await session.commit()
            self.on_graph_edited((GLOBAL_SCOPE,), session.tx.edits)

        logger.info("supervisor.login_user", user=user, client=client)
        return LoginUserResponse(
            user=user._to_data(),
            client=client._to_data(),
            access_token=client.access_token,
        )

    async def logout_user(
        self, subject: Subject, request: "LogoutUserRequest"
    ) -> "LogoutUserResponse":
        if subject.client is None:
            raise GRPCError(GRPCStatus.UNAUTHENTICATED, "not logged in")

        async with global_session() as session:
            # log out the current or the specified clients
            if request.clients:
                client_ids = {to_uuid(c.id) for c in request.clients}
                clients = await Client.where(parent=subject.user, id__in=client_ids).tolist()
                if len(clients) != len(request.clients):
                    missing_ids = client_ids - {c.id for c in clients}
                    raise GRPCError(GRPCStatus.NOT_FOUND, f"clients not found: {missing_ids}")
            elif request.logout_all:
                clients = await Client.where(parent=subject.user).tolist()
            else:
                clients = (subject.client,)
                session.track(subject.client)
            for client in clients:
                client.logged_in_at = None
                client.access_token = None
                client.last_seen_at = utcnow_with_tz()
            await session.commit()
            self.on_graph_edited((GLOBAL_SCOPE,), session.tx.edits)

        logger.info("supervisor.logout_user", user=subject.user, clients=clients)
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
        if not request.slug:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "slug not specified")
        if not request.region:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "region not specified")
        region = wiring.unpack_enum(Region, request.region)
        if region == Region.GLOBAL:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "cannot create bench in global region")

        owner_ptr: NodeReference = wiring.unpack_struct(request.owner)
        async with global_session() as session:
            # check (and reload owner to get Handles)
            if owner_ptr.type == NodeType.USER:
                if owner_ptr.id != user.id:
                    raise GRPCError(
                        GRPCStatus.PERMISSION_DENIED, "cannot create bench for other user"
                    )
                owner = await User.descendants(Handle).get(id=owner_ptr.id)
            elif owner_ptr.type == NodeType.ORGANIZATION:
                owner = await Organization.descendants(Handle).get(id=owner_ptr.id)
                if owner.created_by_id != user.id:
                    raise GRPCError(
                        GRPCStatus.PERMISSION_DENIED, "cannot create bench for other organization"
                    )
            else:
                raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "invalid owner type")
            if owner.status < UserStatus.REGISTERED:
                raise GRPCError(GRPCStatus.FAILED_PRECONDITION, "owner not registered")

            # create bench (assumes it's the primary bench)
            if owner.status == UserStatus.ACTIVATED or owner.slug != request.slug:
                raise GRPCError(GRPCStatus.ALREADY_EXISTS, "cannot create secondary Benches (yet)")
            assert owner.main_handle is not None, f"{owner!r} has no main handle"
            bench = await create_default_bench(
                main_handle=owner.main_handle, owner=owner, region=region, session=session
            )

            # 'activate' owner
            if isinstance(owner, User) and owner.status != UserStatus.ACTIVATED:
                owner.main_bench = bench
                owner.status = UserStatus.ACTIVATED
            elif isinstance(owner, Organization) and owner.status != OrganizationStatus.ACTIVATED:
                owner.main_bench = bench
                owner.status = OrganizationStatus.ACTIVATED
            else:
                raise RuntimeError(f"unexpected owner/owner status: {owner!r}")

            await session.commit()
            self.on_graph_edited((GLOBAL_SCOPE,), session.tx.edits)

        logger.info("supervisor.create_bench", bench=bench)
        return CreateBenchResponse(bench=bench._to_data())

    async def get_host(self, subject: "Subject", request: "GetHostRequest") -> "GetHostResponse":
        key, value = betterproto.which_one_of(request, "bench")
        async with global_session():
            if key == "id":
                await Bench.get(id=to_uuid(value))
            elif key == "slug":
                await Bench.get(slug=value)
            else:
                raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "no bench specified")
        raise GRPCError(GRPCStatus.UNIMPLEMENTED, "not implemented :SingleHostService")
