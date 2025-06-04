from typing import Callable, override

import structlog
from fastuuid import UUID
from grpclib import GRPCError
from grpclib import Status as GRPCStatus
from opentelemetry import trace

from bench.language import (
    Bench,
    BenchStatus,
    Client,
    Database,
    DatabaseInfo,
    Handle,
    IsSubject,
    NodeArea,
    NodeType,
    Package,
    PackageType,
    Region,
    Session,
    Tenancy,
    User,
    UserStatus,
    bittuple,
)
from bench.pb2 import RpcMetadata
from bench.proto import (
    ChangeUserPasswordRequest,
    ChangeUserPasswordResponse,
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
from bench.sharding import CellProvider, DatabaseProvider
from bench.store import DatabaseStore, SplitStore
from bench.utils.func import generate_access_token, generate_salt
from bench.utils.oracle import Oracle

from .access import ACCESS_TOKEN_LENGTH, SALT_LENGTH, check_password, hash_password

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
        network: Network,
        oracle: Oracle,
        global_database: DatabaseInfo,
        cell_provider: CellProvider,
        database_provider: DatabaseProvider,
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
        self.cell_provider = cell_provider
        self.database_provider = database_provider

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
        database_store = DatabaseStore(database=self.global_database, area=NodeArea.GLOBAL_DATABASE)
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
        if not request.password:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "password required")

        region = Region(request.region)
        bench = Bench(
            status=BenchStatus.CREATING,
            slug=request.slug,
            name=request.slug,
            region=region,
        )

        # create User with Bench
        user = User(
            slug=request.slug,
            name=request.name or request.slug,
            email=request.email,
            status=UserStatus.CREATING,
            last_logged_in_at=self.oracle.utc(),
            bench=bench,
        )
        bench.owned_by = user
        user.password_salt = generate_salt(SALT_LENGTH)
        user.password_hash = hash_password(request.password, user.password_salt)
        session.create(user)
        session.create(bench)
        await session.stage()

        # create Client
        client = Client.from_proto(request.client)
        client.access_token = generate_access_token(ACCESS_TOKEN_LENGTH)
        user.add_child(client)
        await session.stage()
        assert user.slug, f"{user!r} has no slug"
        bench.handle = user.handle = Handle(slug=user.slug)
        bench.add_child(user.handle)
        await session.commit()

        # provision Bench
        cell = await self.cell_provider.acquire(region)
        main_database = await self.database_provider.acquire(region)
        database = Database(
            tenancy=Tenancy.SHARED,
            name="Main Database",
            region=region,
            cell_name=cell.name,
            external_name=main_database.external_name,
            custom_schema_name=f"bench-{bench.id}",
        )
        bench.database = database
        session.store = SplitStore(
            store_by_area={
                NodeArea.GLOBAL_DATABASE: DatabaseStore(
                    database=self.global_database, area=NodeArea.GLOBAL_DATABASE
                ),
                NodeArea.MAIN_DATABASE: DatabaseStore(
                    database=main_database, area=NodeArea.MAIN_DATABASE
                ),
            },
        )
        await session.stage()

        # create main Package
        main_package = Package(parent=bench, type=PackageType.HOME, name="Home", slug="home")
        bench.add_child(main_package)
        bench.main_package = main_package
        await session.stage()

        bench.status = BenchStatus.RUNNING

        # done
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
        client = Client.from_proto(request.client)
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
