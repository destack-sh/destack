import structlog
from django.core.management.base import BaseCommand
from django.db import transaction

from bench import models
from bench.models import Project
from bench.search.client import os_client_sync
from bench.server.search import (
    create_global_search_index,
    create_global_search_role,
    create_local_search_index,
)

logger = structlog.get_logger(__name__)


class Command(BaseCommand):
    help = "Manage OS indices."

    def add_arguments(self, parser):
        parser.add_argument("action", type=str)
        # optional arguments
        parser.add_argument("slug", type=str)

    def get_project(self, slug: str) -> Project:
        owner, project_name = slug.split("/")
        project = Project.objects.get_by_slug(owner, project_name)
        return project

    @transaction.atomic
    def handle(self, action: str, slug: str, *args, **options):
        if action == "create":
            if slug == "-":
                # create_global_search_index(upsert=True)
                create_global_search_role(upsert=True)
            elif slug == "all":
                for project in models.Project.objects.all():
                    create_local_search_index(project, upsert=True)
            else:
                project = self.get_project(slug)
                create_local_search_index(project, upsert=True)
        elif action == "delete":
            if slug == "all":
                for project in models.Project.objects.all():
                    try:
                        logger.info("opensearch.delete", project=project)
                        os_client_sync.indices.delete(index=project.os_name)
                    except Exception as e:
                        logger.error("opensearch.delete.error", project=project, error=e)
            else:
                project = self.get_project(slug)
                logger.info("opensearch.delete", project=project)
                os_client_sync.indices.delete(index=project.os_name)
