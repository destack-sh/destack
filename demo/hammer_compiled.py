from bench.model.base import ModelHandler

Model = ModelHandler

# </preamble>

# Task: lang-to-cmd
# Name:

model: Model = bench.get_model("model")


def generate_command(input: str) -> str:
    return "echo " + input
