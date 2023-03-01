import abc
import dataclasses
import functools
from typing import Any, Callable, cast

import strawberry
import structlog
from graphql import GraphQLResolveInfo
from social_django.strategy import DjangoStrategy
from strawberry import Private
from strawberry.channels import StrawberryChannelsContext
from strawberry.schema_directive import Location
from strawberry_django_plus.directives import SchemaDirectiveHelper
from strawberry_django_plus.permissions import (
    ConditionDirective,
    _user_ensured_attr,
    get_user_or_anonymous,
)
from strawberry_django_plus.utils import aio
from strawberry_django_plus.utils.typing import UserType

from bench.models import User

logger = structlog.get_logger(__name__)


class ChannelsConditionDirective(ConditionDirective, abc.ABC):
    def resolve(
        self,
        helper: SchemaDirectiveHelper,
        _next: Callable,
        root: Any,
        info: GraphQLResolveInfo,
        *args,
        **kwargs,
    ):
        # exactly like AuthDirective.resolve, but get user from channels context
        context = cast(StrawberryChannelsContext, info.context)
        resolver = functools.partial(_next, root, info, *args, **kwargs)

        user = cast(UserType, context.request.scope["user"]._wrapped)
        if not getattr(context, _user_ensured_attr, False):
            return aio.resolve(
                cast(UserType, get_user_or_anonymous(user)),
                functools.partial(
                    self.resolve_for_user,
                    helper,
                    resolver,
                    root,
                    info,
                    **kwargs,
                ),
                info=info,
            )

        return self.resolve_for_user(
            helper,
            resolver,
            root,
            info,
            cast(UserType, user),
            **kwargs,
        )


@strawberry.schema_directive(
    locations=[Location.FIELD_DEFINITION],
    description="Can only be resolved by user with project view permission",
)
class CanViewProject(ChannelsConditionDirective):
    message: Private[str] = dataclasses.field(default="User cannot view this.")

    def check_condition(
        self, root: Any, info: GraphQLResolveInfo, user: UserType, **kwargs
    ) -> bool:
        return True  # TODO @Auth: actually check view permission


@strawberry.schema_directive(
    locations=[Location.FIELD_DEFINITION],
    description="Can only be resolved by user with project write permission",
)
class CanWriteProject(ChannelsConditionDirective):
    message: Private[str] = dataclasses.field(default="User cannot write to this.")

    def check_condition(
        self, root: Any, info: GraphQLResolveInfo, user: UserType, **kwargs
    ) -> bool:
        return True  # TODO @Auth: actually check write permission


def social_create_user(strategy: DjangoStrategy, details, backend, user=None, *args, **kwargs):
    if user:
        return {"is_new": False}

    username = details.get("username")
    email = details["email"][0] if isinstance(details["email"], (list, tuple)) else details["email"]
    full_name = (
        details.get("fullname")
        or f"{details.get('first_name') or ''} {details.get('last_name') or ''}".strip()
        or details.get("username")
    )
    # incomplete signup, need to set more properties (like username)
    user = User.objects.create_user(username, email, full_name, completed_signup=False)
    strategy.session_set("backend", backend.name)

    logger.info("social_create_user", user=user)
    return {"is_new": True, "user": user}
