
# type: ignore
# ruff: noqa

from typing import TYPE_CHECKING, Union
    

from . import common_pb2 as _common_pb2
from . import lang_pb2 as _lang_pb2
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from typing import ClassVar as _ClassVar, Mapping as _Mapping, Optional as _Optional, Union as _Union

DESCRIPTOR: _descriptor.FileDescriptor

class ProcessRunRequest(_message.Message):
    __slots__ = ("run", "is_blocking")
    RUN_FIELD_NUMBER: _ClassVar[int]
    IS_BLOCKING_FIELD_NUMBER: _ClassVar[int]
    run: _lang_pb2.RunData
    is_blocking: bool
    def __init__(self, run: _Optional[_Union[_lang_pb2.RunData, _Mapping]] = ..., is_blocking: bool = ...) -> None: ...

class ProcessRunResponse(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class PauseRunRequest(_message.Message):
    __slots__ = ("run",)
    RUN_FIELD_NUMBER: _ClassVar[int]
    run: _lang_pb2.RunData
    def __init__(self, run: _Optional[_Union[_lang_pb2.RunData, _Mapping]] = ...) -> None: ...

class PauseRunResponse(_message.Message):
    __slots__ = ("is_processed",)
    IS_PROCESSED_FIELD_NUMBER: _ClassVar[int]
    is_processed: bool
    def __init__(self, is_processed: bool = ...) -> None: ...

class ResumeRunRequest(_message.Message):
    __slots__ = ("run",)
    RUN_FIELD_NUMBER: _ClassVar[int]
    run: _lang_pb2.RunData
    def __init__(self, run: _Optional[_Union[_lang_pb2.RunData, _Mapping]] = ...) -> None: ...

class ResumeRunResponse(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class KillRunRequest(_message.Message):
    __slots__ = ("run",)
    RUN_FIELD_NUMBER: _ClassVar[int]
    run: _lang_pb2.RunData
    def __init__(self, run: _Optional[_Union[_lang_pb2.RunData, _Mapping]] = ...) -> None: ...

class KillRunResponse(_message.Message):
    __slots__ = ("is_processed",)
    IS_PROCESSED_FIELD_NUMBER: _ClassVar[int]
    is_processed: bool
    def __init__(self, is_processed: bool = ...) -> None: ...
