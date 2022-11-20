from __future__ import annotations

import uuid
from typing import Optional

import strawberry
from strawberry.extensions import QueryDepthLimiter
from strawberry_django_plus import gql
from strawberry_django_plus.directives import SchemaDirectiveExtension
from strawberry_django_plus.optimizer import DjangoOptimizerExtension

from bench import models
from bench.api.types import Organization, Project, ProjectVersion, User


@strawberry.type
class Query:
    user: Optional[User] = gql.relay.node()
    users: gql.relay.Connection[User] = gql.relay.connection()
    project: Optional[Project] = gql.relay.node()
    projectBySlug: Optional[Project] = gql.django.field(resolver=models.Project.objects.get_by_slug)
    projects: gql.relay.Connection[Project] = gql.relay.connection()
    projectVersion: Optional[ProjectVersion] = gql.relay.node()
    organization: Optional[Organization] = gql.relay.node()
    organizationBySlug: Optional[Organization] = gql.django.field(
        resolver=models.Organization.objects.get_by_slug
    )
    organizations: gql.relay.Connection[Organization] = gql.relay.connection()


@strawberry.type
class Mutation:
    @strawberry.mutation
    def compile_task(self, task_id: uuid.UUID) -> None:
        pass

    @strawberry.mutation
    def run_program(self, project_id: uuid.UUID) -> None:
        pass


schema = strawberry.Schema(
    Query,
    Mutation,
    extensions=[
        DjangoOptimizerExtension,
        QueryDepthLimiter(max_depth=10),
        SchemaDirectiveExtension,
    ],
)
