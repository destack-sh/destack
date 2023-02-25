import structlog
from django.core.management.base import BaseCommand
from django.db import transaction

from bench.models import Organization
from bench.models.organization import OrganizationMembership
from bench.models.user import User

TEST_USER_EMAIL = "yatima@symbolx.com"

logger = structlog.get_logger(__name__)


class Command(BaseCommand):
    help = "Sets up dev environment with sample data"

    @transaction.atomic
    def handle(self, *args, **options):
        organization = Organization.objects.get_by_slug("symbolx")
        # create test organization and user if they don't exist
        user = User.objects.filter(email=TEST_USER_EMAIL).first()
        if user is None:
            user = User.objects.create_user(
                email=TEST_USER_EMAIL,
                username="yatima",
                password="password",
                full_name="Yatima",
                is_staff=True,
                bot=True,
            )
            user.join_organization(organization, OrganizationMembership.Level.Owner)
            logger.info(f"Created bootstrap user: {user}")
        else:
            logger.info(f"Bootstrap user already exists: {user}")
