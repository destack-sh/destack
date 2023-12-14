import hashlib
import json
from typing import TYPE_CHECKING, Annotated, Optional
from uuid import uuid4

import strawberry
import strawberry_django
from strawberry import auto, lazy, relay
from strawberry.relay import GlobalID
from strawberry.scalars import JSON
from strawberry.types import Info
from strawberry_django.fields.types import OperationInfo

from bench import models
from bench.api.auth import CheckTarget, HasModuleAccess, check_module_access
from bench.api.utils import safe_mutation
from bench.models import ModuleAccessLevel

if TYPE_CHECKING:
    from bench.api.project import Project


def reveal_secret_value(root: models.Secret) -> str:
    return json.loads(root.value)  # :SecretJson


@strawberry_django.type(models.Secret)
class Secret(relay.Node):
    created_at: auto
    updated_at: auto
    sha512: str
    name: Optional[str]
    project: Annotated["Project", lazy(".project")]
    value_revealed: JSON = strawberry_django.field(
        resolver=reveal_secret_value,
        extensions=[
            # note that this doesn't seem to be working so the entire secret is >=Edit level
            HasModuleAccess(
                level=ModuleAccessLevel.Edit, target=CheckTarget.ROOT, map=lambda s: s.project
            )
        ],
    )


@strawberry.input
class SecretCreateInput:
    name: Optional[str]
    value: JSON
    project_id: GlobalID


@strawberry.input
class SecretUpdateInput(strawberry_django.NodeInput):
    name: Optional[str]
    value: JSON


@strawberry.input
class SecretDeleteInput(strawberry_django.NodeInput):
    pass


@strawberry.type
class SecretMutation:
    @safe_mutation
    def create_secret(self, info: Info, input: SecretCreateInput) -> Secret | OperationInfo:
        project = models.Project.objects.get(id=input.project_id.node_id)
        check_module_access(info, project, ModuleAccessLevel.Edit)
        value_str = json.dumps(input.value, indent=0)  # :SecretJson
        sha512 = hashlib.sha512(value_str.encode("utf-8")).hexdigest()
        secret_id = uuid4()
        secret = models.Secret.objects.create(
            id=secret_id,
            ck=secret_id,
            project=project,
            name=input.name,
            value=value_str,
            sha512=sha512,
        )
        return secret

    @safe_mutation
    def update_secret(self, info: Info, input: SecretUpdateInput) -> Secret | OperationInfo:
        secret = models.Secret.objects.get(id=input.id.node_id)
        check_module_access(info, secret.project_id, ModuleAccessLevel.Edit)
        secret.name = input.name
        secret.value = json.dumps(input.value, indent=0)
        secret.sha512 = hashlib.sha512(secret.value.encode("utf-8")).hexdigest()
        secret.save()
        return secret

    @safe_mutation
    def delete_secret(self, info: Info, input: SecretDeleteInput) -> None | OperationInfo:
        secret = models.Secret.objects.get(id=input.id.node_id)
        check_module_access(info, secret.project_id, ModuleAccessLevel.Edit)
        secret.delete()
        return None
