from typing import Any

from bench.model.base import ModelHandler
from bench.models import DatasetVersion

Model = ModelHandler
Dataset = DatasetVersion
benv: Any
# </preamble>

# @bench task: generate_command
# Description: Generate CLI command from natural language
# Schema: str -> str

model: Model = benv.get_model("model")
prompt: str = benv.get_argument("prompt")


async def generate_command(input: str) -> str:
    completion, logodds = await model.complete(prompt.format(input=input))
    return completion


# @bench examples: generate_command
generate_command_examples = [
    {
        "input": "list files in the current directory",
        "output": "ls -l",
    },
    {
        "input": "commit",
        "output": "git commit",
    },
]

# @bench define concept: destructive
