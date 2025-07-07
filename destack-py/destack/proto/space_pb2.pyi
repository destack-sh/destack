
# type: ignore
# ruff: noqa

from typing import TYPE_CHECKING, Union, AsyncIterator, Mapping

if TYPE_CHECKING:
    from destack.language import Session, Session, IsSubject, Client



from google.protobuf import struct_pb2 as _struct_pb2
from . import common_pb2 as _common_pb2
from . import language_pb2 as _language_pb2
from google.protobuf.internal import containers as _containers
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from typing import ClassVar as _ClassVar, Iterable as _Iterable, Mapping as _Mapping, Optional as _Optional, Union as _Union

DESCRIPTOR: _descriptor.FileDescriptor

class ScopeProto(_message.Message):
    __slots__ = ("space_id",)
    SPACE_ID_FIELD_NUMBER: _ClassVar[int]
    space_id: str
    def __init__(self, space_id: _Optional[str] = ...) -> None: ...

class QueryRequest(_message.Message):
    __slots__ = ("scope", "query")
    SCOPE_FIELD_NUMBER: _ClassVar[int]
    QUERY_FIELD_NUMBER: _ClassVar[int]
    scope: ScopeProto
    query: _language_pb2.QueryProto
    def __init__(self, scope: _Optional[_Union[ScopeProto, _Mapping]] = ..., query: _Optional[_Union[_language_pb2.QueryProto, _Mapping]] = ...) -> None: ...

class QueryResponse(_message.Message):
    __slots__ = ("result",)
    RESULT_FIELD_NUMBER: _ClassVar[int]
    result: _language_pb2.QueryResultProto
    def __init__(self, result: _Optional[_Union[_language_pb2.QueryResultProto, _Mapping]] = ...) -> None: ...

class CommitRequest(_message.Message):
    __slots__ = ("scope", "events")
    SCOPE_FIELD_NUMBER: _ClassVar[int]
    EVENTS_FIELD_NUMBER: _ClassVar[int]
    scope: ScopeProto
    events: _containers.RepeatedCompositeFieldContainer[_language_pb2.SomeEventProto]
    def __init__(self, scope: _Optional[_Union[ScopeProto, _Mapping]] = ..., events: _Optional[_Iterable[_Union[_language_pb2.SomeEventProto, _Mapping]]] = ...) -> None: ...

class CommitResponse(_message.Message):
    __slots__ = ("epoch", "events")
    EPOCH_FIELD_NUMBER: _ClassVar[int]
    EVENTS_FIELD_NUMBER: _ClassVar[int]
    epoch: int
    events: _containers.RepeatedCompositeFieldContainer[_language_pb2.SomeEventProto]
    def __init__(self, epoch: _Optional[int] = ..., events: _Optional[_Iterable[_Union[_language_pb2.SomeEventProto, _Mapping]]] = ...) -> None: ...

class SubscribeRequest(_message.Message):
    __slots__ = ("query",)
    QUERY_FIELD_NUMBER: _ClassVar[int]
    query: _language_pb2.QueryProto
    def __init__(self, query: _Optional[_Union[_language_pb2.QueryProto, _Mapping]] = ...) -> None: ...

class SubscribeResponse(_message.Message):
    __slots__ = ("update",)
    UPDATE_FIELD_NUMBER: _ClassVar[int]
    update: _language_pb2.QueryUpdateProto
    def __init__(self, update: _Optional[_Union[_language_pb2.QueryUpdateProto, _Mapping]] = ...) -> None: ...

class UploadFilesRequest(_message.Message):
    __slots__ = ("scope", "files")
    SCOPE_FIELD_NUMBER: _ClassVar[int]
    FILES_FIELD_NUMBER: _ClassVar[int]
    scope: ScopeProto
    files: _containers.RepeatedCompositeFieldContainer[_language_pb2.FileProto]
    def __init__(self, scope: _Optional[_Union[ScopeProto, _Mapping]] = ..., files: _Optional[_Iterable[_Union[_language_pb2.FileProto, _Mapping]]] = ...) -> None: ...

class UploadFilesResponse(_message.Message):
    __slots__ = ("handles",)
    class UploadHandle(_message.Message):
        __slots__ = ("file", "post_url", "fields", "get_url")
        FILE_FIELD_NUMBER: _ClassVar[int]
        POST_URL_FIELD_NUMBER: _ClassVar[int]
        FIELDS_FIELD_NUMBER: _ClassVar[int]
        GET_URL_FIELD_NUMBER: _ClassVar[int]
        file: _language_pb2.FileProto
        post_url: str
        fields: _struct_pb2.Struct
        get_url: str
        def __init__(self, file: _Optional[_Union[_language_pb2.FileProto, _Mapping]] = ..., post_url: _Optional[str] = ..., fields: _Optional[_Union[_struct_pb2.Struct, _Mapping]] = ..., get_url: _Optional[str] = ...) -> None: ...
    HANDLES_FIELD_NUMBER: _ClassVar[int]
    handles: _containers.RepeatedCompositeFieldContainer[UploadFilesResponse.UploadHandle]
    def __init__(self, handles: _Optional[_Iterable[_Union[UploadFilesResponse.UploadHandle, _Mapping]]] = ...) -> None: ...

class DownloadFilesRequest(_message.Message):
    __slots__ = ("scope", "files")
    SCOPE_FIELD_NUMBER: _ClassVar[int]
    FILES_FIELD_NUMBER: _ClassVar[int]
    scope: ScopeProto
    files: _containers.RepeatedCompositeFieldContainer[_language_pb2.NodeReferenceProto]
    def __init__(self, scope: _Optional[_Union[ScopeProto, _Mapping]] = ..., files: _Optional[_Iterable[_Union[_language_pb2.NodeReferenceProto, _Mapping]]] = ...) -> None: ...

class DownloadFilesResponse(_message.Message):
    __slots__ = ("handles",)
    class DownloadHandle(_message.Message):
        __slots__ = ("file", "get_url")
        FILE_FIELD_NUMBER: _ClassVar[int]
        GET_URL_FIELD_NUMBER: _ClassVar[int]
        file: _language_pb2.FileProto
        get_url: str
        def __init__(self, file: _Optional[_Union[_language_pb2.FileProto, _Mapping]] = ..., get_url: _Optional[str] = ...) -> None: ...
    HANDLES_FIELD_NUMBER: _ClassVar[int]
    handles: _containers.RepeatedCompositeFieldContainer[DownloadFilesResponse.DownloadHandle]
    def __init__(self, handles: _Optional[_Iterable[_Union[DownloadFilesResponse.DownloadHandle, _Mapping]]] = ...) -> None: ...
