from abc import ABC, abstractmethod
from typing import override

from bench.language import File


class Tokenizer(ABC):
    @abstractmethod
    def estimate_string_tokens(self, text: str) -> int:
        """Estimate the number of tokens in a string."""
        ...

    @abstractmethod
    def estimate_image_tokens(self, image: File) -> int:
        """Estimate the number of tokens in an image."""
        ...

    @abstractmethod
    def estimate_audio_tokens(self, audio: File) -> int:
        """Estimate the number of tokens in an audio."""
        ...


class StupidTokenizer(Tokenizer):
    @override
    def estimate_string_tokens(self, text: str) -> int:
        return len(text)

    @override
    def estimate_image_tokens(self, image: File) -> int:
        return 1000

    @override
    def estimate_audio_tokens(self, audio: File) -> int:
        return 1000


TiktokenTokenizer = StupidTokenizer  # nocheckin
