from typing import Optional, Type, Union, cast

from rest_framework import routers
from rest_framework.routers import SimpleRouter
from rest_framework.viewsets import ViewSetMixin
from rest_framework_nested.routers import IDENTIFIER_REGEX


def _get_lookup_regex_nested(viewset: Type[ViewSetMixin], lookup: tuple[str, ...]) -> str:
    if not lookup:
        raise ValueError("can't use lookup_omit_field when lookup_prefix is not set")

    nested_lookup_regexes = []
    # assemble lookups from partial lookups
    for lookup_key in lookup:
        # simpler lookup regex that does not include the lookup_field
        # so e.g. instead of /artifacts/{artifact_name}/versions/{version_version}
        # it just becomes /artifacts/{artifact}/versions/{version}
        base_regex = r"(?P<{lookup_prefix}>{lookup_value})"
        lookup_value = getattr(viewset, "lookup_value_regex", "[^/.]+")
        # noinspection StrFormat
        lookup_regex = base_regex.format(lookup_prefix=lookup_key, lookup_value=lookup_value)
        nested_lookup_regexes.append(lookup_regex)

    combined_lookup_regex = "/".join(nested_lookup_regexes)
    return combined_lookup_regex


class ExtendedDefaultRouter(routers.DefaultRouter):
    def __init__(self, *args, lookup_omit_field: bool, **kwargs):
        super().__init__(*args, **kwargs)
        self.lookup_omit_field = lookup_omit_field
        self.trailing_slash = r"/?"
        self.child_routers: list[ExtendedNestedRouter] = []

    def register_nested(
        self,
        prefix: str,
        viewset: Type[ViewSetMixin],
        lookup: Union[str, tuple[str, ...]],
    ) -> "ExtendedNestedRouter":
        # convert multi-lookups to prefix so that we get /artifacts/<organization> instead of just /artifacts
        # TODO @Cleanup: nested routes are a messy mix of custom hacks
        #  See both the hack below to get /artifacts/<org> endpoints and the
        #  _get_lookup_regex_nested impl. and call sites for /artifacts/<org>/<name> endpoints.
        if isinstance(lookup, tuple):
            prefix = r"{prefix}/(?P<{lookup_prefix}>{lookup_value})".format(
                prefix=prefix, lookup_prefix=lookup[0], lookup_value=r"[\w.\-]+"
            )
            lookup = lookup[1:]

        self.register(prefix, viewset)
        nested_router = ExtendedNestedRouter(self, parent_prefix=prefix, lookup=lookup)
        self.child_routers.append(nested_router)
        return nested_router

    def get_lookup_regex(
        self, viewset: Type[ViewSetMixin], lookup_prefix: Union[str, tuple[str, ...]] = ""
    ) -> str:
        if lookup_prefix:
            if isinstance(lookup_prefix, str):
                lookup_prefix = (lookup_prefix,)
            return _get_lookup_regex_nested(viewset, lookup_prefix)
        else:
            return super().get_lookup_regex(viewset)

    @property
    def descendant_routers(self):
        def _get_descendants(child_router: Union[ExtendedDefaultRouter, ExtendedNestedRouter]):
            yield from child_router.child_routers
            for grandchild_router in child_router.child_routers:
                yield from _get_descendants(grandchild_router)

        return list(_get_descendants(self))


class ExtendedNestedRouter(SimpleRouter):
    """
    Adapted version of rest_framework_nested.NestedMixin & NestedSimpleRouter to enable multiple parent lookups

    e.g. /artifacts/<organization>/<model>
    """

    def __init__(
        self,
        parent_router,
        parent_prefix,
        lookup: Union[str, tuple[str, ...]],
    ):
        super().__init__()
        self.parent_router = parent_router
        self.parent_prefix = parent_prefix
        self.nest_count = getattr(parent_router, "nest_count", 0) + 1

        # coerce lookup to tuple[str, ...]
        if isinstance(lookup, str):
            self.lookup = cast(tuple[str, ...], (lookup,))
        else:
            self.lookup = lookup

        parent_registry = [
            registered
            for registered in self.parent_router.registry
            if registered[0] == self.parent_prefix
        ]
        try:
            parent_registry = parent_registry[0]
            parent_prefix, parent_viewset, parent_basename = parent_registry
        except ValueError:
            raise RuntimeError("parent registered resource not found")

        for lookup_key in self.lookup:
            self.check_valid_name(lookup_key)

        nested_routes = []
        parent_lookup_regex = _get_lookup_regex_nested(parent_viewset, self.lookup)

        self.parent_regex = "{parent_prefix}/{parent_lookup_regex}/".format(
            parent_prefix=parent_prefix, parent_lookup_regex=parent_lookup_regex
        )
        # If there is no parent prefix, the first part of the url is probably
        #   controlled by the project's urls.py and the router is in an app,
        #   so a slash in the beginning will (A) cause Django to give warnings
        #   and (B) generate URLs that will require using `//`
        if not self.parent_prefix and self.parent_regex[0] == "/":
            self.parent_regex = self.parent_regex[1:]
        if hasattr(parent_router, "parent_regex"):
            self.parent_regex = parent_router.parent_regex + self.parent_regex

        for route in self.routes:
            route_contents = route._asdict()

            # This will get passed through .format in a little bit, so we need
            # to escape it
            escaped_parent_regex = self.parent_regex.replace("{", "{{").replace("}", "}}")

            route_contents["url"] = route.url.replace("^", "^" + escaped_parent_regex)
            nested_routes.append(type(route)(**route_contents))

        self.routes = nested_routes
        self.trailing_slash = r"/?"
        self.child_routers: list[ExtendedNestedRouter] = []

    def register(
        self,
        prefix: str,
        viewset: Type[ViewSetMixin],
        basename: Optional[str] = None,
        base_name: Optional[str] = None,
    ) -> None:
        basename = basename or base_name or f"{self.parent_prefix}_{prefix}"
        super().register(prefix, viewset, basename)

    def register_nested(
        self,
        prefix: str,
        viewset: Type[ViewSetMixin],
        lookup: Union[str, tuple[str, ...]],
        basename: str = None,
        lookup_omit_field: bool = None,
    ) -> "ExtendedNestedRouter":
        basename = basename or f"{self.parent_prefix}_{prefix}"
        self.register(prefix, viewset, basename)
        nested_router = ExtendedNestedRouter(self, parent_prefix=prefix, lookup=lookup)
        self.child_routers.append(nested_router)
        return nested_router

    def check_valid_name(self, value):
        if IDENTIFIER_REGEX.match(value) is None:
            raise ValueError(
                "lookup argument '{}' needs to be valid python identifier".format(value)
            )
