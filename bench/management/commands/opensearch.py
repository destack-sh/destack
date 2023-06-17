import structlog
from django.core.management.base import BaseCommand
from django.db import transaction

from bench.models import Project
from bench.opensearch.client import os_client
from bench.opensearch.index import create_bench_index
from bench.opensearch.type import IndexType

logger = structlog.get_logger(__name__)


class Command(BaseCommand):
    help = "Manage OS indices."

    def add_arguments(self, parser):
        parser.add_argument("action", type=str)
        parser.add_argument("slug", type=str)

    @transaction.atomic
    def handle(self, action: str, slug: str, *args, **options):
        owner, project_name = slug.split("/")
        project = Project.objects.get_by_slug(owner, project_name)

        if action == "create":
            create_bench_index(project.id, upsert=True)
        elif action == "delete":
            os_client.indices.delete(index=IndexType.BENCH.get_index_name(project.id))
