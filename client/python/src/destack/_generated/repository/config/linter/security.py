# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_array,
    json_bool,
    json_field,
    json_int,
    json_object,
    json_string,
)


@dataclass(frozen=True, slots=True)
class LinterSecurityOptions:
    """Security-category linter options."""

    # allow `rel="noreferrer"` without `noopener` in `no-blank-target`
    no_blank_target_allow_no_referrer: bool
    # domains allowed to use `target="_blank"` without rel hardening
    no_blank_target_allow_domains: Sequence[str]
    # entropy threshold in tenths for `no-secrets`
    no_secrets_entropy_threshold: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_linter_security_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> LinterSecurityOptions:
        """Decode one LinterSecurityOptions."""
        return decode_linter_security_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_linter_security_options(self)

    @classmethod
    def from_json(cls, value: Json) -> LinterSecurityOptions:
        """Return one LinterSecurityOptions from one JSON value."""
        return from_json_linter_security_options(value)


def encode_linter_security_options(
    writer: BinaryWriter, value: LinterSecurityOptions
) -> None:
    """Encode one LinterSecurityOptions."""
    writer.write_bool(value.no_blank_target_allow_no_referrer)
    writer.write_unsigned(len(value.no_blank_target_allow_domains))
    for (
        item_value_no_blank_target_allow_domains_0
    ) in value.no_blank_target_allow_domains:
        writer.write_string(item_value_no_blank_target_allow_domains_0)
    writer.write_unsigned(value.no_secrets_entropy_threshold)


def decode_linter_security_options(reader: BinaryReader) -> LinterSecurityOptions:
    """Decode one LinterSecurityOptions."""
    no_blank_target_allow_no_referrer = reader.read_bool()
    no_blank_target_allow_domains = [
        reader.read_string() for _ in range(reader.read_number())
    ]
    no_secrets_entropy_threshold = reader.read_number()

    return LinterSecurityOptions(
        no_blank_target_allow_no_referrer=no_blank_target_allow_no_referrer,
        no_blank_target_allow_domains=no_blank_target_allow_domains,
        no_secrets_entropy_threshold=no_secrets_entropy_threshold,
    )


def to_json_linter_security_options(value: LinterSecurityOptions) -> Json:
    """Return one JSON value for one LinterSecurityOptions."""
    return {
        "noBlankTargetAllowNoReferrer": value.no_blank_target_allow_no_referrer,
        "noBlankTargetAllowDomains": [
            item_0 for item_0 in value.no_blank_target_allow_domains
        ],
        "noSecretsEntropyThreshold": value.no_secrets_entropy_threshold,
    }


def from_json_linter_security_options(value: Json) -> LinterSecurityOptions:
    """Return one LinterSecurityOptions from one JSON value."""
    object_ = json_object(value)

    return LinterSecurityOptions(
        no_blank_target_allow_no_referrer=json_bool(
            json_field(object_, "noBlankTargetAllowNoReferrer")
        ),
        no_blank_target_allow_domains=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "noBlankTargetAllowDomains"))
        ],
        no_secrets_entropy_threshold=json_int(
            json_field(object_, "noSecretsEntropyThreshold")
        ),
    )


__all__ = [
    "LinterSecurityOptions",
    "encode_linter_security_options",
    "decode_linter_security_options",
    "to_json_linter_security_options",
    "from_json_linter_security_options",
]
