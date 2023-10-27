import asyncio
import json
import threading
from dataclasses import dataclass
from functools import wraps
from typing import Any
from uuid import UUID

import structlog
from asgiref.sync import sync_to_async
from django.core.exceptions import ObjectDoesNotExist, PermissionDenied, ValidationError
from django.http import HttpRequest, HttpResponse, JsonResponse
from nats.errors import NoRespondersError
from rest_framework import serializers
from rest_framework_dataclasses.serializers import DataclassSerializer

from bench import models
from bench.language import TriggerType
from bench.msg import NMessageType
from bench.msg.core import MessagingError, NMessage, request
from bench.msg.messages import RepStartRunPayload, ReqStartRunPayload, ReqWakeWorkerSetPayload

logger = structlog.get_logger(__name__)


def async_csrf_exempt(view_func):
    async def wrapped_view(request, *args, **kwargs):
        return await view_func(request, *args, **kwargs)

    wrapped_view.csrf_exempt = True
    return wraps(view_func)(wrapped_view)


def async_check_is_main_thread(view_func):
    async def wrapped_view(request, *args, **kwargs):
        if threading.current_thread().name != "django-main-thread":
            # someone fucked up
            logger.error(
                "rest.not_main",
                view_func=view_func,
                current_thread=threading.current_thread(),
                main_thread=threading.main_thread(),
            )
            return HttpResponse(status=500)
        return await view_func(request, *args, **kwargs)

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


@dataclass
class RunRequest:
    statement: str
    inputs: dict[str, Any]
    block: bool = True
    keyed: bool = False
    wait_if_sleeping = True


class RunRequestSerializer(DataclassSerializer):
    class Meta:
        dataclass = RunRequest
        fields = "__all__"


@dataclass
class RunResponse:
    id: UUID
    inputs: dict[str, Any]
    outputs: dict[str, Any]
    error: dict[str, Any]
    value: dict[str, Any]


class RunResponseSerializer(DataclassSerializer):
    class Meta:
        dataclass = RunResponse
        fields = "__all__"


@async_csrf_exempt
@async_check_is_main_thread
@async_api_view(methods=["POST"])
async def run(req: HttpRequest, owner: str, project: str) -> HttpResponse:
    # validate access
    access_token = req.headers.get("Authorization")
    access_token = access_token.split(" ")[1] if access_token else None
    if not access_token:
        return HttpResponse(status=401)
    access_token = await sync_to_async(models.AccessToken.objects.get_by_raw_token)(access_token)
    if not access_token:
        return HttpResponse(status=401)
    project = await sync_to_async(models.Project.objects.get_by_slug)(owner, project)
    if project.owner_id != access_token.owner_id:
        return HttpResponse(status=403)

    # parse request
    req = RunRequestSerializer(data=json.loads(req.body.decode()))
    if not req.is_valid():
        return JsonResponse(req.errors, safe=False, status=400)
    req = req.validated_data

    # do run
    statement = req.statement
    if not statement.startswith("."):
        statement = f".{statement}"
    start_req = ReqStartRunPayload(
        project_id=project.id,
        module_id=project.head_id,
        statement=statement,
        trigger_type=TriggerType.API,
        trigger_id=access_token.id,
        inputs=req.inputs,
        block=req.block,
        keyed=req.keyed,
        keyed_return=req.keyed,
    )
    retries = 3
    retry_delay = 5
    while retries >= 0:
        try:
            rep: NMessage[RepStartRunPayload] = await request(
                NMessageType.START_RUN,
                start_req,
                reply_t=RepStartRunPayload,
                timeout=30,
            )
            if not rep.p.run:
                # unknown statement
                return HttpResponse(status=404)

            response = RunResponse(
                id=rep.p.run_id,
                inputs=rep.p.run.inputs,
                outputs=rep.p.run.outputs,
                error=rep.p.run.error,
                value=rep.p.run.value,
            )
            logger.info("rest.run.done", response=response)
            return JsonResponse(RunResponseSerializer(response).data, safe=False)
        except MessagingError as e:
            logger.info("rest.run.failed", retries=retries, e=e)
            # retry on error (start workers if necessary)
            if isinstance(e.__cause__, NoRespondersError):
                _ = await request(
                    NMessageType.WAKE_WORKER_SET, ReqWakeWorkerSetPayload(project_id=project.id)
                )
            await asyncio.sleep(retry_delay)
            retries -= 1
            continue

    # give up
    logger.error("rest.run.failed", retries=retries, rep=rep)
    return HttpResponse(status=500)
