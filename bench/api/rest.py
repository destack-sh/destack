import zmq
from rest_framework import serializers
from rest_framework.decorators import api_view
from rest_framework.request import Request
from rest_framework.response import Response

from bench.models import Project
from bench.msg import ZMessageType, recv_message_with, send_message, zmq_ctx
from bench.msg.messages import RepModuleRunPayload, ReqModuleRunPayload
from bench.settings import ZMQ_WORKER_REP_ADDR


class RunInputSerializer(serializers.Serializer):
    version = serializers.CharField()  # project version tag
    symbol = serializers.CharField()
    build = serializers.CharField(allow_null=True)
    inputs = serializers.JSONField(allow_null=True)


class RunOutputSerializer(serializers.Serializer):
    execution_id = serializers.UUIDField()
    output = serializers.JSONField(allow_null=True)


@api_view(["POST"])
async def run(request: Request, owner: str, project: str) -> Response:
    serializer = RunInputSerializer(data=request.data)
    serializer.is_valid(raise_exception=True)
    data = serializer.validated_data
    worker_req_sock = zmq_ctx.socket(zmq.REQ)
    worker_req_sock.connect(ZMQ_WORKER_REP_ADDR)

    project_version = Project.objects.get_by_slug(owner, project)
    deployment = project_version.deployments.get(project_version___tag=data["version"])

    # :BlockingWorkerMessages
    send_message(
        worker_req_sock,
        ZMessageType.REQ_MODULE_RUN,
        ReqModuleRunPayload(
            module_id=project_version.id,
            runnable=data["symbol"],
            build=data["build"],
            inputs=data["inputs"],
        ),
    )
    _, rep = await recv_message_with(worker_req_sock, RepModuleRunPayload)

    return Response(RunOutputSerializer(rep).data)
