import os

import aiohttp

from bench.runtime.type import DecoderSettings, ModelInstance, TextGeneration


class Inference:
    async def generate(self, prompt: str, settings: DecoderSettings) -> TextGeneration:
        raise NotImplementedError()

    def end(self):
        pass


class LocalHfTransformersInference(Inference):
    def __init__(self, model: ModelInstance):
        from transformers import AutoModelForCausalLM, AutoTokenizer

        self.tokenizer = AutoTokenizer.from_pretrained(model.external_name)
        self.hf_model = AutoModelForCausalLM.from_pretrained(model.external_name)

    async def generate(self, prompt: str, settings: DecoderSettings) -> TextGeneration:
        from transformers.utils import ModelOutput

        input_ids = self.tokenizer(prompt, return_tensors="pt").input_ids
        generated: ModelOutput = self.hf_model.generate(
            input_ids,
            max_length=settings.max_tokens,
            temperature=settings.temperature,
            output_scores=True,
            return_dict_in_generate=True,
        )
        generated_ids = generated["sequences"].tolist()
        generated_logits = generated["scores"].tolist()
        generated_text = self.tokenizer.decode(generated_ids[0], skip_special_tokens=True)
        generated_tokens = self.tokenizer.convert_ids_to_tokens(generated_ids[0])
        return TextGeneration(
            text=generated_text,
            tokens=generated_tokens,
            logits=generated_logits,
        )

    def end(self):
        self.hf_model = None
        self.tokenizer = None


class OpenAIInference(Inference):
    def __init__(self, model: ModelInstance):
        if model.provider != "openai":
            raise ValueError(f"cannot use {model}")
        self.model = model
        # TODO @Security: don't pass internal secrets to workers via environment variables
        #  (maybe proxy on the worker pod through a sidecar container.. or something)
        self._api_key = os.environ["OPENAI_API_KEY"]

    @property
    def headers(self):
        return {"Content-Type": "application/json", "Authorization": f"Bearer {self._api_key}"}

    async def generate(self, prompt: str, settings: DecoderSettings) -> TextGeneration:
        request = {
            "model": self.model.external_name,
            "prompt": prompt,
            "max_tokens": settings.max_tokens,
            "temperature": settings.temperature,
            "stop": settings.stop,
            "logprobs": 4,
        }

        async with aiohttp.ClientSession(headers=self.headers) as session:
            async with session.post(
                "https://api.openai.com/v1/completions", json=request
            ) as response:
                if response.status != 200:
                    raise RuntimeError(await response.text())
                output = await response.json()

        outputs = [choice for choice in output["choices"]]
        generations = []
        for output in outputs:
            generation = TextGeneration(
                text=output["text"],
                tokens=output.get("logprobs", {}).get("tokens", None),
                logits=output.get("logprobs", {}).get("token_logprobs", None),
            )
            generations.append(generation)
        if len(generations) != 1:
            raise ValueError(f"expected exactly one generation: {generations}")
        return generations[0]

    def end(self):
        pass
