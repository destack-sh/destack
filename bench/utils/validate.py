from typing import Any, Callable, Union, cast

from bench.utils.record import Record, RecordBatch
from bench.utils.spec import (
    ConfigTypeSpec,
    EnumType,
    FieldSpec,
    FieldTypePrimitive,
    FieldTypeSpec,
    _Type,
)


def validate_record_batch_type(
    records: Union[RecordBatch, list[Record]],
    record_type: Union[FieldTypeSpec, FieldTypePrimitive],
    ignore_extraneous: bool,
    lazy: bool,
):
    records_len = len(records)
    if records_len == 0:
        return
    if lazy:
        # validate first and last only
        validate_record_type(records[0], record_type, ignore_extraneous)
        if records_len > 1:
            validate_record_type(records[records_len - 1], record_type, ignore_extraneous)
    else:
        # validate each record individually
        for record in records:
            validate_record_type(record, record_type, ignore_extraneous)


def _fail(path: list[str], message: str):
    path_str = ".".join(path) or "<root>"
    raise ValueError(f"{path_str} {message}")


def _check_isinstance(path: list[str], value: Any, cls: Any):
    if not isinstance(value, cls):
        _fail(path, f"is not a {cls} but is {type(value)}: {str(value)}")


def _check_none(path: list[str], optional: bool, value_type: Any):
    if not optional:
        _fail(path, f"is None but {value_type} is not optional")


def _validate_rec(
    path: list[str], value: Record, value_type: Union[FieldTypeSpec, FieldTypePrimitive]
):
    if isinstance(value_type, FieldSpec):
        # skip to inner validation
        _validate_rec(path, value, value_type.type)
        return
    optional = isinstance(value_type, _Type) and getattr(value_type, "optional", False)
    if value is None:
        _check_none(path, optional, value_type)
    elif isinstance(value_type, (type, _Type)):
        _check_isinstance(path, value, value_type)
    elif isinstance(value_type, (tuple, list)):
        _check_isinstance(path, value, (tuple, list))
        for i, element_type in enumerate(value):
            _validate_rec(path + [f"[{i}]"], cast(list, value)[i], value_type[0])
    elif isinstance(value_type, dict):
        _check_isinstance(path, value, dict)
        for key, element_type in value_type.items():
            _validate_rec(path + [key], cast(dict, value).get(key), element_type)


def validate_record_type(
    record: Record, record_type: Union[FieldTypeSpec, FieldTypePrimitive], ignore_extraneous: bool
):
    _validate_rec([], record, record_type)


def validate_config_type(
    arguments: dict[str, Any], config_type: ConfigTypeSpec, ignore_extraneous: bool
):
    for key, value in arguments.items():
        # value_type is not actually guaranteed to be this type but unknown types are just ignored
        _validate_rec([key], value, config_type[key])


def _walk_rec(
    path: list[str],
    value: Record,
    value_type: Union[FieldTypeSpec, FieldTypePrimitive],
    walk_value: Callable,
):
    if isinstance(value_type, FieldSpec):
        # skip to inner
        return _walk_rec(path, value, value_type.type, walk_value)

    if isinstance(value_type, (tuple, list)):
        cast_values: list[Any] = []
        for i, item in enumerate(value):
            cast_values[i] = _walk_rec(
                path + [f"[{i}]"], cast(list, value)[i], value_type[0], walk_value
            )
        return cast_values
    elif isinstance(value_type, dict):
        cast_dict = {}
        for key, element_type in value_type.items():
            if key not in value:
                continue
            cast_dict[key] = _walk_rec(
                path + [key], cast(dict, value)[key], element_type, walk_value
            )
        return cast_dict
    return walk_value(path, value, value_type)


def cast_config_arguments(arguments: dict[str, Any], config_type: ConfigTypeSpec):
    def cast_argument(path: list[str], value: Any, value_type: FieldTypePrimitive) -> Any:
        if isinstance(value_type, EnumType) and value_type.ptype is not None:
            return value_type.ptype[value]
        else:
            return value

    # config_type is not actually guaranteed to be this type but unknown types are just ignored
    return _walk_rec([], arguments, config_type, cast_argument)
