from typing import Any, Optional

from bench.backend.base import Completion, ModelHandle
from bench.utils.record import RecordBatch

# Builtins are only called with trusted strings or in a sandbox.


# noinspection StrFormat
async def llm(
    model: ModelHandle,
    prompt: str,
    return_full: bool = False,
    strip: bool = True,
    **variables: dict[str, Any],
):
    """
    Complete a zero-shot LLM prompt
    """
    if variables:
        prompt = prompt.format(**variables)

    # TODO @Cleanup: reduce duplication across llm builtins
    completion = await model.complete(prompt)
    if not isinstance(completion, dict):
        raise NotImplementedError(f"list result not supported yet: {model}")

    if strip:
        completion = Completion(
            text=completion["text"].strip(),
            logits=completion.get("logits"),
            tokens=completion.get("tokens"),
        )
    if return_full:
        return completion
    else:
        return completion["text"]


# noinspection StrFormat
async def llm_fewshot(
    model: ModelHandle,
    prompt_prefix: str,
    prompt_example: str,
    prompt_input: str,
    examples: RecordBatch,
    return_full: bool = False,
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

    completion = await model.complete(prompt)
    if not isinstance(completion, dict):
        raise NotImplementedError(f"list result not supported yet: {model}")

    if strip:
        completion = Completion(
            text=completion["text"].strip(),
            logits=completion.get("logits"),
            tokens=completion.get("tokens"),
        )
    if return_full:
        return completion
    else:
        return completion["text"]


# noinspection StrFormat
async def llm_classify(
    model: ModelHandle,
    text: str,
    examples: RecordBatch,
    prompt_prefix: str = "",
    text_key: str = "text",
    label_key: str = "label",
    label: Optional[str] = None,
    return_full: bool = False,
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

    completion = await model.complete(prompt)
    if not isinstance(completion, dict):
        raise NotImplementedError(f"list result not supported yet: {model}")

    if return_full:
        return completion
    else:
        return completion["text"]


instruction_builtins: dict = {
    "llm": llm,
    "llm_fewshot": llm_fewshot,
    "llm_classify": llm_classify,
}
