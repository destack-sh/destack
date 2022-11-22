from asgiref.sync import async_to_sync
from rest_framework.decorators import api_view
from rest_framework.request import Request
from rest_framework.response import Response

from bench.executor import Executor
from bench.models import Project, Task

executor = Executor()


# TODO @Performance: execute run program endpoint non-blocking (async)
@api_view(["POST"])
def run_program(request: Request, organization: str, project: str) -> Response:
    project_instance = Project.objects.get(organization__slug=organization, slug=project)
    variables = request.GET

    # as in runprogram, just use the latest implementation of main head's program
    project_version = project_instance.head
    if project_version is None:
        raise ValueError(f"project has no head: {project_instance}")

    program: Task = project_version.program
    compiled_program = program.implementations.order_by("-created_at").first()

    try:
        output = async_to_sync(executor.run)(compiled_program, variables)
        return Response({"output": output})
    except Exception as e:
        return Response({"error": str(e)}, status=400)
