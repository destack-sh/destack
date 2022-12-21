# @library openai/stdlib
# @define ignore
import random
from typing import Callable

from bench.backend.base import ModelHandle
from bench.utils.record import RecordBatch

Model = ModelHandle
Dataset = RecordBatch
Code: Callable
llm: Callable
llm_fewshot: Callable
llm_classify: Callable
random: random.Random

# @path temperature
# @import text-davinci-003 as model from openai.stdlib.text

# @define data: temperature_examples
temperature_examples = [
    {"text": "Write a poem about a space cat.", "temperature": "high"},
    {"text": "Repeat the following back to me: I am", "temperature": "low"},
    {"text": "Summarize the given news stories", "temperature": "medium"},
    {
        "text": "You are a helpful and truthful chat agent. If a user asks you a question, you should answer it truthfully. If you don't know, say so.",
        "temperature": "medium",
    },
    {"text": "You are a calculator", "temperature": "low"},
    {
        "text": "This is a conversation between a chatty and imaginative buddy and his friends.",
        "temperature": "high",
    },
    {
        "text": "Given an input command, write a program that implements it in Python",
        "temperature": "medium",
    },
]

# @define code: get_temperature


async def get_temperature(description: str) -> float:
    # ignore task examples for now
    temperature_map = {
        "high": 0.9,
        "medium": 0.5,
        "low": 0.0,
    }
    temperature_label = await llm_classify(
        model,
        text=description,
        examples=temperature_examples,
        prompt_prefix="Assess the temperature (that is, the desired degree of variation or creativity required)",
        label_key="temperature",
    )
    return temperature_map.get(temperature_label, 0.5)
