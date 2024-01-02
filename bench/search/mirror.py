from dataclasses import dataclass, replace
from datetime import datetime
from typing import TYPE_CHECKING, Any, Generic, Optional, TypeVar
from uuid import UUID

import bench.search.core as os
from bench.language.const import RunStatus
from bench.proto import wire
from bench.utils.utils import IS_WORKER

if TYPE_CHECKING or not IS_WORKER:
    from bench import models
else:

    class _NullPackage:
        def __getattr__(self, item):
            return type(item, (), {})

    models = _NullPackage()

if TYPE_CHECKING:
    from django.db.models import Model
else:
    Model = object


@dataclass
class ModuleInfo:
    id: UUID
    bench_id: UUID


NAME_FIELD = os.Field(
    os.FT.TEXT,
    fields={
        os.SubfieldType.starts_with: os.Field(os.FieldType.SEARCH_AS_YOU_TYPE),
        os.SubfieldType.key: os.Field(os.FieldType.KEYWORD),
    },
)
HTML_FIELD = os.Field(os.FieldType.TEXT, analyzer=os.Analyzer.HTML)
TEXT_FIELD = HTML_FIELD

ModelT = TypeVar("ModelT", bound=Model)
MirrorT = TypeVar("MirrorT", bound=os.Document)
DataT = TypeVar("DataT", bound=Any)

DOCUMENT_CLASS_BY_TYPE: dict[os.DocumentType, type[os.Document]] = {}


def document(type: os.DocumentType, *, store_type: bool = True):
    """Decorator to register a Document subclass."""

    def decorator(cls):
        cls = os.document(cls, type.value, store_type=store_type)
        if type in DOCUMENT_CLASS_BY_TYPE:
            raise ValueError(
                f"document already registered for {type}: {DOCUMENT_CLASS_BY_TYPE[type]}"
            )
        DOCUMENT_CLASS_BY_TYPE[type] = cls
        return cls

    return decorator


class Packer(Generic[ModelT, MirrorT, DataT]):
    def mirror(self, bench_v: models.BenchVersion | ModuleInfo, node: ModelT) -> MirrorT:
        raise NotImplementedError(f"mirror not implemented for {type(self)}")

    def pack(self, mirror: MirrorT) -> DataT:
        raise NotImplementedError(f"pack not implemented for {type(self)}")

    def unpack(
        self, bench_v: models.BenchVersion | ModuleInfo, data: DataT, parent: ModelT
    ) -> MirrorT:
        raise NotImplementedError(f"unpack not implemented for {type(self)}")


_packers_by_model: dict[type[ModelT], Packer[ModelT, MirrorT, DataT]] = {}
_packers_by_mirror: dict[type[MirrorT], Packer[ModelT, MirrorT, DataT]] = {}
_packers_by_data: dict[type[DataT], Packer[ModelT, MirrorT, DataT]] = {}


def packer(model_t: type[ModelT], mirror_t: type[MirrorT], data_t: Optional[type[DataT]] = None):
    """Decorator to register a NodePacker for a model."""

    def decorator(cls: type[Packer[ModelT, MirrorT, DataT]]):
        if model_t in _packers_by_model:
            raise ValueError(
                f"packer already registered for {model_t}: {_packers_by_model[model_t]}"
            )
        if mirror_t in _packers_by_mirror:
            raise ValueError(
                f"packer already registered for {mirror_t}: {_packers_by_mirror[mirror_t]}"
            )
        if data_t is not None and data_t in _packers_by_data:
            raise ValueError(f"packer already registered for {data_t}: {_packers_by_data[data_t]}")
        packer = cls()
        if model_t:  # may be None if loaded outside django
            _packers_by_model[model_t] = packer
        _packers_by_mirror[mirror_t] = packer
        _packers_by_mirror[mirror_t.Partial] = packer
        if data_t is not None:
            _packers_by_data[data_t] = packer
        return cls

    return decorator


def has_mirror(node: ModelT) -> bool:
    return type(node) in _packers_by_model


def mirror_node(bench_v: models.BenchVersion | ModuleInfo, node: ModelT) -> MirrorT:
    packer = _packers_by_model[type(node)]
    return packer.mirror(bench_v, node)


def pack_node_flat(node: MirrorT) -> DataT:
    packer = _packers_by_mirror[type(node)]
    return packer.pack(node)


def unpack_node_flat(
    bench_v: models.BenchVersion | ModuleInfo, data: DataT, parent: Optional[ModelT]
) -> MirrorT:
    packer = _packers_by_data[type(data)]
    return packer.unpack(bench_v, data, parent)


def get_node_packer(mirror_t: type[MirrorT]) -> Packer[ModelT, MirrorT, DataT]:
    return _packers_by_mirror[mirror_t]


# basic

Document = os.Document


@os.document
class CrudThing(os.Document):
    created_at: datetime = os.field(os.FT.DATE)
    updated_at: datetime = os.field(os.FT.DATE)
    # to allow reset to 'null'  via invalid date
    deleted_at: Optional[datetime] = os.field(os.FT.DATE, ignore_malformed=True)
    created_by_id: Optional[UUID] = os.field(os.FT.KEYWORD)
    last_edited_at: datetime = os.field(os.FT.DATE)
    last_edited_by_id: Optional[UUID] = os.field(os.FT.KEYWORD)


class Revisioned(os.Document):
    revision: int = os.field(os.FT.INTEGER, index=False)


@packer(models.CrudModel, CrudThing, None)
class CrudThingPacker(Packer):
    def mirror(self, bench_v: models.BenchVersion | ModuleInfo, node: ModelT) -> MirrorT:
        return CrudThing(
            id=node.id,
            created_at=node.created_at,
            updated_at=node.updated_at,
            deleted_at=node.deleted_at,
            created_by_id=node.created_by_id,
            last_edited_at=node.last_edited_at,
            last_edited_by_id=node.last_edited_by_id,
        )


@document(os.DocumentType.RECORD)
class Record(CrudThing, os.Document):
    ck: UUID = os.field(os.FT.KEYWORD)
    bench_version_id: UUID = os.field(os.FT.KEYWORD)
    statement_key: Optional[str] = os.field(os.FT.KEYWORD)
    # single name field to copy all data names to :RecordNameField
    name: Optional[str] = replace(NAME_FIELD, can_set_directly=False, store=False)
    value: dict = os.field(os.FT.OBJECT, dynamic="strict")  # user defined
    revision: Optional[int] = os.field(os.FT.INTEGER)


@packer(wire.RecordData, Record, wire.RecordData)
class RecordPacker(CrudThingPacker, Packer[wire.RecordData, Record, wire.RecordData]):
    def mirror(self, bench_v: models.BenchVersion | ModuleInfo, node: Record) -> Record:
        return Record(
            id=node.id,
            ck=node.ck,
            bench_version_id=bench_v.id,
            statement_key=node.statement_key,
            value=node.value,
            revision=node.revision,
            created_at=node.created_at,
            created_by_id=node.created_by_id,
            updated_at=node.updated_at,
            deleted_at=node.deleted_at if node.deleted_at else "-",  # reset with invalid date
            last_edited_at=node.last_edited_at,
            last_edited_by_id=node.last_edited_by_id,
        )

    def pack(self, node: Record) -> wire.RecordData:
        return wire.RecordData(
            id=node.id,
            ck=node.ck,
            parent_id=None,  # unknown?
            value=node.value,
            revision=node.revision,
            created_at=node.created_at,
            updated_at=node.updated_at,
            deleted_at=node.deleted_at,
            last_edited_at=node.last_edited_at,
            last_changed_at=None,
        )

    def unpack(
        self,
        bench_v: models.BenchVersion | ModuleInfo,
        data: wire.RecordData,
        parent: models.Statement,
    ) -> Record:
        return Record(
            id=data.id,
            ck=data.ck,
            bench_version_id=bench_v.id,
            statement_key=parent.key,
            value=data.value,
            revision=data.revision,
            created_at=data.created_at,
            created_by_id=None,
            updated_at=data.updated_at,
            deleted_at=data.deleted_at,
            last_edited_at=data.last_edited_at,
            last_edited_by_id=None,
        )


# sessions/logs


@document(os.DocumentType.SESSION)
class Session(os.Document):
    bench_version_id: UUID = os.field(os.FT.KEYWORD)
    opened_at: Optional[datetime] = os.field(os.FT.DATE)
    closed_at: Optional[datetime] = os.field(os.FT.DATE)
    trigger_type: Optional[str] = os.field(os.FT.KEYWORD)


@packer(models.Session, Session, wire.SessionData)
class SessionPacker(Packer[models.Session, Session, wire.SessionData]):
    def mirror(self, bench_v: models.BenchVersion | ModuleInfo, node: models.Session) -> Session:
        return Session(
            id=node.id,
            bench_version_id=node.bench_version_id,
            opened_at=node.opened_at,
            closed_at=node.closed_at,
            trigger_type=node.trigger_type,
        )


@document(os.DocumentType.RUN)
class Run(os.Document):
    bench_id: UUID = os.field(os.FT.KEYWORD)
    worker_node_id: Optional[str] = os.field(os.FT.KEYWORD)
    worker_process_id: Optional[str] = os.field(os.FT.KEYWORD)
    session_id: Optional[UUID] = os.field(os.FT.KEYWORD)
    trigger_type: Optional[str] = os.field(os.FT.KEYWORD)
    root_id: Optional[UUID] = os.field(os.FT.KEYWORD)
    parent_id: Optional[UUID] = os.field(os.FT.KEYWORD)
    statement_ck: UUID = os.field(os.FT.KEYWORD)
    created_at: datetime = os.field(os.FT.DATE)
    updated_at: datetime = os.field(os.FT.DATE)
    scheduled_at: Optional[datetime] = os.field(os.FT.DATE)
    started_at: Optional[datetime] = os.field(os.FT.DATE)
    terminated_at: Optional[datetime] = os.field(os.FT.DATE)
    duration: Optional[float] = os.field(os.FT.FLOAT)
    status: str = os.field(os.FT.KEYWORD)
    inputs: Optional[dict] = os.field(os.FT.OBJECT, dynamic="strict")  # user defined
    outputs: Optional[dict] = os.field(os.FT.OBJECT, dynamic="strict")  # user defined
    error: Optional[dict] = os.field(os.FT.OBJECT, dynamic="strict")  # user defined (mostly?)
    value: Optional[dict] = os.field(os.FT.OBJECT, dynamic="strict")  # user defined (mostly?)


@packer(models.Run, Run, wire.RunData)
class RunPacker(Packer[models.Run, Run, wire.RunData]):
    def mirror(self, bench_v: models.BenchVersion | ModuleInfo, node: models.Run) -> MirrorT:
        return Run(
            id=node.id,
            bench_id=node.bench_id,
            worker_node_id=node.worker_node_id,
            worker_process_id=node.worker_process_id,
            session_id=node.session_id,
            trigger_type=node.trigger_type,
            root_id=node.root_id,
            parent_id=node.parent_id,
            statement_ck=node.statement_ck,
            created_at=node.created_at,
            updated_at=node.updated_at,
            scheduled_at=node.scheduled_at,
            started_at=node.started_at,
            terminated_at=node.terminated_at,
            duration=node.duration,
            status=node.status,
            inputs=node.inputs,
            outputs=node.outputs,
            error=None,  # not stored for now
            value=node.value,
        )

    def pack(self, mirror: Run) -> wire.RunData:
        statement_type = (
            wire.StatementType(mirror.statement_type) if mirror.statement_type else None
        )
        return wire.RunData(
            id=mirror.id,
            bench_id=mirror.bench_id,
            worker_node_id=mirror.worker_node_id,
            worker_process_id=mirror.worker_process_id,
            module_id=mirror.bench_version_id,
            session_id=mirror.session_id,
            trigger_type=mirror.trigger_type,
            trigger_id=None,  # not stored
            root_id=mirror.root_id,
            parent_id=mirror.parent_id,
            statement_id=mirror.statement_id,
            statement_ck=mirror.statement_ck,
            statement_type=statement_type,
            created_at=mirror.created_at,
            updated_at=mirror.updated_at,
            scheduled_at=mirror.scheduled_at,
            started_at=mirror.started_at,
            terminated_at=mirror.terminated_at,
            status=RunStatus(mirror.status),
            inputs=mirror.inputs,
            outputs=mirror.outputs,
            error=None,
            value=mirror.value,
        )

    def unpack(self, bench_v: None, data: wire.RunData, parent: None) -> Run:
        if data.started_at and data.terminated_at:
            duration = (data.terminated_at - data.started_at).total_seconds()
        else:
            duration = None
        return Run(
            id=data.id,
            bench_id=data.bench_id,
            worker_node_id=data.worker_node_id,
            worker_process_id=data.worker_process_id,
            session_id=data.session_id,
            trigger_type=data.trigger_type,
            root_id=data.root_id,
            parent_id=data.parent_id,
            statement_ck=data.statement_ck,
            created_at=data.created_at,
            updated_at=data.updated_at,
            scheduled_at=data.scheduled_at,
            started_at=data.started_at,
            terminated_at=data.terminated_at,
            duration=duration,
            status=data.status,
            inputs=data.inputs,
            outputs=data.outputs,
            error=None,
            value=data.value,
        )


@document(os.DocumentType.LOG_ENTRY)
class LogEntry(os.Document):
    bench_id: UUID = os.field(os.FT.KEYWORD)
    session_id: Optional[UUID] = os.field(os.FT.KEYWORD)
    run_id: Optional[UUID] = os.field(os.FT.KEYWORD)
    statement_id: Optional[UUID] = os.field(os.FT.KEYWORD)
    statement_ck: Optional[UUID] = os.field(os.FT.KEYWORD)
    created_at: datetime = os.field(os.FT.DATE)
    stream: str = os.field(os.FT.KEYWORD)
    level: Optional[str] = os.field(os.FT.KEYWORD)
    logger: Optional[str] = os.field(os.FT.KEYWORD)
    message: Optional[str] = os.field(os.FT.TEXT)
    value: Optional[dict] = os.field(os.FT.OBJECT, dynamic="strict")  # user defined


@packer(LogEntry, LogEntry, wire.LogEntryData)
class LogEntryPacker(Packer[LogEntry, LogEntry, wire.LogEntryData]):
    def pack(self, mirror: LogEntry) -> wire.LogEntryData:
        return wire.LogEntryData(
            id=mirror.id,
            bench_id=mirror.bench_id,
            module_id=mirror.bench_version_id,
            session_id=mirror.session_id,
            run_id=mirror.run_id,
            statement_id=mirror.statement_id,
            statement_ck=mirror.statement_ck,
            created_at=mirror.created_at,
            stream=mirror.stream,
            level=mirror.level,
            logger=mirror.logger,
            message=mirror.message,
            value=mirror.value,
        )

    def unpack(
        self, bench_v: models.BenchVersion | ModuleInfo, data: wire.LogEntryData, parent: None
    ) -> LogEntry:
        return LogEntry(
            id=data.id,
            bench_id=data.bench_id,
            bench_version_id=bench_v.id,
            session_id=data.session_id,
            run_id=data.run_id,
            statement_id=data.statement_id,
            statement_ck=data.statement_ck,
            created_at=data.created_at,
            stream=data.stream,
            level=data.level,
            logger=data.logger,
            message=data.message,
            value=data.value,
        )
