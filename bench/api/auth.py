import dataclasses
from typing import Any

import strawberry
import structlog
from graphql import GraphQLResolveInfo
from social_django.strategy import DjangoStrategy
from strawberry import Private
from strawberry.schema_directive import Location
from strawberry_django_plus.permissions import ConditionDirective
from strawberry_django_plus.utils.typing import UserType

from bench.models import User

logger = structlog.get_logger(__name__)


@strawberry.schema_directive(
    locations=[Location.FIELD_DEFINITION],
    description="Can only be resolved by user with project view permission",
)
class CanViewProjectDirective(ConditionDirective):
    message: Private[str] = dataclasses.field(default="User cannot view this.")

    def check_condition(
        self, root: Any, info: GraphQLResolveInfo, user: UserType, **kwargs
    ) -> bool:
        return False


@strawberry.schema_directive(
    locations=[Location.FIELD_DEFINITION],
    description="Can only be resolved by user with project write permission",
)
class CanWriteProjectDirective(ConditionDirective):
    message: Private[str] = dataclasses.field(default="User cannot write to this.")

    def check_condition(
        self, root: Any, info: GraphQLResolveInfo, user: UserType, **kwargs
    ) -> bool:
        return False


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
