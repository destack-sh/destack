from abc import ABC, abstractmethod
from typing import TYPE_CHECKING, Sequence

from .prompt import Prompt

if TYPE_CHECKING:
    pass


class ModelRunner[IP, P](ABC):
    """
    Run a Model that takes Prompts and returns some Model-specific output (maybe streaming).
    NOTE :Architecture: shouldn't ModelRunner be a subclass of Runner?
    """

    @abstractmethod
    async def compile(self, prompt: Prompt, budget: float) -> Sequence[IP]:
        """Compile the Prompt into a list of basic prompt parts."""
        ...

    @abstractmethod
    async def assemble(self, parts: Sequence[IP]) -> Sequence[P]:
        """Assemble basic Prompt parts into some rendered prompt."""
        ...

    #
    # ... Model-specific methods ...
    #
