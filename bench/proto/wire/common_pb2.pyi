# type: ignore
# ruff: noqa

from typing import ClassVar as _ClassVar
from typing import Iterable as _Iterable
from typing import Mapping as _Mapping
from typing import Optional as _Optional
from typing import Union as _Union

from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from google.protobuf.internal import containers as _containers
from google.protobuf.internal import enum_type_wrapper as _enum_type_wrapper

from proto import lang_pb2 as _lang_pb2

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
    __slots__ = ("badges", "client_access_token", "client_id", "client_nonce", "client_type")

    class BadgeInfo(_message.Message):
        __slots__ = ("id", "key", "password")
        ID_FIELD_NUMBER: _ClassVar[int]
        KEY_FIELD_NUMBER: _ClassVar[int]
        PASSWORD_FIELD_NUMBER: _ClassVar[int]
        id: str
        key: str
        password: str

        def __init__(
            self,
            id: _Optional[str] = ...,
            key: _Optional[str] = ...,
            password: _Optional[str] = ...,
        ) -> None: ...

    CLIENT_TYPE_FIELD_NUMBER: _ClassVar[int]
    CLIENT_ID_FIELD_NUMBER: _ClassVar[int]
    CLIENT_NONCE_FIELD_NUMBER: _ClassVar[int]
    CLIENT_ACCESS_TOKEN_FIELD_NUMBER: _ClassVar[int]
    BADGES_FIELD_NUMBER: _ClassVar[int]
    client_type: _lang_pb2.ClientType
    client_id: str
    client_nonce: str
    client_access_token: str
    badges: _containers.RepeatedCompositeFieldContainer[RpcMetadata.BadgeInfo]

    def __init__(
        self,
        client_type: _Optional[_Union[_lang_pb2.ClientType, str]] = ...,
        client_id: _Optional[str] = ...,
        client_nonce: _Optional[str] = ...,
        client_access_token: _Optional[str] = ...,
        badges: _Optional[_Iterable[_Union[RpcMetadata.BadgeInfo, _Mapping]]] = ...,
    ) -> None: ...
