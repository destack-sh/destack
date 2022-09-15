from django.core.management.base import BaseCommand, CommandParser

from bench.models.user import User


class Command(BaseCommand):
    help = "Sets up dev environment with sample data"

    def add_arguments(self, parser: CommandParser):
        pass

    def handle(self, *args, **options):
        _, _, user = User.objects.bootstrap(
            email="test@symbolx.com",
            password="password",
            first_name="Yatima",
            organization_name="Localhost, inc.",
            organization_kwargs={"slug": "local"},
            is_staff=True,
        )
        self.stdout.write(self.style.SUCCESS(f"Created bootstrap user: {user}"))
