import betterproto
import structlog
from grpclib import GRPCError
from grpclib import Status as GRPCStatus

from bench.language import Client, User, NodeReference, Bench, Tenancy
from bench.language.access import (
    Subject,
)
from bench.language.const import (
    USER_NODE_TYPES,
    NodeType,
    StoreKind,
    StoreEngineType,
)
from bench.language.resource import ServerProfile, Region
from bench.language.user import UserStatus, Organization, Handle
from bench.proto import wiring
from bench.proto.services import BenchServiceBase
from bench.proto.wire import (
    ChangeUserPasswordRequest,
    ChangeUserPasswordResponse,
    CreateBenchRequest,
    CreateBenchResponse,
    LoginUserRequest,
    LoginUserResponse,
    LogoutUserRequest,
    LogoutUserResponse,
    SignupUserRequest,
    SignupUserResponse,
    SupervisorBase,
    SupervisorStub,
)
from bench.system.auth import (
    check_password,
    generate_access_token,
    generate_salt,
    hash_password,
    generate_encryption_key,
)
from bench.system.graph import GraphIoService
from bench.system.utils import global_session, GLOBAL_POSTGRES_ENGINE
from bench.utils.dt import utcnow_with_tz
from bench.utils.func import to_uuid

logger = structlog.get_logger(__name__)


class Supervisor(BenchServiceBase[SupervisorStub], GraphIoService, SupervisorBase):
    def __init__(self):
        BenchServiceBase.__init__(self, loopback_stub_to=SupervisorStub)
        GraphIoService.__init__(self, bench_id=None, node_types=USER_NODE_TYPES)

    def __str__(self):
        return "shards=[*]"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    async def start_quick(self) -> None:
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

    async def signup_user(
        self, subject: Subject, request: "SignupUserRequest"
    ) -> "SignupUserResponse":
        if subject.is_authenticated:
            raise GRPCError(GRPCStatus.ALREADY_EXISTS, "already logged in")

        async with global_session() as session:
            user = User(
                id=to_uuid(request.id),
                slug=request.slug,
                name=request.name,
                email=request.email,
                status=UserStatus.REGISTERED,
                _is_new=True,  # force create (despite already having an id)
            )
            user.password_salt = generate_salt()
            user.password_hash = hash_password(request.password, user.password_salt)
            client = Client(
                id=to_uuid(request.client.id),
                parent=user,
                name=request.client.name,
                device_name=request.client.device_name,
                browser_name=request.client.browser_name,
                last_seen_at=utcnow_with_tz(),
                _is_new=True,  # also force create
            )
            client.access_token = generate_access_token()
            session.create(user, client)
            await session.flush()
            user.main_handle = user.handles.create(slug=user.slug)
            await session.commit()
            self.on_graph_edited(session.tx.edits)

        return SignupUserResponse(
            user=user._to_data(), access_token=client.access_token, epoch=self.epoch
        )

    async def change_user_password(
        self, subject: Subject, request: "ChangeUserPasswordRequest"
    ) -> "ChangeUserPasswordResponse":
        if not subject.is_authenticated:
            raise GRPCError(GRPCStatus.UNAUTHENTICATED, "not logged in")

        async with global_session() as session:
            user = subject.user
            if not await check_password(
                request.old_password, user.password_salt, user.password_hash
            ):
                raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "incorrect password")

            # set new password
            session.track(user)  # user is in other session
            user.password_salt = generate_salt()
            user.password_hash = hash_password(request.password, user.password_salt)
            await session.commit()
            self.on_graph_edited(session.tx.edits)

        return ChangeUserPasswordResponse(user=user._to_data(), epoch=self.epoch)

    async def login_user(
        self, subject: Subject, request: "LoginUserRequest"
    ) -> "LoginUserResponse":
        if subject.is_authenticated:
            raise GRPCError(GRPCStatus.ALREADY_EXISTS, "already logged in")

        async with global_session() as session:
            key_name, key_value = betterproto.which_one_of(request, "user")
            if key_value is None:
                raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "no user provided")
            user = await User.include(User.password_salt, User.password_hash).get(
                User.__properties__[key_name] == key_value
            )
            if not await check_password(request.password, user.password_salt, user.password_hash):
                raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "incorrect password")

            now = utcnow_with_tz()
            client = Client(
                id=to_uuid(request.client.id),
                parent=user,
                name=request.client.name,
                device_name=request.client.device_name,
                browser_name=request.client.browser_name,
                last_seen_at=now,
                logged_in_at=now,
                access_token=generate_access_token(),
                _is_new=True,  # force create
            )
            session.upsert(client)
            await session.commit()
            self.on_graph_edited(session.tx.edits)

        return LoginUserResponse(
            user=user._to_data(),
            client=client._to_data(),
            access_token=client.access_token,
            epoch=self.epoch,
        )

    async def logout_user(
        self, subject: Subject, request: "LogoutUserRequest"
    ) -> "LogoutUserResponse":
        if subject.client is None:
            raise GRPCError(GRPCStatus.UNAUTHENTICATED, "not logged in")

        async with global_session() as session:
            # log out the current or the specified clients
            if request.client_ids:
                clients = await Client.filter(
                    parent=subject.user, id__in=request.client_ids
                ).tolist()
                if len(clients) != len(request.client_ids):
                    missing_ids = set(request.client_ids) - {c.id for c in clients}
                    raise GRPCError(GRPCStatus.NOT_FOUND, f"clients not found: {missing_ids}")
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
            self.on_graph_edited(session.tx.edits)

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
            # check
            owner: Organization | User
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
            if owner.status == UserStatus.ACTIVATED or owner.slug != request.slug:
                raise GRPCError(GRPCStatus.ALREADY_EXISTS, "cannot create secondary Benches (yet)")
            if owner.status < UserStatus.REGISTERED:
                raise GRPCError(GRPCStatus.FAILED_PRECONDITION, "owner not registered")

            # create bench
            assert owner.main_handle is not None, f"{owner!r} has no main handle"
            main_handle = owner.main_handle
            bench = Bench(
                main_handle=main_handle,
                slug=main_handle.slug,
                name=main_handle.slug,
                owner=owner,
                encryption_key=generate_encryption_key(),
                region=region,
            )
            session.create(bench)
            await session.flush()

            # create resources (in pending state)
            server = bench.servers.create(
                region=bench.region,
                tenancy=Tenancy.SHARED,
                profile=ServerProfile.SMALL,
                name="Main Server",
            )
            store = bench.stores.create(
                region=bench.region,
                tenancy=Tenancy.DEDICATED,
                kind=StoreKind.RELATIONAL,
                engine=StoreEngineType.POSTGRES,
                name="Main Store",
            )
            search = bench.stores.create(
                region=bench.region,
                tenancy=Tenancy.DEDICATED,
                kind=StoreKind.SEARCH,
                engine=StoreEngineType.OPENSEARCH,
                name="Main Search",
            )
            drive = bench.drives.create(
                region=bench.region, tenancy=Tenancy.SHARED, name="Main Drive"
            )

            # create main environment/branch/package
            environment = bench.environments.create(
                name="Main", store=store, search=search, drive=drive, server=server
            )
            branch = bench.branches.create(name="Main")
            package = bench.packages.create(environment=environment, branch=branch)
            await session.flush()
            branch.main_package = package
            bench.main_package = branch.main_package
            bench.main_branch = branch
            bench.main_environment = environment

            # 'activate' owner
            if owner.status != UserStatus.ACTIVATED:
                owner.main_bench = bench
                owner.status = UserStatus.ACTIVATED

            await session.commit()
            self.on_graph_edited(session.tx.edits)

        raise GRPCError(GRPCStatus.UNIMPLEMENTED)
