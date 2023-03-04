import json
from datetime import datetime
from functools import wraps

import structlog
import zmq
from asgiref.sync import sync_to_async
from django.core.exceptions import ObjectDoesNotExist, PermissionDenied, ValidationError
from django.db.models import Q
from django.http import HttpRequest, HttpResponse, JsonResponse
from rest_framework import serializers

from bench.models import Deployment, Project, ProjectVersion
from bench.models.token import AccessTokenScope, digest_raw_token
from bench.msg import ZMessageType, recv_message_with, send_message, zmq_ctx
from bench.msg.messages import RepModuleRunPayload, ReqModuleRunPayload
from bench.settings import ZMQ_WORKER_REP_ADDR

logger = structlog.get_logger(__name__)


class RunInputSerializer(serializers.Serializer):
    version = serializers.CharField()  # project version tag
    task = serializers.CharField(default=None, allow_null=True)
    code = serializers.CharField(default=None, allow_null=True)
    build = serializers.CharField(default=None, allow_null=True)
    inputs = serializers.JSONField(default=None, allow_null=True)


class RunOutputSerializer(serializers.Serializer):
    execution_id = serializers.UUIDField()
    output = serializers.JSONField(allow_null=True)
    success = serializers.BooleanField()
    error = serializers.JSONField(allow_null=True)


def csrf_exempt_async(view_func):
    async def wrapped_view(request, *args, **kwargs):
        return await view_func(request, *args, **kwargs)

    wrapped_view.csrf_exempt = True
    return wraps(view_func)(wrapped_view)


def async_api_view(methods: list[str] = None):
    """DRF's api view does not support async, so we make our own."""

    def make_view(view_func):
        async def wrapped_view(request, *args, **kwargs):
            if methods is not None and request.method not in methods:
                return HttpResponse(status=405)
            try:
                return await view_func(request, *args, **kwargs)
            except (serializers.ValidationError, ValidationError) as e:
                return JsonResponse(e.detail, safe=False, status=400)
            except PermissionDenied:
                return HttpResponse(status=403)
            except ObjectDoesNotExist:
                return HttpResponse(status=404)

        wrapped_view.methods = methods
        return wraps(view_func)(wrapped_view)

    return make_view


def get_deployment(
    owner: str, project: str, tag: str, token_digest: str, scope=AccessTokenScope.RUN
) -> tuple[ProjectVersion, Deployment]:
    # TODO @Feature: implement semver range tags? https://devhints.io/semver
    # TODO @Performance: cache get_deployment
    # TODO @Performance: do get_deployment in one SQL query (incl. access token check)
    if tag in ("*", "^", "x"):
        # use latest version
        project_version = Project.objects.get_by_slug(owner, project).head
    else:
        project_version = ProjectVersion.objects.get_by_slug(owner, project, tag=tag)
    if (
        not project_version.project.owner.access_tokens.filter(
            revoked_at__isnull=True,
            digest=token_digest,
            scopes__contains=[scope],
        )
        .filter(Q(expires_at__gte=datetime.utcnow()) | Q(expires_at__isnull=True))
        .exists()
    ):
        raise PermissionDenied("cannot access this deployment")
    deployment = project_version.deployments.get(owned=True)  # should only be one for now
    return project_version.id, deployment.id


@csrf_exempt_async
@async_api_view(methods=["POST"])
async def run(request: HttpRequest, owner: str, project: str) -> HttpResponse:
    try:
        data = json.loads(request.body)
        # map input__key to inputs[key]
        data["inputs"] = data.get("inputs", {})
        data["inputs"].update(
            {k.split("__", 1)[1]: v for k, v in data.items() if k.startswith("input__")}
        )
        serializer = RunInputSerializer(data=data)
    except json.JSONDecodeError:
        return HttpResponse("Invalid JSON", status=400)
    serializer.is_valid(raise_exception=True)
    data = serializer.validated_data
    if data.get("task") is not None:
        runnable = data["task"]
        runnable_type = "task"
        if data["build"] is None:
            raise serializers.ValidationError("Build must be set if task is set")
    elif data.get("code") is not None:
        runnable = data["code"]
        runnable_type = "code"
    else:
        raise serializers.ValidationError("Either task or code must be set")

    access_token = request.headers.get("Authorization", "").split(" ", 1)[-1]
    if not access_token:
        raise PermissionDenied("no access token provided")
    token_digest = digest_raw_token(access_token)
    del access_token

    project_version_id, deployment_id = await sync_to_async(get_deployment)(
        owner=owner, project=project, tag=data["version"], token_digest=token_digest
    )

    worker_req_sock = zmq_ctx.socket(zmq.REQ)
    worker_req_sock.connect(ZMQ_WORKER_REP_ADDR)
    # :BlockingWorkerMessages
    send_message(
        worker_req_sock,
        ZMessageType.REQ_MODULE_RUN,
        ReqModuleRunPayload(
            deployment_id=deployment_id,
            module_id=project_version_id,
            runnable=runnable,
            runnable_type=runnable_type,
            build=data["build"],
            arguments=data["inputs"],
            blocking=True,
        ),
    )
    _, rep = await recv_message_with(worker_req_sock, RepModuleRunPayload)

    output = dict(
        execution_id=rep.execution_id,
        output=rep.output,
        success=not rep.error,
        error=dict(type=rep.error.value, details=rep.error_details) if rep.error else None,
    )
    return JsonResponse(output, status=200)
