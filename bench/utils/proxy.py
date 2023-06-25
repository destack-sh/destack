from typing import Any, Callable, Collection, Mapping


def _curry_path(onfn: Callable[[str], None], key: str) -> Callable[[str], Any]:
    return lambda path: onfn(f"{key}.{path}")


def proxy_value(value: Any, onread: Callable[[str], None], onwrite: Callable[[str], None]) -> Any:
    """Recursively proxy the given value, calling onread/onwrite when a key is accessed."""
    if isinstance(value, dict):
        return ProxyDict(value, onread, onwrite)
    elif isinstance(value, list):
        return ProxyList(value, onread, onwrite)
    else:
        return value


def unproxy_value(value: Any) -> Any:
    """Recursively unproxy the given value."""
    if isinstance(value, ProxyDict):
        return {key: unproxy_value(value) for key, value in value.items()}
    elif isinstance(value, ProxyList):
        return [unproxy_value(value) for value in value]
    else:
        return value


class ProxyDict(Mapping):
    """Proxy a dict, calling onread/onwrite when a key is accessed."""

    def __init__(self, inner: dict, onread: Callable[[str], None], onwrite: Callable[[str], None]):
        self._inner = inner
        self._onread = onread
        self._onwrite = onwrite

    def __str__(self):
        return str(self._inner)

    def __repr__(self):
        return f"<ProxyDict {self._inner}>"

    def items(self):
        self._onread("")
        return self._inner.items()

    def keys(self):
        self._onread("")
        return self._inner.keys()

    def values(self):
        self._onread("")
        return self._inner.values()

    def __getitem__(self, key: str) -> Any:
        self._onread(key)
        return self._inner[key]

    def __setitem__(self, key: str, value: Any) -> None:
        value = proxy_value(value, _curry_path(self._onread, key), _curry_path(self._onwrite, key))
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

    # for typed dicts

    def __setattr__(self, item, value):
        if item in ("_inner", "_onread", "_onwrite"):
            return super().__setattr__(item, value)
        super().__setattr__(self._inner, item, value)
        self._onwrite(item)


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
        i = len(self._inner)
        value = proxy_value(
            value, _curry_path(self._onread, str(i)), _curry_path(self._onwrite, str(i))
        )
        self._inner.append(value)
        self._onwrite("")

    def extend(self, value: Any) -> None:
        for v in value:
            i = len(self._inner)
            v = proxy_value(
                v, _curry_path(self._onread, str(i)), _curry_path(self._onwrite, str(i))
            )
            self._inner.append(v)
        self._onwrite("")
