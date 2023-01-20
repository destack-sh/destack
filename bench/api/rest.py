from rest_framework.decorators import api_view
from rest_framework.request import Request
from rest_framework.response import Response


@api_view(["POST"])
async def run_program(request: Request, organization: str, project: str) -> Response:
    raise NotImplementedError
