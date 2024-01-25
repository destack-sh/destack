from typing import Any, Callable, Collection, Mapping


def _curry_path(onfn: Callable[[str], None], key: str) -> Callable[[str], Any]:
    return lambda path: onfn(f"{key}.{path}")


def proxy_value(
    value: Any,
    onwrite: Callable[[str], None],
    default_none: bool = False,
) -> Any:
    """Recursively proxy the given value, calling onread/onwrite when a key is accessed."""
    if isinstance(value, dict):
        return ProxyDict(value, onwrite, default_none=default_none)
    elif isinstance(value, list):
        return ProxyList(value, onwrite)
    else:
        return value


def unproxy_value(value: Any) -> Any:
    """Recursively unproxy the given value."""
    if isinstance(value, ProxyDict):
        return value._inner
    elif isinstance(value, ProxyList):
        return value._inner
    else:
        return value


class ProxyDict(Mapping):
    """Proxy a dict, behave as a type dict, calling onread/onwrite when a key is accessed."""

    def __init__(
        self,
        inner: dict,
        onwrite: Callable[[str], None],
        default_none: bool = False,
    ):
        self._inner = inner
        self._onwrite = onwrite
        self._default_none = default_none

    def __str__(self):
        return str(self._inner)

    def __repr__(self):
        return f"<ProxyDict {self._inner}>"

    def items(self):
        return self._inner.items()

    def keys(self):
        return self._inner.keys()

    def values(self):
        return self._inner.values()

    def __getitem__(self, key: str) -> Any:
        return self._inner[key]

    def update(self, other: dict) -> None:
        for key, value in other.items():
            self[key] = value

    def __setitem__(self, key: str, value: Any) -> None:
        value = proxy_value(value, _curry_path(self._onwrite, key))
        self._inner[key] = value
        self._onwrite(key)

    def __delitem__(self, key: str) -> None:
        del self._inner[key]
        self._onwrite(key)

    def __contains__(self, key: str) -> bool:
        self._onread(key)
        return key in self._inner

    def __len__(self):
        self._onread("")
        return len(self._inner)

    def __iter__(self):
        self._onread("")
        return iter(self._inner)

    # dot dict

    def __setattr__(self, item, value):
        if item in ("_inner", "_onread", "_onwrite", "_default_none"):
            return super().__setattr__(item, value)
        self._inner[item] = value
        self._onwrite(item)

    def __getattr__(self, name):
        try:
            return self[name]
        except KeyError:
            if self._default_none:
                return None
            raise AttributeError(name)


class ProxyList(Collection):
    """Proxy a list, calling onread and onwrite when a key is accessed."""

    def __init__(self, inner: list, onread: Callable[[str], None], onwrite: Callable[[str], None]):
        self._inner = inner
        self._onread = onread
        self._onwrite = onwrite

    def __str__(self):
        return str(self._inner)

    def __repr__(self):
        return f"<ProxyList {self._inner}>"

    def __delitem__(self, key: int) -> None:
        self._onwrite(str(key))
        del self._inner[key]

    def __contains__(self, key: int) -> bool:
        self._onread(str(key))
        return key in self._inner

    def __getitem__(self, item: int | slice) -> Any:
        self._onread(str(item))
        return self._inner[item]

    def __len__(self):
        self._onread("")
        return len(self._inner)

    def __iter__(self):
        self._onread("")
        return iter(self._inner)

    def append(self, value: Any) -> None:
        len(self._inner)
        self._inner.append(value)
        self._onwrite("")

    def extend(self, value: Any) -> None:
        for v in value:
            len(self._inner)
            self._inner.append(v)
        self._onwrite("")
