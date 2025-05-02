from typing import Callable, Mapping, override
from uuid import UUID, uuid4, uuid5

import structlog
from google.protobuf.message import Message as ProtoMessage
from grpclib import GRPCError
from grpclib import Status as GRPCStatus
from opentelemetry import trace

from bench.language import (
    EMPTY_SCOPE_DATA,
    USER_NODE_TYPES,
    Bench,
    Client,
    ClientType,
    Handle,
    Node,
    NodeArea,
    NodeReference,
    NodeType,
    Organization,
    OrganizationStatus,
    PolicySubject,
    Region,
    Store,
    User,
    UserStatus,
    bittuple,
)
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
    RpcMetadata,
    ServiceKind,
    SignupUserRequest,
    SignupUserResponse,
    SupervisorBase,
    wiring,
)
from bench.system.core import (
    ACCESS_TOKEN_LENGTH,
    SALT_LENGTH,
    HostMap,
    PostgresMap,
    check_password,
    get_client_or_error,
    global_session,
    hash_password,
    pg_engine_from_store,
    purge_client_caches,
)
from bench.system.graph import GraphServiceBase
from bench.utils.func import generate_access_token, generate_salt, to_uuid
from bench.utils.oracle import Oracle

from .bench import CreateBenchOptions, create_default_bench

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

SUPERVISOR_NODE_TYPES = USER_NODE_TYPES | bittuple(NodeType.BENCH)


class SupervisorService(GraphServiceBase, SupervisorBase):
    kind = ServiceKind.PUBLIC  # :ServiceKind
    name = "supervisor"

    def __init__(
        self,
        id: str,
        global_store: Store,
        store_map: PostgresMap,
        network: Network,
        oracle: Oracle,
        host_map: HostMap,
        create_bench_options: CreateBenchOptions,
        on_error: Callable[[Exception], None] | None = None,
    ):
        GraphServiceBase.__init__(
            self,
            id=id,
            bench_id=None,
            node_types=SUPERVISOR_NODE_TYPES,
            logger=logger,
            tracer=tracer,
            network=network,
            oracle=oracle,
            scope=EMPTY_SCOPE_DATA,
        )
        self._global_store = global_store
        self._global_pg_engine = pg_engine_from_store("pg-global", global_store, NodeArea.GLOBAL)
        self._store_map = store_map
        self._host_map = host_map
        self._create_bench_options = create_bench_options
        self._on_error = on_error

    def __str__(self):
        return "shards=[*]"

    async def start(self) -> None:
        await super().start()

    def stop(self) -> None:
        super().stop()

    async def wait_stopped(self) -> None:
        await super().wait_stopped()

    @override
    def get_engines(self):
        return (self._global_pg_engine,)

    @tracer.start_as_current_span("supervisor.get_request_subject")
    async def get_request_subject(
        self, request: ProtoMessage, metadata: RpcMetadata
    ) -> PolicySubject:
        # NOTE :Architecture: for simplicity we don't get the full Subject auth in Supervisor
        #  (like we do in Host, since we have the entire Bench cached and ready there,
        #   and we don't expect to need Bench-level auth in the Supervisor for now).
        async with global_session(self._global_store, self.get_engines(), self.oracle):
            # request will use the subject's supergraph, so ensure all subjects are created in session
            if not metadata.client_id or not metadata.client_access_token:
                return PolicySubject(is_authenticated=False)
            client_id = UUID(metadata.client_id)
            client = await get_client_or_error(client_id, metadata.client_access_token)
            if isinstance(client.parent, User):
                return PolicySubject(
                    is_authenticated=True,
                    is_staff=client.parent.is_staff,
                    client=client,
                    user=client.parent,
                    owned=[client.parent],
                )
            elif isinstance(client.parent, Bench):
                bench = client.parent
                assert bench is not None, f"{client!r} has no bench"
                return PolicySubject(
                    is_authenticated=True, client=client, computer=client.computer, owned=[bench]
                )
            else:
                raise RuntimeError(f"unexpected client: {client!r}")

    @override
    async def resolve_request_base(self, node_ptr: UUID | NodeReference) -> Node | None:
        raise RuntimeError("supervisor does not support database-level requests")

    #
    # User management
    #

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
        subject = await self.get_request_subject(request, metadata)
        if subject.is_authenticated:
            raise GRPCError(GRPCStatus.ALREADY_EXISTS, "already logged in")

        # get regional Store
        region = Region(request.region)
        regional_store = self._store_map.get(region=region)
        regional_pg_engine = pg_engine_from_store(
            name=f"pg-regional-{regional_store.region.name.lower()}",
            store=regional_store,
            area=NodeArea.REGIONAL,
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
            session._create(user)
            session.stage()

            # create Client
            client = await self._make_client(user, request.client)
            client.access_token = generate_access_token(ACCESS_TOKEN_LENGTH)
            session._create(client)
            session.stage()
            user.handle = user.handles.create(slug=user.slug)
            await session.commit()

            # immediately create User's main Bench
            if request.activate:
                bench = await create_default_bench(
                    main_handle=user.handle,
                    owned_by=user,
                    region=user.region,
                    session=session,
                    options=self._create_bench_options,
                )
                user.bench = bench
                user.status = UserStatus.ACTIVATED
                await session.commit()

        logger.info("supervisor.signup_user", user=user, client=client, span="current")
        return SignupUserResponse(
            user=user._to_data(), client=client._to_data(), access_token=client.access_token
        )

    @override
    async def change_user_password(
        self, request: "ChangeUserPasswordRequest", headers: Mapping
    ) -> "ChangeUserPasswordResponse":
        metadata = wiring.unpack_rpc_headers(headers)
        subject = await self.get_request_subject(request, metadata)
        if not subject.user:
            raise GRPCError(GRPCStatus.UNAUTHENTICATED, "not logged in")

        async with self.new_request_session(
            supergraph=subject._supergraph, readonly=False
        ) as session:
            user = subject.user
            if user.password_salt is None or user.password_hash is None:
                raise GRPCError(GRPCStatus.FAILED_PRECONDITION, "password not set")
            if not await check_password(
                request.old_password, user.password_salt, user.password_hash, self.oracle
            ):
                raise GRPCError(GRPCStatus.UNAUTHENTICATED, "incorrect password")

            # set new password
            session._track(user)  # user is from another session
            user.password_salt = generate_salt(SALT_LENGTH)
            user.password_hash = hash_password(request.new_password, user.password_salt)
            await session.commit()

        purge_client_caches(user)
        logger.info("supervisor.change_user_password", user=user, span="current")
        return ChangeUserPasswordResponse(user=user._to_data())

    @override
    async def login_user(
        self, request: "LoginUserRequest", headers: Mapping
    ) -> "LoginUserResponse":
        metadata = wiring.unpack_rpc_headers(headers)
        subject = await self.get_request_subject(request, metadata)
        if subject.is_authenticated:
            raise GRPCError(GRPCStatus.ALREADY_EXISTS, "already logged in")

        async with self.new_request_session(
            supergraph=subject._supergraph, readonly=False
        ) as session:
            key_name = request.WhichOneof("user")
            key_value = getattr(request, key_name)
            if key_value is None:
                raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "no user provided")
            user = (
                await User.include(
                    User.get_property("password_salt"), User.get_property("password_hash")
                )
                .include_descendants(NodeType.CLIENT)
                .get(User.__properties__[key_name] == key_value)
            )
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
        purge_client_caches(user)
        return LoginUserResponse(
            user=user._to_data(),
            client=client._to_data(),
            access_token=client.access_token,
        )

    @override
    async def logout_user(
        self, request: "LogoutUserRequest", headers: Mapping
    ) -> "LogoutUserResponse":
        metadata = wiring.unpack_rpc_headers(headers)
        subject = await self.get_request_subject(request, metadata)
        if subject.client is None:
            raise GRPCError(GRPCStatus.UNAUTHENTICATED, "not logged in")
        if subject.user is None:
            raise GRPCError(GRPCStatus.FAILED_PRECONDITION, "not a user")

        async with self.new_request_session(
            supergraph=subject._supergraph, readonly=False
        ) as session:
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
                session._track(subject.client)
            for client in clients:
                client.logged_in_at = None
                client.access_token = None
                client.seen_at = self.oracle.utc()
            await session.commit()

        purge_client_caches(subject.user)
        logger.info("supervisor.logout_user", user=subject.user, clients=clients, span="current")
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
        subject = await self.get_request_subject(request, metadata)
        user: User | None = subject.user
        if not user:
            raise GRPCError(GRPCStatus.UNAUTHENTICATED, "not logged in")
        if not request.slug:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "slug not specified")
        if not request.region:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "region not specified")
        if user.status == UserStatus.WAITLISTED:
            raise GRPCError(GRPCStatus.PERMISSION_DENIED, "cannot create bench for waitlisted user")
        region = wiring.unpack_enum(Region, request.region)

        # get regional store
        regional_store = self._store_map.get(region=region)
        regional_pg_engine = pg_engine_from_store(
            name=f"pg-regional-{regional_store.region.name.lower()}",
            store=regional_store,
            area=NodeArea.REGIONAL,
        )

        # create bench
        owner_ptr = wiring.unpack_builtin_object(
            request.owner, supergraph=None, expect=NodeReference
        )
        async with self.new_request_session(
            supergraph=subject._supergraph,
            readonly=False,
            engines=(self._global_pg_engine, regional_pg_engine),
        ) as session:
            # check (and reload owner to get Handles)
            if owner_ptr.node_type == NodeType.USER:
                if owner_ptr.id != user.id:
                    raise GRPCError(
                        GRPCStatus.PERMISSION_DENIED, "cannot create bench for other user"
                    )
                owner = await User.include_descendants(Handle).get(id=owner_ptr.id)
            elif owner_ptr.node_type == NodeType.ORGANIZATION:
                owner = await Organization.include_descendants(Handle).get(id=owner_ptr.id)
                if owner.created_by_id != user.id:
                    raise GRPCError(
                        GRPCStatus.PERMISSION_DENIED, "cannot create bench for other organization"
                    )
            else:
                raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "invalid owner type")
            if owner.status < UserStatus.REGISTERED:
                raise GRPCError(GRPCStatus.FAILED_PRECONDITION, "owner not registered")

            # create bench (assumes it's the primary bench)
            if owner.status == UserStatus.ACTIVATED:
                raise GRPCError(GRPCStatus.ALREADY_EXISTS, "cannot create secondary Benches (yet)")
            if owner.slug != request.slug:
                raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "slug mismatch")
            assert owner.handle is not None, f"{owner!r} has no main handle"
            bench = await create_default_bench(
                main_handle=owner.handle,
                owned_by=owner,
                region=region,
                session=session,
                options=self._create_bench_options,
            )

            # 'activate' owner
            if isinstance(owner, User) and owner.status != UserStatus.ACTIVATED:
                owner.bench = bench
                owner.status = UserStatus.ACTIVATED
            elif isinstance(owner, Organization) and owner.status != OrganizationStatus.ACTIVATED:
                owner.bench = bench
                owner.status = OrganizationStatus.ACTIVATED
            else:
                raise RuntimeError(f"unexpected owner/owner status: {owner!r}")

            await session.commit()

        # update user with new bench (supervisor and host may be in same process)
        if isinstance(owner, User):
            purge_client_caches(owner)

        logger.info("supervisor.create_bench", bench=bench, span="current")
        return CreateBenchResponse(bench=bench._to_data())

    @override
    async def resolve_hosts(
        self, request: "ResolveHostsRequest", headers: Mapping
    ) -> "ResolveHostsResponse":
        metadata = wiring.unpack_rpc_headers(headers)
        subject = await self.get_request_subject(request, metadata)
        async with self.new_request_session(supergraph=subject._supergraph):
            hosts: list[ResolveHostsResponse.HostInfo] = []
            for bench_key in request.benches:
                # NOTE :Performance: batch resolve_hosts lookups
                key = bench_key.WhichOneof("bench")
                value = getattr(bench_key, key)
                if key == "id":
                    bench = await Bench.get(id=to_uuid(value))
                elif key == "slug":
                    bench = await Bench.get(slug=value)
                else:
                    raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "no bench specified")
                host_info = self._host_map.get(bench.id, bench.region)
                host_info = ResolveHostsResponse.HostInfo(
                    domain=host_info.host_domain,
                    grpc_port=host_info.grpc_port,
                    grpc_web_port=host_info.grpc_web_port,
                    ssl=host_info.ssl,
                    bench=bench._to_ref_data(),
                )
                hosts.append(host_info)
        return ResolveHostsResponse(hosts=hosts)
