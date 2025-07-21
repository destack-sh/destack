from typing import Literal

from destack.language import (
    EnumDefinition,
    NodeDefinition,
    StructDefinition,
    TraitDefinition,
)

GENERATION_PATH = "destack-ts/src/language"
MARKER_START = "/* ==== DESTACK_GENERATED_START:{kind}:{id} ==== */"
MARKER_END = "/* ==== DESTACK_GENERATED_END:{kind}:{id} ==== */"
MARKER_CUSTOM_START = "/* ==== DESTACK_CUSTOM_START ==== */"
MARKER_CUSTOM_END = "/* ==== DESTACK_CUSTOM_END ==== */"

Kind = Literal["ENUM", "STRUCT", "TRAIT", "NODE", "CONSTANT"]
Definition = EnumDefinition | StructDefinition | TraitDefinition | NodeDefinition
