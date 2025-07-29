from typing import Literal

from destack.language import EnumDefinition, NodeDefinition, StructDefinition

GENERATION_PATH = "destack-ts/src"
MARKER_START = "/* ==== DESTACK_GENERATED_START:{kind}:{id} ==== */"
MARKER_END = "/* ==== DESTACK_GENERATED_END:{kind}:{id} ==== */"
MARKER_CUSTOM_START = "/* ==== DESTACK_CUSTOM_START ==== */"
MARKER_CUSTOM_END = "/* ==== DESTACK_CUSTOM_END ==== */"

Kind = Literal["ENUM", "STRUCT", "NODE", "CONSTANT"]
Definition = EnumDefinition | StructDefinition | NodeDefinition
