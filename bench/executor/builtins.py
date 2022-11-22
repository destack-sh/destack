from typing import Any

from bench.backend.base import ModelHandle
from bench.utils.record import RecordBatch

# Builtins are only called with trusted strings or in a sandbox.


# noinspection StrFormat
async def llm(
    model: ModelHandle,
    prompt: str,
    return_logprobs: bool = False,
    strip: bool = True,
    **variables: dict[str, Any],
):
    """
    Complete a zero-shot LLM prompt
    """
    if variables:
        prompt = prompt.format(**variables)

    completion, logprobs = await model.complete(prompt)
    if strip:
        completion = completion.strip()
    if return_logprobs:
        return completion, logprobs
    else:
        return completion


# noinspection StrFormat
async def llm_fewshot(
    model: ModelHandle,
    prompt_prefix: str,
    prompt_example: str,
    prompt_input: str,
    examples: RecordBatch,
    return_logprobs: bool = False,
    strip: bool = True,
    **variables: dict[str, Any],
):
    """
    Complete a few-shot LLM prompt
    """
    if variables is not None:
        prompt_suffix = prompt_input.format(**variables)
    else:
        prompt_suffix = prompt_input

    prompt = (
        prompt_prefix + "".join(prompt_example.format(**row) for row in examples) + prompt_suffix
    )
    completion, logprobs = await model.complete(prompt)
    if strip:
        completion = completion.strip()
    if return_logprobs:
        return completion, logprobs
    else:
        return completion


# noinspection StrFormat
async def llm_classify(
    model: ModelHandle,
    text: str,
    examples: RecordBatch,
    prompt_prefix: str = "",
    text_key: str = "text",
    label_key: str = "label",
    label: str = None,
    return_logprobs: bool = False,
    **variables: dict[str, Any],
):
    """
    Classify text using a few-shot LLM prompt
    """
    if variables:
        text = text.format(**variables)

    prompt = (
        prompt_prefix
        + "\n".join(f"{row[text_key]}: {label or row[label_key]}" for row in examples)
        + f"{text_key}: {text}"
    )
    label, logprobs = await model.complete(prompt)
    if return_logprobs:
        return label, logprobs
    else:
        return label


instruction_builtins = {
    "llm": llm,
    "llm_fewshot": llm_fewshot,
    "llm_classify": llm_classify,
}
