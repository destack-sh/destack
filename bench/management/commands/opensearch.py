import structlog
from django.core.management.base import BaseCommand
from django.db import transaction

from bench.models import Project
from bench.opensearch.index import create_bench_index

logger = structlog.get_logger(__name__)


class Command(BaseCommand):
    help = "Manage OS indices."

    def add_arguments(self, parser):
        parser.add_argument("slug", type=str)

    @transaction.atomic
    def handle(self, slug: str, *args, **options):
        owner, project_name = slug.split("/")
        project = Project.objects.get_by_slug(owner, project_name)
        create_bench_index(project.id)
