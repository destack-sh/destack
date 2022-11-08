from django.core.management import BaseCommand
from django.core.management.base import CommandParser
from django.db import transaction


class Command(BaseCommand):
    help = "Sets up dev environment with sample data"

    def add_arguments(self, parser: CommandParser):
        # task file path (must exist and end in .py)
        parser.add_argument("task", type=str)
        # models to consider
        parser.add_argument("--models", type=str, nargs="+", default=None)
        # target to optimize for (level and priority: latency, cost, accuracy, robustness)
        parser.add_argument("--optimize", type=str, default=None)

    @transaction.atomic
    def handle(self, *args, **options):
        raise NotImplementedError
