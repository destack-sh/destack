import structlog
from django.core.management import BaseCommand, CommandParser
from django.db import transaction

from bench import models

logger = structlog.get_logger(__name__)


class Command(BaseCommand):
    help = "Edit, load and dump Bench modules"

    def add_arguments(self, parser: CommandParser) -> None:
        parser.add_argument("action", type=str, help="Action")
        parser.add_argument("module", type=str, help="Module")
        # optional path
        parser.add_argument("path", type=str, nargs="?", help="Path")

    @transaction.atomic
    def handle(self, module: str, action: str, path: str = None, **options):
        owner_slug, project_slug = module.split("/")
        project = models.Project.objects.get_by_slug(owner_slug, project_slug)
        module = project.head

        if action in ("paste", "splice"):
            if path == "-":
                files = list(module.files.all())
            else:
                try:
                    files = [module.files.get(name=path)]
                except models.File.DoesNotExist:
                    raise ValueError(
                        f"file '{path}' not found in {module}, others: {list(module.files.all().values_list('name', flat=True))}"
                    )
            models.ProjectVersion.objects.copy(
                source=module, target=module, files=files, keep_cks=False, include_interp=False
            )
            if action == "splice":
                # delete the original files
                models.File.objects.filter(id__in=[f.id for f in files]).delete()
            logger.info(action, files=files)
        else:
            raise ValueError(f"unknown action: {action}")
