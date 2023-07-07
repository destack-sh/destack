import functools
import typing
from dataclasses import dataclass
from uuid import UUID, uuid5

from bench.bench.const import TypeTag
from bench.bench.core import File
from bench.bench.type import instantiate_py_value, strip_py_value, type_from_py_type

_UUID_VERSION_KEY = UUID("00000000-0000-0000-0000-000000000000")


def _versioned_id(path: str) -> UUID:
    # we assign stable ids to all reflected types based on their path
    return uuid5(_UUID_VERSION_KEY, path)


def x_enum(name: str, description: str, *, file: File):
    """Map a Python type into a Bench type."""

    def decorator(cls):
        # check that names and values are equal
        for n, value in cls.__members__.items():
            if n != value.name:
                raise ValueError(f"name must equal value in {cls}: {n} != {value.name}")
        bench_type = type_from_py_type(cls, name=name)
        bench_type.description = description
        if bench_type.tag != TypeTag.ENUM:
            raise TypeError(f"expected enum, got {bench_type.tag}")
        file.append(bench_type)
        bench_type.id = _versioned_id(bench_type.path)
        return cls

    return decorator


def x_struct(name: str, description: str, *, file: File):
    def decorator(cls):
        # turn it into dataclass that behaves like a dict
        # it needs to be a dataclass for getattr and getitem access
        # and it needs to be a real distinct class to type map references to it properly
        cls = dataclass(cls)
        bench_type = type_from_py_type(cls, name=name)
        bench_type.description = description
        if bench_type.tag != TypeTag.STRUCT:
            raise TypeError(f"Expected {TypeTag.STRUCT}, got {bench_type.tag}")
        file.append(bench_type)
        bench_type.id = _versioned_id(bench_type.path)

        cls.__getitem__ = lambda self, key: getattr(self, key, None)
        cls.__setitem__ = lambda self, key, value: setattr(self, key, value)
        cls.__contains__ = lambda self, key: hasattr(self, key) and getattr(self, key) is not None
        cls.instantiate_from = lambda value: instantiate_py_value(value, bench_type)
        cls.strip = lambda self: strip_py_value(self, bench_type)

        return cls

    return decorator


def x_task(name: str, description: str, *, file: File):
    def decorator(fn):
        from bench.bench.task import Task

        task = Task(name=name, description=description)
        file.append(task)
        task.id = _versioned_id(task.path)
        task_type = type_from_py_type(fn, name=None)
        task.fields = task_type._copy_fields(to=task)
        return task

    return decorator


def x_tag(name: str, description: str, key: str, *, file: File):
    def decorator(cls):
        from bench.bench.tag import Tag

        # also turn tag into dataclass, it's basically a struct
        cls = dataclass(cls)
        tag = Tag(name=name, key=key, description=description)
        file.append(tag)
        tag.id = _versioned_id(tag.path)
        tag_type = type_from_py_type(cls, name=None)
        tag.fields = tag_type._copy_fields(to=tag)
        return cls

    return decorator


_model_impls: dict[str, typing.Callable] = {}
_model_compilers: dict[str, typing.Callable] = {}


def x_model(name: str, description: str, *, external_name: str, file: File):
    def decorator(cls):
        from bench.bench.model import Model

        model = Model(name=name, external_name=external_name, description=description)
        model.id = _versioned_id(model.path)
        file.append(model)
        model_type = type_from_py_type(cls._endpoint, name=None)
        model.fields = model_type._copy_fields(to=model)

        _model_impls[model.path] = cls._endpoint
        _model_compilers[model.path] = cls._compiler
        return cls

    return decorator


_symbolx_reflect = File(name="reflect")

reflect_enum = functools.partial(x_enum, file=_symbolx_reflect)
reflect_struct = functools.partial(x_struct, file=_symbolx_reflect)
reflect_task = functools.partial(x_task, file=_symbolx_reflect)
reflect_model = functools.partial(x_model, file=_symbolx_reflect)
