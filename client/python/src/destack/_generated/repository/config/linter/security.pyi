# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

@dataclass(frozen=True, slots=True)
class LinterSecurityOptions:
    """Security-category linter options."""

    # allow `rel="noreferrer"` without `noopener` in `no-blank-target`
    no_blank_target_allow_no_referrer: bool
    # domains allowed to use `target="_blank"` without rel hardening
    no_blank_target_allow_domains: Sequence[str]
    # entropy threshold in tenths for `no-secrets`
    no_secrets_entropy_threshold: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> LinterSecurityOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> LinterSecurityOptions: ...

def encode_linter_security_options(
    writer: BinaryWriter, value: LinterSecurityOptions
) -> None: ...
def decode_linter_security_options(reader: BinaryReader) -> LinterSecurityOptions: ...
def to_json_linter_security_options(value: LinterSecurityOptions) -> Json: ...
def from_json_linter_security_options(value: Json) -> LinterSecurityOptions: ...

__all__ = [
    "LinterSecurityOptions",
    "encode_linter_security_options",
    "decode_linter_security_options",
    "to_json_linter_security_options",
    "from_json_linter_security_options",
]
