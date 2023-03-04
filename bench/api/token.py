from datetime import datetime
from typing import Annotated, Optional, Union

from django.core.exceptions import PermissionDenied
from strawberry import lazy
from strawberry_django_plus import gql
from strawberry_django_plus.gql import auto
from strawberry_django_plus.relay import GlobalID
from strawberry_django_plus.types import OperationInfo

from bench import models
from bench.api.auth import is_owner_or_member
from bench.api.util import safe_mutation
from bench.models import Organization, User

AccessTokenScope = gql.enum(models.AccessTokenScope)
AccessTokenStatus = gql.enum(models.AccessTokenStatus)


@gql.django.type(models.AccessToken)
class AccessToken(gql.Node):
    token: Optional[str]
    name: auto
    token_key: str
    created_at: auto
    updated_at: auto
    expires_at: auto
    revoked_at: auto
    status: AccessTokenStatus
    scopes: list[AccessTokenScope]
    owner: Union[Annotated["User", lazy(".user")], Annotated["Organization", lazy(".organization")]]


@gql.input
class AccessTokenCreateInput:
    owner_id: GlobalID
    scopes: list[AccessTokenScope]
    expires_at: Optional[datetime] = None
    name: Optional[str] = None


@gql.type
class AccessTokenCreatePayload:
    access_token: AccessToken
    token: str


@gql.type
class AccessTokenMutation:
    @safe_mutation
    def create_access_token(
        self, info, input: AccessTokenCreateInput
    ) -> AccessTokenCreatePayload | OperationInfo:
        requesting_user = info.context.request.scope["user"]
        owner = input.owner_id.resolve_node(info, required=True)
        if not is_owner_or_member(requesting_user, owner):
            raise PermissionDenied("cannot create access token for this owner")
        access_token, raw_token = models.AccessToken.objects.create_token(
            owner=owner, scopes=input.scopes, expires_at=input.expires_at, name=input.name
        )
        return AccessTokenCreatePayload(access_token=access_token, token=raw_token)

    @safe_mutation
    def revoke_access_token(self, info, id: GlobalID) -> AccessToken | OperationInfo:
        requesting_user = info.context.request.scope["user"]
        access_token = models.AccessToken.objects.get(id=id.node_id)
        if not is_owner_or_member(requesting_user, access_token.owner):
            raise PermissionDenied("cannot revoke access token for this owner")
        access_token.revoke()
        return access_token
