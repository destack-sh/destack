import hashlib
import json
from typing import TYPE_CHECKING, Annotated, Optional

from strawberry import lazy
from strawberry.scalars import JSON
from strawberry.types import Info
from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID
from strawberry_django_plus.types import OperationInfo

from bench import models
from bench.api.auth import check_can_write_project
from bench.api.util import safe_mutation

if TYPE_CHECKING:
    from bench.api.project import Project


def reveal_secret_value(root: models.Secret) -> str:
    return json.loads(root.value)


@gql.django.type(models.Secret)
class Secret(gql.Node):
    sha512: str
    name: Optional[str]
    project: Annotated["Project", lazy(".project")]
    value_revealed: JSON = gql.field(resolver=reveal_secret_value)


@gql.input
class SecretCreateInput:
    name: Optional[str]
    value: JSON
    project_id: GlobalID


@gql.input
class SecretUpdateInput(gql.NodeInput):
    name: Optional[str]
    value: JSON


@gql.input
class SecretDeleteInput(gql.NodeInput):
    pass


@gql.type
class SecretMutation:
    @safe_mutation
    def create_secret(self, info: Info, input: SecretCreateInput) -> Secret | OperationInfo:
        project = models.Project.objects.get(id=input.project_id.node_id)
        check_can_write_project(info, project)
        value_str = json.dumps(input.value, indent=0)
        sha512 = hashlib.sha512(value_str.encode("utf-8")).hexdigest()
        secret = models.Secret.objects.create(
            project=project, name=input.name, value=value_str, sha512=sha512
        )
        return secret

    @safe_mutation
    def update_secret(self, info: Info, input: SecretUpdateInput) -> Secret | OperationInfo:
        secret = models.Secret.objects.get(id=input.id.node_id)
        check_can_write_project(info, secret.project)
        secret.name = input.name
        secret.value = json.dumps(input.value, indent=0)
        secret.sha512 = hashlib.sha512(secret.value.encode("utf-8")).hexdigest()
        secret.save()
        return secret

    @safe_mutation
    def delete_secret(self, info: Info, input: SecretDeleteInput) -> None | OperationInfo:
        secret = models.Secret.objects.get(id=input.id.node_id)
        check_can_write_project(info, secret.project)
        secret.delete()
        return None
