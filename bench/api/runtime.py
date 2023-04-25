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

from bench import language, models, runtime
from bench.api.auth import check_can_view_project_by_id, check_can_write_project
from bench.api.execution import ExecutionTriggerType
from bench.api.statement import SimpleTypeNode, SimplyTyped, StatementType, SymbolType, TypeTag
from bench.api.util import asafe_mutation, asafe_subscription
from bench.language import wire
from bench.language.type import StatementModifier
from bench.models import mapper
from bench.msg import NMessageType, messages
from bench.msg.core import NMessage, request, subscribe
from bench.msg.messages import (
    InterpChangedPayload,
    RepBuildPayload,
    RepInterpPayload,
    RepRunPayload,
    ReqBuildPayload,
    ReqInterpPayload,
    ReqRunPayload,
)

logger = structlog.get_logger(__name__)


@gql.type
class InterpFile:
    id: GlobalID
    module: "InterpModule"
    path: str
    symbols: list["InterpSymbol"]


JobType = gql.enum(models.JobType)
JobStatus = gql.enum(models.JobStatus)


@gql.type
class InterpSimpleType(SimpleTypeNode):
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
    stale_symbols: list["InterpSymbol"]


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
    available_builds: Optional[list[GlobalID]]


InterpErrorType = gql.enum(language.ErrorType)


@gql.type
class InterpError:
    type: InterpErrorType
    message: str
    symbol: Optional[InterpSymbol]


def rmap_module(
    wire_module: wire.ModuleData, builds_by_symbol: dict[UUID, list[UUID]]
) -> InterpModule:
    interp_module = InterpModule(
        id=GlobalID("ProjectVersion", str(wire_module.id)),
        name=wire_module.name,
        files=[],
        # these are unknown at this point
        dependencies=[],
        errors=[],
        stale_symbols=[],
    )
    interp_module.files = rmap_files(wire_module.files, interp_module, builds_by_symbol)
    return interp_module


def rmap_files(
    wire_files: list[wire.FileData],
    interp_module: InterpModule,
    builds_by_symbol: dict[UUID, list[UUID]],
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
            root_type_tag, type_nodes = mapper.wmap_type_nodes(
                None, statement.type_nodes, impute_type_reference=True
            )
            available_builds = builds_by_symbol.get(statement.id)
            if available_builds:  # to GlobalID
                available_builds = [
                    GlobalID("Statement", str(build_id)) for build_id in available_builds
                ]
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
                root_type_tag=root_type_tag,
                type_nodes=type_nodes,
                available_builds=available_builds,
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


BuildScope = gql.enum(runtime.type.BuildScope)


@gql.input
class BuildInput:
    project_version_id: GlobalID
    scope: BuildScope
    buildable_id: Optional[GlobalID] = None


@gql.type
class BuildState:
    project_version_id: GlobalID
    success: bool


ExecutionTracingLevel = gql.enum(wire.ExecutionTracingLevel)


@gql.input
class RunInput:
    project_version_id: GlobalID
    runnable_id: Optional[GlobalID] = None
    build_id: Optional[GlobalID] = None
    arguments: JSON
    trace: ExecutionTracingLevel = ExecutionTracingLevel.ALL_FRAMES_WITH_DATA
    block: bool = True
    timeout_seconds: Optional[int] = None


ModuleRunErrorType = gql.enum(messages.RunErrorType)


@gql.type
class RunState:
    project_version_id: GlobalID
    runnable_id: Optional[GlobalID]
    build_id: Optional[GlobalID]
    output: Optional[JSON]
    success: bool
    error: Optional[ModuleRunErrorType]
    error_details: Optional[JSON]


@gql.type
class ModuleRuntimeMutation:
    @asafe_mutation
    async def build(self, info: Info, input: BuildInput) -> BuildState | OperationInfo:
        project_version_id = UUID(input.project_version_id.node_id)
        await sync_to_async(check_can_write_project)(info, project_version_id)
        user = cast(models.User, info.context.request.scope["user"]._wrapped)
        posthog.capture(str(user.id), "build", {"project_version_id": str(project_version_id)})

        req = ReqBuildPayload(
            module_id=project_version_id, scope=input.scope, buildable_id=input.buildable_id.node_id
        )
        rep = await request(NMessageType.REQUEST_BUILD, req, RepBuildPayload)
        return BuildState(
            project_version_id=input.project_version_id,
            success=rep.p.error is None,
        )

    @asafe_mutation
    async def run(self, info: Info, input: RunInput) -> RunState | OperationInfo:
        project_version_id = UUID(input.project_version_id.node_id)
        user = cast(models.User, info.context.request.scope["user"]._wrapped)
        # TODO @Auth: should run be a guest-level permission for projects?
        await sync_to_async(check_can_write_project)(info, project_version_id)
        # :SingleOwnedDeployment
        deployment_id = (
            await models.Deployment.objects.filter(
                owned=True, project_version_id=project_version_id
            )
            .values_list("id", flat=True)
            .afirst()
        )
        if deployment_id is None:
            raise ValueError("no available deployment found")

        run = ReqRunPayload(
            module_id=project_version_id,
            runnable=UUID(input.runnable_id.node_id) if input.runnable_id else None,
            runnable_type=None,
            build=UUID(input.build_id.node_id) if input.build_id else None,
            arguments=input.arguments,
            block=input.block,
            tracing_level=input.trace,
            deployment_id=deployment_id,
            trigger_type=ExecutionTriggerType.UI_INTERACTIVE,
            trigger_id=user.id,
        )
        try:
            rep = await request(
                NMessageType.REQUEST_RUN,
                run,
                RepRunPayload,
                timeout=input.timeout_seconds,
            )
            success = rep.p.error is None
            output = rep.p.output
            error = rep.p.error
            error_details = rep.p.error_details
        except TimeoutError:
            success = False
            output = None
            error = ModuleRunErrorType.TIMEOUT
            error_details = None
        posthog.capture(
            str(user.id), "run", {"project_version_id": str(project_version_id), "error": error}
        )
        return RunState(
            project_version_id=input.project_version_id,
            runnable_id=input.runnable_id,
            build_id=input.build_id,
            output=output,
            success=success,
            error=error,
            error_details=error_details,
        )


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
        module = rmap_module(rep.p.module, rep.p.builds_by_symbol)
        interp = InterpModule(
            id=module.id,
            name=module.name,
            files=module.files,
            dependencies=[
                rmap_module(dep, rep.p.builds_by_symbol) for dep in new_interp.dependencies
            ],
            errors=rmap_errors(new_interp.errors, module),
            stale_symbols=[
                _get_symbol_from_module(module, symbol_id) for symbol_id in new_interp.stale_symbols
            ],
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
                interp.files = rmap_files(
                    new_interp.module.files, interp, new_interp.builds_by_symbol
                )
            if new_interp.dependencies is not None:
                interp.dependencies = [
                    rmap_module(dep, new_interp.builds_by_symbol) for dep in new_interp.dependencies
                ]
            if new_interp.errors is not None:
                interp.errors = rmap_errors(new_interp.errors, interp)
            if new_interp.stale_symbols is not None:
                interp.stale_symbols = [
                    _get_symbol_from_module(interp, symbol_id)
                    for symbol_id in new_interp.stale_symbols
                ]
            interp.updated_at = new_interp.updated_at
            yield interp
