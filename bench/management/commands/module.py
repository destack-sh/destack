from uuid import uuid4

import structlog
from django.core.management import BaseCommand, CommandParser
from django.db import transaction

from bench import models
from bench.language.core import get_node_id

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
            try:
                file = module.files.get(name=path)
            except models.File.DoesNotExist:
                raise ValueError(
                    f"file '{path}' not found in {module}, others: {list(module.files.all().values_list('name', flat=True))}"
                )
            target_ck = uuid4()
            target_id = get_node_id(project.head_id, target_ck)
            models.File.objects.copy(
                file,
                source=module,
                target=module,
                target_id=target_id,
                target_ck=target_ck,
                keep_cks=False,
            )
            if action == "splice":
                file.delete()  # delete the original file
            logger.info(action, file=file, target_id=target_id)
        else:
            raise ValueError(f"unknown action: {action}")
