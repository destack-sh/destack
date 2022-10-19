import json
import random

from django.core.management.base import BaseCommand, CommandParser

from bench.dataset.db import DbDataset
from bench.management.commands.updatedataset import update_dataset


class Command(BaseCommand):
    help = "Generates questions for inverse scaling challenge"

    def add_arguments(self, parser: CommandParser) -> None:
        pass

    def handle(self, *args, **options) -> None:
        capitals_ds = update_dataset(
            "local", "inverse-scaling.capitals", "data/inverse-scaling/capitals.json"
        )
        capitals = DbDataset(capitals_ds.artifact.id, capitals_ds.version)

        template = """
{{ CAPITAL }} is now the capital of {{ COUNTRY }}.
Question: What is the primary language spoken in the capital of {{ COUNTRY }}?
Answer (name of the language):"""

        n_samples = 5
        random.seed(0)
        outputs = []
        while len(outputs) < n_samples:
            i = random.randint(0, len(capitals) - 1)
            # select two different random indices from capitals
            first_capital = capitals[i]
            new_capital = capitals[(i + 1) % len(capitals)]

            if first_capital["city_language"] == new_capital["city_language"]:
                continue

            rendered_text = template.replace("{{ CAPITAL }}", new_capital["city"])
            rendered_text = rendered_text.replace("{{ COUNTRY }}", first_capital["country"])

            outputs.append(
                {
                    "text": rendered_text,
                    "answers": [first_capital["city_language"], new_capital["city_language"]],
                    "answer_index": 1,
                }
            )

        print(json.dumps(outputs, indent=4))
