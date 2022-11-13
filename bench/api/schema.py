from __future__ import annotations

from typing import Optional

import strawberry
from strawberry.extensions import QueryDepthLimiter
from strawberry_django_plus import gql
from strawberry_django_plus.directives import SchemaDirectiveExtension
from strawberry_django_plus.optimizer import DjangoOptimizerExtension

from bench.api.types import Organization, Project


@strawberry.type
class Query:
    project: Optional[Project] = gql.relay.node()
    projects_connection: gql.relay.Connection[Project] = gql.relay.connection()
    organization: Optional[Organization] = gql.relay.node()
    organizations_connection: gql.relay.Connection[Organization] = gql.relay.connection()


schema = strawberry.Schema(
    Query,
    extensions=[
        DjangoOptimizerExtension,
        QueryDepthLimiter(max_depth=10),
        SchemaDirectiveExtension,
    ],
)
