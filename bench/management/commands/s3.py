import structlog
from django.core.management.base import BaseCommand
from django.db import transaction

from bench.models.project import create_global_user_bucket

logger = structlog.get_logger(__name__)


class Command(BaseCommand):
    help = "Manage S3 buckets."

    def add_arguments(self, parser):
        parser.add_argument("action", type=str)

    @transaction.atomic
    def handle(self, action: str, *args, **options):
        if action == "create":
            create_global_user_bucket(ignore_exists=True)
