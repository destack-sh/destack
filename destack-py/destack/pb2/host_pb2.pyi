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
from typing import (
    ClassVar as _ClassVar,
    Iterable as _Iterable,
    Mapping as _Mapping,
    Optional as _Optional,
    Union as _Union,
)

DESCRIPTOR: _descriptor.FileDescriptor

class QueryRequest(_message.Message):
    __slots__ = ("scope", "query")
    SCOPE_FIELD_NUMBER: _ClassVar[int]
    QUERY_FIELD_NUMBER: _ClassVar[int]
    scope: _language_pb2.ScopeData
    query: _language_pb2.QueryData
    def __init__(
        self,
        scope: _Optional[_Union[_language_pb2.ScopeData, _Mapping]] = ...,
        query: _Optional[_Union[_language_pb2.QueryData, _Mapping]] = ...,
    ) -> None: ...

class QueryResponse(_message.Message):
    __slots__ = ("result",)
    RESULT_FIELD_NUMBER: _ClassVar[int]
    result: _language_pb2.QueryResultData
    def __init__(
        self, result: _Optional[_Union[_language_pb2.QueryResultData, _Mapping]] = ...
    ) -> None: ...

class SubscribeRequest(_message.Message):
    __slots__ = ("query",)
    QUERY_FIELD_NUMBER: _ClassVar[int]
    query: _language_pb2.QueryData
    def __init__(
        self, query: _Optional[_Union[_language_pb2.QueryData, _Mapping]] = ...
    ) -> None: ...

class SubscribeResponse(_message.Message):
    __slots__ = ("update",)
    UPDATE_FIELD_NUMBER: _ClassVar[int]
    update: _language_pb2.QueryUpdateData
    def __init__(
        self, update: _Optional[_Union[_language_pb2.QueryUpdateData, _Mapping]] = ...
    ) -> None: ...

class CommitRequest(_message.Message):
    __slots__ = ("scope", "changes")
    SCOPE_FIELD_NUMBER: _ClassVar[int]
    CHANGES_FIELD_NUMBER: _ClassVar[int]
    scope: _language_pb2.ScopeData
    changes: _containers.RepeatedCompositeFieldContainer[_language_pb2.ChangeData]
    def __init__(
        self,
        scope: _Optional[_Union[_language_pb2.ScopeData, _Mapping]] = ...,
        changes: _Optional[_Iterable[_Union[_language_pb2.ChangeData, _Mapping]]] = ...,
    ) -> None: ...

class CommitResponse(_message.Message):
    __slots__ = ("epoch", "results")
    EPOCH_FIELD_NUMBER: _ClassVar[int]
    RESULTS_FIELD_NUMBER: _ClassVar[int]
    epoch: int
    results: _containers.RepeatedCompositeFieldContainer[_language_pb2.ChangeResultData]
    def __init__(
        self,
        epoch: _Optional[int] = ...,
        results: _Optional[_Iterable[_Union[_language_pb2.ChangeResultData, _Mapping]]] = ...,
    ) -> None: ...

class UploadFilesRequest(_message.Message):
    __slots__ = ("scope", "files")
    SCOPE_FIELD_NUMBER: _ClassVar[int]
    FILES_FIELD_NUMBER: _ClassVar[int]
    scope: _language_pb2.ScopeData
    files: _containers.RepeatedCompositeFieldContainer[_language_pb2.FileData]
    def __init__(
        self,
        scope: _Optional[_Union[_language_pb2.ScopeData, _Mapping]] = ...,
        files: _Optional[_Iterable[_Union[_language_pb2.FileData, _Mapping]]] = ...,
    ) -> None: ...

class UploadFilesResponse(_message.Message):
    __slots__ = ("handles",)
    class UploadHandle(_message.Message):
        __slots__ = ("file", "post_url", "fields", "get_url")
        FILE_FIELD_NUMBER: _ClassVar[int]
        POST_URL_FIELD_NUMBER: _ClassVar[int]
        FIELDS_FIELD_NUMBER: _ClassVar[int]
        GET_URL_FIELD_NUMBER: _ClassVar[int]
        file: _language_pb2.FileData
        post_url: str
        fields: _struct_pb2.Struct
        get_url: str
        def __init__(
            self,
            file: _Optional[_Union[_language_pb2.FileData, _Mapping]] = ...,
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
    __slots__ = ("scope", "files")
    SCOPE_FIELD_NUMBER: _ClassVar[int]
    FILES_FIELD_NUMBER: _ClassVar[int]
    scope: _language_pb2.ScopeData
    files: _containers.RepeatedCompositeFieldContainer[_language_pb2.NodeReferenceData]
    def __init__(
        self,
        scope: _Optional[_Union[_language_pb2.ScopeData, _Mapping]] = ...,
        files: _Optional[_Iterable[_Union[_language_pb2.NodeReferenceData, _Mapping]]] = ...,
    ) -> None: ...

class DownloadFilesResponse(_message.Message):
    __slots__ = ("handles",)
    class DownloadHandle(_message.Message):
        __slots__ = ("file", "get_url")
        FILE_FIELD_NUMBER: _ClassVar[int]
        GET_URL_FIELD_NUMBER: _ClassVar[int]
        file: _language_pb2.FileData
        get_url: str
        def __init__(
            self,
            file: _Optional[_Union[_language_pb2.FileData, _Mapping]] = ...,
            get_url: _Optional[str] = ...,
        ) -> None: ...

    HANDLES_FIELD_NUMBER: _ClassVar[int]
    handles: _containers.RepeatedCompositeFieldContainer[DownloadFilesResponse.DownloadHandle]
    def __init__(
        self,
        handles: _Optional[_Iterable[_Union[DownloadFilesResponse.DownloadHandle, _Mapping]]] = ...,
    ) -> None: ...
