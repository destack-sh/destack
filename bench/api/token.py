from datetime import datetime
from typing import Annotated, Optional, Union

import strawberry
import strawberry_django
from django.core.exceptions import PermissionDenied
from strawberry import auto, lazy, relay
from strawberry.relay import GlobalID
from strawberry_django.fields.types import OperationInfo

from bench import models
from bench.api.auth import CheckTarget, HasModuleAccess, is_owner_or_member
from bench.api.utils import get_user_from_info, safe_mutation
from bench.models import ModuleAccessLevel, Organization, User

AccessTokenScope = strawberry.enum(models.AccessTokenScope)
AccessTokenStatus = strawberry.enum(models.AccessTokenStatus)


def reveal_access_token_value(root: models.AccessToken) -> str:
    return root.value


@strawberry_django.type(models.AccessToken)
class AccessToken(relay.Node):
    name: auto
    token_key: str
    created_at: auto
    updated_at: auto
    expires_at: auto
    revoked_at: auto
    status: AccessTokenStatus
    scopes: list[AccessTokenScope]
    owner: Union[Annotated["User", lazy(".user")], Annotated["Organization", lazy(".organization")]]
    value_revealed: str = strawberry_django.field(
        resolver=reveal_access_token_value,
        extensions=[
            # note that this doesn't seem to be working so the entire secret is >=Edit level
            HasModuleAccess(
                level=ModuleAccessLevel.Edit, target=CheckTarget.ROOT, map=lambda s: s.project
            )
        ],
    )


@strawberry.input
class AccessTokenCreateInput:
    owner_id: GlobalID
    scopes: list[AccessTokenScope]
    expires_at: Optional[datetime] = None
    name: Optional[str] = None


@strawberry.type
class AccessTokenCreatePayload:
    access_token: AccessToken
    token: str


@strawberry.type
class AccessTokenMutation:
    @safe_mutation
    def create_access_token(
        self, info, input: AccessTokenCreateInput
    ) -> AccessTokenCreatePayload | OperationInfo:
        requesting_user = get_user_from_info(info)
        owner = input.owner_id.resolve_node_sync(info, required=True)
        if not is_owner_or_member(requesting_user, owner):
            raise PermissionDenied("cannot create access token for this owner")
        access_token, raw_token = models.AccessToken.objects.create_token(
            owner=owner, scopes=input.scopes, expires_at=input.expires_at, name=input.name
        )
        return AccessTokenCreatePayload(access_token=access_token, token=raw_token)

    @safe_mutation
    def revoke_access_token(self, info, id: GlobalID) -> AccessToken | OperationInfo:
        requesting_user = get_user_from_info(info)
        access_token = models.AccessToken.objects.get(id=id.node_id)
        if not is_owner_or_member(requesting_user, access_token.owner):
            raise PermissionDenied("cannot revoke access token for this owner")
        access_token.revoke()
        return access_token
