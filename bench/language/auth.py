from datetime import datetime
from os import urandom
from typing import TYPE_CHECKING, Optional
from uuid import UUID

from bench.language.const import (
    ActionKind,
    BadgeType,
    BenchType,
    NodeType,
    PolicyEffect,
    StructType,
)
from bench.language.node import Bench, Node, Struct, node, node_parent, struct, struct_internal

if TYPE_CHECKING:
    from bench.language import Expression, Field, User


@struct(StructType.POLICY)
class Policy(Struct):
    """A policy regulating access to nodes within its scope."""

    name: Optional[str] = struct_internal(30, default=None)
    rules: list["PolicyRule"] = struct_internal(
        31, default_factory=list, struct_t=StructType.POLICY_RULE
    )
    hidden: bool = struct_internal(
        32, default=False, description="Hide this policy and its effects from the denied."
    )


@struct(StructType.POLICY_RULE)
class PolicyRule(Struct):
    """A rule in a policy: <subject> + can/cannot <verb> + <object> [if condition]."""

    # subject
    subject_authenticated: bool = struct_internal(30, default=False)
    subject_users: Optional[list["User"]] = struct_internal(
        31, require=False, array=True, references=NodeType.USER
    )
    # subject_groups, subject_roles, ...
    # verb
    effect: PolicyEffect = struct_internal(40)
    verb: Optional[list[ActionKind]] = struct_internal(41)
    # object
    object_types: Optional[list[BenchType]] = struct_internal(50, default=None)
    object_nodes: list[Node] | None = struct_internal(
        51, require=False, array=True, references=tuple(NodeType)
    )
    object_fields: list["Field"] | None = struct_internal(
        52, require=False, array=True, references=NodeType.FIELD
    )
    # [condition]
    condition: Optional["Expression"] = struct_internal(
        60, default=None, struct_t=StructType.EXPRESSION
    )


@node(NodeType.BADGE, in_module=False)
class Badge(Node):
    """A badge for a non-member to access parts of this Bench (via web or programmatically)."""

    parent: Bench = node_parent(4, NodeType.BENCH)
    type: BadgeType = struct_internal(30)
    name: Optional[str] = struct_internal(31)
    policy: Policy = struct_internal(32, struct_t=StructType.POLICY)
    expires_at: Optional[datetime] = struct_internal(33, default=None)
    # sharing link badge
    link_token: Optional[UUID] = struct_internal(40, default=None, unique=True)
    link_password: Optional[str] = struct_internal(41, default=None, encrypt=True, defer=True)
    link_password_digest: Optional[str] = struct_internal(42, default=None, encrypt=True)
    # access key badge
    key_value: Optional[str] = struct_internal(50, default=None, encrypt=True, defer=True)
    key_value_digest: Optional[str] = struct_internal(51, default=None, encrypt=True)


PASSWORD_MIN_LENGTH = 8  # characters
PASSWORD_MAX_LENGTH = 128  # characters
SALT_LENGTH = 16  # bytes
SCRYPT_N = 2**15  # iterations count
SCRYPT_R = 8  # block size in bytes
SCRYPT_P = 1  # threads to use
SCRYPT_MAXMEM = 2**24  # max memory to use in bytes
SCRYPT_DKLEN = 32  # hash length in bytes
ACCESS_TOKEN_LENGTH = 32  # bytes


def generate_salt() -> bytes:
    """Generate a random salt."""
    raise urandom(SALT_LENGTH)


def hash_password(password: str, salt: bytes) -> bytes:
    """Hash a password using scrypt."""
    from hashlib import scrypt

    assert len(salt) == SALT_LENGTH, f"invalid salt length: {len(salt)} != {SALT_LENGTH}"
    assert (
        len(password) >= PASSWORD_MIN_LENGTH
    ), f"password too short: {len(password)} < {PASSWORD_MIN_LENGTH}"
    assert (
        len(password) <= PASSWORD_MAX_LENGTH
    ), f"password too long: {len(password)} > {PASSWORD_MAX_LENGTH}"

    return scrypt(
        password,
        salt=salt,
        n=SCRYPT_N,
        r=SCRYPT_R,
        p=SCRYPT_P,
        maxmem=SCRYPT_MAXMEM,
        dklen=SCRYPT_DKLEN,
    )


def check_password(password: str, salt: bytes, password_hash: bytes) -> bool:
    """Check if a password matches its hash."""
    return hash_password(password, salt) == password_hash


def generate_access_token() -> str:
    """Generate a random access token."""
    return urandom(ACCESS_TOKEN_LENGTH).hex()
