
# type: ignore
# ruff: noqa

from typing import TYPE_CHECKING, Union, AsyncIterator, Mapping

if TYPE_CHECKING:
    from destack.language import Session, Session, IsSubject, Client



from . import common_pb2 as _common_pb2
from . import language_pb2 as _language_pb2
from google.protobuf.internal import containers as _containers
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from typing import ClassVar as _ClassVar, Iterable as _Iterable, Mapping as _Mapping, Optional as _Optional, Union as _Union

DESCRIPTOR: _descriptor.FileDescriptor

class SignupUserRequest(_message.Message):
    __slots__ = ("slug", "name", "email", "password", "region", "client")
    SLUG_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    EMAIL_FIELD_NUMBER: _ClassVar[int]
    PASSWORD_FIELD_NUMBER: _ClassVar[int]
    REGION_FIELD_NUMBER: _ClassVar[int]
    CLIENT_FIELD_NUMBER: _ClassVar[int]
    slug: str
    name: str
    email: str
    password: str
    region: _language_pb2.Region
    client: _language_pb2.ClientData
    def __init__(self, slug: _Optional[str] = ..., name: _Optional[str] = ..., email: _Optional[str] = ..., password: _Optional[str] = ..., region: _Optional[_Union[_language_pb2.Region, str]] = ..., client: _Optional[_Union[_language_pb2.ClientData, _Mapping]] = ...) -> None: ...

class SignupUserResponse(_message.Message):
    __slots__ = ("user", "client", "space", "access_token")
    USER_FIELD_NUMBER: _ClassVar[int]
    CLIENT_FIELD_NUMBER: _ClassVar[int]
    SPACE_FIELD_NUMBER: _ClassVar[int]
    ACCESS_TOKEN_FIELD_NUMBER: _ClassVar[int]
    user: _language_pb2.UserData
    client: _language_pb2.ClientData
    space: _language_pb2.SpaceData
    access_token: str
    def __init__(self, user: _Optional[_Union[_language_pb2.UserData, _Mapping]] = ..., client: _Optional[_Union[_language_pb2.ClientData, _Mapping]] = ..., space: _Optional[_Union[_language_pb2.SpaceData, _Mapping]] = ..., access_token: _Optional[str] = ...) -> None: ...

class ChangeUserPasswordRequest(_message.Message):
    __slots__ = ("old_password", "new_password")
    OLD_PASSWORD_FIELD_NUMBER: _ClassVar[int]
    NEW_PASSWORD_FIELD_NUMBER: _ClassVar[int]
    old_password: str
    new_password: str
    def __init__(self, old_password: _Optional[str] = ..., new_password: _Optional[str] = ...) -> None: ...

class ChangeUserPasswordResponse(_message.Message):
    __slots__ = ("user", "client", "epoch")
    USER_FIELD_NUMBER: _ClassVar[int]
    CLIENT_FIELD_NUMBER: _ClassVar[int]
    EPOCH_FIELD_NUMBER: _ClassVar[int]
    user: _language_pb2.UserData
    client: _language_pb2.ClientData
    epoch: int
    def __init__(self, user: _Optional[_Union[_language_pb2.UserData, _Mapping]] = ..., client: _Optional[_Union[_language_pb2.ClientData, _Mapping]] = ..., epoch: _Optional[int] = ...) -> None: ...

class LoginUserRequest(_message.Message):
    __slots__ = ("id", "slug", "email", "password", "client")
    ID_FIELD_NUMBER: _ClassVar[int]
    SLUG_FIELD_NUMBER: _ClassVar[int]
    EMAIL_FIELD_NUMBER: _ClassVar[int]
    PASSWORD_FIELD_NUMBER: _ClassVar[int]
    CLIENT_FIELD_NUMBER: _ClassVar[int]
    id: str
    slug: str
    email: str
    password: str
    client: _language_pb2.ClientData
    def __init__(self, id: _Optional[str] = ..., slug: _Optional[str] = ..., email: _Optional[str] = ..., password: _Optional[str] = ..., client: _Optional[_Union[_language_pb2.ClientData, _Mapping]] = ...) -> None: ...

class LoginUserResponse(_message.Message):
    __slots__ = ("user", "client", "access_token")
    USER_FIELD_NUMBER: _ClassVar[int]
    CLIENT_FIELD_NUMBER: _ClassVar[int]
    ACCESS_TOKEN_FIELD_NUMBER: _ClassVar[int]
    user: _language_pb2.UserData
    client: _language_pb2.ClientData
    access_token: str
    def __init__(self, user: _Optional[_Union[_language_pb2.UserData, _Mapping]] = ..., client: _Optional[_Union[_language_pb2.ClientData, _Mapping]] = ..., access_token: _Optional[str] = ...) -> None: ...

class LogoutUserRequest(_message.Message):
    __slots__ = ("clients", "logout_all")
    CLIENTS_FIELD_NUMBER: _ClassVar[int]
    LOGOUT_ALL_FIELD_NUMBER: _ClassVar[int]
    clients: _containers.RepeatedCompositeFieldContainer[_language_pb2.NodeReferenceData]
    logout_all: bool
    def __init__(self, clients: _Optional[_Iterable[_Union[_language_pb2.NodeReferenceData, _Mapping]]] = ..., logout_all: bool = ...) -> None: ...

class LogoutUserResponse(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class CreateOrganizationRequest(_message.Message):
    __slots__ = ("organization",)
    ORGANIZATION_FIELD_NUMBER: _ClassVar[int]
    organization: _language_pb2.OrganizationData
    def __init__(self, organization: _Optional[_Union[_language_pb2.OrganizationData, _Mapping]] = ...) -> None: ...

class CreateOrganizationResponse(_message.Message):
    __slots__ = ("organization", "epoch")
    ORGANIZATION_FIELD_NUMBER: _ClassVar[int]
    EPOCH_FIELD_NUMBER: _ClassVar[int]
    organization: _language_pb2.OrganizationData
    epoch: int
    def __init__(self, organization: _Optional[_Union[_language_pb2.OrganizationData, _Mapping]] = ..., epoch: _Optional[int] = ...) -> None: ...

class ResolveSpacesRequest(_message.Message):
    __slots__ = ("spaces",)
    class SpaceKey(_message.Message):
        __slots__ = ("id", "slug")
        ID_FIELD_NUMBER: _ClassVar[int]
        SLUG_FIELD_NUMBER: _ClassVar[int]
        id: str
        slug: str
        def __init__(self, id: _Optional[str] = ..., slug: _Optional[str] = ...) -> None: ...
    SPACES_FIELD_NUMBER: _ClassVar[int]
    spaces: _containers.RepeatedCompositeFieldContainer[ResolveSpacesRequest.SpaceKey]
    def __init__(self, spaces: _Optional[_Iterable[_Union[ResolveSpacesRequest.SpaceKey, _Mapping]]] = ...) -> None: ...

class ResolveSpacesResponse(_message.Message):
    __slots__ = ("spaces",)
    class SpaceInfo(_message.Message):
        __slots__ = ("domain", "grpc_port", "grpc_web_port", "ssl", "space")
        DOMAIN_FIELD_NUMBER: _ClassVar[int]
        GRPC_PORT_FIELD_NUMBER: _ClassVar[int]
        GRPC_WEB_PORT_FIELD_NUMBER: _ClassVar[int]
        SSL_FIELD_NUMBER: _ClassVar[int]
        SPACE_FIELD_NUMBER: _ClassVar[int]
        domain: str
        grpc_port: int
        grpc_web_port: int
        ssl: bool
        space: _language_pb2.NodeReferenceData
        def __init__(self, domain: _Optional[str] = ..., grpc_port: _Optional[int] = ..., grpc_web_port: _Optional[int] = ..., ssl: bool = ..., space: _Optional[_Union[_language_pb2.NodeReferenceData, _Mapping]] = ...) -> None: ...
    SPACES_FIELD_NUMBER: _ClassVar[int]
    spaces: _containers.RepeatedCompositeFieldContainer[ResolveSpacesResponse.SpaceInfo]
    def __init__(self, spaces: _Optional[_Iterable[_Union[ResolveSpacesResponse.SpaceInfo, _Mapping]]] = ...) -> None: ...
