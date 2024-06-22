from typing import cast, override
from uuid import UUID, uuid4, uuid5

import betterproto
import structlog
from grpclib import GRPCError
from grpclib import Status as GRPCStatus
from opentelemetry import trace

from bench.language import Bench, Client, NodeReference, Server, Store, User
from bench.language.access import Subject
from bench.language.bench import Region, ServerProfile
from bench.language.const import USER_NODE_TYPES, ClientType, NodeType, OrganizationStatus
from bench.language.graph import generate_node_name
from bench.language.session import Session
from bench.language.user import Handle, Organization, UserStatus
from bench.proto import wiring
from bench.proto.wire import (
    ChangeUserPasswordRequest,
    ChangeUserPasswordResponse,
    ClientDataIn,
    CreateBenchRequest,
    CreateBenchResponse,
    GetHostRequest,
    GetHostResponse,
    LoginUserRequest,
    LoginUserResponse,
    LogoutUserRequest,
    LogoutUserResponse,
    RpcMetadata,
    ServiceKind,
    SignupUserRequest,
    SignupUserResponse,
    SupervisorBase,
)
from bench.system.access import (
    ACCESS_TOKEN_LENGTH,
    SALT_LENGTH,
    check_password,
    get_client,
    hash_password,
    purge_client_caches,
)
from bench.system.core import HostProxy, global_pg_engine_from_store, global_session
from bench.system.graph import GraphIoServiceBase
from bench.system.provisioner import provision
from bench.utils.func import generate_access_token, generate_salt, to_uuid
from bench.utils.oracle import Oracle

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class Supervisor(GraphIoServiceBase, SupervisorBase):
    kind = ServiceKind.PUBLIC  # :ServiceKind

    def __init__(self, global_store: Store, oracle: Oracle):
        GraphIoServiceBase.__init__(
            self,
            bench_id=None,
            node_types=USER_NODE_TYPES,
            logger=logger,
            tracer=tracer,
            oracle=oracle,
        )
        self._global_store = global_store
        self._global_pg_engine = global_pg_engine_from_store(global_store)

    def __str__(self):
        return "shards=[*]"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    async def start(self) -> None:
        await super().start()

    def close(self) -> None:
        super().close()

    async def wait_closed(self) -> None:
        await super().wait_closed()

    @override
    def get_engines(self):
        return (self._global_pg_engine,)

    @tracer.start_as_current_span("supervisor.get_request_subject")
    async def get_request_subject(
        self, request: betterproto.Message, metadata: RpcMetadata
    ) -> Subject:
        # NOTE :Architecture: for simplicity we don't get the full Subject auth in Supervisor
        #  (like we do in Host, since we have the entire Bench cached and ready there,
        #   and we don't expect to need Bench-level auth in the supervisor for now).

        async with global_session(self._global_store, self.get_engines(), self.oracle):
            # request will use the subject's supergraph, so ensure all subjects are created in session
            if not metadata.client_id or not metadata.client_access_token:
                return Subject(is_authenticated=False)
            client_id = UUID(metadata.client_id)
            client = await get_client(client_id, metadata.client_access_token)
            if isinstance(client.parent, User):
                return Subject(
                    is_authenticated=True,
                    is_staff=client.parent.is_staff,
                    client=client,
                    user=client.parent,
                    owned=[client.parent],
                )
            elif isinstance(client.parent, Server):
                return Subject(
                    is_authenticated=True, client=client, server=client.parent, owned=[client.bench]
                )
            else:
                raise RuntimeError(f"unexpected client: {client!r}")

    #
    # User management
    #

    async def _make_client(self, user: User, client_data: ClientDataIn) -> Client:
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
            name=name,
            type=cast(ClientType, client_data.type),
            seen_at=self.oracle.utc(),
            _is_new=True,  # force create
        )
        self._patch_client(client, client_data)
        return client

    def _patch_client(self, client: Client, client_data: ClientDataIn) -> Client:
        # copy over other properties
        for key, value in client_data.__dict__.items():
            if key not in ("id", "name") and key in client.__properties__:
                setattr(client, key, value)
        return client

    @override
    async def signup_user(
        self, subject: Subject, request: "SignupUserRequest"
    ) -> "SignupUserResponse":
        if subject.is_authenticated:
            raise GRPCError(GRPCStatus.ALREADY_EXISTS, "already logged in")

        async with self.request_session(supergraph=subject._supergraph, readonly=False) as session:
            user = User(
                id=to_uuid(request.id) or uuid4(),
                slug=request.slug,
                name=request.name or request.slug,
                email=request.email,
                status=UserStatus.REGISTERED,
                last_logged_in_at=self.oracle.utc(),
                _is_new=True,  # force create
            )
            user.password_salt = generate_salt(SALT_LENGTH)
            user.password_hash = hash_password(request.password, user.password_salt)
            session._create(user)
            await session.flush()
            client = await self._make_client(user, request.client)
            client.access_token = generate_access_token(ACCESS_TOKEN_LENGTH)
            session._create(client)
            await session.flush()
            user.main_handle = user.handles.create(slug=user.slug)
            await session.commit()

        logger.info("supervisor.signup_user", user=user, client=client, span="current")
        return SignupUserResponse(
            user=user._to_data(),
            client=client._to_data(),
            access_token=client.access_token,
        )

    @override
    async def change_user_password(
        self, subject: Subject, request: "ChangeUserPasswordRequest"
    ) -> "ChangeUserPasswordResponse":
        if not subject.user:
            raise GRPCError(GRPCStatus.UNAUTHENTICATED, "not logged in")

        async with self.request_session(supergraph=subject._supergraph, readonly=False) as session:
            user = subject.user
            if user.password_salt is None or user.password_hash is None:
                raise GRPCError(GRPCStatus.FAILED_PRECONDITION, "password not set")
            if not await check_password(
                request.old_password, user.password_salt, user.password_hash, self.oracle
            ):
                raise GRPCError(GRPCStatus.UNAUTHENTICATED, "incorrect password")

            # set new password
            session.track(user)  # user is from another session
            user.password_salt = generate_salt(SALT_LENGTH)
            user.password_hash = hash_password(request.new_password, user.password_salt)
            await session.commit()

        purge_client_caches(user)
        logger.info("supervisor.change_user_password", user=user, span="current")
        return ChangeUserPasswordResponse(user=user._to_data())

    @override
    async def login_user(
        self, subject: Subject, request: "LoginUserRequest"
    ) -> "LoginUserResponse":
        if subject.is_authenticated:
            raise GRPCError(GRPCStatus.ALREADY_EXISTS, "already logged in")

        async with self.request_session(supergraph=subject._supergraph, readonly=False) as session:
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
            if not await check_password(
                request.password, user.password_salt, user.password_hash, self.oracle
            ):
                raise GRPCError(GRPCStatus.UNAUTHENTICATED, "incorrect password")

            user.last_logged_in_at = self.oracle.utc()
            if request.client.id:  # upsert
                client = user._supergraph.get(UUID(request.client.id))
                if not isinstance(client, Client):
                    raise GRPCError(GRPCStatus.NOT_FOUND, "client not found")
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
        self, subject: Subject, request: "LogoutUserRequest"
    ) -> "LogoutUserResponse":
        if subject.client is None:
            raise GRPCError(GRPCStatus.UNAUTHENTICATED, "not logged in")
        if subject.user is None:
            raise GRPCError(GRPCStatus.FAILED_PRECONDITION, "not a user")

        async with self.request_session(supergraph=subject._supergraph, readonly=False) as session:
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

        owner_ptr = wiring.unpack_object(request.owner, supergraph=None, expect=NodeReference)
        async with self.request_session(supergraph=subject._supergraph, readonly=False) as session:
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
            if owner.status == UserStatus.ACTIVATED:
                raise GRPCError(GRPCStatus.ALREADY_EXISTS, "cannot create secondary Benches (yet)")
            if owner.slug != request.slug:
                raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "slug mismatch")
            assert owner.main_handle is not None, f"{owner!r} has no main handle"
            bench = await create_default_bench(
                main_handle=owner.main_handle,
                owner=owner,
                region=region,
                global_store=self._global_store,
                session=session,
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

        if isinstance(owner, User):
            # update user with new bench (supervisor and host may be in same process)
            purge_client_caches(owner)
        logger.info("supervisor.create_bench", bench=bench, span="current")
        return CreateBenchResponse(bench=bench._to_data())

    @override
    async def get_host(self, subject: "Subject", request: "GetHostRequest") -> "GetHostResponse":
        key, value = betterproto.which_one_of(request, "bench")
        async with self.request_session(supergraph=subject._supergraph):
            if key == "id":
                await Bench.get(id=to_uuid(value))
            elif key == "slug":
                await Bench.get(slug=value)
            else:
                raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "no bench specified")
        raise GRPCError(GRPCStatus.UNIMPLEMENTED, "not implemented :SingleHostService")


async def create_default_bench(
    *,
    main_handle: Handle,
    owner: User | Organization,
    region: Region,
    global_store: Store,
    session: Session,
) -> Bench:
    # create bench
    bench = Bench(
        main_handle=main_handle,
        slug=main_handle.slug,
        name=main_handle.slug,
        owner=owner,
        region=region,
    )
    session._create(bench)
    await session.flush()

    # create resources (in pending state, resources are managed by hosts)
    server = bench.servers.create(
        region=bench.region,
        profile=ServerProfile.SMALL,
        name="Server",
    )
    store = bench.stores.create(region=bench.region, name="Store")
    drive = bench.drives.create(region=bench.region, name="Drive")

    # create main environment/branch/package
    main_environment = bench.environments.create(
        name="Main", server=server, store=store, drive=drive
    )
    main_branch = bench.branches.create(name="Main", slug="main")
    main_package = main_branch.packages.create(environment=main_environment)
    await session.flush()
    main_branch.main_package = main_package
    bench.main_environment = main_environment
    bench.main_branch = main_branch

    # immediately provision main store (must be ready for Host)
    await provision(HostProxy(global_store, session), bench, (store,))

    return bench
