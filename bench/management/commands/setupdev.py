from django.core.management.base import BaseCommand, CommandParser
from django.db import transaction

from bench.models.user import User


class Command(BaseCommand):
    help = "Sets up dev environment with sample data"

    def add_arguments(self, parser: CommandParser):
        pass

    @transaction.atomic
    def handle(self, *args, **options):
        organization, user = User.objects.bootstrap(
            email="test@symbolx.com",
            password="password",
            first_name="Yatima",
            organization_name="SymbolX AG.",
            organization_kwargs={"slug": "symbolx"},
            is_staff=True,
        )
        self.stdout.write(self.style.SUCCESS(f"Created bootstrap user: {user}"))
