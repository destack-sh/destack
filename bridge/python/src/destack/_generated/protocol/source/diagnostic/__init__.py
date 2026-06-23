# generated bridge target, do not edit

from .diagnostic import (
    Diagnostic,
    decode_diagnostic,
    encode_diagnostic,
)
from .label import (
    DiagnosticHelp,
    DiagnosticLabel,
    DiagnosticNote,
    decode_diagnostic_help,
    decode_diagnostic_label,
    decode_diagnostic_note,
    encode_diagnostic_help,
    encode_diagnostic_label,
    encode_diagnostic_note,
)
from .severity import (
    DiagnosticSeverity,
    DiagnosticTag,
    decode_diagnostic_severity,
    decode_diagnostic_tag,
    encode_diagnostic_severity,
    encode_diagnostic_tag,
)
from .suggestion import (
    Applicability,
    DiagnosticSuggestion,
    decode_applicability,
    decode_diagnostic_suggestion,
    encode_applicability,
    encode_diagnostic_suggestion,
)

__all__ = [
    "Diagnostic",
    "encode_diagnostic",
    "decode_diagnostic",
    "DiagnosticLabel",
    "encode_diagnostic_label",
    "decode_diagnostic_label",
    "DiagnosticNote",
    "encode_diagnostic_note",
    "decode_diagnostic_note",
    "DiagnosticHelp",
    "encode_diagnostic_help",
    "decode_diagnostic_help",
    "DiagnosticSeverity",
    "encode_diagnostic_severity",
    "decode_diagnostic_severity",
    "DiagnosticTag",
    "encode_diagnostic_tag",
    "decode_diagnostic_tag",
    "DiagnosticSuggestion",
    "encode_diagnostic_suggestion",
    "decode_diagnostic_suggestion",
    "Applicability",
    "encode_applicability",
    "decode_applicability",
]
