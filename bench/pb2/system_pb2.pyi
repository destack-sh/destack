
# type: ignore
# ruff: noqa

from typing import TYPE_CHECKING, Union, AsyncIterator, Mapping



from google.protobuf import struct_pb2 as _struct_pb2
from . import common_pb2 as _common_pb2
from . import lang_pb2 as _lang_pb2
from google.protobuf.internal import containers as _containers
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from typing import ClassVar as _ClassVar, Iterable as _Iterable, Mapping as _Mapping, Optional as _Optional, Union as _Union

DESCRIPTOR: _descriptor.FileDescriptor

class ClientDataIn(_message.Message):
    __slots__ = ("id", "type", "name", "device_type", "device_name", "operating_system", "browser_name", "browser_version", "place_id", "access_token", "space_ptr")
    ID_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    DEVICE_TYPE_FIELD_NUMBER: _ClassVar[int]
    DEVICE_NAME_FIELD_NUMBER: _ClassVar[int]
    OPERATING_SYSTEM_FIELD_NUMBER: _ClassVar[int]
    BROWSER_NAME_FIELD_NUMBER: _ClassVar[int]
    BROWSER_VERSION_FIELD_NUMBER: _ClassVar[int]
    PLACE_ID_FIELD_NUMBER: _ClassVar[int]
    ACCESS_TOKEN_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    id: str
    type: _lang_pb2.ClientType
    name: str
    device_type: str
    device_name: str
    operating_system: str
    browser_name: str
    browser_version: str
    place_id: str
    access_token: str
    space_ptr: _lang_pb2.NodeReferenceData
    def __init__(self, id: _Optional[str] = ..., type: _Optional[_Union[_lang_pb2.ClientType, str]] = ..., name: _Optional[str] = ..., device_type: _Optional[str] = ..., device_name: _Optional[str] = ..., operating_system: _Optional[str] = ..., browser_name: _Optional[str] = ..., browser_version: _Optional[str] = ..., place_id: _Optional[str] = ..., access_token: _Optional[str] = ..., space_ptr: _Optional[_Union[_lang_pb2.NodeReferenceData, _Mapping]] = ...) -> None: ...

class SignupUserRequest(_message.Message):
    __slots__ = ("id", "slug", "name", "email", "password", "region", "client", "activate")
    ID_FIELD_NUMBER: _ClassVar[int]
    SLUG_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    EMAIL_FIELD_NUMBER: _ClassVar[int]
    PASSWORD_FIELD_NUMBER: _ClassVar[int]
    REGION_FIELD_NUMBER: _ClassVar[int]
    CLIENT_FIELD_NUMBER: _ClassVar[int]
    ACTIVATE_FIELD_NUMBER: _ClassVar[int]
    id: str
    slug: str
    name: str
    email: str
    password: str
    region: _lang_pb2.Region
    client: ClientDataIn
    activate: bool
    def __init__(self, id: _Optional[str] = ..., slug: _Optional[str] = ..., name: _Optional[str] = ..., email: _Optional[str] = ..., password: _Optional[str] = ..., region: _Optional[_Union[_lang_pb2.Region, str]] = ..., client: _Optional[_Union[ClientDataIn, _Mapping]] = ..., activate: bool = ...) -> None: ...

class SignupUserResponse(_message.Message):
    __slots__ = ("user", "client", "access_token")
    USER_FIELD_NUMBER: _ClassVar[int]
    CLIENT_FIELD_NUMBER: _ClassVar[int]
    ACCESS_TOKEN_FIELD_NUMBER: _ClassVar[int]
    user: _lang_pb2.UserData
    client: _lang_pb2.ClientData
    access_token: str
    def __init__(self, user: _Optional[_Union[_lang_pb2.UserData, _Mapping]] = ..., client: _Optional[_Union[_lang_pb2.ClientData, _Mapping]] = ..., access_token: _Optional[str] = ...) -> None: ...

class ChangeUserPasswordRequest(_message.Message):
    __slots__ = ("old_password", "new_password")
    OLD_PASSWORD_FIELD_NUMBER: _ClassVar[int]
    NEW_PASSWORD_FIELD_NUMBER: _ClassVar[int]
    old_password: str
    new_password: str
    def __init__(self, old_password: _Optional[str] = ..., new_password: _Optional[str] = ...) -> None: ...

class ChangeUserPasswordResponse(_message.Message):
    __slots__ = ("user", "epoch")
    USER_FIELD_NUMBER: _ClassVar[int]
    EPOCH_FIELD_NUMBER: _ClassVar[int]
    user: _lang_pb2.UserData
    epoch: int
    def __init__(self, user: _Optional[_Union[_lang_pb2.UserData, _Mapping]] = ..., epoch: _Optional[int] = ...) -> None: ...

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
    client: ClientDataIn
    def __init__(self, id: _Optional[str] = ..., slug: _Optional[str] = ..., email: _Optional[str] = ..., password: _Optional[str] = ..., client: _Optional[_Union[ClientDataIn, _Mapping]] = ...) -> None: ...

class LoginUserResponse(_message.Message):
    __slots__ = ("user", "client", "access_token")
    USER_FIELD_NUMBER: _ClassVar[int]
    CLIENT_FIELD_NUMBER: _ClassVar[int]
    ACCESS_TOKEN_FIELD_NUMBER: _ClassVar[int]
    user: _lang_pb2.UserData
    client: _lang_pb2.ClientData
    access_token: str
    def __init__(self, user: _Optional[_Union[_lang_pb2.UserData, _Mapping]] = ..., client: _Optional[_Union[_lang_pb2.ClientData, _Mapping]] = ..., access_token: _Optional[str] = ...) -> None: ...

class LogoutUserRequest(_message.Message):
    __slots__ = ("clients", "logout_all")
    CLIENTS_FIELD_NUMBER: _ClassVar[int]
    LOGOUT_ALL_FIELD_NUMBER: _ClassVar[int]
    clients: _containers.RepeatedCompositeFieldContainer[_lang_pb2.NodeReferenceData]
    logout_all: bool
    def __init__(self, clients: _Optional[_Iterable[_Union[_lang_pb2.NodeReferenceData, _Mapping]]] = ..., logout_all: bool = ...) -> None: ...

class LogoutUserResponse(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class CreateOrganizationRequest(_message.Message):
    __slots__ = ("organization",)
    ORGANIZATION_FIELD_NUMBER: _ClassVar[int]
    organization: _lang_pb2.OrganizationData
    def __init__(self, organization: _Optional[_Union[_lang_pb2.OrganizationData, _Mapping]] = ...) -> None: ...

class CreateOrganizationResponse(_message.Message):
    __slots__ = ("organization", "epoch")
    ORGANIZATION_FIELD_NUMBER: _ClassVar[int]
    EPOCH_FIELD_NUMBER: _ClassVar[int]
    organization: _lang_pb2.OrganizationData
    epoch: int
    def __init__(self, organization: _Optional[_Union[_lang_pb2.OrganizationData, _Mapping]] = ..., epoch: _Optional[int] = ...) -> None: ...

class CreateBenchRequest(_message.Message):
    __slots__ = ("owner", "slug", "region", "is_main")
    OWNER_FIELD_NUMBER: _ClassVar[int]
    SLUG_FIELD_NUMBER: _ClassVar[int]
    REGION_FIELD_NUMBER: _ClassVar[int]
    IS_MAIN_FIELD_NUMBER: _ClassVar[int]
    owner: _lang_pb2.NodeReferenceData
    slug: str
    region: _lang_pb2.Region
    is_main: bool
    def __init__(self, owner: _Optional[_Union[_lang_pb2.NodeReferenceData, _Mapping]] = ..., slug: _Optional[str] = ..., region: _Optional[_Union[_lang_pb2.Region, str]] = ..., is_main: bool = ...) -> None: ...

class CreateBenchResponse(_message.Message):
    __slots__ = ("bench",)
    BENCH_FIELD_NUMBER: _ClassVar[int]
    bench: _lang_pb2.BenchData
    def __init__(self, bench: _Optional[_Union[_lang_pb2.BenchData, _Mapping]] = ...) -> None: ...

class ResolveHostsRequest(_message.Message):
    __slots__ = ("benches",)
    class BenchKey(_message.Message):
        __slots__ = ("id", "slug")
        ID_FIELD_NUMBER: _ClassVar[int]
        SLUG_FIELD_NUMBER: _ClassVar[int]
        id: str
        slug: str
        def __init__(self, id: _Optional[str] = ..., slug: _Optional[str] = ...) -> None: ...
    BENCHES_FIELD_NUMBER: _ClassVar[int]
    benches: _containers.RepeatedCompositeFieldContainer[ResolveHostsRequest.BenchKey]
    def __init__(self, benches: _Optional[_Iterable[_Union[ResolveHostsRequest.BenchKey, _Mapping]]] = ...) -> None: ...

class ResolveHostsResponse(_message.Message):
    __slots__ = ("hosts",)
    class HostInfo(_message.Message):
        __slots__ = ("domain", "grpc_port", "grpc_web_port", "ssl", "bench")
        DOMAIN_FIELD_NUMBER: _ClassVar[int]
        GRPC_PORT_FIELD_NUMBER: _ClassVar[int]
        GRPC_WEB_PORT_FIELD_NUMBER: _ClassVar[int]
        SSL_FIELD_NUMBER: _ClassVar[int]
        BENCH_FIELD_NUMBER: _ClassVar[int]
        domain: str
        grpc_port: int
        grpc_web_port: int
        ssl: bool
        bench: _lang_pb2.NodeReferenceData
        def __init__(self, domain: _Optional[str] = ..., grpc_port: _Optional[int] = ..., grpc_web_port: _Optional[int] = ..., ssl: bool = ..., bench: _Optional[_Union[_lang_pb2.NodeReferenceData, _Mapping]] = ...) -> None: ...
    HOSTS_FIELD_NUMBER: _ClassVar[int]
    hosts: _containers.RepeatedCompositeFieldContainer[ResolveHostsResponse.HostInfo]
    def __init__(self, hosts: _Optional[_Iterable[_Union[ResolveHostsResponse.HostInfo, _Mapping]]] = ...) -> None: ...

class QueryRequest(_message.Message):
    __slots__ = ("scope", "query")
    SCOPE_FIELD_NUMBER: _ClassVar[int]
    QUERY_FIELD_NUMBER: _ClassVar[int]
    scope: _lang_pb2.ScopeData
    query: _lang_pb2.QueryData
    def __init__(self, scope: _Optional[_Union[_lang_pb2.ScopeData, _Mapping]] = ..., query: _Optional[_Union[_lang_pb2.QueryData, _Mapping]] = ...) -> None: ...

class QueryResponse(_message.Message):
    __slots__ = ("result",)
    RESULT_FIELD_NUMBER: _ClassVar[int]
    result: _lang_pb2.QueryResultData
    def __init__(self, result: _Optional[_Union[_lang_pb2.QueryResultData, _Mapping]] = ...) -> None: ...

class SubscribeRequest(_message.Message):
    __slots__ = ("query",)
    QUERY_FIELD_NUMBER: _ClassVar[int]
    query: _lang_pb2.QueryData
    def __init__(self, query: _Optional[_Union[_lang_pb2.QueryData, _Mapping]] = ...) -> None: ...

class SubscribeResponse(_message.Message):
    __slots__ = ("update",)
    UPDATE_FIELD_NUMBER: _ClassVar[int]
    update: _lang_pb2.QueryUpdateData
    def __init__(self, update: _Optional[_Union[_lang_pb2.QueryUpdateData, _Mapping]] = ...) -> None: ...

class CommitRequest(_message.Message):
    __slots__ = ("scope", "id", "edits")
    SCOPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    EDITS_FIELD_NUMBER: _ClassVar[int]
    scope: _lang_pb2.ScopeData
    id: str
    edits: _containers.RepeatedCompositeFieldContainer[_lang_pb2.EditData]
    def __init__(self, scope: _Optional[_Union[_lang_pb2.ScopeData, _Mapping]] = ..., id: _Optional[str] = ..., edits: _Optional[_Iterable[_Union[_lang_pb2.EditData, _Mapping]]] = ...) -> None: ...

class CommitResponse(_message.Message):
    __slots__ = ("edits", "cascaded_edits", "epoch")
    EDITS_FIELD_NUMBER: _ClassVar[int]
    CASCADED_EDITS_FIELD_NUMBER: _ClassVar[int]
    EPOCH_FIELD_NUMBER: _ClassVar[int]
    edits: _containers.RepeatedCompositeFieldContainer[_lang_pb2.EditData]
    cascaded_edits: _containers.RepeatedCompositeFieldContainer[_lang_pb2.EditData]
    epoch: int
    def __init__(self, edits: _Optional[_Iterable[_Union[_lang_pb2.EditData, _Mapping]]] = ..., cascaded_edits: _Optional[_Iterable[_Union[_lang_pb2.EditData, _Mapping]]] = ..., epoch: _Optional[int] = ...) -> None: ...

class UploadFilesRequest(_message.Message):
    __slots__ = ("scope", "files")
    SCOPE_FIELD_NUMBER: _ClassVar[int]
    FILES_FIELD_NUMBER: _ClassVar[int]
    scope: _lang_pb2.ScopeData
    files: _containers.RepeatedCompositeFieldContainer[_lang_pb2.FileData]
    def __init__(self, scope: _Optional[_Union[_lang_pb2.ScopeData, _Mapping]] = ..., files: _Optional[_Iterable[_Union[_lang_pb2.FileData, _Mapping]]] = ...) -> None: ...

class UploadFilesResponse(_message.Message):
    __slots__ = ("handles",)
    class UploadHandle(_message.Message):
        __slots__ = ("file", "post_url", "fields", "get_url")
        FILE_FIELD_NUMBER: _ClassVar[int]
        POST_URL_FIELD_NUMBER: _ClassVar[int]
        FIELDS_FIELD_NUMBER: _ClassVar[int]
        GET_URL_FIELD_NUMBER: _ClassVar[int]
        file: _lang_pb2.FileData
        post_url: str
        fields: _struct_pb2.Struct
        get_url: str
        def __init__(self, file: _Optional[_Union[_lang_pb2.FileData, _Mapping]] = ..., post_url: _Optional[str] = ..., fields: _Optional[_Union[_struct_pb2.Struct, _Mapping]] = ..., get_url: _Optional[str] = ...) -> None: ...
    HANDLES_FIELD_NUMBER: _ClassVar[int]
    handles: _containers.RepeatedCompositeFieldContainer[UploadFilesResponse.UploadHandle]
    def __init__(self, handles: _Optional[_Iterable[_Union[UploadFilesResponse.UploadHandle, _Mapping]]] = ...) -> None: ...

class DownloadFilesRequest(_message.Message):
    __slots__ = ("scope", "files")
    SCOPE_FIELD_NUMBER: _ClassVar[int]
    FILES_FIELD_NUMBER: _ClassVar[int]
    scope: _lang_pb2.ScopeData
    files: _containers.RepeatedCompositeFieldContainer[_lang_pb2.NodeReferenceData]
    def __init__(self, scope: _Optional[_Union[_lang_pb2.ScopeData, _Mapping]] = ..., files: _Optional[_Iterable[_Union[_lang_pb2.NodeReferenceData, _Mapping]]] = ...) -> None: ...

class DownloadFilesResponse(_message.Message):
    __slots__ = ("handles",)
    class DownloadHandle(_message.Message):
        __slots__ = ("file", "get_url")
        FILE_FIELD_NUMBER: _ClassVar[int]
        GET_URL_FIELD_NUMBER: _ClassVar[int]
        file: _lang_pb2.FileData
        get_url: str
        def __init__(self, file: _Optional[_Union[_lang_pb2.FileData, _Mapping]] = ..., get_url: _Optional[str] = ...) -> None: ...
    HANDLES_FIELD_NUMBER: _ClassVar[int]
    handles: _containers.RepeatedCompositeFieldContainer[DownloadFilesResponse.DownloadHandle]
    def __init__(self, handles: _Optional[_Iterable[_Union[DownloadFilesResponse.DownloadHandle, _Mapping]]] = ...) -> None: ...
