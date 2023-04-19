import structlog
from django.core.management.base import BaseCommand
from django.db import transaction

from bench.models.user import User

TEST_USER_EMAIL = "yatima@symbolx.com"

logger = structlog.get_logger(__name__)


class Command(BaseCommand):
    help = "Marks the given user as a staff member."

    def add_arguments(self, parser):
        parser.add_argument("username", type=str)

    @transaction.atomic
    def handle(self, username: str, *args, **options):
        User.objects.filter(username=username).update(is_staff=True)
