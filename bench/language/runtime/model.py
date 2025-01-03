from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    EnumType,
    Struct,
    StructType,
    TypeConstraintIn,
    enum_,
    p_regular,
    struct_,
)
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    pass


@enum_(EnumType.MODEL_DEVELOPER)
class ModelDeveloper(IdEnum):
    # internal
    # ...
    # external
    META = 1000
    OPENAI = 2000
    ANTHROPIC = 3000
    GOOGLE = 4000
    AMAZON = 5000
    MICROSOFT = 6000


@enum_(EnumType.MODEL_PROVIDER)
class ModelProvider(IdEnum):
    # internal
    # ...
    # external developers (1000-19999)
    META = 1000
    OPENAI = 2000
    ANTHROPIC = 3000
    GOOGLE = 4000
    AMAZON = 5000
    MICROSOFT = 6000
    # external providers (20000-)
    # ...


@enum_(EnumType.MODEL_FAMILY)
class ModelFamily(IdEnum):
    # internal
    # ...
    # external
    # meta
    META_LLAMA = 1000
    # openai
    OPENAI_GPT = 2000
    OPENAI_O = 2100
    # anthropic
    ANTHROPIC_CLAUDE = 3000
    # google
    GOOGLE_GEMINI = 4000
    # amazon
    # ...
    # microsoft
    # ...


@enum_(EnumType.MODEL_TYPE)
class ModelType(IdEnum):  # :ModelType
    # internal
    ...  # ?
    # external
    # meta-llama
    META_LLAMA_3_1_80B = 1001
    META_LLAMA_3_1_400B = 1002
    # openai-gpt
    OPENAI_GPT4_0 = 2001
    OPENAI_GPT4_O_MINI = 2002
    # openai-o
    OPENAI_O1 = 2100
    OPENAI_O1_MINI = 2101
    # anthropic-claude
    ANTHROPIC_CLAUDE_3_5_SONNET = 3001
    # google-gemini
    GOOGLE_GEMINI_1_5_PRO = 4001
    GOOGLE_GEMINI_2_0_FLASH = 4002


@struct_(StructType.TEXT_OPTIONS)
class TextOptions(Struct):
    """Options for text models (in/out)."""

    temperature: Optional[float] = p_regular(
        30, constraint=TypeConstraintIn(min_value=0.0, max_value=5.0)
    )


@struct_(StructType.AUDIO_OPTIONS)
class AudioOptions(Struct):
    """Options for audio models (in/out)."""

    pass


@struct_(StructType.IMAGE_OPTIONS)
class ImageOptions(Struct):
    """Options for image models (in/out)."""

    pass


@struct_(StructType.VIDEO_OPTIONS)
class VideoOptions(Struct):
    """Options for video models (in/out)."""

    pass
