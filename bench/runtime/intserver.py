import asyncio

import structlog
from asgiref.sync import sync_to_async

from bench import models
from bench.models import Execution, Job, ProjectVersion, mapper
from bench.models.mapper import read_module, write_module
from bench.msg import NMessage
from bench.msg.core import handle_reply, message_handler, nc_init, publish, subscribe
from bench.msg.messages import (
    ExecutionChangedPayload,
    ExecutionSavedPayload,
    JobChangedPayload,
    JobSavedPayload,
    ModuleChangedPayload,
    NMessageType,
    ProjectVersionChangedPayload,
    RepReadModulePayload,
    RepWriteBuildCandidatePayload,
    RepWriteEvaluationPayload,
    RepWriteModulePayload,
    ReqReadModulePayload,
    ReqWriteBuildCandidatePayload,
    ReqWriteBuildPayload,
    ReqWriteEvaluationPayload,
)
from bench.msg.sync import is_semantic_mutation
from bench.runtime.type import BuildCandidateData, EvaluationResultData, ExecutionFrameData, JobData
from bench.utils.utils import sentry_capture_if_enabled

logger = structlog.get_logger(__name__)


class InternalServer:
    """
    Server-side Bench language server for reading and writing modules in DB.
    Can be thought of as a sidecar to the main API server for internal operations.
    """

    def __init__(self):
        self.subs = []

    async def run(self):
        await nc_init.wait()
        logger.info("start")
        self.subs = [
            await handle_reply(NMessageType.REQUEST_READ_MODULE, self.read_module),
            await handle_reply(
                NMessageType.REQUEST_WRITE_BUILD_CANDIDATE, self.write_build_candidate
            ),
            await handle_reply(NMessageType.REQUEST_WRITE_EVALUATION, self.write_evaluation),
            await handle_reply(NMessageType.REQUEST_WRITE_BUILD, self.write_build),
            await subscribe(f"{NMessageType.EXECUTION_CHANGED}.*", cb=self.execution_changed),
            await subscribe(f"{NMessageType.JOB_CHANGED}.*", cb=self.job_changed),
            await subscribe(
                f"{NMessageType.PROJECT_VERSION_CHANGED}.*", cb=self.project_version_changed
            ),
        ]

    @message_handler
    async def read_module(self, msg: NMessage[ReqReadModulePayload]) -> None:
        project_v = await ProjectVersion.objects.aget(id=msg.payload.module_id)
        module = await sync_to_async(read_module)(project_v, exclude_non_semantic=True)
        await msg.reply(RepReadModulePayload(module=module, project_id=project_v.project_id))

    @message_handler
    async def write_build(self, msg: NMessage[ReqWriteBuildPayload]) -> None:
        logger.info("module.write_build", files=msg.payload.files, module_id=msg.payload.module_id)
        project_v = await ProjectVersion.objects.aget(id=msg.payload.module_id)
        try:
            if project_v.committed:
                raise ValueError(f"cannot write to committed {project_v}")
            await sync_to_async(write_module)(
                files=msg.payload.files,
                generated_mappings=msg.payload.generated_mappings,
                project_v=project_v,
                overwrite=True,
                delete_generators={msg.p.build_id} if msg.p.delete_previous else None,
            )
            success = True
        except Exception as e:
            sentry_enabled = sentry_capture_if_enabled(e)
            logger.error("module.write.failed", exc_info=e, sentry_enabled=sentry_enabled)
            success = False
        await msg.reply(RepWriteModulePayload(success=success))

        # republish entire module  :PartialModuleUpdates
        # the worker should probably just do this directly
        module = await sync_to_async(read_module)(project_v, exclude_non_semantic=True)
        await publish(
            NMessageType.MODULE_CHANGED, ModuleChangedPayload(module_id=module.id, module=module)
        )

    @message_handler
    async def write_build_candidate(self, msg: NMessage[ReqWriteBuildCandidatePayload]) -> None:
        logger.info(
            "module.write_build_candidate",
            module_id=msg.p.module_id,
            build_id=msg.p.build_id,
            candidates=msg.payload.build_candidates,
        )
        project_v = await ProjectVersion.objects.aget(id=msg.payload.module_id)
        try:
            if project_v.committed:
                raise ValueError(f"cannot write to committed {project_v}")
            await sync_to_async(write_build_candidates)(candidates=msg.payload.build_candidates)
            success = True
        except Exception as e:
            sentry_enabled = sentry_capture_if_enabled(e)
            logger.error(
                "module.write_build_candidate.failed", exc_info=e, sentry_enabled=sentry_enabled
            )
            success = False
        await msg.reply(RepWriteBuildCandidatePayload(success=success))

    @message_handler
    async def write_evaluation(self, msg: NMessage[ReqWriteEvaluationPayload]) -> None:
        logger.info(
            "module.write_evaluation",
            module_id=msg.p.module_id,
            build_id=msg.p.build_id,
            evaluations=msg.payload.evaluations,
        )
        project_v = await ProjectVersion.objects.aget(id=msg.payload.module_id)
        try:
            if project_v.committed:
                raise ValueError(f"cannot write to committed {project_v}")
            await sync_to_async(write_evaluation_results)(evaluations=msg.payload.evaluations)
            success = True
        except Exception as e:
            sentry_enabled = sentry_capture_if_enabled(e)
            logger.error(
                "module.write_evaluation.failed", exc_info=e, sentry_enabled=sentry_enabled
            )
            success = False
        await msg.reply(RepWriteEvaluationPayload(success=success))

    @message_handler
    async def execution_changed(self, msg: NMessage[ExecutionChangedPayload]) -> None:
        save_success = await sync_to_async(save_execution_frames)(msg.payload.frames)
        if save_success:
            # forward to API clients now that DB frames are saved
            await publish(
                NMessageType.EXECUTION_SAVED,
                ExecutionSavedPayload(module_id=msg.p.module_id, frames=msg.p.frames),
            )

    @message_handler
    async def job_changed(self, msg: NMessage[JobChangedPayload]) -> None:
        save_success = await sync_to_async(save_jobs)(msg.payload.jobs)
        if save_success:
            # forward to API clients now that DB jobs are saved
            await publish(
                NMessageType.JOB_SAVED,
                JobSavedPayload(module_id=msg.p.module_id, jobs=msg.p.jobs),
            )

    @message_handler
    async def project_version_changed(self, msg: NMessage[ProjectVersionChangedPayload]) -> None:
        # reload project version as module
        # TODO @Performance: send partial module updates :PartialModuleUpdates
        if not any(is_semantic_mutation(mutation) for mutation in msg.p.mutations):
            return  # ignore non-semantic changes to modules
        project_v = await ProjectVersion.objects.filter(id=msg.p.project_version_id).afirst()
        if project_v is None:
            return  # just ignore, was probably deleted
        module = await sync_to_async(read_module)(project_v, exclude_non_semantic=True)
        await publish(
            NMessageType.MODULE_CHANGED, ModuleChangedPayload(module_id=module.id, module=module)
        )

    async def stop(self):
        logger.info("stop")
        await asyncio.gather(sub.unsubscribe() for sub in self.subs)


def save_execution_frames(frames: list[ExecutionFrameData]) -> bool:
    model_executions: list[Execution] = []
    for frame in frames:
        execution = mapper.rmap_execution_frame(frame)
        model_executions.append(execution)

    try:
        # upsert frames
        Execution.objects.bulk_create(
            model_executions,
            update_conflicts=True,
            unique_fields=["id"],
            update_fields=["status", "terminated_at", "outputs", "error"],
        )
        logger.debug("save_execution_frames", executions=model_executions)
        return True
    except Exception as e:
        logger.error("save_execution_frames_failed", exc_info=e, executions=model_executions)
        return False


def write_evaluation_results(evaluations: list[EvaluationResultData]) -> None:
    model_evaluations: list[models.EvaluationResult] = []
    for evaluation in evaluations:
        model_evaluation = models.EvaluationResult(
            kind=evaluation.kind,
            scope=evaluation.scope,
            project_id=evaluation.project_id,
            project_version_id=evaluation.project_version_id,
            job_id=evaluation.job_id,
            build_id=evaluation.build_id,
            build_candidate_id=evaluation.build_candidate_id,
            statement_id=evaluation.statement_id,
        )
        model_evaluations.append(model_evaluation)
    # insert (not upsert, should only be written once?)
    models.EvaluationResult.objects.bulk_create(model_evaluations)


def write_build_candidates(candidates: list[BuildCandidateData]) -> None:
    model_candidates: list[models.BuildCandidate] = []
    for candidate in candidates:
        model_candidate = models.BuildCandidate(
            id=candidate.id,
            build_id=candidate.build_id,
            status=candidate.status,
            name=candidate.name,
            evaluation_id=candidate.evaluation_id,
            job_id=candidate.job_id,
            file_id=candidate.file_id,
            project_id=candidate.project_id,
            project_version_id=candidate.project_version_id,
        )
        model_candidates.append(model_candidate)

    # upsert candidates
    models.BuildCandidate.objects.bulk_create(
        model_candidates,
        update_conflicts=True,
        unique_fields=["id"],
        update_fields=["status", "job_id", "evaluation_id", "file_id"],
    )


def save_jobs(jobs: list[JobData]) -> bool:
    model_jobs: list[Job] = []
    for job in jobs:
        model_job = mapper.rmap_job(job)
        model_jobs.append(model_job)

    try:
        # upsert jobs
        Job.objects.bulk_create(
            model_jobs,
            update_conflicts=True,
            unique_fields=["id"],
            update_fields=["status", "terminated_at"],
        )
        logger.debug("save_jobs", jobs=model_jobs)
        return True
    except Exception as e:
        logger.error("save_jobs_failed", exc_info=e, jobs=model_jobs)
        return False
