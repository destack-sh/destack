from typing import Callable, override

import structlog
from fastuuid import UUID
from grpclib import GRPCError
from grpclib import Status as GRPCStatus
from opentelemetry import trace

from bench.language import (
    Client,
    ClientType,
    Database,
    Handle,
    IsSubject,
    NodeArea,
    NodeType,
    Region,
    Session,
    User,
    UserStatus,
    bittuple,
)
from bench.pb2 import RpcMetadata
from bench.proto import (
    ChangeUserPasswordRequest,
    ChangeUserPasswordResponse,
    ClientDataIn,
    LoginUserRequest,
    LoginUserResponse,
    LogoutUserRequest,
    LogoutUserResponse,
    Network,
    ResolveHostsRequest,
    ResolveHostsResponse,
    ServiceBase,
    ServiceKind,
    SignupUserRequest,
    SignupUserResponse,
    SupervisorBase,
)
from bench.system.host import HostMap
from bench.system.store import DatabaseMap
from bench.system.store.database import DatabaseStore
from bench.utils.func import generate_access_token, generate_salt
from bench.utils.oracle import Oracle

from .access import ACCESS_TOKEN_LENGTH, SALT_LENGTH, check_password, hash_password
from .bench import CreateBenchOptions, create_default_bench

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

SUPERVISOR_NODE_TYPES = bittuple(
    NodeType.USER,
    NodeType.ORGANIZATION,
    NodeType.CLIENT,
    NodeType.HANDLE,
    NodeType.BENCH,
)


class SupervisorService(ServiceBase, SupervisorBase):
    kind = ServiceKind.PUBLIC  # :ServiceKind
    name = "supervisor"

    def __init__(
        self,
        id: str,
        global_database: Database,
        database_map: DatabaseMap,
        network: Network,
        oracle: Oracle,
        host_map: HostMap,
        on_error: Callable[[BaseException], None] | None = None,
    ):
        super().__init__(
            id=id,
            network=network,
            oracle=oracle,
            logger=logger,
            tracer=tracer,
            on_error=on_error,
        )
        self.global_database = global_database
        self.database_map = database_map
        self.host_map = host_map

    def __str__(self):
        return ""

    async def start(self) -> None:
        await super().start()

    def stop(self) -> None:
        super().stop()

    async def wait_stopped(self) -> None:
        await super().wait_stopped()

    #
    # User management
    #

    @override
    async def make_session(self, metadata: RpcMetadata) -> "Session":
        database_store = DatabaseStore({NodeArea.GLOBAL_POSTGRES: self.global_database})
        return Session(store=database_store)

    @override
    async def resolve_client(
        self, request, metadata: RpcMetadata
    ) -> tuple[IsSubject | None, Client | None]:
        if not metadata.client_access_token:
            return None, None
        client = await Client.get(
            where=Client.property("access_token").eq(metadata.client_access_token),
        ).execute_one_or_none()
        if client is None:
            return None, None
        return client.parent, client

    async def _make_client(self, user: User, client_data: ClientDataIn) -> Client:
        """Maps the given client info to a Client instance, trying to preserve a stable identity."""
        name = client_data.name
        client = Client(
            parent=user,
            name=name,
            type=ClientType(client_data.type),
            seen_at=self.oracle.utc(),
        )
        self._patch_client(client, client_data)
        return client

    def _patch_client(self, client: Client, client_data: ClientDataIn) -> Client:
        # copy over Client properties
        client.type = ClientType(client_data.type)
        client.device_type = client_data.device_type or None
        client.device_name = client_data.device_name or None
        client.operating_system = client_data.operating_system or None
        client.browser_name = client_data.browser_name or None
        client.browser_version = client_data.browser_version or None
        return client

    @override
    async def signup_user(
        self,
        request: "SignupUserRequest",
        session: Session,
        subject: IsSubject | None,
        client: Client | None,
        metadata: RpcMetadata,
    ) -> "SignupUserResponse":
        if subject is not None:
            raise GRPCError(GRPCStatus.ALREADY_EXISTS, "already logged in")

        # get regional Database
        region = Region(request.region)
        regional_database = self.database_map.get(region=region)
        session.store = DatabaseStore(
            {
                NodeArea.GLOBAL_POSTGRES: self.global_database,
                NodeArea.REGIONAL_POSTGRES: regional_database,
            }
        )

        # create User
        user = User(
            slug=request.slug,
            name=request.name or request.slug,
            email=request.email,
            status=UserStatus.REGISTERED,
            region=region,
            last_logged_in_at=self.oracle.utc(),
        )
        if not request.password:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "password required")
        user.password_salt = generate_salt(SALT_LENGTH)
        user.password_hash = hash_password(request.password, user.password_salt)
        session.create(user)
        await session.stage()

        # create Client
        client = await self._make_client(user, request.client)
        client.access_token = generate_access_token(ACCESS_TOKEN_LENGTH)
        session.create(client)
        await session.stage()
        assert user.slug, f"{user!r} has no slug"
        user.handle = Handle(slug=user.slug)
        user.add_child(user.handle)
        await session.commit()

        # immediately create User's main Bench
        if request.activate:
            bench = await create_default_bench(
                handle=user.handle,
                owned_by=user,
                region=user.region,
                session=session,
                options=CreateBenchOptions(),
            )
            user.bench = bench
            user.status = UserStatus.ACTIVE
            await session.commit()

        logger.info("supervisor.signup_user", user=user, client=client, span="current")
        return SignupUserResponse(
            user=user.to_proto(), client=client.to_proto(), access_token=client.access_token
        )

    @override
    async def change_user_password(
        self,
        request: "ChangeUserPasswordRequest",
        session: Session,
        subject: IsSubject | None,
        client: Client | None,
        metadata: RpcMetadata,
    ) -> "ChangeUserPasswordResponse":
        if not isinstance(subject, User):
            raise GRPCError(GRPCStatus.UNAUTHENTICATED, "not logged in")
        if subject.password_salt is None or subject.password_hash is None:
            raise GRPCError(GRPCStatus.FAILED_PRECONDITION, "password not set")
        if not await check_password(
            request.old_password, subject.password_salt, subject.password_hash, self.oracle
        ):
            raise GRPCError(GRPCStatus.UNAUTHENTICATED, "incorrect password")

        # set new password
        subject.password_salt = generate_salt(SALT_LENGTH)
        subject.password_hash = hash_password(request.new_password, subject.password_salt)
        await session.commit()

        logger.info("supervisor.change_user_password", user=subject, span="current")
        return ChangeUserPasswordResponse(user=subject.to_proto())

    @override
    async def login_user(
        self,
        request: "LoginUserRequest",
        session: Session,
        subject: IsSubject | None,
        client: Client | None,
        metadata: RpcMetadata,
    ) -> "LoginUserResponse":
        if subject is not None:
            raise GRPCError(GRPCStatus.ALREADY_EXISTS, "already logged in")
        key_name = request.WhichOneof("user")
        key_value = getattr(request, key_name)
        if key_value is None:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "no user provided")
        user = await User.get(
            where=User.property(key_name).eq(key_value),
            Clients=Client.search(),
        ).execute_one()
        if user.password_salt is None or user.password_hash is None:
            raise GRPCError(GRPCStatus.FAILED_PRECONDITION, "password not set")
        if not await check_password(
            request.password, user.password_salt, user.password_hash, self.oracle
        ):
            raise GRPCError(GRPCStatus.UNAUTHENTICATED, "incorrect password")

        user.last_logged_in_at = self.oracle.utc()
        client = await self._make_client(user, request.client)
        client.access_token = generate_access_token(ACCESS_TOKEN_LENGTH)
        session.upsert(client)
        await session.commit()

        logger.info("supervisor.login_user", user=user, client=client, span="current")
        return LoginUserResponse(
            user=user.to_proto(),
            client=client.to_proto(),
            access_token=client.access_token,
        )

    @override
    async def logout_user(
        self,
        request: "LogoutUserRequest",
        session: Session,
        subject: IsSubject | None,
        client: Client | None,
        metadata: RpcMetadata,
    ) -> "LogoutUserResponse":
        if client is None:
            raise GRPCError(GRPCStatus.UNAUTHENTICATED, "not logged in")
        if not isinstance(subject, User):
            raise GRPCError(GRPCStatus.FAILED_PRECONDITION, "not a user")

        # log out the current or the specified clients
        if request.clients:
            client_ids = [UUID(c.id) for c in request.clients]
            clients = await Client.search(
                where=Client.property("parent").eq(subject) & Client.property("id").in_(client_ids),
            ).execute_list()
        elif request.logout_all:
            clients = await Client.search(
                where=Client.property("parent").eq(subject)
            ).execute_list()
        else:
            clients = (client,)
        for client in clients:
            client.logged_in_at = None
            client.access_token = None
            client.seen_at = self.oracle.utc()
        await session.commit()

        logger.info("supervisor.logout_user", user=subject, clients=clients, span="current")
        return LogoutUserResponse()

    @override
    async def resolve_hosts(
        self,
        request: "ResolveHostsRequest",
        session: Session,
        subject: IsSubject | None,
        client: Client | None,
        metadata: RpcMetadata,
    ) -> "ResolveHostsResponse":
        raise NotImplementedError
