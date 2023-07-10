import structlog
from django.core.management.base import BaseCommand
from django.db import transaction

from bench.models.user import User, UserStatus

logger = structlog.get_logger(__name__)


class Command(BaseCommand):
    help = "Set the status of the given users"

    def add_arguments(self, parser):
        parser.add_argument("action", type=str, choices=["activate", "waitlist", "suspend"])
        parser.add_argument("user", type=str, nargs="+")

    @transaction.atomic
    def handle(self, action: str, user: list[str], *args, **options):
        if action == "activate":
            target_status = UserStatus.ACTIVE
        elif action == "waitlist":
            target_status = UserStatus.WAITLISTED
        elif action == "suspend":
            target_status = UserStatus.SUSPENDED
        else:
            raise ValueError(f"unexpected action {action}")
        # find user by username or email
        for u in user:
            if "@" in u:
                u = User.objects.get(email=u)
            else:
                u = User.objects.get(username=u)
            u.status = target_status
            u.save()
            logger.info("user.status", user=u, status=target_status)
