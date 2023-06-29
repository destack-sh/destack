import functools
import typing
from dataclasses import dataclass

from bench.bench.const import TypeTag
from bench.bench.core import File
from bench.bench.type import instantiate_py_value, strip_py_value, type_from_py_type


def x_enum(name: str, *, file: File):
    """Map a Python type into a Bench type."""

    def decorator(cls):
        # check that names and values are equal
        for name, value in cls.__members__.items():
            if name != value.name:
                raise ValueError(f"name must equal value in {cls}: {name} != {value.name}")
        bench_type = type_from_py_type(cls, name=name)
        if bench_type.tag != TypeTag.ENUM:
            raise TypeError(f"expected enum, got {bench_type.tag}")
        file.append(bench_type)
        return cls

    return decorator


def x_struct(name: str, *, file: File):
    def decorator(cls):
        # turn it into dataclass that behaves like a dict
        # it needs to be a dataclass for getattr and getitem access
        # and it needs to be a real distinct class to type map references to it properly
        cls = dataclass(cls)
        bench_type = type_from_py_type(cls, name=name)
        if bench_type.tag != TypeTag.STRUCT:
            raise TypeError(f"Expected {TypeTag.STRUCT}, got {bench_type.tag}")
        file.append(bench_type)

        cls.__getitem__ = lambda self, key: getattr(self, key, None)
        cls.__setitem__ = lambda self, key, value: setattr(self, key, value)
        cls.__contains__ = lambda self, key: hasattr(self, key) and getattr(self, key) is not None
        cls.instantiate_from = lambda value: instantiate_py_value(value, bench_type)
        cls.strip = lambda self: strip_py_value(self, bench_type)

        return cls

    return decorator


def x_task(name: str, *, file: File):
    def decorator(fn):
        from bench.bench.task import Task

        task = Task(name=name)
        task_type = type_from_py_type(fn, name=None)
        task.fields = task_type._copy_fields(to=task)
        file.append(task)
        return task

    return decorator


_model_impls: dict[str, typing.Callable] = {}


def x_model(name: str, *, external_name: str, file: File):
    def decorator(cls):
        from bench.bench.model import Model

        model = Model(name=name, external_name=external_name)
        model_type = type_from_py_type(cls._impl, name=None)
        model.fields = model_type._copy_fields(to=model)
        file.append(model)
        _model_impls[model.path] = cls._impl
        return cls

    return decorator


_symbolx_reflect = File(name="reflect")

reflect_enum = functools.partial(x_enum, file=_symbolx_reflect)
reflect_struct = functools.partial(x_struct, file=_symbolx_reflect)
reflect_task = functools.partial(x_task, file=_symbolx_reflect)
reflect_model = functools.partial(x_model, file=_symbolx_reflect)
