import zmq
from rest_framework import serializers
from rest_framework.decorators import api_view
from rest_framework.request import Request
from rest_framework.response import Response

from bench.msg import zmq_ctx
from bench.settings import ZMQ_WORKER_REP_ADDR


class RunInputSerializer(serializers.Serializer):
    version = serializers.CharField()
    symbol = serializers.CharField()
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

    raise NotImplementedError
