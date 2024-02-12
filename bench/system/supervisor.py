import betterproto
import structlog
from grpclib import GRPCError
from grpclib import Status as GRPCStatus

from bench.language import Client, User
from bench.language.access import (
    Subject,
)
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
from bench.system.auth import check_password, generate_access_token, generate_salt, hash_password
from bench.system.graph import GraphIoService
from bench.system.utils import global_session
from bench.utils.dt import utcnow_with_tz
from bench.utils.func import to_uuid

logger = structlog.get_logger(__name__)


class Supervisor(BenchServiceBase[SupervisorStub], GraphIoService, SupervisorBase):
    def __init__(self):
        super().__init__(loopback_stub_to=SupervisorStub)

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

        async with global_session() as session:
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
            client: Client = wiring.unpack_node(request.client, parent=user, session=session)
            client.access_token = generate_access_token()
            session.create(user, client)
            await session.flush()
            user.main_handle = user.handles.create(slug=user.slug)
            await session.commit()

        return SignupUserResponse(
            user=user._to_data(), access_token=client.access_token, epoch=self._epoch
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

        return ChangeUserPasswordResponse(user=user._to_data(), epoch=self._epoch)

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

        async with global_session() as session:
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

        async with global_session() as session:
            session.track(user)
            if user.bench:  # can't create secondary benches yet
                raise GRPCError(GRPCStatus.ALREADY_EXISTS, "bench already exists")

        raise GRPCError(GRPCStatus.UNIMPLEMENTED)
