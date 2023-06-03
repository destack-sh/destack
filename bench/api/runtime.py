from itertools import chain
from typing import AsyncGenerator, Optional, cast
from uuid import UUID

import posthog
import structlog
from asgiref.sync import sync_to_async
from django.core.exceptions import PermissionDenied
from strawberry.scalars import JSON
from strawberry.types import Info
from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID
from strawberry_django_plus.types import OperationInfo

from bench import language, models
from bench.api.auth import check_can_view_project_by_id, check_can_write_project
from bench.api.execution import Execution, ExecutionTriggerType
from bench.api.statement import Field, SimplyTyped, StatementType, SymbolType, TypeTag
from bench.api.util import asafe_mutation, asafe_subscription, to_uuid
from bench.language import wire
from bench.language.type import StatementModifier
from bench.models import mapper
from bench.msg import NMessageType, messages
from bench.msg.core import NMessage, request, subscribe
from bench.msg.messages import (
    InterpChangedPayload,
    RepCancelRunPayload,
    RepInterpPayload,
    RepRunPayload,
    ReqCancelRunPayload,
    ReqInterpPayload,
    ReqRunPayload,
)
from bench.runtime.instance import SessionTracingLevel

logger = structlog.get_logger(__name__)


@gql.type
class InterpFile:
    id: GlobalID
    module: "InterpModule"
    path: str
    symbols: list["InterpSymbol"]


@gql.type
class InterpSimpleType(Field):
    """
    Proxy type to SimpleType to avoid overwriting source SimpleType references
    (no extra fields yet but needed since (SimpleType, id) global id would be the same
     for the simple types output by the runtime and by the source types put in).
    """

    pass


@gql.type
class InterpModule:
    id: GlobalID
    name: str
    files: list["InterpFile"]
    dependencies: list["InterpModule"]
    errors: list["InterpError"]


# not exactly like language.InterpSymbol, but should be eventually
# which is not what we get out of the runtime worker yet
@gql.type
class InterpSymbol(SimplyTyped):
    id: GlobalID
    file: InterpFile
    order_key: str
    parent_id: Optional[GlobalID]
    name: Optional[str]
    fqn: str
    type: StatementType
    generated: bool
    modifier: Optional[StatementModifier]
    symbol_type: Optional[SymbolType]
    root_type_tag: Optional[TypeTag]
    type_nodes: Optional[list[InterpSimpleType]]


InterpErrorType = gql.enum(language.ErrorType)


@gql.type
class InterpError:
    type: InterpErrorType
    message: str
    symbol: Optional[InterpSymbol]


def rmap_module(wire_module: wire.ModuleData) -> InterpModule:
    interp_module = InterpModule(
        id=GlobalID("ProjectVersion", str(wire_module.id)),
        name=wire_module.name,
        files=[],
        # these are unknown at this point
        dependencies=[],
        errors=[],
    )
    interp_module.files = rmap_files(wire_module.files, interp_module)
    return interp_module


def rmap_files(
    wire_files: list[wire.FileData],
    interp_module: InterpModule,
) -> list[InterpFile]:
    """Maps a wire module into a GQL interpreted module"""
    interp_files = []
    for file in wire_files:
        interp_file = InterpFile(
            id=GlobalID("File", str(file.id)),
            module=interp_module,
            path=file.path,
            symbols=[],
        )
        interp_files.append(interp_file)
        for statement in file.statements:
            if statement.type in (StatementType.COMMENT, StatementType.BLANK):
                continue  # ignore non-symbol statements
            if statement.fields is not None:
                type_nodes = [mapper.wmap_field(statement.id, node) for node in statement.fields]
            else:
                type_nodes = None
            interp_symbol = InterpSymbol(
                id=GlobalID("Statement", str(statement.id)),
                file=interp_file,
                order_key=statement.order_key,
                parent_id=statement.parent_id,
                name=statement.name,
                fqn=statement.fqn,
                type=statement.type,
                generated=statement.generated,
                modifier=statement.modifier,
                symbol_type=statement.symbol_type,
                root_type_tag=statement.root_type_tag,
                type_nodes=type_nodes,
            )
            interp_file.symbols.append(interp_symbol)
    return interp_files


def _get_symbol_from_module(module: InterpModule, symbol_id: UUID) -> Optional[InterpSymbol]:
    symbol_id_str = str(symbol_id)
    for symbol in chain.from_iterable(file.symbols for file in module.files):
        if symbol.id.node_id == symbol_id_str:
            return symbol
    # technically we should never get here, but it sometimes happens?
    return None


def rmap_errors(wire_errors: list[wire.ErrorData], module: InterpModule) -> list[InterpError]:
    """Maps a wire error into a GQL error"""
    errors = []
    for error in wire_errors:
        symbol = _get_symbol_from_module(module, error.statement_id)
        error = InterpError(type=InterpErrorType(error.type), message=error.message, symbol=symbol)
        errors.append(error)
    return errors


@gql.input
class RunInput:
    project_version_id: GlobalID
    runnable_id: Optional[GlobalID] = None
    build_id: Optional[GlobalID] = None
    execution_id: Optional[GlobalID] = None
    arguments: Optional[JSON] = None
    trace: int = SessionTracingLevel.ALL
    block: bool = True
    timeout_seconds: Optional[int] = None


ModuleRunErrorType = gql.enum(messages.RunErrorType)


@gql.type
class RunState:
    project_version_id: GlobalID
    runnable_id: Optional[GlobalID]
    default_build_id: Optional[GlobalID]
    success: bool
    execution_id: Optional[GlobalID]
    execution: Optional[Execution]


@gql.input
class CancelRunInput:
    project_version_id: GlobalID
    execution_id: GlobalID


@gql.type
class CancelRunPayload:
    success: bool
    execution: Optional[Execution]


@gql.type
class RuntimeMutation:
    @asafe_mutation
    async def run(self, info: Info, input: RunInput) -> RunState | OperationInfo:
        project_version_id = UUID(input.project_version_id.node_id)
        user = cast(models.User, info.context.request.scope["user"]._wrapped)
        project_version = await models.ProjectVersion.objects.aget(id=project_version_id)
        # TODO @Auth: should run be a guest-level permission for projects?
        await sync_to_async(check_can_write_project)(info, project_version)

        run = ReqRunPayload(
            module_id=project_version_id,
            runnable=to_uuid(input.runnable_id),
            runnable_type=None,
            default_build_id=to_uuid(input.build_id),
            arguments=input.arguments,
            block=input.block,
            tracing_level=input.trace,
            trigger_type=ExecutionTriggerType.UI,
            trigger_id=user.id,
            execution_id=to_uuid(input.execution_id),
        )
        try:
            rep: NMessage[RepRunPayload] = await request(
                NMessageType.REQUEST_RUN,
                run,
                reply_t=RepRunPayload,
                timeout=input.timeout_seconds,
            )
            success = rep.p.error is None
            error = rep.p.error
        except TimeoutError:
            success = False
            error = ModuleRunErrorType.TIMEOUT
        posthog.capture(
            str(user.id),
            "run",
            {"project_version_id": str(project_version_id), "success": success, "error": error},
        )
        execution = mapper.rmap_execution_frame(rep.p.execution) if rep.p.execution else None
        return RunState(
            project_version_id=input.project_version_id,
            runnable_id=input.runnable_id,
            default_build_id=input.build_id,
            success=success,
            execution=execution,
            execution_id=rep.p.execution_id,
        )

    @asafe_mutation
    async def cancel_run(
        self, info: Info, input: CancelRunInput
    ) -> CancelRunPayload | OperationInfo:
        project_version_id = UUID(input.project_version_id.node_id)
        user = cast(models.User, info.context.request.scope["user"]._wrapped)
        project_version = await models.ProjectVersion.objects.aget(id=project_version_id)
        await sync_to_async(check_can_write_project)(info, project_version)
        cancel = ReqCancelRunPayload(
            module_id=project_version_id,
            execution_id=to_uuid(input.execution_id),
        )
        try:
            rep: NMessage[RepCancelRunPayload] = await request(
                NMessageType.REQUEST_CANCEL_RUN,
                cancel,
                reply_t=RepCancelRunPayload,
            )
            success = rep.p.success
        except TimeoutError:
            success = False
        posthog.capture(
            str(user.id),
            "cancel_run",
            {"project_version_id": str(project_version_id), "success": success},
        )
        return CancelRunPayload(success=success, execution=None)


@gql.type
class InterpSubscription:
    @asafe_subscription
    async def interp_changed(
        self, info: Info, project_version_id: GlobalID
    ) -> AsyncGenerator[InterpModule, None]:
        project_version_id = UUID(project_version_id.node_id)
        user = cast(models.User, info.context.request.scope["user"]._wrapped)
        log = logger.bind(
            project_version_id=project_version_id,
            user=user,
        )
        try:
            await sync_to_async(check_can_view_project_by_id)(
                user, project_version_id=project_version_id
            )
        except PermissionDenied:
            log.debug("runtime.subscribe_denied", project_version_id=project_version_id)
            return

        log.info("interp.subscribe")
        interp_sub = await subscribe(
            f"{NMessageType.INTERP_CHANGED}.{project_version_id}",
            payload_t=InterpChangedPayload,
        )

        # get initial runtime
        rep: NMessage[RepInterpPayload] = await request(
            NMessageType.REQUEST_INTERP,
            ReqInterpPayload(module_id=project_version_id),
            RepInterpPayload,
        )
        new_interp = rep.payload
        module = rmap_module(rep.p.module)
        interp = InterpModule(
            id=module.id,
            name=module.name,
            files=module.files,
            dependencies=[rmap_module(dep) for dep in new_interp.dependencies],
            errors=rmap_errors(new_interp.errors, module),
        )
        yield interp

        # get runtime changes
        log.info("interp.listen")
        while True:
            update: NMessage[InterpChangedPayload] = await interp_sub.next_msg()
            new_interp = update.payload
            log.debug("interp.update", updated_at=new_interp.updated_at)
            # module updates aren't really partial end-to-end yet (only complete fields for worker<->here)
            # :PartialModuleUpdates
            # also the mapping duplication is a bit ugly
            if new_interp.module is not None:
                interp.files = rmap_files(new_interp.module.files, interp)
            if new_interp.dependencies is not None:
                interp.dependencies = [rmap_module(dep) for dep in new_interp.dependencies]
            if new_interp.errors is not None:
                interp.errors = rmap_errors(new_interp.errors, interp)
            interp.updated_at = new_interp.updated_at
            yield interp
