
# type: ignore
# ruff: noqa

from typing import TYPE_CHECKING, Union, AsyncIterator, Mapping

if TYPE_CHECKING:
    from bench.language import Session, Session, IsSubject, Client



from . import language_pb2 as _language_pb2
from google.protobuf import descriptor_pb2 as _descriptor_pb2
from google.protobuf.internal import containers as _containers
from google.protobuf.internal import enum_type_wrapper as _enum_type_wrapper
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from typing import ClassVar as _ClassVar, Iterable as _Iterable, Mapping as _Mapping, Optional as _Optional, Union as _Union

DESCRIPTOR: _descriptor.FileDescriptor

class ServiceKind(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    UNSPECIFIED: _ClassVar[ServiceKind]
    INTERNAL: _ClassVar[ServiceKind]
    PUBLIC: _ClassVar[ServiceKind]
UNSPECIFIED: ServiceKind
INTERNAL: ServiceKind
PUBLIC: ServiceKind
KIND_FIELD_NUMBER: _ClassVar[int]
kind: _descriptor.FieldDescriptor
SENSITIVE_FIELD_NUMBER: _ClassVar[int]
sensitive: _descriptor.FieldDescriptor

class RpcMetadata(_message.Message):
    __slots__ = ("client_type", "client_id", "client_nonce", "client_access_token", "badges")
    class BadgeInfo(_message.Message):
        __slots__ = ("id", "key", "password")
        ID_FIELD_NUMBER: _ClassVar[int]
        KEY_FIELD_NUMBER: _ClassVar[int]
        PASSWORD_FIELD_NUMBER: _ClassVar[int]
        id: str
        key: str
        password: str
        def __init__(self, id: _Optional[str] = ..., key: _Optional[str] = ..., password: _Optional[str] = ...) -> None: ...
    CLIENT_TYPE_FIELD_NUMBER: _ClassVar[int]
    CLIENT_ID_FIELD_NUMBER: _ClassVar[int]
    CLIENT_NONCE_FIELD_NUMBER: _ClassVar[int]
    CLIENT_ACCESS_TOKEN_FIELD_NUMBER: _ClassVar[int]
    BADGES_FIELD_NUMBER: _ClassVar[int]
    client_type: _language_pb2.ClientType
    client_id: str
    client_nonce: str
    client_access_token: str
    badges: _containers.RepeatedCompositeFieldContainer[RpcMetadata.BadgeInfo]
    def __init__(self, client_type: _Optional[_Union[_language_pb2.ClientType, str]] = ..., client_id: _Optional[str] = ..., client_nonce: _Optional[str] = ..., client_access_token: _Optional[str] = ..., badges: _Optional[_Iterable[_Union[RpcMetadata.BadgeInfo, _Mapping]]] = ...) -> None: ...
