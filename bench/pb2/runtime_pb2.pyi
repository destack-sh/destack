
# type: ignore
# ruff: noqa

from typing import TYPE_CHECKING, Union, AsyncIterator, Mapping



from . import common_pb2 as _common_pb2
from . import lang_pb2 as _lang_pb2
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from typing import ClassVar as _ClassVar, Mapping as _Mapping, Optional as _Optional, Union as _Union

DESCRIPTOR: _descriptor.FileDescriptor

class RunRequest(_message.Message):
    __slots__ = ("run_ptr",)
    RUN_PTR_FIELD_NUMBER: _ClassVar[int]
    run_ptr: _lang_pb2.NodeReferenceData
    def __init__(self, run_ptr: _Optional[_Union[_lang_pb2.NodeReferenceData, _Mapping]] = ...) -> None: ...

class RunResponse(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...
