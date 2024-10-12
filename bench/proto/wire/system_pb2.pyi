# type: ignore
# ruff: noqa

from typing import TYPE_CHECKING, Union, AsyncIterator, Mapping

from google.protobuf import struct_pb2 as _struct_pb2
from . import common_pb2 as _common_pb2
from . import lang_pb2 as _lang_pb2
from google.protobuf.internal import containers as _containers
from google.protobuf.internal import enum_type_wrapper as _enum_type_wrapper
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from typing import (
    ClassVar as _ClassVar,
    Iterable as _Iterable,
    Mapping as _Mapping,
    Optional as _Optional,
    Union as _Union,
)

DESCRIPTOR: _descriptor.FileDescriptor

class MachineEnvironment(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    REGULAR: _ClassVar[MachineEnvironment]
    DOCKER: _ClassVar[MachineEnvironment]
    MINIKUBE: _ClassVar[MachineEnvironment]

REGULAR: MachineEnvironment
DOCKER: MachineEnvironment
MINIKUBE: MachineEnvironment

class GetNodesRequest(_message.Message):
    __slots__ = (
        "scope",
        "roots",
        "block_ptr",
        "ancestor_types",
        "descendant_types",
        "select",
        "include_deleted",
        "no_cache",
        "is_optional",
    )
    SCOPE_FIELD_NUMBER: _ClassVar[int]
    ROOTS_FIELD_NUMBER: _ClassVar[int]
    BLOCK_PTR_FIELD_NUMBER: _ClassVar[int]
    ANCESTOR_TYPES_FIELD_NUMBER: _ClassVar[int]
    DESCENDANT_TYPES_FIELD_NUMBER: _ClassVar[int]
    SELECT_FIELD_NUMBER: _ClassVar[int]
    INCLUDE_DELETED_FIELD_NUMBER: _ClassVar[int]
    NO_CACHE_FIELD_NUMBER: _ClassVar[int]
    IS_OPTIONAL_FIELD_NUMBER: _ClassVar[int]
    scope: _lang_pb2.GraphScopeData
    roots: _containers.RepeatedCompositeFieldContainer[_lang_pb2.NodeReferenceData]
    block_ptr: _lang_pb2.NodeReferenceData
    ancestor_types: _containers.RepeatedScalarFieldContainer[_lang_pb2.NodeType]
    descendant_types: _containers.RepeatedScalarFieldContainer[_lang_pb2.NodeType]
    select: _lang_pb2.SelectOptionsData
    include_deleted: bool
    no_cache: bool
    is_optional: bool
    def __init__(
        self,
        scope: _Optional[_Union[_lang_pb2.GraphScopeData, _Mapping]] = ...,
        roots: _Optional[_Iterable[_Union[_lang_pb2.NodeReferenceData, _Mapping]]] = ...,
        block_ptr: _Optional[_Union[_lang_pb2.NodeReferenceData, _Mapping]] = ...,
        ancestor_types: _Optional[_Iterable[_Union[_lang_pb2.NodeType, str]]] = ...,
        descendant_types: _Optional[_Iterable[_Union[_lang_pb2.NodeType, str]]] = ...,
        select: _Optional[_Union[_lang_pb2.SelectOptionsData, _Mapping]] = ...,
        include_deleted: bool = ...,
        no_cache: bool = ...,
        is_optional: bool = ...,
    ) -> None: ...

class GetNodesResponse(_message.Message):
    __slots__ = ("nodes", "epoch", "connection_token")
    NODES_FIELD_NUMBER: _ClassVar[int]
    EPOCH_FIELD_NUMBER: _ClassVar[int]
    CONNECTION_TOKEN_FIELD_NUMBER: _ClassVar[int]
    nodes: _containers.RepeatedCompositeFieldContainer[_lang_pb2.SomeNodeData]
    epoch: int
    connection_token: str
    def __init__(
        self,
        nodes: _Optional[_Iterable[_Union[_lang_pb2.SomeNodeData, _Mapping]]] = ...,
        epoch: _Optional[int] = ...,
        connection_token: _Optional[str] = ...,
    ) -> None: ...

class WatchGetRequest(_message.Message):
    __slots__ = ("scope", "connection_token", "since_epoch")
    SCOPE_FIELD_NUMBER: _ClassVar[int]
    CONNECTION_TOKEN_FIELD_NUMBER: _ClassVar[int]
    SINCE_EPOCH_FIELD_NUMBER: _ClassVar[int]
    scope: _lang_pb2.GraphScopeData
    connection_token: str
    since_epoch: int
    def __init__(
        self,
        scope: _Optional[_Union[_lang_pb2.GraphScopeData, _Mapping]] = ...,
        connection_token: _Optional[str] = ...,
        since_epoch: _Optional[int] = ...,
    ) -> None: ...

class WatchGetResponse(_message.Message):
    __slots__ = ("edits", "cascaded_edits", "added_nodes", "removed_nodes_ptr", "epoch")
    EDITS_FIELD_NUMBER: _ClassVar[int]
    CASCADED_EDITS_FIELD_NUMBER: _ClassVar[int]
    ADDED_NODES_FIELD_NUMBER: _ClassVar[int]
    REMOVED_NODES_PTR_FIELD_NUMBER: _ClassVar[int]
    EPOCH_FIELD_NUMBER: _ClassVar[int]
    edits: _containers.RepeatedCompositeFieldContainer[_lang_pb2.EditData]
    cascaded_edits: _containers.RepeatedCompositeFieldContainer[_lang_pb2.EditData]
    added_nodes: _containers.RepeatedCompositeFieldContainer[_lang_pb2.SomeNodeData]
    removed_nodes_ptr: _containers.RepeatedCompositeFieldContainer[_lang_pb2.NodeReferenceData]
    epoch: int
    def __init__(
        self,
        edits: _Optional[_Iterable[_Union[_lang_pb2.EditData, _Mapping]]] = ...,
        cascaded_edits: _Optional[_Iterable[_Union[_lang_pb2.EditData, _Mapping]]] = ...,
        added_nodes: _Optional[_Iterable[_Union[_lang_pb2.SomeNodeData, _Mapping]]] = ...,
        removed_nodes_ptr: _Optional[
            _Iterable[_Union[_lang_pb2.NodeReferenceData, _Mapping]]
        ] = ...,
        epoch: _Optional[int] = ...,
    ) -> None: ...

class SearchNodesRequest(_message.Message):
    __slots__ = (
        "scope",
        "node_type",
        "block_ptr",
        "filter",
        "sort",
        "ancestor_types",
        "descendant_types",
        "first",
        "skip",
        "count",
        "select",
        "no_cache",
    )
    SCOPE_FIELD_NUMBER: _ClassVar[int]
    NODE_TYPE_FIELD_NUMBER: _ClassVar[int]
    BLOCK_PTR_FIELD_NUMBER: _ClassVar[int]
    FILTER_FIELD_NUMBER: _ClassVar[int]
    SORT_FIELD_NUMBER: _ClassVar[int]
    ANCESTOR_TYPES_FIELD_NUMBER: _ClassVar[int]
    DESCENDANT_TYPES_FIELD_NUMBER: _ClassVar[int]
    FIRST_FIELD_NUMBER: _ClassVar[int]
    SKIP_FIELD_NUMBER: _ClassVar[int]
    COUNT_FIELD_NUMBER: _ClassVar[int]
    SELECT_FIELD_NUMBER: _ClassVar[int]
    NO_CACHE_FIELD_NUMBER: _ClassVar[int]
    scope: _lang_pb2.GraphScopeData
    node_type: _lang_pb2.NodeType
    block_ptr: _lang_pb2.NodeReferenceData
    filter: _lang_pb2.ExpressionData
    sort: _containers.RepeatedCompositeFieldContainer[_lang_pb2.ExpressionData]
    ancestor_types: _containers.RepeatedScalarFieldContainer[_lang_pb2.NodeType]
    descendant_types: _containers.RepeatedScalarFieldContainer[_lang_pb2.NodeType]
    first: int
    skip: int
    count: bool
    select: _lang_pb2.SelectOptionsData
    no_cache: bool
    def __init__(
        self,
        scope: _Optional[_Union[_lang_pb2.GraphScopeData, _Mapping]] = ...,
        node_type: _Optional[_Union[_lang_pb2.NodeType, str]] = ...,
        block_ptr: _Optional[_Union[_lang_pb2.NodeReferenceData, _Mapping]] = ...,
        filter: _Optional[_Union[_lang_pb2.ExpressionData, _Mapping]] = ...,
        sort: _Optional[_Iterable[_Union[_lang_pb2.ExpressionData, _Mapping]]] = ...,
        ancestor_types: _Optional[_Iterable[_Union[_lang_pb2.NodeType, str]]] = ...,
        descendant_types: _Optional[_Iterable[_Union[_lang_pb2.NodeType, str]]] = ...,
        first: _Optional[int] = ...,
        skip: _Optional[int] = ...,
        count: bool = ...,
        select: _Optional[_Union[_lang_pb2.SelectOptionsData, _Mapping]] = ...,
        no_cache: bool = ...,
    ) -> None: ...

class SearchNodesResponse(_message.Message):
    __slots__ = ("roots_ptr", "nodes", "total", "epoch", "connection_token")
    ROOTS_PTR_FIELD_NUMBER: _ClassVar[int]
    NODES_FIELD_NUMBER: _ClassVar[int]
    TOTAL_FIELD_NUMBER: _ClassVar[int]
    EPOCH_FIELD_NUMBER: _ClassVar[int]
    CONNECTION_TOKEN_FIELD_NUMBER: _ClassVar[int]
    roots_ptr: _containers.RepeatedCompositeFieldContainer[_lang_pb2.NodeReferenceData]
    nodes: _containers.RepeatedCompositeFieldContainer[_lang_pb2.SomeNodeData]
    total: int
    epoch: int
    connection_token: str
    def __init__(
        self,
        roots_ptr: _Optional[_Iterable[_Union[_lang_pb2.NodeReferenceData, _Mapping]]] = ...,
        nodes: _Optional[_Iterable[_Union[_lang_pb2.SomeNodeData, _Mapping]]] = ...,
        total: _Optional[int] = ...,
        epoch: _Optional[int] = ...,
        connection_token: _Optional[str] = ...,
    ) -> None: ...

class WatchSearchRequest(_message.Message):
    __slots__ = ("scope", "connection_token", "since_epoch")
    SCOPE_FIELD_NUMBER: _ClassVar[int]
    CONNECTION_TOKEN_FIELD_NUMBER: _ClassVar[int]
    SINCE_EPOCH_FIELD_NUMBER: _ClassVar[int]
    scope: _lang_pb2.GraphScopeData
    connection_token: str
    since_epoch: int
    def __init__(
        self,
        scope: _Optional[_Union[_lang_pb2.GraphScopeData, _Mapping]] = ...,
        connection_token: _Optional[str] = ...,
        since_epoch: _Optional[int] = ...,
    ) -> None: ...

class WatchSearchResponse(_message.Message):
    __slots__ = (
        "edits",
        "cascaded_edits",
        "added_nodes",
        "removed_nodes_ptr",
        "roots_ptr",
        "total",
        "epoch",
    )
    EDITS_FIELD_NUMBER: _ClassVar[int]
    CASCADED_EDITS_FIELD_NUMBER: _ClassVar[int]
    ADDED_NODES_FIELD_NUMBER: _ClassVar[int]
    REMOVED_NODES_PTR_FIELD_NUMBER: _ClassVar[int]
    ROOTS_PTR_FIELD_NUMBER: _ClassVar[int]
    TOTAL_FIELD_NUMBER: _ClassVar[int]
    EPOCH_FIELD_NUMBER: _ClassVar[int]
    edits: _containers.RepeatedCompositeFieldContainer[_lang_pb2.EditData]
    cascaded_edits: _containers.RepeatedCompositeFieldContainer[_lang_pb2.EditData]
    added_nodes: _containers.RepeatedCompositeFieldContainer[_lang_pb2.SomeNodeData]
    removed_nodes_ptr: _containers.RepeatedCompositeFieldContainer[_lang_pb2.NodeReferenceData]
    roots_ptr: _containers.RepeatedCompositeFieldContainer[_lang_pb2.NodeReferenceData]
    total: int
    epoch: int
    def __init__(
        self,
        edits: _Optional[_Iterable[_Union[_lang_pb2.EditData, _Mapping]]] = ...,
        cascaded_edits: _Optional[_Iterable[_Union[_lang_pb2.EditData, _Mapping]]] = ...,
        added_nodes: _Optional[_Iterable[_Union[_lang_pb2.SomeNodeData, _Mapping]]] = ...,
        removed_nodes_ptr: _Optional[
            _Iterable[_Union[_lang_pb2.NodeReferenceData, _Mapping]]
        ] = ...,
        roots_ptr: _Optional[_Iterable[_Union[_lang_pb2.NodeReferenceData, _Mapping]]] = ...,
        total: _Optional[int] = ...,
        epoch: _Optional[int] = ...,
    ) -> None: ...

class AggregateNodesRequest(_message.Message):
    __slots__ = ("scope", "node_type", "block_ptr", "filter", "sort", "aggregation", "no_cache")
    SCOPE_FIELD_NUMBER: _ClassVar[int]
    NODE_TYPE_FIELD_NUMBER: _ClassVar[int]
    BLOCK_PTR_FIELD_NUMBER: _ClassVar[int]
    FILTER_FIELD_NUMBER: _ClassVar[int]
    SORT_FIELD_NUMBER: _ClassVar[int]
    AGGREGATION_FIELD_NUMBER: _ClassVar[int]
    NO_CACHE_FIELD_NUMBER: _ClassVar[int]
    scope: _lang_pb2.GraphScopeData
    node_type: _lang_pb2.NodeType
    block_ptr: _lang_pb2.NodeReferenceData
    filter: _lang_pb2.ExpressionData
    sort: _containers.RepeatedCompositeFieldContainer[_lang_pb2.ExpressionData]
    aggregation: _lang_pb2.ExpressionData
    no_cache: bool
    def __init__(
        self,
        scope: _Optional[_Union[_lang_pb2.GraphScopeData, _Mapping]] = ...,
        node_type: _Optional[_Union[_lang_pb2.NodeType, str]] = ...,
        block_ptr: _Optional[_Union[_lang_pb2.NodeReferenceData, _Mapping]] = ...,
        filter: _Optional[_Union[_lang_pb2.ExpressionData, _Mapping]] = ...,
        sort: _Optional[_Iterable[_Union[_lang_pb2.ExpressionData, _Mapping]]] = ...,
        aggregation: _Optional[_Union[_lang_pb2.ExpressionData, _Mapping]] = ...,
        no_cache: bool = ...,
    ) -> None: ...

class AggregateNodesResponse(_message.Message):
    __slots__ = ("scope", "connection_token", "aggregation", "epoch")
    SCOPE_FIELD_NUMBER: _ClassVar[int]
    CONNECTION_TOKEN_FIELD_NUMBER: _ClassVar[int]
    AGGREGATION_FIELD_NUMBER: _ClassVar[int]
    EPOCH_FIELD_NUMBER: _ClassVar[int]
    scope: _lang_pb2.GraphScopeData
    connection_token: str
    aggregation: _lang_pb2.AggregationData
    epoch: int
    def __init__(
        self,
        scope: _Optional[_Union[_lang_pb2.GraphScopeData, _Mapping]] = ...,
        connection_token: _Optional[str] = ...,
        aggregation: _Optional[_Union[_lang_pb2.AggregationData, _Mapping]] = ...,
        epoch: _Optional[int] = ...,
    ) -> None: ...

class WatchAggregateRequest(_message.Message):
    __slots__ = ("scope", "connection_token", "since_epoch")
    SCOPE_FIELD_NUMBER: _ClassVar[int]
    CONNECTION_TOKEN_FIELD_NUMBER: _ClassVar[int]
    SINCE_EPOCH_FIELD_NUMBER: _ClassVar[int]
    scope: _lang_pb2.GraphScopeData
    connection_token: str
    since_epoch: int
    def __init__(
        self,
        scope: _Optional[_Union[_lang_pb2.GraphScopeData, _Mapping]] = ...,
        connection_token: _Optional[str] = ...,
        since_epoch: _Optional[int] = ...,
    ) -> None: ...

class WatchAggregateResponse(_message.Message):
    __slots__ = ("aggregation", "epoch")
    AGGREGATION_FIELD_NUMBER: _ClassVar[int]
    EPOCH_FIELD_NUMBER: _ClassVar[int]
    aggregation: _lang_pb2.AggregationData
    epoch: int
    def __init__(
        self,
        aggregation: _Optional[_Union[_lang_pb2.AggregationData, _Mapping]] = ...,
        epoch: _Optional[int] = ...,
    ) -> None: ...

class CommitTransactionRequest(_message.Message):
    __slots__ = ("scope", "id", "edits", "context")
    SCOPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    EDITS_FIELD_NUMBER: _ClassVar[int]
    CONTEXT_FIELD_NUMBER: _ClassVar[int]
    scope: _lang_pb2.GraphScopeData
    id: str
    edits: _containers.RepeatedCompositeFieldContainer[_lang_pb2.EditData]
    context: _lang_pb2.SessionContextData
    def __init__(
        self,
        scope: _Optional[_Union[_lang_pb2.GraphScopeData, _Mapping]] = ...,
        id: _Optional[str] = ...,
        edits: _Optional[_Iterable[_Union[_lang_pb2.EditData, _Mapping]]] = ...,
        context: _Optional[_Union[_lang_pb2.SessionContextData, _Mapping]] = ...,
    ) -> None: ...

class CommitTransactionResponse(_message.Message):
    __slots__ = ("cascaded_edits", "epoch")
    CASCADED_EDITS_FIELD_NUMBER: _ClassVar[int]
    EPOCH_FIELD_NUMBER: _ClassVar[int]
    cascaded_edits: _containers.RepeatedCompositeFieldContainer[_lang_pb2.EditData]
    epoch: int
    def __init__(
        self,
        cascaded_edits: _Optional[_Iterable[_Union[_lang_pb2.EditData, _Mapping]]] = ...,
        epoch: _Optional[int] = ...,
    ) -> None: ...

class ClientDataIn(_message.Message):
    __slots__ = (
        "id",
        "type",
        "name",
        "device_type",
        "device_name",
        "operating_system",
        "browser_name",
        "browser_version",
        "place_id",
        "access_token",
        "space_ptr",
    )
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
    def __init__(
        self,
        id: _Optional[str] = ...,
        type: _Optional[_Union[_lang_pb2.ClientType, str]] = ...,
        name: _Optional[str] = ...,
        device_type: _Optional[str] = ...,
        device_name: _Optional[str] = ...,
        operating_system: _Optional[str] = ...,
        browser_name: _Optional[str] = ...,
        browser_version: _Optional[str] = ...,
        place_id: _Optional[str] = ...,
        access_token: _Optional[str] = ...,
        space_ptr: _Optional[_Union[_lang_pb2.NodeReferenceData, _Mapping]] = ...,
    ) -> None: ...

class SignupUserRequest(_message.Message):
    __slots__ = ("id", "slug", "name", "email", "password", "client")
    ID_FIELD_NUMBER: _ClassVar[int]
    SLUG_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    EMAIL_FIELD_NUMBER: _ClassVar[int]
    PASSWORD_FIELD_NUMBER: _ClassVar[int]
    CLIENT_FIELD_NUMBER: _ClassVar[int]
    id: str
    slug: str
    name: str
    email: str
    password: str
    client: ClientDataIn
    def __init__(
        self,
        id: _Optional[str] = ...,
        slug: _Optional[str] = ...,
        name: _Optional[str] = ...,
        email: _Optional[str] = ...,
        password: _Optional[str] = ...,
        client: _Optional[_Union[ClientDataIn, _Mapping]] = ...,
    ) -> None: ...

class SignupUserResponse(_message.Message):
    __slots__ = ("user", "client", "access_token")
    USER_FIELD_NUMBER: _ClassVar[int]
    CLIENT_FIELD_NUMBER: _ClassVar[int]
    ACCESS_TOKEN_FIELD_NUMBER: _ClassVar[int]
    user: _lang_pb2.UserData
    client: _lang_pb2.ClientData
    access_token: str
    def __init__(
        self,
        user: _Optional[_Union[_lang_pb2.UserData, _Mapping]] = ...,
        client: _Optional[_Union[_lang_pb2.ClientData, _Mapping]] = ...,
        access_token: _Optional[str] = ...,
    ) -> None: ...

class ChangeUserPasswordRequest(_message.Message):
    __slots__ = ("old_password", "new_password")
    OLD_PASSWORD_FIELD_NUMBER: _ClassVar[int]
    NEW_PASSWORD_FIELD_NUMBER: _ClassVar[int]
    old_password: str
    new_password: str
    def __init__(
        self, old_password: _Optional[str] = ..., new_password: _Optional[str] = ...
    ) -> None: ...

class ChangeUserPasswordResponse(_message.Message):
    __slots__ = ("user", "epoch")
    USER_FIELD_NUMBER: _ClassVar[int]
    EPOCH_FIELD_NUMBER: _ClassVar[int]
    user: _lang_pb2.UserData
    epoch: int
    def __init__(
        self,
        user: _Optional[_Union[_lang_pb2.UserData, _Mapping]] = ...,
        epoch: _Optional[int] = ...,
    ) -> None: ...

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
    def __init__(
        self,
        id: _Optional[str] = ...,
        slug: _Optional[str] = ...,
        email: _Optional[str] = ...,
        password: _Optional[str] = ...,
        client: _Optional[_Union[ClientDataIn, _Mapping]] = ...,
    ) -> None: ...

class LoginUserResponse(_message.Message):
    __slots__ = ("user", "client", "access_token")
    USER_FIELD_NUMBER: _ClassVar[int]
    CLIENT_FIELD_NUMBER: _ClassVar[int]
    ACCESS_TOKEN_FIELD_NUMBER: _ClassVar[int]
    user: _lang_pb2.UserData
    client: _lang_pb2.ClientData
    access_token: str
    def __init__(
        self,
        user: _Optional[_Union[_lang_pb2.UserData, _Mapping]] = ...,
        client: _Optional[_Union[_lang_pb2.ClientData, _Mapping]] = ...,
        access_token: _Optional[str] = ...,
    ) -> None: ...

class LogoutUserRequest(_message.Message):
    __slots__ = ("clients", "logout_all")
    CLIENTS_FIELD_NUMBER: _ClassVar[int]
    LOGOUT_ALL_FIELD_NUMBER: _ClassVar[int]
    clients: _containers.RepeatedCompositeFieldContainer[_lang_pb2.NodeReferenceData]
    logout_all: bool
    def __init__(
        self,
        clients: _Optional[_Iterable[_Union[_lang_pb2.NodeReferenceData, _Mapping]]] = ...,
        logout_all: bool = ...,
    ) -> None: ...

class LogoutUserResponse(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class CreateOrganizationRequest(_message.Message):
    __slots__ = ("organization",)
    ORGANIZATION_FIELD_NUMBER: _ClassVar[int]
    organization: _lang_pb2.OrganizationData
    def __init__(
        self, organization: _Optional[_Union[_lang_pb2.OrganizationData, _Mapping]] = ...
    ) -> None: ...

class CreateOrganizationResponse(_message.Message):
    __slots__ = ("organization", "epoch")
    ORGANIZATION_FIELD_NUMBER: _ClassVar[int]
    EPOCH_FIELD_NUMBER: _ClassVar[int]
    organization: _lang_pb2.OrganizationData
    epoch: int
    def __init__(
        self,
        organization: _Optional[_Union[_lang_pb2.OrganizationData, _Mapping]] = ...,
        epoch: _Optional[int] = ...,
    ) -> None: ...

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
    def __init__(
        self,
        owner: _Optional[_Union[_lang_pb2.NodeReferenceData, _Mapping]] = ...,
        slug: _Optional[str] = ...,
        region: _Optional[_Union[_lang_pb2.Region, str]] = ...,
        is_main: bool = ...,
    ) -> None: ...

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
    def __init__(
        self, benches: _Optional[_Iterable[_Union[ResolveHostsRequest.BenchKey, _Mapping]]] = ...
    ) -> None: ...

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
        def __init__(
            self,
            domain: _Optional[str] = ...,
            grpc_port: _Optional[int] = ...,
            grpc_web_port: _Optional[int] = ...,
            ssl: bool = ...,
            bench: _Optional[_Union[_lang_pb2.NodeReferenceData, _Mapping]] = ...,
        ) -> None: ...

    HOSTS_FIELD_NUMBER: _ClassVar[int]
    hosts: _containers.RepeatedCompositeFieldContainer[ResolveHostsResponse.HostInfo]
    def __init__(
        self, hosts: _Optional[_Iterable[_Union[ResolveHostsResponse.HostInfo, _Mapping]]] = ...
    ) -> None: ...

class UploadFilesRequest(_message.Message):
    __slots__ = ("scope", "files", "environment")
    SCOPE_FIELD_NUMBER: _ClassVar[int]
    FILES_FIELD_NUMBER: _ClassVar[int]
    ENVIRONMENT_FIELD_NUMBER: _ClassVar[int]
    scope: _lang_pb2.GraphScopeData
    files: _containers.RepeatedCompositeFieldContainer[_lang_pb2.FileData]
    environment: MachineEnvironment
    def __init__(
        self,
        scope: _Optional[_Union[_lang_pb2.GraphScopeData, _Mapping]] = ...,
        files: _Optional[_Iterable[_Union[_lang_pb2.FileData, _Mapping]]] = ...,
        environment: _Optional[_Union[MachineEnvironment, str]] = ...,
    ) -> None: ...

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
        def __init__(
            self,
            file: _Optional[_Union[_lang_pb2.FileData, _Mapping]] = ...,
            post_url: _Optional[str] = ...,
            fields: _Optional[_Union[_struct_pb2.Struct, _Mapping]] = ...,
            get_url: _Optional[str] = ...,
        ) -> None: ...

    HANDLES_FIELD_NUMBER: _ClassVar[int]
    handles: _containers.RepeatedCompositeFieldContainer[UploadFilesResponse.UploadHandle]
    def __init__(
        self,
        handles: _Optional[_Iterable[_Union[UploadFilesResponse.UploadHandle, _Mapping]]] = ...,
    ) -> None: ...

class DownloadFilesRequest(_message.Message):
    __slots__ = ("scope", "files", "environment")
    SCOPE_FIELD_NUMBER: _ClassVar[int]
    FILES_FIELD_NUMBER: _ClassVar[int]
    ENVIRONMENT_FIELD_NUMBER: _ClassVar[int]
    scope: _lang_pb2.GraphScopeData
    files: _containers.RepeatedCompositeFieldContainer[_lang_pb2.NodeReferenceData]
    environment: MachineEnvironment
    def __init__(
        self,
        scope: _Optional[_Union[_lang_pb2.GraphScopeData, _Mapping]] = ...,
        files: _Optional[_Iterable[_Union[_lang_pb2.NodeReferenceData, _Mapping]]] = ...,
        environment: _Optional[_Union[MachineEnvironment, str]] = ...,
    ) -> None: ...

class DownloadFilesResponse(_message.Message):
    __slots__ = ("handles",)
    class DownloadHandle(_message.Message):
        __slots__ = ("file", "get_url")
        FILE_FIELD_NUMBER: _ClassVar[int]
        GET_URL_FIELD_NUMBER: _ClassVar[int]
        file: _lang_pb2.FileData
        get_url: str
        def __init__(
            self,
            file: _Optional[_Union[_lang_pb2.FileData, _Mapping]] = ...,
            get_url: _Optional[str] = ...,
        ) -> None: ...

    HANDLES_FIELD_NUMBER: _ClassVar[int]
    handles: _containers.RepeatedCompositeFieldContainer[DownloadFilesResponse.DownloadHandle]
    def __init__(
        self,
        handles: _Optional[_Iterable[_Union[DownloadFilesResponse.DownloadHandle, _Mapping]]] = ...,
    ) -> None: ...
