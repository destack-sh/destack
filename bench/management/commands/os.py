import structlog
from django.core.management.base import BaseCommand
from django.db import transaction

from bench import models
from bench.models import Project
from bench.search.client import os_client_sync
from bench.server.search import (
    create_global_os_index,
    create_global_os_role,
    create_local_os_index,
    enable_os_strict_mapping,
    update_os_schema_from_db,
    write_module_to_os_from_db,
    write_sessions_to_os_from_db,
)

logger = structlog.get_logger(__name__)


class Command(BaseCommand):
    help = "Manage OS indices."

    def add_arguments(self, parser):
        parser.add_argument("action", type=str)
        # optional arguments
        parser.add_argument("slug", type=str, nargs="?")

    def get_project(self, slug: str) -> Project:
        owner, project_name = slug.split("/")
        project = Project.objects.get_by_slug(owner, project_name)
        return project

    @transaction.atomic
    def handle(self, action: str, slug: str | None, *args, **options):
        if slug == "all":
            projects = models.Project.objects.all()
        elif slug:
            projects = [self.get_project(slug)]
        else:
            projects = []
        if action == "bootstrap":
            create_global_os_index(upsert=True)
            create_global_os_role()
        elif action == "create":
            for project in projects:
                create_local_os_index(project, upsert=True)
        elif action == "delete":
            for project in projects:
                logger.info("opensearch.delete", project=project)
                os_client_sync.indices.delete(index=project.os_name)
        elif action == "index":
            for project in projects:
                # ignore fields not in mapping during reindex
                #  (fields may have existed in between snapshots)
                for project_v in project.versions.all():
                    update_os_schema_from_db(project_v, dynamic="false")
                    write_module_to_os_from_db(project_v, wipe=True, update_mappings=False)
                    write_sessions_to_os_from_db(project_v)
                enable_os_strict_mapping(project.os_name)
        else:
            raise ValueError("Unknown action")
