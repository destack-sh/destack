from typing import Callable, Mapping, override

import structlog
from fastuuid import UUID, uuid4, uuid5
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
    NodeReference,
    NodeType,
    Organization,
    OrganizationStatus,
    Region,
    User,
    UserStatus,
    bittuple,
)
from bench.pb2.common_pb2 import RpcMetadata
from bench.proto import (
    ChangeUserPasswordRequest,
    ChangeUserPasswordResponse,
    ClientDataIn,
    CreateBenchRequest,
    CreateBenchResponse,
    LoginUserRequest,
    LoginUserResponse,
    LogoutUserRequest,
    LogoutUserResponse,
    Network,
    ResolveHostsRequest,
    ResolveHostsResponse,
    ServiceKind,
    SignupUserRequest,
    SignupUserResponse,
    SupervisorBase,
    wiring,
)
from bench.system.core import (
    ACCESS_TOKEN_LENGTH,
    SALT_LENGTH,
    DatabaseMap,
    HostMap,
    check_password,
    hash_password,
)
from bench.utils.func import generate_access_token, generate_salt
from bench.utils.oracle import Oracle

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


class SupervisorService(SupervisorBase):
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
        create_bench_options: CreateBenchOptions,
        on_error: Callable[[Exception], None] | None = None,
    ):
        self.id = id
        self.network = network
        self.oracle = oracle
        self._global_database = global_database
        self._global_pg_engine = pg_engine_from_database(
            "pg-global", global_database, NodeArea.GLOBAL_POSTGRES
        )
        self._database_map = database_map
        self._host_map = host_map
        self.create_bench_options = create_bench_options
        self._on_error = on_error

    def __str__(self):
        return ""

    async def start(self) -> None:
        raise NotImplementedError

    def stop(self) -> None:
        raise NotImplementedError

    async def wait_stopped(self) -> None:
        raise NotImplementedError

    #
    # User management
    #

    async def resolve_client(
        self, request, metadata: RpcMetadata
    ) -> tuple[IsSubject | None, Client | None]:
        raise NotImplementedError

    def _get_client_id(self, user: User, client_data: ClientDataIn) -> UUID | None:
        if client_data.id:
            return UUID(client_data.id)
        elif client_data.place_id:
            return uuid5(user.id, client_data.place_id)
        else:
            return None

    async def _make_client(self, user: User, client_data: ClientDataIn) -> Client:
        """Maps the given client info to a Client instance, trying to preserve a stable identity."""
        client_id = self._get_client_id(user, client_data)
        name = client_data.name
        client = Client(
            id=client_id or uuid4(),
            parent=user,
            name=name,
            type=ClientType(client_data.type),
            seen_at=self.oracle.utc(),
            _is_new=True,  # force create
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
        self, request: "SignupUserRequest", headers: Mapping
    ) -> "SignupUserResponse":
        metadata = wiring.unpack_rpc_headers(headers)
        subject, client = await self.resolve_client(request, metadata)
        if subject is not None:
            raise GRPCError(GRPCStatus.ALREADY_EXISTS, "already logged in")

        # get regional Database
        region = Region(request.region)
        regional_database = self._database_map.get(region=region)
        regional_pg_engine = pg_engine_from_database(
            name=f"pg-regional-{regional_database.region.name.lower()}",
            database=regional_database,
            area=NodeArea.REGIONAL_POSTGRES,
        )

        async with self.new_request_session(
            supergraph=subject._supergraph,
            readonly=False,
            engines=(self._global_pg_engine, regional_pg_engine),
        ) as session:
            # create User
            user = User(
                id=to_uuid(request.id) or uuid4(),
                slug=request.slug,
                name=request.name or request.slug,
                email=request.email,
                status=UserStatus.REGISTERED,
                region=region,
                last_logged_in_at=self.oracle.utc(),
                _is_new=True,  # force create
            )
            if not request.password:
                raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "password required")
            user.password_salt = generate_salt(SALT_LENGTH)
            user.password_hash = hash_password(request.password, user.password_salt)
            session.create(user)
            session.stage()

            # create Client
            client = await self._make_client(user, request.client)
            client.access_token = generate_access_token(ACCESS_TOKEN_LENGTH)
            session.create(client)
            session.stage()
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
                    options=self.create_bench_options,
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
        self, request: "ChangeUserPasswordRequest", headers: Mapping
    ) -> "ChangeUserPasswordResponse":
        metadata = wiring.unpack_rpc_headers(headers)
        async with self.new_request_session() as session:
            subject, client = await self.resolve_client(request, metadata)
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
        self, request: "LoginUserRequest", headers: Mapping
    ) -> "LoginUserResponse":
        metadata = wiring.unpack_rpc_headers(headers)
        async with self.new_request_session() as session:
            subject, client = await self.resolve_client(request, metadata)
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
            client_id = self._get_client_id(user, request.client)
            if client_id and client_id in user._supergraph:  # upsert
                client = user._supergraph[client_id]
                if not isinstance(client, Client):
                    raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "invalid client")
                self._patch_client(client, request.client)
            else:
                client = await self._make_client(user, request.client)
            client.access_token = generate_access_token(ACCESS_TOKEN_LENGTH)
            session._upsert(client)
            await session.commit()

        logger.info("supervisor.login_user", user=user, client=client, span="current")
        return LoginUserResponse(
            user=user.to_proto(),
            client=client.to_proto(),
            access_token=client.access_token,
        )

    @override
    async def logout_user(
        self, request: "LogoutUserRequest", headers: Mapping
    ) -> "LogoutUserResponse":
        async with self.new_request_session(
            supergraph=subject._supergraph, readonly=False
        ) as session:
            metadata = wiring.unpack_rpc_headers(headers)
            subject, client = await self.resolve_client(request, metadata)
            if client is None:
                raise GRPCError(GRPCStatus.UNAUTHENTICATED, "not logged in")
            if not isinstance(subject, User):
                raise GRPCError(GRPCStatus.FAILED_PRECONDITION, "not a user")

            # log out the current or the specified clients
            if request.clients:
                client_ids = [UUID(c.id) for c in request.clients]
                clients = await Client.search(
                    where=Client.property("parent").eq(subject)
                    & Client.property("id").in_(client_ids),
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

    #
    # Bench management
    #

    @override
    async def create_bench(
        self, request: "CreateBenchRequest", headers: Mapping
    ) -> "CreateBenchResponse":
        # get/check user
        metadata = wiring.unpack_rpc_headers(headers)
        subject, _ = await self.resolve_client(request, metadata)
        if not subject:
            raise GRPCError(GRPCStatus.UNAUTHENTICATED, "not logged in")
        if not request.slug:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "slug not specified")
        if not request.region:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "region not specified")
        region = Region(request.region)

        # get regional database
        regional_database = self._database_map.get(region=region)
        regional_pg_engine = pg_engine_from_database(
            name=f"pg-regional-{regional_database.region.name.lower()}",
            database=regional_database,
            area=NodeArea.REGIONAL_POSTGRES,
        )

        # create bench
        owner_ptr = NodeReference.from_proto(request.owner)
        async with self.new_request_session(
            supergraph=subject._supergraph,
            readonly=False,
            engines=(self._global_pg_engine, regional_pg_engine),
        ) as session:
            subject, _ = await self.resolve_client(request, metadata)
            if not isinstance(subject, User):
                raise GRPCError(GRPCStatus.UNAUTHENTICATED, "not logged in")

            # check (and reload owner to get Handles)
            if owner_ptr.node_type == NodeType.USER:
                if owner_ptr.id != subject.id:
                    raise GRPCError(
                        GRPCStatus.PERMISSION_DENIED, "cannot create bench for other user"
                    )
                owner = await User.get(
                    where=User.property("id").eq(owner_ptr.id), Handles=Handle.search()
                ).execute_one()
            elif owner_ptr.node_type == NodeType.ORGANIZATION:
                owner = await Organization.get(
                    where=Organization.property("id").eq(owner_ptr.id),
                    Handles=Handle.search(),
                ).execute_one()
                if owner.created_by_id != subject.id:
                    raise GRPCError(
                        GRPCStatus.PERMISSION_DENIED, "cannot create bench for other organization"
                    )
            else:
                raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "invalid owner type")
            if owner.status < UserStatus.REGISTERED:
                raise GRPCError(GRPCStatus.FAILED_PRECONDITION, "owner not registered")

            # create Bench (assumes it's the primary bench)
            if owner.status == UserStatus.ACTIVE:
                raise GRPCError(GRPCStatus.ALREADY_EXISTS, "cannot create secondary Benches (yet)")
            if owner.slug != request.slug:
                raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "slug mismatch")
            assert owner.handle is not None, f"{owner!r} has no main handle"
            bench = await create_default_bench(
                handle=owner.handle,
                owned_by=owner,
                region=region,
                session=session,
                options=self.create_bench_options,
            )

            # 'activate' owner
            if isinstance(owner, User) and owner.status != UserStatus.ACTIVE:
                owner.bench = bench
                owner.status = UserStatus.ACTIVE
            elif isinstance(owner, Organization) and owner.status != OrganizationStatus.ACTIVE:
                owner.bench = bench
                owner.status = OrganizationStatus.ACTIVE
            else:
                raise RuntimeError(f"unexpected owner/owner status: {owner!r}")

            await session.commit()

        logger.info("supervisor.create_bench", bench=bench, span="current")
        return CreateBenchResponse(bench=bench.to_proto())

    @override
    async def resolve_hosts(
        self, request: "ResolveHostsRequest", headers: Mapping
    ) -> "ResolveHostsResponse":
        raise NotImplementedError
