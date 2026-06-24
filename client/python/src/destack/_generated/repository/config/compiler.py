# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_array,
    json_bool,
    json_field,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.repository.config.target.js


@dataclass(frozen=True, slots=True)
class CompilerOptions:
    """Normalized Destack compiler options."""

    # javaScript module format for output
    module: destack._generated.repository.config.target.js.JsModuleFormat
    # ECMAScript target version
    es_target: destack._generated.repository.config.target.js.EsTarget
    # default profile
    profile: str | None
    # default active source graph modes
    modes: Sequence[str]
    # default active source graph roles
    roles: Sequence[str]
    # default active source graph features
    features: Sequence[str]
    # default active source graph tags
    tags: Sequence[str]
    # comptime environment whitelist (if omitted, all env keys are visible)
    comptime_env: Sequence[str] | None
    # default tree tag builder provider
    tree: str | None
    # global provider modules added to every target profile
    globals: Sequence[str]
    # well-known derives automatically considered for nominal declarations
    derive: Sequence[Derive]
    # static semantic restrictions
    restrictions: CompilerRestrictions
    # root directory of source files (controls output directory structure, not module resolution)
    root_dir: str | None
    # output directory for compiled files
    out_dir: str | None
    # output directory for declaration files. Defaults to out_dir
    declaration_dir: str | None
    # emit declaration maps for `.d.ts` output
    declaration_map: bool
    # do not emit output files
    no_emit: bool
    # emit phase stats sidecars
    emit_stats: bool
    # emit phase event sidecars
    emit_events: bool
    # emit checked type annotation sidecars
    emit_checked_types: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_compiler_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CompilerOptions:
        """Decode one CompilerOptions."""
        return decode_compiler_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_compiler_options(self)

    @classmethod
    def from_json(cls, value: Json) -> CompilerOptions:
        """Return one CompilerOptions from one JSON value."""
        return from_json_compiler_options(value)


def encode_compiler_options(writer: BinaryWriter, value: CompilerOptions) -> None:
    """Encode one CompilerOptions."""
    destack._generated.repository.config.target.js.encode_js_module_format(
        writer, value.module
    )
    destack._generated.repository.config.target.js.encode_es_target(
        writer, value.es_target
    )
    if value.profile is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.profile)
    writer.write_unsigned(len(value.modes))
    for item_value_modes_0 in value.modes:
        writer.write_string(item_value_modes_0)
    writer.write_unsigned(len(value.roles))
    for item_value_roles_0 in value.roles:
        writer.write_string(item_value_roles_0)
    writer.write_unsigned(len(value.features))
    for item_value_features_0 in value.features:
        writer.write_string(item_value_features_0)
    writer.write_unsigned(len(value.tags))
    for item_value_tags_0 in value.tags:
        writer.write_string(item_value_tags_0)
    if value.comptime_env is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(len(value.comptime_env))
        for item_value_comptime_env_1 in value.comptime_env:
            writer.write_string(item_value_comptime_env_1)
    if value.tree is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.tree)
    writer.write_unsigned(len(value.globals))
    for item_value_globals_0 in value.globals:
        writer.write_string(item_value_globals_0)
    writer.write_unsigned(len(value.derive))
    for item_value_derive_0 in value.derive:
        encode_derive(writer, item_value_derive_0)
    encode_compiler_restrictions(writer, value.restrictions)
    if value.root_dir is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.root_dir)
    if value.out_dir is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.out_dir)
    if value.declaration_dir is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.declaration_dir)
    writer.write_bool(value.declaration_map)
    writer.write_bool(value.no_emit)
    writer.write_bool(value.emit_stats)
    writer.write_bool(value.emit_events)
    writer.write_bool(value.emit_checked_types)


def decode_compiler_options(reader: BinaryReader) -> CompilerOptions:
    """Decode one CompilerOptions."""
    module = destack._generated.repository.config.target.js.decode_js_module_format(
        reader
    )
    es_target = destack._generated.repository.config.target.js.decode_es_target(reader)
    profile = reader.read_option(lambda: reader.read_string())
    modes = [reader.read_string() for _ in range(reader.read_number())]
    roles = [reader.read_string() for _ in range(reader.read_number())]
    features = [reader.read_string() for _ in range(reader.read_number())]
    tags = [reader.read_string() for _ in range(reader.read_number())]
    comptime_env = reader.read_option(
        lambda: [reader.read_string() for _ in range(reader.read_number())]
    )
    tree = reader.read_option(lambda: reader.read_string())
    globals = [reader.read_string() for _ in range(reader.read_number())]
    derive = [decode_derive(reader) for _ in range(reader.read_number())]
    restrictions = decode_compiler_restrictions(reader)
    root_dir = reader.read_option(lambda: reader.read_string())
    out_dir = reader.read_option(lambda: reader.read_string())
    declaration_dir = reader.read_option(lambda: reader.read_string())
    declaration_map = reader.read_bool()
    no_emit = reader.read_bool()
    emit_stats = reader.read_bool()
    emit_events = reader.read_bool()
    emit_checked_types = reader.read_bool()

    return CompilerOptions(
        module=module,
        es_target=es_target,
        profile=profile,
        modes=modes,
        roles=roles,
        features=features,
        tags=tags,
        comptime_env=comptime_env,
        tree=tree,
        globals=globals,
        derive=derive,
        restrictions=restrictions,
        root_dir=root_dir,
        out_dir=out_dir,
        declaration_dir=declaration_dir,
        declaration_map=declaration_map,
        no_emit=no_emit,
        emit_stats=emit_stats,
        emit_events=emit_events,
        emit_checked_types=emit_checked_types,
    )


def to_json_compiler_options(value: CompilerOptions) -> Json:
    """Return one JSON value for one CompilerOptions."""
    return {
        "module": destack._generated.repository.config.target.js.to_json_js_module_format(
            value.module
        ),
        "esTarget": destack._generated.repository.config.target.js.to_json_es_target(
            value.es_target
        ),
        **({} if value.profile is None else {"profile": value.profile}),
        "modes": [item_0 for item_0 in value.modes],
        "roles": [item_0 for item_0 in value.roles],
        "features": [item_0 for item_0 in value.features],
        "tags": [item_0 for item_0 in value.tags],
        **(
            {}
            if value.comptime_env is None
            else {"comptimeEnv": [item_0 for item_0 in value.comptime_env]}
        ),
        **({} if value.tree is None else {"tree": value.tree}),
        "globals": [item_0 for item_0 in value.globals],
        "derive": [to_json_derive(item_0) for item_0 in value.derive],
        "restrictions": to_json_compiler_restrictions(value.restrictions),
        **({} if value.root_dir is None else {"rootDir": value.root_dir}),
        **({} if value.out_dir is None else {"outDir": value.out_dir}),
        **(
            {}
            if value.declaration_dir is None
            else {"declarationDir": value.declaration_dir}
        ),
        "declarationMap": value.declaration_map,
        "noEmit": value.no_emit,
        "emitStats": value.emit_stats,
        "emitEvents": value.emit_events,
        "emitCheckedTypes": value.emit_checked_types,
    }


def from_json_compiler_options(value: Json) -> CompilerOptions:
    """Return one CompilerOptions from one JSON value."""
    object_ = json_object(value)

    return CompilerOptions(
        module=destack._generated.repository.config.target.js.from_json_js_module_format(
            json_field(object_, "module")
        ),
        es_target=destack._generated.repository.config.target.js.from_json_es_target(
            json_field(object_, "esTarget")
        ),
        profile=json_optional(object_, "profile", lambda value: json_string(value)),
        modes=[
            json_string(item_0) for item_0 in json_array(json_field(object_, "modes"))
        ],
        roles=[
            json_string(item_0) for item_0 in json_array(json_field(object_, "roles"))
        ],
        features=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "features"))
        ],
        tags=[
            json_string(item_0) for item_0 in json_array(json_field(object_, "tags"))
        ],
        comptime_env=json_optional(
            object_,
            "comptimeEnv",
            lambda value: [json_string(item_0) for item_0 in json_array(value)],
        ),
        tree=json_optional(object_, "tree", lambda value: json_string(value)),
        globals=[
            json_string(item_0) for item_0 in json_array(json_field(object_, "globals"))
        ],
        derive=[
            from_json_derive(item_0)
            for item_0 in json_array(json_field(object_, "derive"))
        ],
        restrictions=from_json_compiler_restrictions(
            json_field(object_, "restrictions")
        ),
        root_dir=json_optional(object_, "rootDir", lambda value: json_string(value)),
        out_dir=json_optional(object_, "outDir", lambda value: json_string(value)),
        declaration_dir=json_optional(
            object_, "declarationDir", lambda value: json_string(value)
        ),
        declaration_map=json_bool(json_field(object_, "declarationMap")),
        no_emit=json_bool(json_field(object_, "noEmit")),
        emit_stats=json_bool(json_field(object_, "emitStats")),
        emit_events=json_bool(json_field(object_, "emitEvents")),
        emit_checked_types=json_bool(json_field(object_, "emitCheckedTypes")),
    )


"""Well-known compiler-owned derive provider."""
Derive: typing.TypeAlias = (
    typing.Literal["compare"]
    | typing.Literal["copy"]
    | typing.Literal["clone"]
    | typing.Literal["debug"]
    | typing.Literal["default"]
    | typing.Literal["deserialize"]
    | typing.Literal["equal"]
    | typing.Literal["hash"]
    | typing.Literal["partialCompare"]
    | typing.Literal["partialEqual"]
    | typing.Literal["serialize"]
    | typing.Literal["tagged"]
)


def encode_derive(writer: BinaryWriter, value: Derive) -> None:
    """Encode one Derive."""
    if value == "compare":
        writer.write_unsigned(0)
    elif value == "copy":
        writer.write_unsigned(1)
    elif value == "clone":
        writer.write_unsigned(2)
    elif value == "debug":
        writer.write_unsigned(3)
    elif value == "default":
        writer.write_unsigned(4)
    elif value == "deserialize":
        writer.write_unsigned(5)
    elif value == "equal":
        writer.write_unsigned(6)
    elif value == "hash":
        writer.write_unsigned(7)
    elif value == "partialCompare":
        writer.write_unsigned(8)
    elif value == "partialEqual":
        writer.write_unsigned(9)
    elif value == "serialize":
        writer.write_unsigned(10)
    elif value == "tagged":
        writer.write_unsigned(11)
    else:
        raise SerdeError("unknown enum variant")


def decode_derive(reader: BinaryReader) -> Derive:
    """Decode one Derive."""
    variant = reader.read_number()

    if variant == 0:
        return "compare"
    elif variant == 1:
        return "copy"
    elif variant == 2:
        return "clone"
    elif variant == 3:
        return "debug"
    elif variant == 4:
        return "default"
    elif variant == 5:
        return "deserialize"
    elif variant == 6:
        return "equal"
    elif variant == 7:
        return "hash"
    elif variant == 8:
        return "partialCompare"
    elif variant == 9:
        return "partialEqual"
    elif variant == 10:
        return "serialize"
    elif variant == 11:
        return "tagged"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_derive(value: Derive) -> Json:
    """Return one JSON value for one Derive."""
    return value


def from_json_derive(value: Json) -> Derive:
    """Return one Derive from one JSON value."""
    variant = json_string(value)

    if variant == "compare":
        return "compare"
    elif variant == "copy":
        return "copy"
    elif variant == "clone":
        return "clone"
    elif variant == "debug":
        return "debug"
    elif variant == "default":
        return "default"
    elif variant == "deserialize":
        return "deserialize"
    elif variant == "equal":
        return "equal"
    elif variant == "hash":
        return "hash"
    elif variant == "partialCompare":
        return "partialCompare"
    elif variant == "partialEqual":
        return "partialEqual"
    elif variant == "serialize":
        return "serialize"
    elif variant == "tagged":
        return "tagged"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class CompilerRestrictions:
    """Static semantic restrictions enforced by the compiler."""

    # policy for managed values and managed allocation
    no_managed: DiagnosticPolicy
    # policy for all heap allocation
    no_heap: DiagnosticPolicy
    # policy for runtime-dependent language features
    no_runtime: DiagnosticPolicy
    # policy for unsafe operations
    no_unsafe: DiagnosticPolicy
    # policy for calls that cannot be statically resolved
    no_dynamic_dispatch: DiagnosticPolicy
    # policy for runtime reflection and RTTI usage
    no_reflection: DiagnosticPolicy
    # policy for unwinding
    no_unwind: DiagnosticPolicy
    # policy banning aliasing mutable borrows
    no_aliasing_mutable_borrows: DiagnosticPolicy
    # policy banning implicit method receivers
    no_implicit_receivers: DiagnosticPolicy

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_compiler_restrictions(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CompilerRestrictions:
        """Decode one CompilerRestrictions."""
        return decode_compiler_restrictions(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_compiler_restrictions(self)

    @classmethod
    def from_json(cls, value: Json) -> CompilerRestrictions:
        """Return one CompilerRestrictions from one JSON value."""
        return from_json_compiler_restrictions(value)


def encode_compiler_restrictions(
    writer: BinaryWriter, value: CompilerRestrictions
) -> None:
    """Encode one CompilerRestrictions."""
    encode_diagnostic_policy(writer, value.no_managed)
    encode_diagnostic_policy(writer, value.no_heap)
    encode_diagnostic_policy(writer, value.no_runtime)
    encode_diagnostic_policy(writer, value.no_unsafe)
    encode_diagnostic_policy(writer, value.no_dynamic_dispatch)
    encode_diagnostic_policy(writer, value.no_reflection)
    encode_diagnostic_policy(writer, value.no_unwind)
    encode_diagnostic_policy(writer, value.no_aliasing_mutable_borrows)
    encode_diagnostic_policy(writer, value.no_implicit_receivers)


def decode_compiler_restrictions(reader: BinaryReader) -> CompilerRestrictions:
    """Decode one CompilerRestrictions."""
    no_managed = decode_diagnostic_policy(reader)
    no_heap = decode_diagnostic_policy(reader)
    no_runtime = decode_diagnostic_policy(reader)
    no_unsafe = decode_diagnostic_policy(reader)
    no_dynamic_dispatch = decode_diagnostic_policy(reader)
    no_reflection = decode_diagnostic_policy(reader)
    no_unwind = decode_diagnostic_policy(reader)
    no_aliasing_mutable_borrows = decode_diagnostic_policy(reader)
    no_implicit_receivers = decode_diagnostic_policy(reader)

    return CompilerRestrictions(
        no_managed=no_managed,
        no_heap=no_heap,
        no_runtime=no_runtime,
        no_unsafe=no_unsafe,
        no_dynamic_dispatch=no_dynamic_dispatch,
        no_reflection=no_reflection,
        no_unwind=no_unwind,
        no_aliasing_mutable_borrows=no_aliasing_mutable_borrows,
        no_implicit_receivers=no_implicit_receivers,
    )


def to_json_compiler_restrictions(value: CompilerRestrictions) -> Json:
    """Return one JSON value for one CompilerRestrictions."""
    return {
        "noManaged": to_json_diagnostic_policy(value.no_managed),
        "noHeap": to_json_diagnostic_policy(value.no_heap),
        "noRuntime": to_json_diagnostic_policy(value.no_runtime),
        "noUnsafe": to_json_diagnostic_policy(value.no_unsafe),
        "noDynamicDispatch": to_json_diagnostic_policy(value.no_dynamic_dispatch),
        "noReflection": to_json_diagnostic_policy(value.no_reflection),
        "noUnwind": to_json_diagnostic_policy(value.no_unwind),
        "noAliasingMutableBorrows": to_json_diagnostic_policy(
            value.no_aliasing_mutable_borrows
        ),
        "noImplicitReceivers": to_json_diagnostic_policy(value.no_implicit_receivers),
    }


def from_json_compiler_restrictions(value: Json) -> CompilerRestrictions:
    """Return one CompilerRestrictions from one JSON value."""
    object_ = json_object(value)

    return CompilerRestrictions(
        no_managed=from_json_diagnostic_policy(json_field(object_, "noManaged")),
        no_heap=from_json_diagnostic_policy(json_field(object_, "noHeap")),
        no_runtime=from_json_diagnostic_policy(json_field(object_, "noRuntime")),
        no_unsafe=from_json_diagnostic_policy(json_field(object_, "noUnsafe")),
        no_dynamic_dispatch=from_json_diagnostic_policy(
            json_field(object_, "noDynamicDispatch")
        ),
        no_reflection=from_json_diagnostic_policy(json_field(object_, "noReflection")),
        no_unwind=from_json_diagnostic_policy(json_field(object_, "noUnwind")),
        no_aliasing_mutable_borrows=from_json_diagnostic_policy(
            json_field(object_, "noAliasingMutableBorrows")
        ),
        no_implicit_receivers=from_json_diagnostic_policy(
            json_field(object_, "noImplicitReceivers")
        ),
    )


"""Diagnostic policy for allow/warn/deny enforcement."""
DiagnosticPolicy: typing.TypeAlias = (
    typing.Literal["allow"] | typing.Literal["warn"] | typing.Literal["deny"]
)


def encode_diagnostic_policy(writer: BinaryWriter, value: DiagnosticPolicy) -> None:
    """Encode one DiagnosticPolicy."""
    if value == "allow":
        writer.write_unsigned(0)
    elif value == "warn":
        writer.write_unsigned(1)
    elif value == "deny":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_diagnostic_policy(reader: BinaryReader) -> DiagnosticPolicy:
    """Decode one DiagnosticPolicy."""
    variant = reader.read_number()

    if variant == 0:
        return "allow"
    elif variant == 1:
        return "warn"
    elif variant == 2:
        return "deny"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_diagnostic_policy(value: DiagnosticPolicy) -> Json:
    """Return one JSON value for one DiagnosticPolicy."""
    return value


def from_json_diagnostic_policy(value: Json) -> DiagnosticPolicy:
    """Return one DiagnosticPolicy from one JSON value."""
    variant = json_string(value)

    if variant == "allow":
        return "allow"
    elif variant == "warn":
        return "warn"
    elif variant == "deny":
        return "deny"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "CompilerOptions",
    "encode_compiler_options",
    "decode_compiler_options",
    "to_json_compiler_options",
    "from_json_compiler_options",
    "Derive",
    "encode_derive",
    "decode_derive",
    "to_json_derive",
    "from_json_derive",
    "CompilerRestrictions",
    "encode_compiler_restrictions",
    "decode_compiler_restrictions",
    "to_json_compiler_restrictions",
    "from_json_compiler_restrictions",
    "DiagnosticPolicy",
    "encode_diagnostic_policy",
    "decode_diagnostic_policy",
    "to_json_diagnostic_policy",
    "from_json_diagnostic_policy",
]
