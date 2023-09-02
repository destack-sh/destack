import functools
import typing
from dataclasses import dataclass
from uuid import UUID, uuid5

from bench.language.basic import BENCH_UUID_NAMESPACE
from bench.language.builtin import symbolx_lib
from bench.language.const import TypeTag
from bench.language.core import File
from bench.language.type import instantiate_value, strip_value, type_from_instance_type


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
            raise TypeError(f"Expected {TypeTag.STRUCT}, got {bench_type.tag}")
        file.append_statement(bench_type)

        cls.__getitem__ = lambda self, key: getattr(self, key, None)
        cls.__setitem__ = lambda self, key, value: setattr(self, key, value)
        cls.__contains__ = lambda self, key: hasattr(self, key) and getattr(self, key) is not None
        cls.instantiate_from = lambda value: instantiate_value(value, bench_type)
        cls.strip = lambda self: strip_value(self, bench_type)

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
        from bench.language.task import Task

        task = Task(name=name, text=text)
        file.append_statement(task)
        task_type = type_from_instance_type(fn, name=None)
        task._take_fields_from(task_type, reset_id=False)
        return task

    return decorator


@typing.dataclass_transform()
def x_tag(name: str, text: str, *, file: File) -> typing.Callable[[typing.Type], typing.Type]:
    def decorator(cls):
        from bench.language.tag import Tag

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
        from bench.language.model import Model

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
reflect_task = functools.partial(x_task, file=_symbolx_reflect)
reflect_model = functools.partial(x_model, file=_symbolx_reflect)
