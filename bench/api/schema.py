from __future__ import annotations

from typing import Optional

import strawberry
from strawberry.extensions import QueryDepthLimiter
from strawberry_django_plus import gql
from strawberry_django_plus.directives import SchemaDirectiveExtension
from strawberry_django_plus.optimizer import DjangoOptimizerExtension

from bench import models
from bench.api.types import Organization, Project, User


@strawberry.type
class Query:
    user: Optional[User] = gql.relay.node()
    users: gql.relay.Connection[User] = gql.relay.connection()
    project: Optional[Project] = gql.relay.node()
    projectBySlug: Optional[Project] = gql.django.field(resolver=models.Project.objects.get_by_slug)
    projects: gql.relay.Connection[Project] = gql.relay.connection()
    organization: Optional[Organization] = gql.relay.node()
    organizations: gql.relay.Connection[Organization] = gql.relay.connection()


schema = strawberry.Schema(
    Query,
    extensions=[
        DjangoOptimizerExtension,
        QueryDepthLimiter(max_depth=10),
        SchemaDirectiveExtension,
    ],
)
