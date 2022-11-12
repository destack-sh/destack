# --
# @bench task: generate_command
# Compiled with:
#  bench compile bench/demo/hmm_simple.py
#   --task generate_command
#   --models openai/code-davinci-002,openai/code-cushman-002
#   --optimize accuracy
# --

# @bench ignore
from typing import Any

from bench.model.base import ModelHandler
from bench.models import Dataset

Model = ModelHandler
benv: Any = {}
# @/bench


# @bench instruct: generate_command
model: Model = benv.get_model("openai/code-davinci-002")
prompt = """
# Translate the natural language comm and into a CLI command.

# do commit
git commit

# commit as "hello world"
git commit -m "hello world"

# {input}
 
"""


async def generate_command(input: str) -> str:
    completion, _ = await model.complete(prompt.format(input=input))
    return completion
    # @/bench
