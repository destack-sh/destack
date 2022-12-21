from typing import Optional, Type, Union

import strawberry
from graphql import NoSchemaIntrospectionCustomRule
from strawberry.extensions import AddValidationRules, Extension, ParserCache, QueryDepthLimiter
from strawberry_django_plus import gql
from strawberry_django_plus.directives import SchemaDirectiveExtension
from strawberry_django_plus.optimizer import DjangoOptimizerExtension

from bench import models
from bench.api.code import Code, CodeMutation, CodeRunMutation
from bench.api.compilation import CompilationMutation
from bench.api.dataset import Dataset
from bench.api.expectation import Expectation, ExpectationMutation
from bench.api.model import Model
from bench.api.organization import Organization
from bench.api.project import File, FileMutation, Project, ProjectVersion, ProjectVersionMutation
from bench.api.schema import Schema
from bench.api.symbol import Statement, StatementMutation
from bench.api.task import Task, TaskMutation
from bench.api.user import User
from bench.settings import DEBUG, TEST


@strawberry.type
class Query:
    user: Optional[User] = gql.relay.node()
    users: gql.relay.Connection[User] = gql.relay.connection()
    organization: Optional[Organization] = gql.relay.node()
    organizationBySlug: Optional[Organization] = gql.django.field(
        resolver=models.Organization.objects.get_by_slug
    )
    project: Optional[Project] = gql.relay.node()
    projectBySlug: Optional[Project] = gql.django.field(resolver=models.Project.objects.get_by_slug)
    projects: gql.relay.Connection[Project] = gql.relay.connection()
    projectVersion: Optional[ProjectVersion] = gql.relay.node()
    file: Optional[File] = gql.relay.node()
    statement: Optional[Statement] = gql.relay.node()


@strawberry.type
class Mutation(
    StatementMutation,
    FileMutation,
    TaskMutation,
    ExpectationMutation,
    CodeMutation,
    CodeRunMutation,
    CompilationMutation,
    ProjectVersionMutation,
):
    pass


default_extensions: list[Union[Type[Extension], Extension]] = [
    DjangoOptimizerExtension,
    QueryDepthLimiter(max_depth=10),
    SchemaDirectiveExtension,
]
prod_extensions: list[Union[Type[Extension], Extension]] = [
    ParserCache(),
    AddValidationRules([NoSchemaIntrospectionCustomRule]),
]
if DEBUG or TEST:
    extensions = default_extensions
else:
    extensions = default_extensions + prod_extensions

schema = strawberry.Schema(
    Query,
    Mutation,
    extensions=extensions,
    # add interface implementation types explicitly
    types=[Schema, Task, Expectation, Code, Model, Dataset],
)
