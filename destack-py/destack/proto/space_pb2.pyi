
# type: ignore
# ruff: noqa

from typing import TYPE_CHECKING, Union, AsyncIterator, Mapping

if TYPE_CHECKING:
    from destack.language import Session, Session, IsActor, Client



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

class AppendRequest(_message.Message):
    __slots__ = ("scope", "events")
    SCOPE_FIELD_NUMBER: _ClassVar[int]
    EVENTS_FIELD_NUMBER: _ClassVar[int]
    scope: ScopeProto
    events: _containers.RepeatedCompositeFieldContainer[_language_pb2.SomeEventProto]
    def __init__(self, scope: _Optional[_Union[ScopeProto, _Mapping]] = ..., events: _Optional[_Iterable[_Union[_language_pb2.SomeEventProto, _Mapping]]] = ...) -> None: ...

class AppendResponse(_message.Message):
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
