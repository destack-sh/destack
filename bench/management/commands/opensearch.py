import structlog
from django.core.management.base import BaseCommand
from django.db import transaction

from bench import models
from bench.models import Project
from bench.opensearch.client import os_client
from bench.opensearch.core import IndexType
from bench.opensearch.index import create_bench_index

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
            if slug == "all":
                for project in models.Project.objects.all():
                    create_bench_index(project.id, upsert=True)
            else:
                project = self.get_project(slug)
                create_bench_index(project.id, upsert=True)
        elif action == "delete":
            if slug == "all":
                for project in models.Project.objects.all():
                    os_client.indices.delete(index=IndexType.BENCH.get_index_name(project.id))
            else:
                project = self.get_project(slug)
                os_client.indices.delete(index=IndexType.BENCH.get_index_name(project.id))
