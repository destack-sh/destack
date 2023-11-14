import structlog
from asgiref.sync import async_to_sync
from django.core.management.base import BaseCommand
from django.db import transaction

from bench import models
from bench.models import Project
from bench.server.storage import create_local_pg_database, update_pg_schema_from_db

logger = structlog.get_logger(__name__)


class Command(BaseCommand):
    help = "Manage user PG databases."

    def add_arguments(self, parser):
        parser.add_argument("action", type=str)
        # optional arguments
        parser.add_argument("slug", type=str, nargs="?")

    def get_project(self, slug: str) -> Project:
        owner, project_name = slug.split("/")
        project = Project.objects.get_by_slug(owner, project_name)
        for attr in ("user", "organization", "pg_username", "pg_password"):
            getattr(project, attr)
        return project

    @transaction.atomic
    def handle(self, action: str, slug: str | None, *args, **options):
        if slug == "all":
            projects = (
                models.Project.objects.select_related("user", "organization").defer(None).all()
            )
        elif slug:
            projects = [self.get_project(slug)]
        else:
            projects = []
        if action == "bootstrap":
            pass
        elif action == "create":
            for project in projects:
                async_to_sync(create_local_pg_database)(project, upsert=True)
        elif action == "schema":
            for project in projects:
                async_to_sync(update_pg_schema_from_db)(project)
        else:
            raise ValueError("Unknown action")
