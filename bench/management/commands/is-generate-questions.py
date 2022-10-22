import json
import random

from django.core.management.base import BaseCommand, CommandParser

from bench.dataset.db import DbDataset
from bench.management.commands.updatedataset import update_dataset
from bench.utils.record import RecordBatch


class Command(BaseCommand):
    help = "Generates questions for inverse scaling challenge"

    def add_arguments(self, parser: CommandParser) -> None:
        # add named argument for task
        parser.add_argument(
            "task",
            type=str,
            help="Task to generate questions for",
            choices=["anchoring", "capitals-pop", "capitals-lang", "syntactic"],
        )

    def handle(self, *args, **options) -> None:
        task = options["task"]
        templates_ds = update_dataset(
            "local",
            f"inverse-scaling.{task}-templates",
            f"data/inverse-scaling/{task}-templates.json",
        )
        templates = DbDataset(templates_ds.artifact.id, templates_ds.version)

        n_samples = 50
        random.seed(0)
        if task.startswith("capitals"):
            questions = self._generate_capitals_questions(task, templates, n_samples)
        elif task.startswith("anchoring"):
            questions = self._generate_anchoring_questions(task, templates, n_samples)
        elif task.startswith("syntactic"):
            questions = self._generate_syntactic_questions(task, templates, n_samples)
        else:
            raise ValueError(f"Unknown task: {task}")

        update_dataset("local", f"inverse-scaling.{task}", records=questions)

        # INSUFFICIENT DATA FOR A MEANINGFUL ANSWER
        print(json.dumps(questions, indent=4))

        # write questions as csv to file
        questions_columns = questions[0].keys()
        with open(f"data/inverse-scaling/{task}.csv", "w") as file:
            file.write(f"{','.join(questions_columns)}\n")
            for question in questions:
                column_values = [question[col] for col in questions_columns]
                # escape non-numbers with "
                column_values = [
                    f'"{col}"' if not isinstance(col, int) else str(col) for col in column_values
                ]
                file.write(f"{','.join(column_values)}\n")

    def _generate_syntactic_questions(self, task: str, templates: RecordBatch, n_samples: int):
        template = """Q: {{ Q1_QUESTION }}?
{{ FIRST_OPTION }}: {{ Q1_FIRST_VALUE }}
{{ SECOND_OPTION }}: {{ Q1_SECOND_VALUE }}
Answer ({{ FIRST_OPTION }} or {{ SECOND_OPTION }}): {{ Q1_ANSWER }}

Q: {{ Q2_QUESTION }}?
{{ FIRST_OPTION }}: {{ Q2_FIRST_VALUE }}
{{ SECOND_OPTION }}: {{ Q2_SECOND_VALUE }}
Answer ({{ SECOND_OPTION }} or {{ FIRST_OPTION }}):"""
        adjectives = ["reasonable", "fantastic", "bad", "yellow"]

        questions = []
        while len(questions) < n_samples:
            options = [("I", "II")]
            first_option, second_option = random.choice(options)

            # subtask = random.choice("smaller-number", "quick-maths")
            subtask = "smaller-number"

            if subtask == "smaller-number":
                # random adjective
                adjective = random.choice(adjectives)
                q1_question = q2_question = f"What is a {adjective} number"

                # correct answer is smaller number for first question
                q1_first_value = random.randint(20, 80)
                q1_second_value = q1_first_value + random.randint(5, 30)
                q1_answer = first_option

                # correct answer is still smaller number but order of options is reversed
                q2_first_value = random.randint(20, 80)
                q2_second_value = q2_first_value + random.randint(1, 10)
                q2_answer = first_option
            elif subtask == "quick-maths":
                # task is multiplying two five-digit numbers
                digits = 1
                base = 10 ** (digits - 1)
                lbase, hbase = 3 * base, 10 * base

                q1_a, q1_b = random.randint(lbase, hbase), random.randint(lbase, hbase)
                q1_question = f"What is {q1_a}*{q1_b}"
                q1_first_value = q1_a * q1_b
                q1_second_value = (q1_a + random.randint(2, 4) * digits) * q1_b
                q1_answer = first_option

                q2_a, q2_b = random.randint(lbase, hbase), random.randint(lbase, hbase)
                q2_question = f"What is {q2_a}*{q2_b}"
                q2_first_value = q2_a * q2_b
                q2_second_value = (q2_a + random.randint(2, 4) * digits) * q2_b
                q2_answer = first_option

                if random.random() < 0.5:
                    # swap first and second value
                    q1_first_value, q1_second_value = q1_second_value, q1_first_value
                    q2_first_value, q2_second_value = q2_second_value, q2_first_value
                    q1_answer = q2_answer = second_option
            else:
                raise ValueError(f"Unknown task: {subtask}")

            rendered_text = template
            rendered_text = rendered_text.replace("{{ Q1_QUESTION }}", q1_question)
            rendered_text = rendered_text.replace("{{ Q2_QUESTION }}", q2_question)
            rendered_text = rendered_text.replace("{{ FIRST_OPTION }}", first_option)
            rendered_text = rendered_text.replace("{{ SECOND_OPTION }}", second_option)
            rendered_text = rendered_text.replace("{{ Q1_FIRST_VALUE }}", str(q1_first_value))
            rendered_text = rendered_text.replace("{{ Q1_SECOND_VALUE }}", str(q1_second_value))
            rendered_text = rendered_text.replace("{{ Q1_ANSWER }}", q1_answer)
            rendered_text = rendered_text.replace("{{ Q2_FIRST_VALUE }}", str(q2_first_value))
            rendered_text = rendered_text.replace("{{ Q2_SECOND_VALUE }}", str(q2_second_value))

            classes = [" " + first_option, " " + second_option]
            answer_index = 0 if q2_answer == first_option else 1
            questions.append(
                {"prompt": rendered_text, "classes": classes, "answer_index": answer_index}
            )

        return questions

    def _generate_anchoring_questions(self, task: str, templates: RecordBatch, n_samples: int):
        questions = []
        while len(questions) < n_samples:
            template = random.choice(templates["template"])

            anchor_price = random.randint(20, 40) * 10
            first_price = anchor_price - random.randint(3, 6) * 10
            second_price = random.randint(15, 40)

            # swap randomly
            if random.random() > 0.5:
                first_price, second_price = second_price, first_price

            rendered_text = template
            rendered_text = rendered_text.replace("{{ ANCHOR_PRICE }}", f"${anchor_price}")
            rendered_text = rendered_text.replace("{{ FIRST_PRICE }}", f"${first_price}")
            rendered_text = rendered_text.replace("{{ SECOND_PRICE }}", f"${second_price}")

            classes = [" A", " B"]
            answer_index = 0 if first_price < second_price else 1
            questions.append(
                {
                    "prompt": rendered_text,
                    "classes": classes,
                    "answer_index": answer_index,
                }
            )
        return questions

    def _generate_capitals_questions(self, task: str, templates: RecordBatch, n_samples: int):
        capitals_ds = update_dataset(
            "local", "inverse-scaling.capitals", "data/inverse-scaling/capitals.json"
        )
        capitals = DbDataset(capitals_ds.artifact.id, capitals_ds.version)

        questions = []
        while len(questions) < n_samples:
            i = random.randint(0, len(capitals) - 1)
            # select two different random indices from capitals
            first_capital = capitals[i]
            new_capital = capitals[(i + 1) % len(capitals)]

            if first_capital["city_language"] == new_capital["city_language"]:
                continue
            population_diff = abs(new_capital["city_population"] - first_capital["city_population"])
            if population_diff < 200000:
                continue
            mean_population = (
                new_capital["city_population"] + first_capital["city_population"]
            ) // 2

            for template in templates["template"]:
                rendered_text = template.replace("{{ CAPITAL }}", new_capital["city"])
                rendered_text = rendered_text.replace("{{ COUNTRY }}", first_capital["country"])
                rendered_text = rendered_text.replace("{{ POPULATION }}", str(mean_population))

                if task == "capitals-pop-q":
                    classes = [" Yes", " No"]
                    answer_index = 0 if new_capital["city_population"] > mean_population else 1
                elif task == "capitals-lang-q":
                    classes = [
                        " " + first_capital["city_language"],
                        " " + new_capital["city_language"],
                    ]
                    answer_index = 1
                else:
                    raise ValueError(f"Unexpected capitals task: {task}")
                questions.append(
                    {
                        "prompt": rendered_text,
                        "classes": classes,
                        "answer_index": answer_index,
                    }
                )
        return questions
