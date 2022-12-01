# @import openai/stdlib
# @symbol ignore
import random
from typing import Callable

from bench.backend.base import ModelHandle
from bench.utils.record import RecordBatch

Model = ModelHandle
Dataset = RecordBatch
Instruction: Callable
llm: Callable
llm_fewshot: Callable
llm_classify: Callable
random: random.Random

# @path main

# @symbol instruct: get_temperature
model: Model  # @param
description: str  # @param
examples: Dataset  # @param


async def get_temperature(model: Model, description: str, examples: Dataset) -> float:
    # TODO @Feature: implement this nocheckin
    return 0.5
