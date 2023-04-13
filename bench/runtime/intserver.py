import asyncio
from datetime import datetime, timedelta
from uuid import UUID

import pytz
import structlog
from asgiref.sync import sync_to_async

from bench import models
from bench.models import (
    Execution,
    ExecutionStatus,
    Job,
    ProjectVersion,
    Worker,
    WorkerStatus,
    mapper,
)
from bench.models.execution import PENDING_EXECUTION_STATUSES
from bench.models.job import PENDING_JOB_STATUSES, JobStatus
from bench.models.mapper import read_module, write_module
from bench.msg import NMessage
from bench.msg.core import handle_reply, message_handler, nc_init, publish, subscribe
from bench.msg.messages import (
    EvaluationSavedPayload,
    ExecutionChangedPayload,
    ExecutionSavedPayload,
    JobSavedPayload,
    ModuleChangedPayload,
    NMessageType,
    ProjectVersionChangedPayload,
    RepReadModulePayload,
    RepRegisterWorkerPayload,
    RepWriteBuildCandidatePayload,
    RepWriteBuildPayload,
    RepWriteEvaluationPayload,
    RepWriteJobPayload,
    RepWriteModulePayload,
    ReqReadModulePayload,
    ReqRegisterWorkerPayload,
    ReqWriteBuildCandidatePayload,
    ReqWriteBuildPayload,
    ReqWriteEvaluationPayload,
    ReqWriteJobPayload,
    ReqWriteModulePayload,
    WorkerHeartbeatPayload,
)
from bench.msg.sync import is_semantic_mutation
from bench.runtime.type import BuildCandidateData, EvaluationResultData, ExecutionFrameData, JobData
from bench.utils.cache import redis
from bench.utils.func import wrap_task
from bench.utils.utils import sentry_capture_if_enabled

logger = structlog.get_logger(__name__)


def create_wrapped_task(coro, task_id: str = None):
    asyncio.create_task(wrap_task(coro, task_id))


WORKER_HEARTBEAT_TIMEOUT = 30


class InternalServer:
    """
    Server-side Bench language server for reading and writing modules in DB.
    Can be thought of as a sidecar to the main API server for internal operations.
    Also manages worker lifecycle (for now?).
    """

    def __init__(self):
        self.subs = []
        self.tasks = []

    async def run(self):
        await nc_init.wait()
        logger.info("start")
        self.subs = [
            await handle_reply(NMessageType.REQUEST_REGISTER_WORKER, self.register_worker),
            await subscribe(NMessageType.WORKER_HEARTBEAT, cb=self.worker_heartbeat),
            await handle_reply(NMessageType.REQUEST_READ_MODULE, self.read_module),
            await handle_reply(NMessageType.REQUEST_WRITE_MODULE, self.write_module),
            await handle_reply(
                NMessageType.REQUEST_WRITE_BUILD_CANDIDATE, self.write_build_candidate
            ),
            await handle_reply(NMessageType.REQUEST_WRITE_EVALUATION, self.write_evaluation),
            await handle_reply(NMessageType.REQUEST_WRITE_BUILD, self.write_build),
            await handle_reply(NMessageType.REQUEST_WRITE_JOB, self.write_job),
            await subscribe(f"{NMessageType.EXECUTION_CHANGED}.*", cb=self.execution_changed),
            await subscribe(
                f"{NMessageType.PROJECT_VERSION_CHANGED}.*", cb=self.project_version_changed
            ),
        ]
        self.tasks = [create_wrapped_task(self.manage_workers(interval_seconds=10))]

    @message_handler
    async def register_worker(self, msg: NMessage[ReqRegisterWorkerPayload]) -> None:
        try:
            worker = await Worker.objects.acreate(
                id=msg.payload.worker_id,
                status=WorkerStatus.ACTIVE,
                deployment_id=msg.p.deployment_id,
                project_id=msg.p.project_id,
                type=msg.p.type,
                started_at=datetime.utcnow().replace(tzinfo=pytz.utc),
            )
            success = True
            logger.info("register_worker", worker=worker)
        except Exception as e:
            sentry_capture_if_enabled(e)
            logger.error("register_worker.failed", msg=msg, exc_info=True)
            success = False
        await msg.reply(RepRegisterWorkerPayload(success=success))

    @message_handler
    async def worker_heartbeat(self, msg: NMessage[WorkerHeartbeatPayload]) -> None:
        last_seen = datetime.utcnow().replace(tzinfo=pytz.utc)
        await redis.set(
            f"worker.{msg.payload.worker_id}.heartbeat", str(last_seen), ex=WORKER_HEARTBEAT_TIMEOUT
        )

    async def manage_workers(self, interval_seconds: int):
        while True:
            # get last seen for all workers
            live_worker_keys = [
                worker_id async for worker_id in redis.scan_iter("worker.*.heartbeat")
            ]
            live_worker_ids = [UUID(key.decode().split(".")[1]) for key in live_worker_keys]
            last_seen = await redis.mget(keys=live_worker_keys)
            if len(live_worker_ids) != len(last_seen):
                continue  # try again?
            last_seen = [datetime.fromisoformat(ts.decode()) for ts in last_seen]

            # batch update last seen for live workers
            live_workers = [w async for w in Worker.objects.filter(id__in=live_worker_ids)]
            for ls, worker in zip(last_seen, live_workers):
                worker.last_seen_at = ls
            await Worker.objects.abulk_update(live_workers, ["last_seen_at"])

            # check if there are any dead workers
            liveness_cutoff = datetime.utcnow().replace(tzinfo=pytz.utc) - timedelta(
                seconds=WORKER_HEARTBEAT_TIMEOUT
            )
            dead_workers = [
                worker
                async for worker in Worker.objects.filter(
                    status=WorkerStatus.ACTIVE, last_seen_at__lt=liveness_cutoff
                )
            ]
            logger.debug("manage_workers", live_workers=live_workers, dead_workers=dead_workers)

            if dead_workers:
                # mark all relevant jobs and executions as failed
                dead_ids = [worker.id for worker in dead_workers]
                await Execution.objects.filter(
                    status__in=PENDING_EXECUTION_STATUSES, worker_id__in=dead_ids
                ).aupdate(status=ExecutionStatus.Failed)
                dead_jobs = [
                    job
                    async for job in Job.objects.filter(
                        status__in=PENDING_JOB_STATUSES, worker_id__in=dead_ids
                    )
                ]
                # publish job updates, then update
                for job in dead_jobs:
                    job_data = mapper.wmap_job(job)
                    job.status = JobStatus.Failed
                    await publish(
                        NMessageType.JOB_SAVED,
                        JobSavedPayload(job=job_data, module_id=job.project_version_id),
                    )
                await Job.objects.abulk_update(dead_jobs, ["status"])
                for worker in dead_workers:
                    worker.status = WorkerStatus.TERMINATED
                    worker.terminated_at = datetime.utcnow().replace(tzinfo=pytz.utc)
                await Worker.objects.abulk_update(dead_workers, ["status", "terminated_at"])

            await asyncio.sleep(interval_seconds)

    @message_handler
    async def read_module(self, msg: NMessage[ReqReadModulePayload]) -> None:
        project_v = await ProjectVersion.objects.aget(id=msg.payload.module_id)
        module = await sync_to_async(read_module)(project_v, exclude_non_semantic=True)
        await msg.reply(RepReadModulePayload(module=module, project_id=project_v.project_id))

    @message_handler
    async def write_module(self, msg: NMessage[ReqWriteModulePayload]) -> None:
        # TODO @Architecture @Cleanup: intserver.write_module == write_build?
        logger.info("module.write", files=msg.payload.files, module_id=msg.payload.module_id)
        project_v = await ProjectVersion.objects.aget(id=msg.payload.module_id)
        try:
            if project_v.committed:
                raise ValueError(f"cannot write to committed {project_v.id}")
            await sync_to_async(write_module)(
                files=msg.payload.files,
                generated_mappings=msg.payload.generated_mappings,
                project_v=project_v,
                overwrite=True,
            )
            success = True
        except Exception as e:
            sentry_enabled = sentry_capture_if_enabled(e)
            logger.error("module.write.failed", exc_info=e, sentry_enabled=sentry_enabled)
            success = False
        await msg.reply(RepWriteModulePayload(success=success))

        # republish entire module  :PartialModuleUpdates  :ImmediateModuleWrites
        # the worker should probably just do this directly
        module = await sync_to_async(read_module)(project_v, exclude_non_semantic=True)
        await publish(
            NMessageType.MODULE_CHANGED, ModuleChangedPayload(module_id=module.id, module=module)
        )

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
                delete_files=msg.payload.delete_files,
            )
            success = True
        except Exception as e:
            sentry_enabled = sentry_capture_if_enabled(e)
            logger.error("module.write.failed", exc_info=e, sentry_enabled=sentry_enabled)
            success = False
        await msg.reply(RepWriteBuildPayload(success=success))

        # republish entire module  :PartialModuleUpdates  :ImmediateModuleWrites
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
            candidates=[c.id for c in msg.payload.build_candidates],
        )
        project_v = await ProjectVersion.objects.aget(id=msg.payload.module_id)
        try:
            if project_v.committed:
                raise ValueError(f"cannot write to committed {project_v}")
            await sync_to_async(write_build_candidates)(
                candidates=msg.payload.build_candidates, delete_others=msg.payload.delete_others
            )
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
            evaluations=len(msg.p.evaluations),
        )
        try:
            await sync_to_async(write_evaluation_results)(evaluations=msg.p.evaluations)
            success = True
        except Exception as e:
            sentry_enabled = sentry_capture_if_enabled(e)
            logger.error(
                "module.write_evaluation.failed", exc_info=e, sentry_enabled=sentry_enabled
            )
            success = False
        await msg.reply(RepWriteEvaluationPayload(success=success))
        if success:
            await publish(
                NMessageType.EVALUATION_SAVED,
                EvaluationSavedPayload(module_id=msg.p.module_id, evaluations=msg.p.evaluations),
            )

    @message_handler
    async def write_job(self, msg: NMessage[ReqWriteJobPayload]) -> None:
        save_success = await sync_to_async(save_jobs)([msg.payload.job])
        await msg.reply(RepWriteJobPayload(success=save_success))
        if save_success:
            # forward to API clients now that DB jobs are saved
            await publish(
                NMessageType.JOB_SAVED,
                JobSavedPayload(module_id=msg.p.module_id, job=msg.p.job),
            )

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
            update_fields=[
                "status",
                "terminated_at",
                "cached_generated_at",
                "cached_duration",
                "outputs",
                "error",
            ],
        )
        return True
    except Exception as e:
        logger.error("save_execution_frames_failed", exc_info=e, executions=model_executions)
        return False


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
        return True
    except Exception as e:
        logger.error("save_jobs_failed", exc_info=e, jobs=model_jobs)
        return False


def write_evaluation_results(evaluations: list[EvaluationResultData]) -> None:
    model_evaluations: list[models.EvaluationResult] = [
        mapper.rmap_evaluation_result(evaluation) for evaluation in evaluations
    ]
    # upsert evaluations (by environment & system)
    models.EvaluationResult.objects.bulk_create(
        model_evaluations,
        update_conflicts=True,
        unique_fields=["id"],
        update_fields=["updated_at", "job_id", "self_metrics", "aggregated_metrics"],
    )


def write_build_candidates(candidates: list[BuildCandidateData], delete_others: bool) -> None:
    if delete_others:
        # delete candidates associated with builds not in the list
        build_ids = {c.build_id for c in candidates}
        candidates_ids = {c.id for c in candidates}
        for build_candidate in models.BuildCandidate.objects.filter(build_id__in=build_ids).exclude(
            id__in=candidates_ids
        ):
            build_candidate.delete()

    model_candidates: list[models.BuildCandidate] = [
        mapper.rmap_build_candidate(candidate) for candidate in candidates
    ]
    # upsert candidates (by id)
    models.BuildCandidate.objects.bulk_create(
        model_candidates,
        update_conflicts=True,
        unique_fields=["id"],
        update_fields=["updated_at", "status", "job_id", "evaluation_id", "file_id", "order_key"],
    )
