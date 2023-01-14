from __future__ import annotations

from typing import Union

from bench.language.types import TypeTag

PyValueType = Union[int, float, bool, str, dict, list]

PYTYPE_TO_VALUE_TYPE = {
    int: TypeTag.NUMBER,
    float: TypeTag.NUMBER,
    bool: TypeTag.BOOLEAN,
    str: TypeTag.STRING,
    dict: TypeTag.MAP,
    list: TypeTag.ARRAY,
    type(None): TypeTag.NULL,
}


def derive_type_from_value(obj: PyValueType) -> TypeTag:
    for pytype, value_type in PYTYPE_TO_VALUE_TYPE.items():
        if isinstance(obj, pytype):
            return value_type
    raise ValueError(f"unknown value type: {obj}")
