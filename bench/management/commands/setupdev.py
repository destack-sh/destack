from django.core.management.base import BaseCommand
from django.db import transaction

from bench.models import Organization
from bench.models.organization import OrganizationMembership
from bench.models.user import User

TEST_USER_EMAIL = "yatima@symbolx.com"


class Command(BaseCommand):
    help = "Sets up dev environment with sample data"

    @transaction.atomic
    def handle(self, *args, **options):
        organization = Organization.objects.get_by_slug("symbolx")
        # create test organization and user if they don't exist
        if not User.objects.filter(email=TEST_USER_EMAIL).exists():
            user = User.objects.create(
                email=TEST_USER_EMAIL,
                password="password",
                first_name="Yatima",
                is_staff=True,
            )
            user.join_organization(organization, OrganizationMembership.Level.Owner)
            self.stdout.write(self.style.SUCCESS(f"Created bootstrap user: {user}"))
