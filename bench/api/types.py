from __future__ import annotations

from strawberry import auto
from strawberry_django_plus import gql

from bench import models


@gql.django.type(models.User)
class User(gql.relay.Node):
    username: auto
    first_name: auto
    last_name: auto
    email: auto
    created_at: auto
    updated_at: auto
    organizations: list[Organization]


@gql.django.type(models.Organization)
class Organization(gql.relay.Node):
    name: auto
    slug: auto
    created_at: auto
    updated_at: auto
    projects: list[Project]
    members: list[User]


@gql.django.type(models.Project)
class Project(gql.Node):
    name: auto
    slug: auto
    organization: Organization = gql.django.field()
    created_at: auto
    updated_at: auto
