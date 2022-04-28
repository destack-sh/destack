from typing import Any, Callable, Dict, Generic, Optional, Sequence, Tuple, TypeVar

import structlog


class RegistryError(ValueError):
    pass


T = TypeVar("T")

Mapper = Callable[[Any, Optional[T]], T]


class Registry(Generic[T]):
    """
    A generic object registry, optionally namespaced and with registrar mapper.
    """

    def __init__(self, namespace: Sequence[str] = (), mapper: Optional[Mapper] = None):
        self.namespace: Tuple[str, ...] = tuple(namespace)
        self.logger = structlog.stdlib.get_logger(namespace=namespace)
        self._mapper = mapper
        self._registered_objects: Dict[Tuple[str, ...], T] = {}

    def _get_key(self, name: str) -> Tuple[str, ...]:
        return self.namespace + (name,)

    def _do_register(self, name: str, obj: Any, impl: Optional[T]):
        if self._mapper is not None:
            obj = self._mapper(obj, impl or None)

        key = self._get_key(name)
        if name in self._registered_objects:
            raise RegistryError(
                f"name {name} is already registered in {self.namespace}"
            )
        self._registered_objects[key] = obj
        self.logger.info("register", name=name, obj=obj, impl=impl)
        return obj

    def register(
        self, name: str, *, impl: Optional[T] = None, obj: Optional[Any] = None
    ):
        if obj is not None:
            return self._do_register(name, obj=obj, impl=impl)

        def _do_registration(obj: Any):
            return self._do_register(name, obj=obj, impl=impl)

        return _do_registration

    def get(self, name: str) -> Optional[T]:
        return self._registered_objects.get(self._get_key(name))

    def __getitem__(self, item: str) -> T:
        obj = self.get(item)
        if obj is None:
            raise RegistryError(f"object {item} does not exist")
        return obj


def get_qualified_name(obj: Any) -> str:
    return ".".join([obj.__module__, obj.__name__])
