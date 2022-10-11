from django.core.management import BaseCommand
from django.core.management.base import CommandParser
from django.db import transaction

from bench.models import Flow, Organization
from bench.models.versioning import get_head


class Command(BaseCommand):
    help = "Runs a flow with the given inputs"

    def add_arguments(self, parser: CommandParser):
        parser.add_argument("name", type=str)
        parser.add_argument("--path", type=str, required=True)
        parser.add_argument("--organization", type=str, default="local")

    @transaction.atomic
    def handle(self, *args, **options):
        organization = Organization.objects.get(slug=options["organization"])
        flow = Flow.objects.get(organization=organization, name=options["name"])
        flow_version = get_head(flow)

        self.stdout.write(self.style.NOTICE(f"Running flow {flow_version}"))

        self.stdout.write(self.style.NOTICE(f"Completed flow {flow_version}"))
