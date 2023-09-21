import functools
import typing
from dataclasses import dataclass
from datetime import datetime
from typing import Optional
from uuid import UUID, uuid5

from bench.language.builtin import symbolx_lib
from bench.language.const import BENCH_UUID_NAMESPACE, TypeTag
from bench.language.file import File
from bench.language.mapping import pack_value, type_from_instance_type, unpack_value


def _derive_constant_key(path: str) -> UUID:
    # do not change this, it's a *constant* key
    return uuid5(BENCH_UUID_NAMESPACE, f"reflect:{path}")


def x_enum(name: str, text: str, *, file: File):
    """Map a Python type into a Bench type."""

    def decorator(cls):
        # check that names and values are equal
        for n, value in cls.__members__.items():
            if n != value.name:
                raise ValueError(f"name must equal value in {cls}: {n} != {value.name}")
        bench_type = type_from_instance_type(cls, name=name)
        bench_type.text = text
        if bench_type.tag != TypeTag.ENUM:
            raise TypeError(f"expected enum, got {bench_type.tag}")
        file.append_statement(bench_type)
        return cls

    return decorator


@typing.dataclass_transform()
def x_struct(
    name: str, text: str, *, file: File, return_type: bool = False
) -> typing.Callable[[typing.Type], typing.Type]:
    def decorator(cls):
        # turn it into dataclass that behaves like a dict
        # it needs to be a dataclass for getattr and getitem access
        # and it needs to be a real distinct class to type map references to it properly
        cls = dataclass(cls)
        bench_type = type_from_instance_type(cls, name=name)
        bench_type.text = text
        if bench_type.tag != TypeTag.STRUCT:
            raise TypeError(f"expected {TypeTag.STRUCT} for {cls}, got {bench_type.tag}")
        # add any parent classes as base types
        for base in cls.__bases__:
            if base is object:
                continue
            base_type = type_from_instance_type(base, name=None)
            if base_type.tag != TypeTag.STRUCT:
                raise TypeError(f"expected {TypeTag.STRUCT} for {base_type}, got {base_type.tag}")
            bench_type.extend_type(base_type)
        file.append_statement(bench_type)

        cls.__getitem__ = lambda self, key: getattr(self, key, None)
        cls.__setitem__ = lambda self, key, value: setattr(self, key, value)
        cls.__contains__ = lambda self, key: hasattr(self, key) and getattr(self, key) is not None
        cls.instantiate_from = lambda value: unpack_value(value, bench_type)
        cls.strip = lambda self: pack_value(self, bench_type)

        if return_type:
            return bench_type
        else:
            return cls

    return decorator


@typing.dataclass_transform()
def x_task(
    name: str, text: str, *, file: File
) -> typing.Callable[[typing.Callable], typing.Callable]:
    def decorator(fn):
        from bench.language.statement import Task

        task = Task(name=name, text=text)
        file.append_statement(task)
        task_type = type_from_instance_type(fn, name=None)
        task._take_fields_from(task_type, reset_id=False)
        return task

    return decorator


@typing.dataclass_transform()
def x_tag(name: str, text: str, *, file: File) -> typing.Callable[[typing.Type], typing.Type]:
    def decorator(cls):
        from bench.language.statement import Tag

        # also turn tag into dataclass, it's basically a struct
        cls = dataclass(cls)
        tag = Tag(name=name, text=text)
        file.append_statement(tag)
        tag_type = type_from_instance_type(cls, name=None)
        tag._take_fields_from(tag_type, reset_id=False)
        return cls

    return decorator


_model_impls: dict[str, typing.Callable] = {}
_model_compilers: dict[str, typing.Callable] = {}


@typing.dataclass_transform()
def x_model(
    name: str, text: str, *, external_name: str, file: File
) -> typing.Callable[[typing.Type], typing.Type]:
    def decorator(cls):
        from bench.language.statement import Model

        model = Model(name=name, external_name=external_name, text=text)
        file.append_statement(model)
        model_type = type_from_instance_type(cls._endpoint, name=None)
        model._take_fields_from(model_type, reset_id=True)

        _model_impls[model.path] = cls._endpoint
        _model_compilers[model.path] = cls._compiler
        return cls

    return decorator


_symbolx_reflect = File(name="reflect", module=symbolx_lib)
symbolx_lib.add_file(_symbolx_reflect)

reflect_enum = functools.partial(x_enum, file=_symbolx_reflect)
reflect_struct = functools.partial(x_struct, file=_symbolx_reflect)
reflect_struct = typing.dataclass_transform()(reflect_struct)


# defined here to avoid import cycles


@reflect_struct("FieldMetadata", "Default metadata for any field")
class FieldMetadata:
    pass


@reflect_struct("TaggingMetadata", "Default metadata for any tagging")
class TaggingMetadata:
    pass


@reflect_struct("RunMetadata", "Default metadata for any run")
class RunMetadata:
    name: Optional[str]
    test: Optional[bool]
    queue_position: Optional[int]
    cached_at: Optional[datetime]
    cached_in: Optional[UUID]
    cached_duration: Optional[float]
    progress: Optional[float]
    retries: Optional[int]
    retry: Optional[int]
    batch_size: Optional[int]
    nonce: Optional[str]
    keys: Optional[list[str]]
    verdict: Optional[str]
    verdict_reason: Optional[str]
