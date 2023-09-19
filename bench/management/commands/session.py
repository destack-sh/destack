import structlog
from django.core.management import BaseCommand, CommandParser
from django.db import transaction

logger = structlog.get_logger(__name__)


class Command(BaseCommand):
    help = "Edit, dump and load Bench modules"

    def add_arguments(self, parser: CommandParser) -> None:
        parser.add_argument("action", type=str, help="Action")
        # optional module path
        parser.add_argument("module", type=str, nargs="?", help="Module")

    @transaction.atomic
    def handle(
        self,
        action: str,
        module: str = None,
        **options,
    ):
        if module is not None:
            pass

        if action == "prune":
            raise NotImplementedError("not yet supported")
        else:
            raise ValueError(f"unknown action: {action}")
