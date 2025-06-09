
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

class WakeRequest(_message.Message):
    __slots__ = ("scope", "machine_ptr", "thread_ptrs")
    SCOPE_FIELD_NUMBER: _ClassVar[int]
    MACHINE_PTR_FIELD_NUMBER: _ClassVar[int]
    THREAD_PTRS_FIELD_NUMBER: _ClassVar[int]
    scope: _language_pb2.ScopeData
    machine_ptr: _language_pb2.NodeReferenceData
    thread_ptrs: _containers.RepeatedCompositeFieldContainer[_language_pb2.NodeReferenceData]
    def __init__(self, scope: _Optional[_Union[_language_pb2.ScopeData, _Mapping]] = ..., machine_ptr: _Optional[_Union[_language_pb2.NodeReferenceData, _Mapping]] = ..., thread_ptrs: _Optional[_Iterable[_Union[_language_pb2.NodeReferenceData, _Mapping]]] = ...) -> None: ...

class WakeResponse(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class RunRequest(_message.Message):
    __slots__ = ("scope", "machine_ptr", "thread_ptr", "run_ptrs")
    SCOPE_FIELD_NUMBER: _ClassVar[int]
    MACHINE_PTR_FIELD_NUMBER: _ClassVar[int]
    THREAD_PTR_FIELD_NUMBER: _ClassVar[int]
    RUN_PTRS_FIELD_NUMBER: _ClassVar[int]
    scope: _language_pb2.ScopeData
    machine_ptr: _language_pb2.NodeReferenceData
    thread_ptr: _language_pb2.NodeReferenceData
    run_ptrs: _containers.RepeatedCompositeFieldContainer[_language_pb2.NodeReferenceData]
    def __init__(self, scope: _Optional[_Union[_language_pb2.ScopeData, _Mapping]] = ..., machine_ptr: _Optional[_Union[_language_pb2.NodeReferenceData, _Mapping]] = ..., thread_ptr: _Optional[_Union[_language_pb2.NodeReferenceData, _Mapping]] = ..., run_ptrs: _Optional[_Iterable[_Union[_language_pb2.NodeReferenceData, _Mapping]]] = ...) -> None: ...

class RunResponse(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...
