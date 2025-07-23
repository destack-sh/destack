import type { BuiltinObject, Graph, GraphConnection, Session, Encoder } from '@destack/language';
import { NODE_CLASS_BY_TYPE, STRUCT_CLASS_BY_TYPE } from '@destack/language/registry';
import { PROTO_OBJECT_ENCODERS, _ProtoObjectEncoder, getObjectKey } from '@destack/encoder/proto/generate';
import { Temporal } from 'temporal-polyfill';
import { uuid4, uuid7, toNanoId } from '@destack/utils/uuid';
import { packProtoDuration, packProtoTimestamp, packProtoJson, unpackProtoDuration, unpackProtoTimestamp, unpackProtoJson } from '@destack/encoder/proto/wiring';
import { timedeltaToISOFormat, timedeltaFromISOFormat, base64Encode, base64Decode } from '@destack/utils';
import type { AnyNodeProto, AnyStructProto } from '@destack/proto';
import type { Metric, RunFailedEvent, NotificationRescindedEvent, Query, Stroke, Membership, CustomProperty, FollowRemovedEvent, CustomOption, InviteSentEvent, TagDefinition, SanctionRevokedEvent, RunResumeRequestedEvent, Tag, Reaction, TimerResumedEvent, EntitlementExpiredEvent, SanctionEvent, RunPauseRequestedEvent, GradientStyle, Shadow, Method, Timer, Star, CustomEnum, Effect, Theme, MigrationOperation, SpanEvent, Node, Constraint, Script, NumberInputView, Scene, StrokeCap, Vector4, StructDefinitionReference, File, Snapshot, TextSpan, Font, Migration, ReactionEvent, Datum, Folder, Transition, DragStartEvent, Action, RunPausedEvent, CopyEvent, Color, ActionDefinition, Quaternion, Inset2, Grid2, Space, DragEndEvent, Tagging, Event, DropEvent, PropertyReference, ArrowShape2D, SanctionExpiredEvent, DragLeaveEvent, MigrationOperationDefinition, NumberConstraint, TriggerEvent, OptionDefinition, IndexDefinition, MethodDefinition, LineShape2D, Style, KeyUpEvent, Route, Permission, EntitlementRequestedEvent, NotificationDismissedEvent, MigrationDefinition, PointerLeaveEvent, StructDefinition, ShadowStyle, DragOverEvent, Rectangle2D, Value, Line2D, Service, GaugeMeasurementEvent, LogEvent, MouseEvent, StrokePath, DatumMutable, PathShape2D, ColorStyle, Entity2D, Role, Universe, FocusEvent, ClickEvent, GridSpan2, GaugeMetric, PointerMoveEvent, Handle, Arrow2D, TimerStartedEvent, NotificationEvent, Ellipse2D, Sort, Corner2, StarAddedEvent, Vector3, RoleUnassignedEvent, Aggregation, LabelView, Record, InviteEvent, ClipboardEvent, RoleEvent, Palette, PointerEvent, Condition, Text, Follow, TextView, Length, MembershipLeftEvent, PropertyDefinition, WheelEvent, InviteRejectedEvent, Layer, PointerDownEvent, Team, Invite, Sanction, KeyPressEvent, Function, TimerPausedEvent, Gradient, SceneEvent, CounterMeasurementEvent, Stage, InviteAcceptedEvent, NodeDefinition, Fill, Icon, ReactionRemovedEvent, Axis3, User, Path2D, FocusOutEvent, SanctionRequestedEvent, EntitlementGrantedEvent, Shape2D, SplitView, RunResumedEvent, Machine, Notification, Client, Schedule, MeasurementEvent, NotificationSentEvent, FollowEvent, PasteEvent, CounterMetric, ContentView, KeyEvent, PointerOverEvent, Vector2, Entity3D, FocusInEvent, FrameView, Select, DoubleClickEvent, PointerLongPressEvent, Shape3D, RunCompletedEvent, Vector3i, InputEvent, EntitlementEvent, Variant, RectangleShape2D, Border, TimerCompletedEvent, PermissionDefinition, CollectionConstraint, DragEnterEvent, StarEvent, Resource, GradientStop, KeyDownEvent, EntitlementRevokedEvent, Organization, Expression, RoleAssignedEvent, Database, Vector4i, FontStyle, InputView, RunEvent, SliderInputView, TimerCancelledEvent, ConstantDefinition, StarRemovedEvent, Environment, Vector2i, ConstraintDefinition, MembershipEvent, MembershipJoinedEvent, EnumDefinition, CustomEvent, InviteRescindedEvent, CutEvent, StrokeStyle, View, NodeDefinitionReference, EffectStyle, Axis2, PointerUpEvent, ViewEvent, TripleClickEvent, StringConstraint, NotificationExpiredEvent, RunStartedEvent, SingleClickEvent, PointerEnterEvent, FollowAddedEvent, TransitionStyle, Entitlement, TraitDefinition, Offset2, Struct, LayoutView, PolygonShape2D, SanctionGrantedEvent, BorderStyle, Index, Run, Branch, DragEvent, TimerEvent, FillStyle, NotificationReadEvent, Entity, ObjectDefinitionReference, Join, BuiltinDefinition, CustomStruct, HistogramMeasurementEvent, Trigger, NodeReference, ReactionAddedEvent, StrokePoint, SignalEvent, EditEvent, RunStopRequestedEvent, EllipseShape2D, Polygon2D, Type, HistogramMetric } from '@destack/language';
import { InputEventProto, Inset2Proto, CustomStructProto, KeyUpEventProto, UserProto, ObjectDefinitionReferenceProto, RunResumedEventProto, IndexDefinitionProto, CustomOptionProto, ResourceProto, TransitionStyleProto, EnumDefinitionProto, SplitViewProto, ArrowShape2DProto, BorderStyleProto, FocusInEventProto, MembershipProto, RunStopRequestedEventProto, PointerMoveEventProto, ValueProto, JoinProto, MachineProto, StructDefinitionProto, FocusEventProto, InviteAcceptedEventProto, SanctionRevokedEventProto, CutEventProto, StringConstraintProto, ReactionRemovedEventProto, CustomEnumProto, WheelEventProto, SceneEventProto, SanctionEventProto, FolderProto, EntityProto, FollowEventProto, LayoutViewProto, TriggerEventProto, TripleClickEventProto, StrokeStyleProto, StageProto, GradientProto, FunctionProto, MethodProto, EditEventProto, TextSpanProto, IconProto, Offset2Proto, LineShape2DProto, ShadowProto, SignalEventProto, PermissionProto, KeyPressEventProto, TimerEventProto, TaggingProto, TriggerProto, NumberConstraintProto, DragLeaveEventProto, ReactionProto, EntitlementRevokedEventProto, EntitlementRequestedEventProto, TypeProto, SanctionExpiredEventProto, InviteRescindedEventProto, ConstantDefinitionProto, EntitlementGrantedEventProto, CounterMeasurementEventProto, NotificationProto, TimerProto, OptionDefinitionProto, DragEndEventProto, Shape2DProto, Ellipse2DProto, ScheduleProto, ViewEventProto, CustomEventProto, IndexProto, KeyEventProto, DatumProto, ConditionProto, InviteSentEventProto, AggregationProto, ClipboardEventProto, DragOverEventProto, ScriptProto, PointerDownEventProto, TeamProto, RoleAssignedEventProto, SanctionRequestedEventProto, RectangleShape2DProto, MeasurementEventProto, NodeDefinitionReferenceProto, NodeReferenceProto, DoubleClickEventProto, SanctionProto, StarProto, StrokeProto, RunEventProto, CounterMetricProto, Corner2Proto, StructProto, ReactionAddedEventProto, GaugeMetricProto, MetricProto, DatabaseProto, EffectProto, OrganizationProto, PointerOverEventProto, BuiltinDefinitionProto, RecordProto, RoleUnassignedEventProto, PaletteProto, VariantProto, TimerCancelledEventProto, PropertyReferenceProto, SliderInputViewProto, DropEventProto, PasteEventProto, StarAddedEventProto, TraitDefinitionProto, EnvironmentProto, NotificationEventProto, MigrationDefinitionProto, Line2DProto, ViewProto, ActionDefinitionProto, FillStyleProto, EventProto, ActionProto, PointerEventProto, Vector3Proto, TimerPausedEventProto, HistogramMeasurementEventProto, MigrationProto, CustomPropertyProto, StrokePathProto, ColorProto, RunCompletedEventProto, RunPausedEventProto, ClickEventProto, TagProto, FrameViewProto, NotificationExpiredEventProto, SpaceProto, LogEventProto, LayerProto, SceneProto, ThemeProto, LengthProto, GaugeMeasurementEventProto, Grid2Proto, StyleProto, RunProto, GridSpan2Proto, GradientStyleProto, StrokeCapProto, UniverseProto, MigrationOperationProto, InviteEventProto, DragStartEventProto, InputViewProto, CollectionConstraintProto, SpanEventProto, PathShape2DProto, RunResumeRequestedEventProto, EllipseShape2DProto, DragEventProto, TimerCompletedEventProto, BorderProto, FontProto, Vector2iProto, BranchProto, StarRemovedEventProto, RunPauseRequestedEventProto, Entity3DProto, FocusOutEventProto, RunStartedEventProto, DatumMutableProto, SingleClickEventProto, NotificationDismissedEventProto, EntitlementEventProto, ColorStyleProto, PointerLongPressEventProto, RunFailedEventProto, PointerLeaveEventProto, ClientProto, Arrow2DProto, Vector2Proto, Vector4Proto, Axis3Proto, StarEventProto, Polygon2DProto, TransitionProto, MethodDefinitionProto, MembershipLeftEventProto, RoleProto, MembershipEventProto, PointerUpEventProto, HistogramMetricProto, EntitlementExpiredEventProto, FillProto, FollowAddedEventProto, NodeProto, ServiceProto, TimerResumedEventProto, QueryProto, QuaternionProto, NotificationReadEventProto, Axis2Proto, HandleProto, StructDefinitionReferenceProto, FollowProto, StrokePointProto, MouseEventProto, PolygonShape2DProto, NotificationSentEventProto, PermissionDefinitionProto, ContentViewProto, SelectProto, KeyDownEventProto, NodeDefinitionProto, Vector3iProto, CopyEventProto, SanctionGrantedEventProto, PropertyDefinitionProto, RouteProto, LabelViewProto, Path2DProto, MigrationOperationDefinitionProto, ExpressionProto, ReactionEventProto, Entity2DProto, RoleEventProto, EffectStyleProto, FollowRemovedEventProto, FileProto, PointerEnterEventProto, TextViewProto, Vector4iProto, TextProto, SnapshotProto, Shape3DProto, EntitlementProto, InviteProto, FontStyleProto, ConstraintProto, TagDefinitionProto, SortProto, MembershipJoinedEventProto, InviteRejectedEventProto, ShadowStyleProto, ConstraintDefinitionProto, NotificationRescindedEventProto, TimerStartedEventProto, DragEnterEventProto, NumberInputViewProto, Rectangle2DProto, GradientStopProto } from '@destack/proto';
export const PROTO_ENCODERS: { [key: string]: _ProtoObjectEncoder } = {};
let loaded = false;
export function loadEncoders(): void {
    if (loaded) {
      return;
    }
    loaded = true;

    class TagProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Tag): TagProto {
        const objectProto: Partial<TagProto> = { metatype: 12000 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        if (object._icon != null) {
          objectProto.icon = object._icon.pack(10);
        }
        return objectProto as TagProto;
      }

      unpackObject(objectProto: TagProto, _session: Session | null): Tag {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[12000] as typeof Tag)({
          icon: objectProto.icon != undefined ? _Icon.unpack(10, objectProto.icon, _session) as Icon : null,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: Tag): Uint8Array {
        const proto = this.packObject(object);
        return TagProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Tag {
        const proto = TagProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 12000)] = new TagProtoEncoder();

    class TaggingProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Tagging): TaggingProto {
        const objectProto: Partial<TaggingProto> = { metatype: 12100 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        objectProto.tagPtr = object._tagPtr.pack(10);
        return objectProto as TaggingProto;
      }

      unpackObject(objectProto: TaggingProto, _session: Session | null): Tagging {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[12100] as typeof Tagging)({
          tag: _NodeReference.unpack(10, objectProto.tagPtr, _session) as NodeReference,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: Tagging): Uint8Array {
        const proto = this.packObject(object);
        return TaggingProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Tagging {
        const proto = TaggingProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 12100)] = new TaggingProtoEncoder();

    class CustomEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: CustomEvent): CustomEventProto {
        const objectProto: Partial<CustomEventProto> = { metatype: 20000 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        if (object._icon != null) {
          objectProto.icon = object._icon.pack(10);
        }
        if (object._baseType != null) {
          objectProto.baseType = object._baseType.pack(10);
        }
        if (object._selfTraits) {
          const packedSelfTraits: any[] = [];
          for (const item of object._selfTraits) {
            packedSelfTraits.push(item.pack(10));
          }
          objectProto.selfTraits = packedSelfTraits;
        }
        objectProto.isAbstract = object._isAbstract;
        return objectProto as CustomEventProto;
      }

      unpackObject(objectProto: CustomEventProto, _session: Session | null): CustomEvent {
        const _NodeDefinitionReference = STRUCT_CLASS_BY_TYPE[13] as typeof NodeDefinitionReference;
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
        const unpackedSelfTraits: any[] = [];
        if (objectProto.selfTraits) {
          for (const item of objectProto.selfTraits) {
            unpackedSelfTraits.push(_NodeDefinitionReference.unpack(10, item, _session) as NodeDefinitionReference);
          }
        }
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[20000] as typeof CustomEvent)({
          icon: objectProto.icon != undefined ? _Icon.unpack(10, objectProto.icon, _session) as Icon : null,
          baseType: objectProto.baseType != undefined ? _NodeDefinitionReference.unpack(10, objectProto.baseType, _session) as NodeDefinitionReference : null,
          selfTraits: unpackedSelfTraits,
          isAbstract: objectProto.isAbstract,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: CustomEvent): Uint8Array {
        const proto = this.packObject(object);
        return CustomEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): CustomEvent {
        const proto = CustomEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 20000)] = new CustomEventProtoEncoder();

    class EditEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: EditEvent): EditEventProto {
        const objectProto: Partial<EditEventProto> = { metatype: 90100 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        objectProto.type = Number(object.type) as any;
        objectProto.nodePtr = object.nodePtr.pack(10);
        if (object.operation != null) {
          objectProto.operation = Number(object.operation) as any;
        }
        if (object.propertyId != null) {
          objectProto.propertyId = object.propertyId;
        }
        if (object.customPropertyPtr != null) {
          objectProto.customPropertyPtr = object.customPropertyPtr.pack(10);
        }
        if (object.key != null) {
          objectProto.key = object.key.pack(10);
        }
        if (object.value != null) {
          objectProto.value = object.value.pack(10);
        }
        if (object.reverseOperation != null) {
          objectProto.reverseOperation = Number(object.reverseOperation) as any;
        }
        if (object.reverseValue != null) {
          objectProto.reverseValue = object.reverseValue.pack(10);
        }
        return objectProto as EditEventProto;
      }

      unpackObject(objectProto: EditEventProto, _session: Session | null): EditEvent {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[90100] as typeof EditEvent)({
          type: Number(objectProto.type) as any,
          node: _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference,
          operation: objectProto.operation != undefined ? Number(objectProto.operation) as any : null,
          propertyId: objectProto.propertyId != undefined ? Number(objectProto.propertyId) : null,
          customProperty: objectProto.customPropertyPtr != undefined ? _NodeReference.unpack(10, objectProto.customPropertyPtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? _Value.unpack(10, objectProto.key, _session) as Value : null,
          value: objectProto.value != undefined ? _Value.unpack(10, objectProto.value, _session) as Value : null,
          reverseOperation: objectProto.reverseOperation != undefined ? Number(objectProto.reverseOperation) as any : null,
          reverseValue: objectProto.reverseValue != undefined ? _Value.unpack(10, objectProto.reverseValue, _session) as Value : null,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: EditEvent): Uint8Array {
        const proto = this.packObject(object);
        return EditEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): EditEvent {
        const proto = EditEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 90100)] = new EditEventProtoEncoder();

    class PermissionProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Permission): PermissionProto {
        const objectProto: Partial<PermissionProto> = { metatype: 50000 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        return objectProto as PermissionProto;
      }

      unpackObject(objectProto: PermissionProto, _session: Session | null): Permission {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[50000] as typeof Permission)({
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: Permission): Uint8Array {
        const proto = this.packObject(object);
        return PermissionProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Permission {
        const proto = PermissionProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 50000)] = new PermissionProtoEncoder();

    class MethodProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Method): MethodProto {
        const objectProto: Partial<MethodProto> = { metatype: 40000 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        objectProto.type = Number(object._type) as any;
        if (object._text != null) {
          objectProto.text = object._text.pack(10);
        }
        objectProto.cardinality = Number(object._cardinality) as any;
        if (object._platforms) {
          const packedPlatforms: any[] = [];
          for (const item of object._platforms) {
            packedPlatforms.push(Number(item) as any);
          }
          objectProto.platforms = packedPlatforms;
        }
        if (object._languages) {
          const packedLanguages: any[] = [];
          for (const item of object._languages) {
            packedLanguages.push(Number(item) as any);
          }
          objectProto.languages = packedLanguages;
        }
        return objectProto as MethodProto;
      }

      unpackObject(objectProto: MethodProto, _session: Session | null): Method {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Text = STRUCT_CLASS_BY_TYPE[400020] as typeof Text;
        const unpackedPlatforms: any[] = [];
        if (objectProto.platforms) {
          for (const item of objectProto.platforms) {
            unpackedPlatforms.push(Number(item) as any);
          }
        }
        const unpackedLanguages: any[] = [];
        if (objectProto.languages) {
          for (const item of objectProto.languages) {
            unpackedLanguages.push(Number(item) as any);
          }
        }
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[40000] as typeof Method)({
          type: Number(objectProto.type) as any,
          text: objectProto.text != undefined ? _Text.unpack(10, objectProto.text, _session) as Text : null,
          cardinality: Number(objectProto.cardinality) as any,
          platforms: unpackedPlatforms,
          languages: unpackedLanguages,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: Method): Uint8Array {
        const proto = this.packObject(object);
        return MethodProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Method {
        const proto = MethodProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 40000)] = new MethodProtoEncoder();

    class ActionProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Action): ActionProto {
        const objectProto: Partial<ActionProto> = { metatype: 40100 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        objectProto.type = Number(object._type) as any;
        if (object._text != null) {
          objectProto.text = object._text.pack(10);
        }
        objectProto.cardinality = Number(object._cardinality) as any;
        if (object._platforms) {
          const packedPlatforms: any[] = [];
          for (const item of object._platforms) {
            packedPlatforms.push(Number(item) as any);
          }
          objectProto.platforms = packedPlatforms;
        }
        if (object._languages) {
          const packedLanguages: any[] = [];
          for (const item of object._languages) {
            packedLanguages.push(Number(item) as any);
          }
          objectProto.languages = packedLanguages;
        }
        return objectProto as ActionProto;
      }

      unpackObject(objectProto: ActionProto, _session: Session | null): Action {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Text = STRUCT_CLASS_BY_TYPE[400020] as typeof Text;
        const unpackedPlatforms: any[] = [];
        if (objectProto.platforms) {
          for (const item of objectProto.platforms) {
            unpackedPlatforms.push(Number(item) as any);
          }
        }
        const unpackedLanguages: any[] = [];
        if (objectProto.languages) {
          for (const item of objectProto.languages) {
            unpackedLanguages.push(Number(item) as any);
          }
        }
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[40100] as typeof Action)({
          type: Number(objectProto.type) as any,
          text: objectProto.text != undefined ? _Text.unpack(10, objectProto.text, _session) as Text : null,
          cardinality: Number(objectProto.cardinality) as any,
          platforms: unpackedPlatforms,
          languages: unpackedLanguages,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: Action): Uint8Array {
        const proto = this.packObject(object);
        return ActionProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Action {
        const proto = ActionProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 40100)] = new ActionProtoEncoder();

    class CustomEnumProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: CustomEnum): CustomEnumProto {
        const objectProto: Partial<CustomEnumProto> = { metatype: 20200 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        if (object._icon != null) {
          objectProto.icon = object._icon.pack(10);
        }
        return objectProto as CustomEnumProto;
      }

      unpackObject(objectProto: CustomEnumProto, _session: Session | null): CustomEnum {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[20200] as typeof CustomEnum)({
          icon: objectProto.icon != undefined ? _Icon.unpack(10, objectProto.icon, _session) as Icon : null,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: CustomEnum): Uint8Array {
        const proto = this.packObject(object);
        return CustomEnumProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): CustomEnum {
        const proto = CustomEnumProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 20200)] = new CustomEnumProtoEncoder();

    class CustomOptionProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: CustomOption): CustomOptionProto {
        const objectProto: Partial<CustomOptionProto> = { metatype: 20400 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        if (object._icon != null) {
          objectProto.icon = object._icon.pack(10);
        }
        return objectProto as CustomOptionProto;
      }

      unpackObject(objectProto: CustomOptionProto, _session: Session | null): CustomOption {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[20400] as typeof CustomOption)({
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          icon: objectProto.icon != undefined ? _Icon.unpack(10, objectProto.icon, _session) as Icon : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: CustomOption): Uint8Array {
        const proto = this.packObject(object);
        return CustomOptionProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): CustomOption {
        const proto = CustomOptionProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 20400)] = new CustomOptionProtoEncoder();

    class IndexProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Index): IndexProto {
        const objectProto: Partial<IndexProto> = { metatype: 30100 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        objectProto.type = Number(object._type) as any;
        if (object._properties) {
          const packedProperties: any[] = [];
          for (const item of object._properties) {
            packedProperties.push(item.pack(10));
          }
          objectProto.properties = packedProperties;
        }
        return objectProto as IndexProto;
      }

      unpackObject(objectProto: IndexProto, _session: Session | null): Index {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _PropertyReference = STRUCT_CLASS_BY_TYPE[1001] as typeof PropertyReference;
        const unpackedProperties: any[] = [];
        if (objectProto.properties) {
          for (const item of objectProto.properties) {
            unpackedProperties.push(_PropertyReference.unpack(10, item, _session) as PropertyReference);
          }
        }
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[30100] as typeof Index)({
          type: Number(objectProto.type) as any,
          properties: unpackedProperties,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: Index): Uint8Array {
        const proto = this.packObject(object);
        return IndexProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Index {
        const proto = IndexProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 30100)] = new IndexProtoEncoder();

    class ConstraintProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Constraint): ConstraintProto {
        const objectProto: Partial<ConstraintProto> = { metatype: 30200 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        objectProto.type = Number(object._type) as any;
        if (object._properties) {
          const packedProperties: any[] = [];
          for (const item of object._properties) {
            packedProperties.push(item.pack(10));
          }
          objectProto.properties = packedProperties;
        }
        return objectProto as ConstraintProto;
      }

      unpackObject(objectProto: ConstraintProto, _session: Session | null): Constraint {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _PropertyReference = STRUCT_CLASS_BY_TYPE[1001] as typeof PropertyReference;
        const unpackedProperties: any[] = [];
        if (objectProto.properties) {
          for (const item of objectProto.properties) {
            unpackedProperties.push(_PropertyReference.unpack(10, item, _session) as PropertyReference);
          }
        }
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[30200] as typeof Constraint)({
          type: Number(objectProto.type) as any,
          properties: unpackedProperties,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: Constraint): Uint8Array {
        const proto = this.packObject(object);
        return ConstraintProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Constraint {
        const proto = ConstraintProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 30200)] = new ConstraintProtoEncoder();

    class MigrationProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Migration): MigrationProto {
        const objectProto: Partial<MigrationProto> = { metatype: 31000 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        objectProto.type = Number(object._type) as any;
        return objectProto as MigrationProto;
      }

      unpackObject(objectProto: MigrationProto, _session: Session | null): Migration {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[31000] as typeof Migration)({
          type: Number(objectProto.type) as any,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: Migration): Uint8Array {
        const proto = this.packObject(object);
        return MigrationProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Migration {
        const proto = MigrationProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 31000)] = new MigrationProtoEncoder();

    class MigrationOperationProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: MigrationOperation): MigrationOperationProto {
        const objectProto: Partial<MigrationOperationProto> = { metatype: 31100 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        return objectProto as MigrationOperationProto;
      }

      unpackObject(objectProto: MigrationOperationProto, _session: Session | null): MigrationOperation {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[31100] as typeof MigrationOperation)({
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: MigrationOperation): Uint8Array {
        const proto = this.packObject(object);
        return MigrationOperationProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): MigrationOperation {
        const proto = MigrationOperationProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 31100)] = new MigrationOperationProtoEncoder();

    class CustomPropertyProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: CustomProperty): CustomPropertyProto {
        const objectProto: Partial<CustomPropertyProto> = { metatype: 20300 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        objectProto.type = Number(object._type) as any;
        if (object._icon != null) {
          objectProto.icon = object._icon.pack(10);
        }
        objectProto.cardinality = Number(object._cardinality) as any;
        objectProto.scalarType = Number(object._scalarType) as any;
        if (object._primitiveType != null) {
          objectProto.primitiveType = Number(object._primitiveType) as any;
        }
        if (object._enumType != null) {
          objectProto.enumType = Number(object._enumType) as any;
        }
        if (object._nodeTypes) {
          const packedNodeTypes: any[] = [];
          for (const item of object._nodeTypes) {
            packedNodeTypes.push(Number(item) as any);
          }
          objectProto.nodeTypes = packedNodeTypes;
        }
        if (object._structType != null) {
          objectProto.structType = Number(object._structType) as any;
        }
        if (object._keyType != null) {
          objectProto.keyType = object._keyType.pack(10);
        }
        if (object._value != null) {
          objectProto.value = object._value.pack(10);
        }
        if (object._valueFactory != null) {
          objectProto.valueFactory = Number(object._valueFactory) as any;
        }
        if (object._collectionConstraint != null) {
          objectProto.collectionConstraint = object._collectionConstraint.pack(10);
        }
        if (object._stringConstraint != null) {
          objectProto.stringConstraint = object._stringConstraint.pack(10);
        }
        if (object._numberConstraint != null) {
          objectProto.numberConstraint = object._numberConstraint.pack(10);
        }
        if (object._edgeType != null) {
          objectProto.edgeType = Number(object._edgeType) as any;
        }
        if (object._cascade != null) {
          objectProto.cascade = Number(object._cascade) as any;
        }
        if (object._isRequired != null) {
          objectProto.isRequired = object._isRequired;
        }
        if (object._isUnique != null) {
          objectProto.isUnique = object._isUnique;
        }
        if (object._isComputed != null) {
          objectProto.isComputed = object._isComputed;
        }
        if (object._isReadonly != null) {
          objectProto.isReadonly = object._isReadonly;
        }
        if (object._isMain != null) {
          objectProto.isMain = object._isMain;
        }
        return objectProto as CustomPropertyProto;
      }

      unpackObject(objectProto: CustomPropertyProto, _session: Session | null): CustomProperty {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _Type = STRUCT_CLASS_BY_TYPE[101] as typeof Type;
        const _NumberConstraint = STRUCT_CLASS_BY_TYPE[110] as typeof NumberConstraint;
        const _StringConstraint = STRUCT_CLASS_BY_TYPE[111] as typeof StringConstraint;
        const _CollectionConstraint = STRUCT_CLASS_BY_TYPE[112] as typeof CollectionConstraint;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
        const unpackedNodeTypes: any[] = [];
        if (objectProto.nodeTypes) {
          for (const item of objectProto.nodeTypes) {
            unpackedNodeTypes.push(Number(item) as any);
          }
        }
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[20300] as typeof CustomProperty)({
          type: Number(objectProto.type) as any,
          icon: objectProto.icon != undefined ? _Icon.unpack(10, objectProto.icon, _session) as Icon : null,
          cardinality: Number(objectProto.cardinality) as any,
          scalarType: Number(objectProto.scalarType) as any,
          primitiveType: objectProto.primitiveType != undefined ? Number(objectProto.primitiveType) as any : null,
          enumType: objectProto.enumType != undefined ? Number(objectProto.enumType) as any : null,
          nodeTypes: unpackedNodeTypes,
          structType: objectProto.structType != undefined ? Number(objectProto.structType) as any : null,
          keyType: objectProto.keyType != undefined ? _Type.unpack(10, objectProto.keyType, _session) as Type : null,
          value: objectProto.value != undefined ? _Value.unpack(10, objectProto.value, _session) as Value : null,
          valueFactory: objectProto.valueFactory != undefined ? Number(objectProto.valueFactory) as any : null,
          collectionConstraint: objectProto.collectionConstraint != undefined ? _CollectionConstraint.unpack(10, objectProto.collectionConstraint, _session) as CollectionConstraint : null,
          stringConstraint: objectProto.stringConstraint != undefined ? _StringConstraint.unpack(10, objectProto.stringConstraint, _session) as StringConstraint : null,
          numberConstraint: objectProto.numberConstraint != undefined ? _NumberConstraint.unpack(10, objectProto.numberConstraint, _session) as NumberConstraint : null,
          edgeType: objectProto.edgeType != undefined ? Number(objectProto.edgeType) as any : null,
          cascade: objectProto.cascade != undefined ? Number(objectProto.cascade) as any : null,
          isRequired: objectProto.isRequired != undefined ? objectProto.isRequired : null,
          isUnique: objectProto.isUnique != undefined ? objectProto.isUnique : null,
          isComputed: objectProto.isComputed != undefined ? objectProto.isComputed : null,
          isReadonly: objectProto.isReadonly != undefined ? objectProto.isReadonly : null,
          isMain: objectProto.isMain != undefined ? objectProto.isMain : null,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: CustomProperty): Uint8Array {
        const proto = this.packObject(object);
        return CustomPropertyProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): CustomProperty {
        const proto = CustomPropertyProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 20300)] = new CustomPropertyProtoEncoder();

    class SpaceProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Space): SpaceProto {
        const objectProto: Partial<SpaceProto> = { metatype: 1100 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        objectProto.slug = object._slug;
        if (object._handlePtr != null) {
          objectProto.handlePtr = object._handlePtr.pack(10);
        }
        objectProto.region = Number(object._region) as any;
        return objectProto as SpaceProto;
      }

      unpackObject(objectProto: SpaceProto, _session: Session | null): Space {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[1100] as typeof Space)({
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          slug: objectProto.slug,
          handle: objectProto.handlePtr != undefined ? _NodeReference.unpack(10, objectProto.handlePtr, _session) as NodeReference : null,
          region: Number(objectProto.region) as any,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          _session,
        });
      }

      packObjectBytes(object: Space): Uint8Array {
        const proto = this.packObject(object);
        return SpaceProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Space {
        const proto = SpaceProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1100)] = new SpaceProtoEncoder();

    class CustomStructProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: CustomStruct): CustomStructProto {
        const objectProto: Partial<CustomStructProto> = { metatype: 20100 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        if (object._icon != null) {
          objectProto.icon = object._icon.pack(10);
        }
        if (object._baseType != null) {
          objectProto.baseType = object._baseType.pack(10);
        }
        return objectProto as CustomStructProto;
      }

      unpackObject(objectProto: CustomStructProto, _session: Session | null): CustomStruct {
        const _StructDefinitionReference = STRUCT_CLASS_BY_TYPE[16] as typeof StructDefinitionReference;
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[20100] as typeof CustomStruct)({
          icon: objectProto.icon != undefined ? _Icon.unpack(10, objectProto.icon, _session) as Icon : null,
          baseType: objectProto.baseType != undefined ? _StructDefinitionReference.unpack(10, objectProto.baseType, _session) as StructDefinitionReference : null,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: CustomStruct): Uint8Array {
        const proto = this.packObject(object);
        return CustomStructProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): CustomStruct {
        const proto = CustomStructProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 20100)] = new CustomStructProtoEncoder();

    class BranchProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Branch): BranchProto {
        const objectProto: Partial<BranchProto> = { metatype: 2000 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        objectProto.type = Number(object._type) as any;
        return objectProto as BranchProto;
      }

      unpackObject(objectProto: BranchProto, _session: Session | null): Branch {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[2000] as typeof Branch)({
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          type: Number(objectProto.type) as any,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: Branch): Uint8Array {
        const proto = this.packObject(object);
        return BranchProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Branch {
        const proto = BranchProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 2000)] = new BranchProtoEncoder();

    class SnapshotProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Snapshot): SnapshotProto {
        const objectProto: Partial<SnapshotProto> = { metatype: 2100 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        objectProto.type = Number(object._type) as any;
        objectProto.status = Number(object._status) as any;
        return objectProto as SnapshotProto;
      }

      unpackObject(objectProto: SnapshotProto, _session: Session | null): Snapshot {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[2100] as typeof Snapshot)({
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          type: Number(objectProto.type) as any,
          status: Number(objectProto.status) as any,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: Snapshot): Uint8Array {
        const proto = this.packObject(object);
        return SnapshotProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Snapshot {
        const proto = SnapshotProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 2100)] = new SnapshotProtoEncoder();

    class EntitlementRequestedEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: EntitlementRequestedEvent): EntitlementRequestedEventProto {
        const objectProto: Partial<EntitlementRequestedEventProto> = { metatype: 360502 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        objectProto.nodePtr = object.nodePtr.pack(10);
        objectProto.targetPtr = object.targetPtr.pack(10);
        return objectProto as EntitlementRequestedEventProto;
      }

      unpackObject(objectProto: EntitlementRequestedEventProto, _session: Session | null): EntitlementRequestedEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[360502] as typeof EntitlementRequestedEvent)({
          node: _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference,
          target: _NodeReference.unpack(10, objectProto.targetPtr, _session) as NodeReference,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: EntitlementRequestedEvent): Uint8Array {
        const proto = this.packObject(object);
        return EntitlementRequestedEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): EntitlementRequestedEvent {
        const proto = EntitlementRequestedEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 360502)] = new EntitlementRequestedEventProtoEncoder();

    class EntitlementGrantedEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: EntitlementGrantedEvent): EntitlementGrantedEventProto {
        const objectProto: Partial<EntitlementGrantedEventProto> = { metatype: 360503 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        objectProto.nodePtr = object.nodePtr.pack(10);
        objectProto.targetPtr = object.targetPtr.pack(10);
        return objectProto as EntitlementGrantedEventProto;
      }

      unpackObject(objectProto: EntitlementGrantedEventProto, _session: Session | null): EntitlementGrantedEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[360503] as typeof EntitlementGrantedEvent)({
          node: _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference,
          target: _NodeReference.unpack(10, objectProto.targetPtr, _session) as NodeReference,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: EntitlementGrantedEvent): Uint8Array {
        const proto = this.packObject(object);
        return EntitlementGrantedEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): EntitlementGrantedEvent {
        const proto = EntitlementGrantedEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 360503)] = new EntitlementGrantedEventProtoEncoder();

    class EntitlementRevokedEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: EntitlementRevokedEvent): EntitlementRevokedEventProto {
        const objectProto: Partial<EntitlementRevokedEventProto> = { metatype: 360504 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        objectProto.nodePtr = object.nodePtr.pack(10);
        objectProto.targetPtr = object.targetPtr.pack(10);
        return objectProto as EntitlementRevokedEventProto;
      }

      unpackObject(objectProto: EntitlementRevokedEventProto, _session: Session | null): EntitlementRevokedEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[360504] as typeof EntitlementRevokedEvent)({
          node: _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference,
          target: _NodeReference.unpack(10, objectProto.targetPtr, _session) as NodeReference,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: EntitlementRevokedEvent): Uint8Array {
        const proto = this.packObject(object);
        return EntitlementRevokedEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): EntitlementRevokedEvent {
        const proto = EntitlementRevokedEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 360504)] = new EntitlementRevokedEventProtoEncoder();

    class EntitlementExpiredEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: EntitlementExpiredEvent): EntitlementExpiredEventProto {
        const objectProto: Partial<EntitlementExpiredEventProto> = { metatype: 360505 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        objectProto.nodePtr = object.nodePtr.pack(10);
        objectProto.targetPtr = object.targetPtr.pack(10);
        return objectProto as EntitlementExpiredEventProto;
      }

      unpackObject(objectProto: EntitlementExpiredEventProto, _session: Session | null): EntitlementExpiredEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[360505] as typeof EntitlementExpiredEvent)({
          node: _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference,
          target: _NodeReference.unpack(10, objectProto.targetPtr, _session) as NodeReference,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: EntitlementExpiredEvent): Uint8Array {
        const proto = this.packObject(object);
        return EntitlementExpiredEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): EntitlementExpiredEvent {
        const proto = EntitlementExpiredEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 360505)] = new EntitlementExpiredEventProtoEncoder();

    class EntitlementProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Entitlement): EntitlementProto {
        const objectProto: Partial<EntitlementProto> = { metatype: 360500 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        objectProto.type = Number(object._type) as any;
        if (object._expiresAt != null) {
          objectProto.expiresAt = packProtoTimestamp(object._expiresAt);
        }
        objectProto.targetPtr = object._targetPtr.pack(10);
        return objectProto as EntitlementProto;
      }

      unpackObject(objectProto: EntitlementProto, _session: Session | null): Entitlement {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[360500] as typeof Entitlement)({
          type: Number(objectProto.type) as any,
          expiresAt: objectProto.expiresAt != undefined ? unpackProtoTimestamp(objectProto.expiresAt!) : null,
          target: _NodeReference.unpack(10, objectProto.targetPtr, _session) as NodeReference,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: Entitlement): Uint8Array {
        const proto = this.packObject(object);
        return EntitlementProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Entitlement {
        const proto = EntitlementProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 360500)] = new EntitlementProtoEncoder();

    class InviteSentEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: InviteSentEvent): InviteSentEventProto {
        const objectProto: Partial<InviteSentEventProto> = { metatype: 360102 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        objectProto.nodePtr = object.nodePtr.pack(10);
        objectProto.joinablePtr = object.joinablePtr.pack(10);
        objectProto.memberPtr = object.memberPtr.pack(10);
        objectProto.rolePtr = object.rolePtr.pack(10);
        objectProto.roleType = Number(object.roleType) as any;
        return objectProto as InviteSentEventProto;
      }

      unpackObject(objectProto: InviteSentEventProto, _session: Session | null): InviteSentEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[360102] as typeof InviteSentEvent)({
          role: _NodeReference.unpack(10, objectProto.rolePtr, _session) as NodeReference,
          roleType: Number(objectProto.roleType) as any,
          node: _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference,
          joinable: _NodeReference.unpack(10, objectProto.joinablePtr, _session) as NodeReference,
          member: _NodeReference.unpack(10, objectProto.memberPtr, _session) as NodeReference,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: InviteSentEvent): Uint8Array {
        const proto = this.packObject(object);
        return InviteSentEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): InviteSentEvent {
        const proto = InviteSentEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 360102)] = new InviteSentEventProtoEncoder();

    class InviteRescindedEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: InviteRescindedEvent): InviteRescindedEventProto {
        const objectProto: Partial<InviteRescindedEventProto> = { metatype: 360103 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        objectProto.nodePtr = object.nodePtr.pack(10);
        objectProto.joinablePtr = object.joinablePtr.pack(10);
        objectProto.memberPtr = object.memberPtr.pack(10);
        return objectProto as InviteRescindedEventProto;
      }

      unpackObject(objectProto: InviteRescindedEventProto, _session: Session | null): InviteRescindedEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[360103] as typeof InviteRescindedEvent)({
          node: _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference,
          joinable: _NodeReference.unpack(10, objectProto.joinablePtr, _session) as NodeReference,
          member: _NodeReference.unpack(10, objectProto.memberPtr, _session) as NodeReference,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: InviteRescindedEvent): Uint8Array {
        const proto = this.packObject(object);
        return InviteRescindedEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): InviteRescindedEvent {
        const proto = InviteRescindedEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 360103)] = new InviteRescindedEventProtoEncoder();

    class InviteAcceptedEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: InviteAcceptedEvent): InviteAcceptedEventProto {
        const objectProto: Partial<InviteAcceptedEventProto> = { metatype: 360104 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        objectProto.nodePtr = object.nodePtr.pack(10);
        objectProto.joinablePtr = object.joinablePtr.pack(10);
        objectProto.memberPtr = object.memberPtr.pack(10);
        objectProto.rolePtr = object.rolePtr.pack(10);
        objectProto.roleType = Number(object.roleType) as any;
        return objectProto as InviteAcceptedEventProto;
      }

      unpackObject(objectProto: InviteAcceptedEventProto, _session: Session | null): InviteAcceptedEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[360104] as typeof InviteAcceptedEvent)({
          role: _NodeReference.unpack(10, objectProto.rolePtr, _session) as NodeReference,
          roleType: Number(objectProto.roleType) as any,
          node: _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference,
          joinable: _NodeReference.unpack(10, objectProto.joinablePtr, _session) as NodeReference,
          member: _NodeReference.unpack(10, objectProto.memberPtr, _session) as NodeReference,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: InviteAcceptedEvent): Uint8Array {
        const proto = this.packObject(object);
        return InviteAcceptedEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): InviteAcceptedEvent {
        const proto = InviteAcceptedEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 360104)] = new InviteAcceptedEventProtoEncoder();

    class InviteRejectedEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: InviteRejectedEvent): InviteRejectedEventProto {
        const objectProto: Partial<InviteRejectedEventProto> = { metatype: 360105 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        objectProto.nodePtr = object.nodePtr.pack(10);
        objectProto.joinablePtr = object.joinablePtr.pack(10);
        objectProto.memberPtr = object.memberPtr.pack(10);
        return objectProto as InviteRejectedEventProto;
      }

      unpackObject(objectProto: InviteRejectedEventProto, _session: Session | null): InviteRejectedEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[360105] as typeof InviteRejectedEvent)({
          node: _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference,
          joinable: _NodeReference.unpack(10, objectProto.joinablePtr, _session) as NodeReference,
          member: _NodeReference.unpack(10, objectProto.memberPtr, _session) as NodeReference,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: InviteRejectedEvent): Uint8Array {
        const proto = this.packObject(object);
        return InviteRejectedEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): InviteRejectedEvent {
        const proto = InviteRejectedEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 360105)] = new InviteRejectedEventProtoEncoder();

    class InviteProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Invite): InviteProto {
        const objectProto: Partial<InviteProto> = { metatype: 360100 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        objectProto.memberPtr = object._memberPtr.pack(10);
        if (object._rolePtr != null) {
          objectProto.rolePtr = object._rolePtr.pack(10);
        }
        if (object._roleType != null) {
          objectProto.roleType = Number(object._roleType) as any;
        }
        return objectProto as InviteProto;
      }

      unpackObject(objectProto: InviteProto, _session: Session | null): Invite {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[360100] as typeof Invite)({
          member: _NodeReference.unpack(10, objectProto.memberPtr, _session) as NodeReference,
          role: objectProto.rolePtr != undefined ? _NodeReference.unpack(10, objectProto.rolePtr, _session) as NodeReference : null,
          roleType: objectProto.roleType != undefined ? Number(objectProto.roleType) as any : null,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: Invite): Uint8Array {
        const proto = this.packObject(object);
        return InviteProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Invite {
        const proto = InviteProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 360100)] = new InviteProtoEncoder();

    class MembershipJoinedEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: MembershipJoinedEvent): MembershipJoinedEventProto {
        const objectProto: Partial<MembershipJoinedEventProto> = { metatype: 360002 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        objectProto.nodePtr = object.nodePtr.pack(10);
        objectProto.joinablePtr = object.joinablePtr.pack(10);
        objectProto.memberPtr = object.memberPtr.pack(10);
        objectProto.rolePtr = object.rolePtr.pack(10);
        objectProto.roleType = Number(object.roleType) as any;
        return objectProto as MembershipJoinedEventProto;
      }

      unpackObject(objectProto: MembershipJoinedEventProto, _session: Session | null): MembershipJoinedEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[360002] as typeof MembershipJoinedEvent)({
          role: _NodeReference.unpack(10, objectProto.rolePtr, _session) as NodeReference,
          roleType: Number(objectProto.roleType) as any,
          node: _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference,
          joinable: _NodeReference.unpack(10, objectProto.joinablePtr, _session) as NodeReference,
          member: _NodeReference.unpack(10, objectProto.memberPtr, _session) as NodeReference,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: MembershipJoinedEvent): Uint8Array {
        const proto = this.packObject(object);
        return MembershipJoinedEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): MembershipJoinedEvent {
        const proto = MembershipJoinedEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 360002)] = new MembershipJoinedEventProtoEncoder();

    class MembershipLeftEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: MembershipLeftEvent): MembershipLeftEventProto {
        const objectProto: Partial<MembershipLeftEventProto> = { metatype: 360003 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        objectProto.nodePtr = object.nodePtr.pack(10);
        objectProto.joinablePtr = object.joinablePtr.pack(10);
        objectProto.memberPtr = object.memberPtr.pack(10);
        return objectProto as MembershipLeftEventProto;
      }

      unpackObject(objectProto: MembershipLeftEventProto, _session: Session | null): MembershipLeftEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[360003] as typeof MembershipLeftEvent)({
          node: _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference,
          joinable: _NodeReference.unpack(10, objectProto.joinablePtr, _session) as NodeReference,
          member: _NodeReference.unpack(10, objectProto.memberPtr, _session) as NodeReference,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: MembershipLeftEvent): Uint8Array {
        const proto = this.packObject(object);
        return MembershipLeftEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): MembershipLeftEvent {
        const proto = MembershipLeftEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 360003)] = new MembershipLeftEventProtoEncoder();

    class MembershipProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Membership): MembershipProto {
        const objectProto: Partial<MembershipProto> = { metatype: 360000 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        objectProto.memberPtr = object._memberPtr.pack(10);
        if (object._rolePtr != null) {
          objectProto.rolePtr = object._rolePtr.pack(10);
        }
        if (object._roleType != null) {
          objectProto.roleType = Number(object._roleType) as any;
        }
        return objectProto as MembershipProto;
      }

      unpackObject(objectProto: MembershipProto, _session: Session | null): Membership {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[360000] as typeof Membership)({
          member: _NodeReference.unpack(10, objectProto.memberPtr, _session) as NodeReference,
          role: objectProto.rolePtr != undefined ? _NodeReference.unpack(10, objectProto.rolePtr, _session) as NodeReference : null,
          roleType: objectProto.roleType != undefined ? Number(objectProto.roleType) as any : null,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: Membership): Uint8Array {
        const proto = this.packObject(object);
        return MembershipProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Membership {
        const proto = MembershipProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 360000)] = new MembershipProtoEncoder();

    class RoleAssignedEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: RoleAssignedEvent): RoleAssignedEventProto {
        const objectProto: Partial<RoleAssignedEventProto> = { metatype: 360202 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        objectProto.nodePtr = object.nodePtr.pack(10);
        objectProto.actorPtr = object.actorPtr.pack(10);
        return objectProto as RoleAssignedEventProto;
      }

      unpackObject(objectProto: RoleAssignedEventProto, _session: Session | null): RoleAssignedEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[360202] as typeof RoleAssignedEvent)({
          node: _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference,
          actor: _NodeReference.unpack(10, objectProto.actorPtr, _session) as NodeReference,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: RoleAssignedEvent): Uint8Array {
        const proto = this.packObject(object);
        return RoleAssignedEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): RoleAssignedEvent {
        const proto = RoleAssignedEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 360202)] = new RoleAssignedEventProtoEncoder();

    class RoleUnassignedEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: RoleUnassignedEvent): RoleUnassignedEventProto {
        const objectProto: Partial<RoleUnassignedEventProto> = { metatype: 360203 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        objectProto.nodePtr = object.nodePtr.pack(10);
        objectProto.actorPtr = object.actorPtr.pack(10);
        return objectProto as RoleUnassignedEventProto;
      }

      unpackObject(objectProto: RoleUnassignedEventProto, _session: Session | null): RoleUnassignedEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[360203] as typeof RoleUnassignedEvent)({
          node: _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference,
          actor: _NodeReference.unpack(10, objectProto.actorPtr, _session) as NodeReference,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: RoleUnassignedEvent): Uint8Array {
        const proto = this.packObject(object);
        return RoleUnassignedEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): RoleUnassignedEvent {
        const proto = RoleUnassignedEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 360203)] = new RoleUnassignedEventProtoEncoder();

    class RoleProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Role): RoleProto {
        const objectProto: Partial<RoleProto> = { metatype: 360200 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        objectProto.type = Number(object._type) as any;
        if (object._icon != null) {
          objectProto.icon = object._icon.pack(10);
        }
        return objectProto as RoleProto;
      }

      unpackObject(objectProto: RoleProto, _session: Session | null): Role {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[360200] as typeof Role)({
          type: Number(objectProto.type) as any,
          icon: objectProto.icon != undefined ? _Icon.unpack(10, objectProto.icon, _session) as Icon : null,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: Role): Uint8Array {
        const proto = this.packObject(object);
        return RoleProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Role {
        const proto = RoleProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 360200)] = new RoleProtoEncoder();

    class SanctionRequestedEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: SanctionRequestedEvent): SanctionRequestedEventProto {
        const objectProto: Partial<SanctionRequestedEventProto> = { metatype: 360402 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        objectProto.nodePtr = object.nodePtr.pack(10);
        objectProto.targetPtr = object.targetPtr.pack(10);
        return objectProto as SanctionRequestedEventProto;
      }

      unpackObject(objectProto: SanctionRequestedEventProto, _session: Session | null): SanctionRequestedEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[360402] as typeof SanctionRequestedEvent)({
          node: _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference,
          target: _NodeReference.unpack(10, objectProto.targetPtr, _session) as NodeReference,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: SanctionRequestedEvent): Uint8Array {
        const proto = this.packObject(object);
        return SanctionRequestedEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): SanctionRequestedEvent {
        const proto = SanctionRequestedEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 360402)] = new SanctionRequestedEventProtoEncoder();

    class SanctionGrantedEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: SanctionGrantedEvent): SanctionGrantedEventProto {
        const objectProto: Partial<SanctionGrantedEventProto> = { metatype: 360403 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        objectProto.nodePtr = object.nodePtr.pack(10);
        objectProto.targetPtr = object.targetPtr.pack(10);
        return objectProto as SanctionGrantedEventProto;
      }

      unpackObject(objectProto: SanctionGrantedEventProto, _session: Session | null): SanctionGrantedEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[360403] as typeof SanctionGrantedEvent)({
          node: _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference,
          target: _NodeReference.unpack(10, objectProto.targetPtr, _session) as NodeReference,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: SanctionGrantedEvent): Uint8Array {
        const proto = this.packObject(object);
        return SanctionGrantedEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): SanctionGrantedEvent {
        const proto = SanctionGrantedEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 360403)] = new SanctionGrantedEventProtoEncoder();

    class SanctionRevokedEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: SanctionRevokedEvent): SanctionRevokedEventProto {
        const objectProto: Partial<SanctionRevokedEventProto> = { metatype: 360404 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        objectProto.nodePtr = object.nodePtr.pack(10);
        objectProto.targetPtr = object.targetPtr.pack(10);
        return objectProto as SanctionRevokedEventProto;
      }

      unpackObject(objectProto: SanctionRevokedEventProto, _session: Session | null): SanctionRevokedEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[360404] as typeof SanctionRevokedEvent)({
          node: _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference,
          target: _NodeReference.unpack(10, objectProto.targetPtr, _session) as NodeReference,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: SanctionRevokedEvent): Uint8Array {
        const proto = this.packObject(object);
        return SanctionRevokedEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): SanctionRevokedEvent {
        const proto = SanctionRevokedEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 360404)] = new SanctionRevokedEventProtoEncoder();

    class SanctionExpiredEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: SanctionExpiredEvent): SanctionExpiredEventProto {
        const objectProto: Partial<SanctionExpiredEventProto> = { metatype: 360405 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        objectProto.nodePtr = object.nodePtr.pack(10);
        objectProto.targetPtr = object.targetPtr.pack(10);
        return objectProto as SanctionExpiredEventProto;
      }

      unpackObject(objectProto: SanctionExpiredEventProto, _session: Session | null): SanctionExpiredEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[360405] as typeof SanctionExpiredEvent)({
          node: _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference,
          target: _NodeReference.unpack(10, objectProto.targetPtr, _session) as NodeReference,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: SanctionExpiredEvent): Uint8Array {
        const proto = this.packObject(object);
        return SanctionExpiredEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): SanctionExpiredEvent {
        const proto = SanctionExpiredEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 360405)] = new SanctionExpiredEventProtoEncoder();

    class SanctionProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Sanction): SanctionProto {
        const objectProto: Partial<SanctionProto> = { metatype: 360400 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        objectProto.type = Number(object._type) as any;
        if (object._expiresAt != null) {
          objectProto.expiresAt = packProtoTimestamp(object._expiresAt);
        }
        objectProto.targetPtr = object._targetPtr.pack(10);
        return objectProto as SanctionProto;
      }

      unpackObject(objectProto: SanctionProto, _session: Session | null): Sanction {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[360400] as typeof Sanction)({
          type: Number(objectProto.type) as any,
          expiresAt: objectProto.expiresAt != undefined ? unpackProtoTimestamp(objectProto.expiresAt!) : null,
          target: _NodeReference.unpack(10, objectProto.targetPtr, _session) as NodeReference,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: Sanction): Uint8Array {
        const proto = this.packObject(object);
        return SanctionProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Sanction {
        const proto = SanctionProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 360400)] = new SanctionProtoEncoder();

    class ColorStyleProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: ColorStyle): ColorStyleProto {
        const objectProto: Partial<ColorStyleProto> = { metatype: 2100300 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        objectProto.type = Number(object._type) as any;
        if (object._hue != null) {
          objectProto.hue = Number(object._hue) as any;
        }
        if (object._shade != null) {
          objectProto.shade = Number(object._shade) as any;
        }
        if (object._intent != null) {
          objectProto.intent = Number(object._intent) as any;
        }
        if (object._x != null) {
          objectProto.x = object._x;
        }
        if (object._y != null) {
          objectProto.y = object._y;
        }
        if (object._z != null) {
          objectProto.z = object._z;
        }
        if (object._alpha != null) {
          objectProto.alpha = object._alpha;
        }
        if (object._dark != null) {
          objectProto.dark = object._dark.pack(10);
        }
        return objectProto as ColorStyleProto;
      }

      unpackObject(objectProto: ColorStyleProto, _session: Session | null): ColorStyle {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Color = STRUCT_CLASS_BY_TYPE[2100300] as typeof Color;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[2100300] as typeof ColorStyle)({
          type: Number(objectProto.type) as any,
          hue: objectProto.hue != undefined ? Number(objectProto.hue) as any : null,
          shade: objectProto.shade != undefined ? Number(objectProto.shade) as any : null,
          intent: objectProto.intent != undefined ? Number(objectProto.intent) as any : null,
          x: objectProto.x != undefined ? objectProto.x : null,
          y: objectProto.y != undefined ? objectProto.y : null,
          z: objectProto.z != undefined ? objectProto.z : null,
          alpha: objectProto.alpha != undefined ? objectProto.alpha : null,
          dark: objectProto.dark != undefined ? _Color.unpack(10, objectProto.dark, _session) as Color : null,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: ColorStyle): Uint8Array {
        const proto = this.packObject(object);
        return ColorStyleProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): ColorStyle {
        const proto = ColorStyleProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 2100300)] = new ColorStyleProtoEncoder();

    class BorderStyleProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: BorderStyle): BorderStyleProto {
        const objectProto: Partial<BorderStyleProto> = { metatype: 2100600 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        objectProto.type = Number(object._type) as any;
        if (object._color != null) {
          objectProto.color = object._color.pack(10);
        }
        if (object._width != null) {
          objectProto.width = object._width.pack(10);
        }
        if (object._stylePtr != null) {
          objectProto.stylePtr = object._stylePtr.pack(10);
        }
        return objectProto as BorderStyleProto;
      }

      unpackObject(objectProto: BorderStyleProto, _session: Session | null): BorderStyle {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Color = STRUCT_CLASS_BY_TYPE[2100300] as typeof Color;
        const _Inset2 = STRUCT_CLASS_BY_TYPE[2400023] as typeof Inset2;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[2100600] as typeof BorderStyle)({
          type: Number(objectProto.type) as any,
          color: objectProto.color != undefined ? _Color.unpack(10, objectProto.color, _session) as Color : null,
          width: objectProto.width != undefined ? _Inset2.unpack(10, objectProto.width, _session) as Inset2 : null,
          style: objectProto.stylePtr != undefined ? _NodeReference.unpack(10, objectProto.stylePtr, _session) as NodeReference : null,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: BorderStyle): Uint8Array {
        const proto = this.packObject(object);
        return BorderStyleProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): BorderStyle {
        const proto = BorderStyleProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 2100600)] = new BorderStyleProtoEncoder();

    class GradientStyleProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: GradientStyle): GradientStyleProto {
        const objectProto: Partial<GradientStyleProto> = { metatype: 2100800 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        objectProto.type = Number(object._type) as any;
        if (object._angle != null) {
          objectProto.angle = object._angle;
        }
        if (object._stops) {
          const packedStops: any[] = [];
          for (const item of object._stops) {
            packedStops.push(item.pack(10));
          }
          objectProto.stops = packedStops;
        }
        if (object._centerAnchor != null) {
          objectProto.centerAnchor = object._centerAnchor.pack(10);
        }
        if (object._dark != null) {
          objectProto.dark = object._dark.pack(10);
        }
        return objectProto as GradientStyleProto;
      }

      unpackObject(objectProto: GradientStyleProto, _session: Session | null): GradientStyle {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Gradient = STRUCT_CLASS_BY_TYPE[2100800] as typeof Gradient;
        const _GradientStop = STRUCT_CLASS_BY_TYPE[2100801] as typeof GradientStop;
        const _Axis2 = STRUCT_CLASS_BY_TYPE[2400025] as typeof Axis2;
        const unpackedStops: any[] = [];
        if (objectProto.stops) {
          for (const item of objectProto.stops) {
            unpackedStops.push(_GradientStop.unpack(10, item, _session) as GradientStop);
          }
        }
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[2100800] as typeof GradientStyle)({
          type: Number(objectProto.type) as any,
          angle: objectProto.angle != undefined ? objectProto.angle : null,
          stops: unpackedStops,
          centerAnchor: objectProto.centerAnchor != undefined ? _Axis2.unpack(10, objectProto.centerAnchor, _session) as Axis2 : null,
          dark: objectProto.dark != undefined ? _Gradient.unpack(10, objectProto.dark, _session) as Gradient : null,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: GradientStyle): Uint8Array {
        const proto = this.packObject(object);
        return GradientStyleProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): GradientStyle {
        const proto = GradientStyleProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 2100800)] = new GradientStyleProtoEncoder();

    class FillStyleProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: FillStyle): FillStyleProto {
        const objectProto: Partial<FillStyleProto> = { metatype: 2100400 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        objectProto.type = Number(object._type) as any;
        if (object._color != null) {
          objectProto.color = object._color.pack(10);
        }
        if (object._gradient != null) {
          objectProto.gradient = object._gradient.pack(10);
        }
        if (object._imagePtr != null) {
          objectProto.imagePtr = object._imagePtr.pack(10);
        }
        if (object._position != null) {
          objectProto.position = Number(object._position) as any;
        }
        if (object._size != null) {
          objectProto.size = Number(object._size) as any;
        }
        return objectProto as FillStyleProto;
      }

      unpackObject(objectProto: FillStyleProto, _session: Session | null): FillStyle {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Color = STRUCT_CLASS_BY_TYPE[2100300] as typeof Color;
        const _Gradient = STRUCT_CLASS_BY_TYPE[2100800] as typeof Gradient;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[2100400] as typeof FillStyle)({
          type: Number(objectProto.type) as any,
          color: objectProto.color != undefined ? _Color.unpack(10, objectProto.color, _session) as Color : null,
          gradient: objectProto.gradient != undefined ? _Gradient.unpack(10, objectProto.gradient, _session) as Gradient : null,
          image: objectProto.imagePtr != undefined ? _NodeReference.unpack(10, objectProto.imagePtr, _session) as NodeReference : null,
          position: objectProto.position != undefined ? Number(objectProto.position) as any : null,
          size: objectProto.size != undefined ? Number(objectProto.size) as any : null,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: FillStyle): Uint8Array {
        const proto = this.packObject(object);
        return FillStyleProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): FillStyle {
        const proto = FillStyleProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 2100400)] = new FillStyleProtoEncoder();

    class FontStyleProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: FontStyle): FontStyleProto {
        const objectProto: Partial<FontStyleProto> = { metatype: 2100500 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        objectProto.type = Number(object._type) as any;
        if (object._weight != null) {
          objectProto.weight = Number(object._weight) as any;
        }
        if (object._color != null) {
          objectProto.color = object._color.pack(10);
        }
        if (object._size != null) {
          objectProto.size = Number(object._size) as any;
        }
        if (object._align != null) {
          objectProto.align = Number(object._align) as any;
        }
        if (object._lineHeight != null) {
          objectProto.lineHeight = object._lineHeight.pack(10);
        }
        if (object._letterSpacing != null) {
          objectProto.letterSpacing = object._letterSpacing.pack(10);
        }
        if (object._decoration != null) {
          objectProto.decoration = Number(object._decoration) as any;
        }
        if (object._transform != null) {
          objectProto.transform = Number(object._transform) as any;
        }
        return objectProto as FontStyleProto;
      }

      unpackObject(objectProto: FontStyleProto, _session: Session | null): FontStyle {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Length = STRUCT_CLASS_BY_TYPE[1800001] as typeof Length;
        const _Fill = STRUCT_CLASS_BY_TYPE[2100400] as typeof Fill;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[2100500] as typeof FontStyle)({
          type: Number(objectProto.type) as any,
          weight: objectProto.weight != undefined ? Number(objectProto.weight) as any : null,
          color: objectProto.color != undefined ? _Fill.unpack(10, objectProto.color, _session) as Fill : null,
          size: objectProto.size != undefined ? Number(objectProto.size) as any : null,
          align: objectProto.align != undefined ? Number(objectProto.align) as any : null,
          lineHeight: objectProto.lineHeight != undefined ? _Length.unpack(10, objectProto.lineHeight, _session) as Length : null,
          letterSpacing: objectProto.letterSpacing != undefined ? _Length.unpack(10, objectProto.letterSpacing, _session) as Length : null,
          decoration: objectProto.decoration != undefined ? Number(objectProto.decoration) as any : null,
          transform: objectProto.transform != undefined ? Number(objectProto.transform) as any : null,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: FontStyle): Uint8Array {
        const proto = this.packObject(object);
        return FontStyleProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): FontStyle {
        const proto = FontStyleProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 2100500)] = new FontStyleProtoEncoder();

    class PaletteProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Palette): PaletteProto {
        const objectProto: Partial<PaletteProto> = { metatype: 2100100 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        if (object._icon != null) {
          objectProto.icon = object._icon.pack(10);
        }
        return objectProto as PaletteProto;
      }

      unpackObject(objectProto: PaletteProto, _session: Session | null): Palette {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[2100100] as typeof Palette)({
          icon: objectProto.icon != undefined ? _Icon.unpack(10, objectProto.icon, _session) as Icon : null,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: Palette): Uint8Array {
        const proto = this.packObject(object);
        return PaletteProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Palette {
        const proto = PaletteProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 2100100)] = new PaletteProtoEncoder();

    class ShadowStyleProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: ShadowStyle): ShadowStyleProto {
        const objectProto: Partial<ShadowStyleProto> = { metatype: 2100700 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        objectProto.type = Number(object._type) as any;
        if (object._color != null) {
          objectProto.color = object._color.pack(10);
        }
        objectProto.position = Number(object._position) as any;
        if (object._offset != null) {
          objectProto.offset = object._offset.pack(10);
        }
        if (object._blur != null) {
          objectProto.blur = object._blur;
        }
        if (object._spread != null) {
          objectProto.spread = object._spread;
        }
        if (object._diffusion != null) {
          objectProto.diffusion = object._diffusion;
        }
        return objectProto as ShadowStyleProto;
      }

      unpackObject(objectProto: ShadowStyleProto, _session: Session | null): ShadowStyle {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Color = STRUCT_CLASS_BY_TYPE[2100300] as typeof Color;
        const _Axis2 = STRUCT_CLASS_BY_TYPE[2400025] as typeof Axis2;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[2100700] as typeof ShadowStyle)({
          type: Number(objectProto.type) as any,
          color: objectProto.color != undefined ? _Color.unpack(10, objectProto.color, _session) as Color : null,
          position: Number(objectProto.position) as any,
          offset: objectProto.offset != undefined ? _Axis2.unpack(10, objectProto.offset, _session) as Axis2 : null,
          blur: objectProto.blur != undefined ? Number(objectProto.blur) : null,
          spread: objectProto.spread != undefined ? Number(objectProto.spread) : null,
          diffusion: objectProto.diffusion != undefined ? objectProto.diffusion : null,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: ShadowStyle): Uint8Array {
        const proto = this.packObject(object);
        return ShadowStyleProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): ShadowStyle {
        const proto = ShadowStyleProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 2100700)] = new ShadowStyleProtoEncoder();

    class StrokeStyleProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: StrokeStyle): StrokeStyleProto {
        const objectProto: Partial<StrokeStyleProto> = { metatype: 2101100 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        objectProto.type = Number(object._type) as any;
        objectProto.size = object._size;
        objectProto.thinning = object._thinning;
        objectProto.smoothing = object._smoothing;
        objectProto.streamline = object._streamline;
        objectProto.easing = Number(object._easing) as any;
        if (object._start != null) {
          objectProto.start = object._start.pack(10);
        }
        if (object._end != null) {
          objectProto.end = object._end.pack(10);
        }
        return objectProto as StrokeStyleProto;
      }

      unpackObject(objectProto: StrokeStyleProto, _session: Session | null): StrokeStyle {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _StrokeCap = STRUCT_CLASS_BY_TYPE[2101101] as typeof StrokeCap;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[2101100] as typeof StrokeStyle)({
          type: Number(objectProto.type) as any,
          size: Number(objectProto.size),
          thinning: objectProto.thinning,
          smoothing: objectProto.smoothing,
          streamline: objectProto.streamline,
          easing: Number(objectProto.easing) as any,
          start: objectProto.start != undefined ? _StrokeCap.unpack(10, objectProto.start, _session) as StrokeCap : null,
          end: objectProto.end != undefined ? _StrokeCap.unpack(10, objectProto.end, _session) as StrokeCap : null,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: StrokeStyle): Uint8Array {
        const proto = this.packObject(object);
        return StrokeStyleProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): StrokeStyle {
        const proto = StrokeStyleProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 2101100)] = new StrokeStyleProtoEncoder();

    class ThemeProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Theme): ThemeProto {
        const objectProto: Partial<ThemeProto> = { metatype: 2100000 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        return objectProto as ThemeProto;
      }

      unpackObject(objectProto: ThemeProto, _session: Session | null): Theme {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[2100000] as typeof Theme)({
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: Theme): Uint8Array {
        const proto = this.packObject(object);
        return ThemeProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Theme {
        const proto = ThemeProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 2100000)] = new ThemeProtoEncoder();

    class TransitionStyleProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: TransitionStyle): TransitionStyleProto {
        const objectProto: Partial<TransitionStyleProto> = { metatype: 2200000 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        objectProto.type = Number(object._type) as any;
        if (object._delay != null) {
          objectProto.delay = object._delay;
        }
        if (object._duration != null) {
          objectProto.duration = object._duration;
        }
        if (object._ease) {
          const packedEase: any[] = [];
          for (const item of object._ease) {
            packedEase.push(item);
          }
          objectProto.ease = packedEase;
        }
        if (object._stiffness != null) {
          objectProto.stiffness = object._stiffness;
        }
        if (object._damping != null) {
          objectProto.damping = object._damping;
        }
        if (object._mass != null) {
          objectProto.mass = object._mass;
        }
        if (object._bounce != null) {
          objectProto.bounce = object._bounce;
        }
        if (object._springType != null) {
          objectProto.springType = Number(object._springType) as any;
        }
        return objectProto as TransitionStyleProto;
      }

      unpackObject(objectProto: TransitionStyleProto, _session: Session | null): TransitionStyle {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const unpackedEase: any[] = [];
        if (objectProto.ease) {
          for (const item of objectProto.ease) {
            unpackedEase.push(item);
          }
        }
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[2200000] as typeof TransitionStyle)({
          type: Number(objectProto.type) as any,
          delay: objectProto.delay != undefined ? objectProto.delay : null,
          duration: objectProto.duration != undefined ? objectProto.duration : null,
          ease: unpackedEase,
          stiffness: objectProto.stiffness != undefined ? objectProto.stiffness : null,
          damping: objectProto.damping != undefined ? objectProto.damping : null,
          mass: objectProto.mass != undefined ? objectProto.mass : null,
          bounce: objectProto.bounce != undefined ? objectProto.bounce : null,
          springType: objectProto.springType != undefined ? Number(objectProto.springType) as any : null,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: TransitionStyle): Uint8Array {
        const proto = this.packObject(object);
        return TransitionStyleProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): TransitionStyle {
        const proto = TransitionStyleProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 2200000)] = new TransitionStyleProtoEncoder();

    class EffectStyleProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: EffectStyle): EffectStyleProto {
        const objectProto: Partial<EffectStyleProto> = { metatype: 2200100 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        objectProto.type = Number(object._type) as any;
        if (object._opacity != null) {
          objectProto.opacity = object._opacity;
        }
        if (object._offset != null) {
          objectProto.offset = object._offset.pack(10);
        }
        if (object._scale != null) {
          objectProto.scale = object._scale;
        }
        if (object._rotate != null) {
          objectProto.rotate = object._rotate.pack(10);
        }
        if (object._skew != null) {
          objectProto.skew = object._skew.pack(10);
        }
        if (object._perspective != null) {
          objectProto.perspective = object._perspective;
        }
        if (object._delay != null) {
          objectProto.delay = packProtoDuration(object._delay);
        }
        if (object._duration != null) {
          objectProto.duration = object._duration;
        }
        if (object._threshold != null) {
          objectProto.threshold = object._threshold;
        }
        if (object._once != null) {
          objectProto.once = object._once;
        }
        if (object._repeat != null) {
          objectProto.repeat = Number(object._repeat) as any;
        }
        if (object._split != null) {
          objectProto.split = Number(object._split) as any;
        }
        if (object._offscreen != null) {
          objectProto.offscreen = Number(object._offscreen) as any;
        }
        if (object._transition != null) {
          objectProto.transition = object._transition.pack(10);
        }
        return objectProto as EffectStyleProto;
      }

      unpackObject(objectProto: EffectStyleProto, _session: Session | null): EffectStyle {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Transition = STRUCT_CLASS_BY_TYPE[2200000] as typeof Transition;
        const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
        const _Axis3 = STRUCT_CLASS_BY_TYPE[2400026] as typeof Axis3;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[2200100] as typeof EffectStyle)({
          type: Number(objectProto.type) as any,
          opacity: objectProto.opacity != undefined ? objectProto.opacity : null,
          offset: objectProto.offset != undefined ? _Vector2.unpack(10, objectProto.offset, _session) as Vector2 : null,
          scale: objectProto.scale != undefined ? objectProto.scale : null,
          rotate: objectProto.rotate != undefined ? _Axis3.unpack(10, objectProto.rotate, _session) as Axis3 : null,
          skew: objectProto.skew != undefined ? _Vector2.unpack(10, objectProto.skew, _session) as Vector2 : null,
          perspective: objectProto.perspective != undefined ? objectProto.perspective : null,
          delay: objectProto.delay != undefined ? unpackProtoDuration(objectProto.delay!) : null,
          duration: objectProto.duration != undefined ? objectProto.duration : null,
          threshold: objectProto.threshold != undefined ? objectProto.threshold : null,
          once: objectProto.once != undefined ? objectProto.once : null,
          repeat: objectProto.repeat != undefined ? Number(objectProto.repeat) as any : null,
          split: objectProto.split != undefined ? Number(objectProto.split) as any : null,
          offscreen: objectProto.offscreen != undefined ? Number(objectProto.offscreen) as any : null,
          transition: objectProto.transition != undefined ? _Transition.unpack(10, objectProto.transition, _session) as Transition : null,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: EffectStyle): Uint8Array {
        const proto = this.packObject(object);
        return EffectStyleProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): EffectStyle {
        const proto = EffectStyleProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 2200100)] = new EffectStyleProtoEncoder();

    class FileProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: File): FileProto {
        const objectProto: Partial<FileProto> = { metatype: 480000 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        objectProto.type = Number(object._type) as any;
        if (object._status != null) {
          objectProto.status = Number(object._status) as any;
        }
        if (object._region != null) {
          objectProto.region = Number(object._region) as any;
        }
        if (object._mimeType != null) {
          objectProto.mimeType = object._mimeType;
        }
        if (object._format != null) {
          objectProto.format = Number(object._format) as any;
        }
        if (object._size != null) {
          objectProto.size = object._size;
        }
        if (object._sha256 != null) {
          objectProto.sha256 = object._sha256;
        }
        if (object._width != null) {
          objectProto.width = object._width;
        }
        if (object._height != null) {
          objectProto.height = object._height;
        }
        if (object._aspectRatio != null) {
          objectProto.aspectRatio = object._aspectRatio;
        }
        if (object._codec != null) {
          objectProto.codec = object._codec;
        }
        if (object._duration != null) {
          objectProto.duration = packProtoDuration(object._duration);
        }
        if (object._url != null) {
          objectProto.url = object._url;
        }
        if (object._contentUrl != null) {
          objectProto.contentUrl = object._contentUrl;
        }
        if (object._thumbnailUrl != null) {
          objectProto.thumbnailUrl = object._thumbnailUrl;
        }
        if (object._faviconUrl != null) {
          objectProto.faviconUrl = object._faviconUrl;
        }
        if (object._thumbnailWidth != null) {
          objectProto.thumbnailWidth = object._thumbnailWidth;
        }
        if (object._thumbnailHeight != null) {
          objectProto.thumbnailHeight = object._thumbnailHeight;
        }
        if (object._content != null) {
          objectProto.content = object._content;
        }
        return objectProto as FileProto;
      }

      unpackObject(objectProto: FileProto, _session: Session | null): File {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[480000] as typeof File)({
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          type: Number(objectProto.type) as any,
          mimeType: objectProto.mimeType != undefined ? objectProto.mimeType : null,
          format: objectProto.format != undefined ? Number(objectProto.format) as any : null,
          size: objectProto.size != undefined ? Number(objectProto.size) : null,
          sha256: objectProto.sha256 != undefined ? objectProto.sha256 : null,
          width: objectProto.width != undefined ? Number(objectProto.width) : null,
          height: objectProto.height != undefined ? Number(objectProto.height) : null,
          aspectRatio: objectProto.aspectRatio != undefined ? objectProto.aspectRatio : null,
          codec: objectProto.codec != undefined ? objectProto.codec : null,
          duration: objectProto.duration != undefined ? unpackProtoDuration(objectProto.duration!) : null,
          url: objectProto.url != undefined ? objectProto.url : null,
          contentUrl: objectProto.contentUrl != undefined ? objectProto.contentUrl : null,
          thumbnailUrl: objectProto.thumbnailUrl != undefined ? objectProto.thumbnailUrl : null,
          faviconUrl: objectProto.faviconUrl != undefined ? objectProto.faviconUrl : null,
          thumbnailWidth: objectProto.thumbnailWidth != undefined ? Number(objectProto.thumbnailWidth) : null,
          thumbnailHeight: objectProto.thumbnailHeight != undefined ? Number(objectProto.thumbnailHeight) : null,
          content: objectProto.content != undefined ? objectProto.content : null,
          status: objectProto.status != undefined ? Number(objectProto.status) as any : null,
          region: objectProto.region != undefined ? Number(objectProto.region) as any : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: File): Uint8Array {
        const proto = this.packObject(object);
        return FileProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): File {
        const proto = FileProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 480000)] = new FileProtoEncoder();

    class EnvironmentProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Environment): EnvironmentProto {
        const objectProto: Partial<EnvironmentProto> = { metatype: 1100000 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        if (object._icon != null) {
          objectProto.icon = object._icon.pack(10);
        }
        return objectProto as EnvironmentProto;
      }

      unpackObject(objectProto: EnvironmentProto, _session: Session | null): Environment {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[1100000] as typeof Environment)({
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          icon: objectProto.icon != undefined ? _Icon.unpack(10, objectProto.icon, _session) as Icon : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: Environment): Uint8Array {
        const proto = this.packObject(object);
        return EnvironmentProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Environment {
        const proto = EnvironmentProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1100000)] = new EnvironmentProtoEncoder();

    class LogEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: LogEvent): LogEventProto {
        const objectProto: Partial<LogEventProto> = { metatype: 1110011 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        if (object.nodePtr != null) {
          objectProto.nodePtr = object.nodePtr.pack(10);
        }
        objectProto.content = object.content;
        if (object.attributes) {
          objectProto.attributes = {} as any;
          for (const [key, value] of Object.entries(object.attributes) ) {
            objectProto.attributes![key] = packProtoJson(value);
          }
        }
        objectProto.level = Number(object.level) as any;
        return objectProto as LogEventProto;
      }

      unpackObject(objectProto: LogEventProto, _session: Session | null): LogEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const unpackedAttributes = {} as any;
        if (objectProto.attributes) {
          for (const [key, value] of Object.entries(objectProto.attributes)) {
            unpackedAttributes.set(key, unpackProtoJson((value as any)!));
          }
        }
        return new (NODE_CLASS_BY_TYPE[1110011] as typeof LogEvent)({
          content: objectProto.content,
          attributes: unpackedAttributes,
          level: Number(objectProto.level) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          node: objectProto.nodePtr != undefined ? _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: LogEvent): Uint8Array {
        const proto = this.packObject(object);
        return LogEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): LogEvent {
        const proto = LogEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1110011)] = new LogEventProtoEncoder();

    class RunStartedEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: RunStartedEvent): RunStartedEventProto {
        const objectProto: Partial<RunStartedEventProto> = { metatype: 1110002 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        objectProto.nodePtr = object.nodePtr.pack(10);
        if (object.targetPtr != null) {
          objectProto.targetPtr = object.targetPtr.pack(10);
        }
        return objectProto as RunStartedEventProto;
      }

      unpackObject(objectProto: RunStartedEventProto, _session: Session | null): RunStartedEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[1110002] as typeof RunStartedEvent)({
          node: _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference,
          target: objectProto.targetPtr != undefined ? _NodeReference.unpack(10, objectProto.targetPtr, _session) as NodeReference : null,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: RunStartedEvent): Uint8Array {
        const proto = this.packObject(object);
        return RunStartedEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): RunStartedEvent {
        const proto = RunStartedEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1110002)] = new RunStartedEventProtoEncoder();

    class RunPauseRequestedEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: RunPauseRequestedEvent): RunPauseRequestedEventProto {
        const objectProto: Partial<RunPauseRequestedEventProto> = { metatype: 1110003 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        objectProto.nodePtr = object.nodePtr.pack(10);
        if (object.targetPtr != null) {
          objectProto.targetPtr = object.targetPtr.pack(10);
        }
        return objectProto as RunPauseRequestedEventProto;
      }

      unpackObject(objectProto: RunPauseRequestedEventProto, _session: Session | null): RunPauseRequestedEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[1110003] as typeof RunPauseRequestedEvent)({
          node: _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference,
          target: objectProto.targetPtr != undefined ? _NodeReference.unpack(10, objectProto.targetPtr, _session) as NodeReference : null,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: RunPauseRequestedEvent): Uint8Array {
        const proto = this.packObject(object);
        return RunPauseRequestedEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): RunPauseRequestedEvent {
        const proto = RunPauseRequestedEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1110003)] = new RunPauseRequestedEventProtoEncoder();

    class RunPausedEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: RunPausedEvent): RunPausedEventProto {
        const objectProto: Partial<RunPausedEventProto> = { metatype: 1110004 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        objectProto.nodePtr = object.nodePtr.pack(10);
        if (object.targetPtr != null) {
          objectProto.targetPtr = object.targetPtr.pack(10);
        }
        return objectProto as RunPausedEventProto;
      }

      unpackObject(objectProto: RunPausedEventProto, _session: Session | null): RunPausedEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[1110004] as typeof RunPausedEvent)({
          node: _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference,
          target: objectProto.targetPtr != undefined ? _NodeReference.unpack(10, objectProto.targetPtr, _session) as NodeReference : null,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: RunPausedEvent): Uint8Array {
        const proto = this.packObject(object);
        return RunPausedEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): RunPausedEvent {
        const proto = RunPausedEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1110004)] = new RunPausedEventProtoEncoder();

    class RunResumeRequestedEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: RunResumeRequestedEvent): RunResumeRequestedEventProto {
        const objectProto: Partial<RunResumeRequestedEventProto> = { metatype: 1110005 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        objectProto.nodePtr = object.nodePtr.pack(10);
        if (object.targetPtr != null) {
          objectProto.targetPtr = object.targetPtr.pack(10);
        }
        return objectProto as RunResumeRequestedEventProto;
      }

      unpackObject(objectProto: RunResumeRequestedEventProto, _session: Session | null): RunResumeRequestedEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[1110005] as typeof RunResumeRequestedEvent)({
          node: _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference,
          target: objectProto.targetPtr != undefined ? _NodeReference.unpack(10, objectProto.targetPtr, _session) as NodeReference : null,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: RunResumeRequestedEvent): Uint8Array {
        const proto = this.packObject(object);
        return RunResumeRequestedEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): RunResumeRequestedEvent {
        const proto = RunResumeRequestedEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1110005)] = new RunResumeRequestedEventProtoEncoder();

    class RunResumedEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: RunResumedEvent): RunResumedEventProto {
        const objectProto: Partial<RunResumedEventProto> = { metatype: 1110006 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        objectProto.nodePtr = object.nodePtr.pack(10);
        if (object.targetPtr != null) {
          objectProto.targetPtr = object.targetPtr.pack(10);
        }
        return objectProto as RunResumedEventProto;
      }

      unpackObject(objectProto: RunResumedEventProto, _session: Session | null): RunResumedEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[1110006] as typeof RunResumedEvent)({
          node: _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference,
          target: objectProto.targetPtr != undefined ? _NodeReference.unpack(10, objectProto.targetPtr, _session) as NodeReference : null,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: RunResumedEvent): Uint8Array {
        const proto = this.packObject(object);
        return RunResumedEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): RunResumedEvent {
        const proto = RunResumedEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1110006)] = new RunResumedEventProtoEncoder();

    class RunStopRequestedEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: RunStopRequestedEvent): RunStopRequestedEventProto {
        const objectProto: Partial<RunStopRequestedEventProto> = { metatype: 1110007 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        objectProto.nodePtr = object.nodePtr.pack(10);
        if (object.targetPtr != null) {
          objectProto.targetPtr = object.targetPtr.pack(10);
        }
        return objectProto as RunStopRequestedEventProto;
      }

      unpackObject(objectProto: RunStopRequestedEventProto, _session: Session | null): RunStopRequestedEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[1110007] as typeof RunStopRequestedEvent)({
          node: _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference,
          target: objectProto.targetPtr != undefined ? _NodeReference.unpack(10, objectProto.targetPtr, _session) as NodeReference : null,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: RunStopRequestedEvent): Uint8Array {
        const proto = this.packObject(object);
        return RunStopRequestedEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): RunStopRequestedEvent {
        const proto = RunStopRequestedEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1110007)] = new RunStopRequestedEventProtoEncoder();

    class RunFailedEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: RunFailedEvent): RunFailedEventProto {
        const objectProto: Partial<RunFailedEventProto> = { metatype: 1110008 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        objectProto.nodePtr = object.nodePtr.pack(10);
        if (object.targetPtr != null) {
          objectProto.targetPtr = object.targetPtr.pack(10);
        }
        return objectProto as RunFailedEventProto;
      }

      unpackObject(objectProto: RunFailedEventProto, _session: Session | null): RunFailedEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[1110008] as typeof RunFailedEvent)({
          node: _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference,
          target: objectProto.targetPtr != undefined ? _NodeReference.unpack(10, objectProto.targetPtr, _session) as NodeReference : null,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: RunFailedEvent): Uint8Array {
        const proto = this.packObject(object);
        return RunFailedEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): RunFailedEvent {
        const proto = RunFailedEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1110008)] = new RunFailedEventProtoEncoder();

    class RunCompletedEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: RunCompletedEvent): RunCompletedEventProto {
        const objectProto: Partial<RunCompletedEventProto> = { metatype: 1110009 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        objectProto.nodePtr = object.nodePtr.pack(10);
        if (object.targetPtr != null) {
          objectProto.targetPtr = object.targetPtr.pack(10);
        }
        return objectProto as RunCompletedEventProto;
      }

      unpackObject(objectProto: RunCompletedEventProto, _session: Session | null): RunCompletedEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[1110009] as typeof RunCompletedEvent)({
          node: _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference,
          target: objectProto.targetPtr != undefined ? _NodeReference.unpack(10, objectProto.targetPtr, _session) as NodeReference : null,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: RunCompletedEvent): Uint8Array {
        const proto = this.packObject(object);
        return RunCompletedEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): RunCompletedEvent {
        const proto = RunCompletedEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1110009)] = new RunCompletedEventProtoEncoder();

    class SpanEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: SpanEvent): SpanEventProto {
        const objectProto: Partial<SpanEventProto> = { metatype: 1110010 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        objectProto.nodePtr = object.nodePtr.pack(10);
        return objectProto as SpanEventProto;
      }

      unpackObject(objectProto: SpanEventProto, _session: Session | null): SpanEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[1110010] as typeof SpanEvent)({
          node: _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: SpanEvent): Uint8Array {
        const proto = this.packObject(object);
        return SpanEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): SpanEvent {
        const proto = SpanEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1110010)] = new SpanEventProtoEncoder();

    class ArrowShape2DProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: ArrowShape2D): ArrowShape2DProto {
        const objectProto: Partial<ArrowShape2DProto> = { metatype: 2410200 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        if (object._position != null) {
          objectProto.position = object._position.pack(10);
        }
        if (object._offset != null) {
          objectProto.offset = object._offset.pack(10);
        }
        if (object._scale != null) {
          objectProto.scale = object._scale.pack(10);
        }
        if (object._rotation != null) {
          objectProto.rotation = object._rotation.pack(10);
        }
        if (object._skew != null) {
          objectProto.skew = object._skew.pack(10);
        }
        if (object._origin != null) {
          objectProto.origin = object._origin.pack(10);
        }
        if (object._anchor != null) {
          objectProto.anchor = Number(object._anchor) as any;
        }
        if (object._stroke != null) {
          objectProto.stroke = object._stroke.pack(10);
        }
        objectProto.startType = Number(object._startType) as any;
        objectProto.start = object._start.pack(10);
        objectProto.endType = Number(object._endType) as any;
        objectProto.end = object._end.pack(10);
        return objectProto as ArrowShape2DProto;
      }

      unpackObject(objectProto: ArrowShape2DProto, _session: Session | null): ArrowShape2D {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Stroke = STRUCT_CLASS_BY_TYPE[2101100] as typeof Stroke;
        const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
        const _Offset2 = STRUCT_CLASS_BY_TYPE[2400020] as typeof Offset2;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[2410200] as typeof ArrowShape2D)({
          startType: Number(objectProto.startType) as any,
          start: _Vector2.unpack(10, objectProto.start, _session) as Vector2,
          endType: Number(objectProto.endType) as any,
          end: _Vector2.unpack(10, objectProto.end, _session) as Vector2,
          stroke: objectProto.stroke != undefined ? _Stroke.unpack(10, objectProto.stroke, _session) as Stroke : null,
          position: objectProto.position != undefined ? _Vector2.unpack(10, objectProto.position, _session) as Vector2 : null,
          offset: objectProto.offset != undefined ? _Offset2.unpack(10, objectProto.offset, _session) as Offset2 : null,
          scale: objectProto.scale != undefined ? _Vector2.unpack(10, objectProto.scale, _session) as Vector2 : null,
          rotation: objectProto.rotation != undefined ? _Vector2.unpack(10, objectProto.rotation, _session) as Vector2 : null,
          skew: objectProto.skew != undefined ? _Vector2.unpack(10, objectProto.skew, _session) as Vector2 : null,
          origin: objectProto.origin != undefined ? _Vector2.unpack(10, objectProto.origin, _session) as Vector2 : null,
          anchor: objectProto.anchor != undefined ? Number(objectProto.anchor) as any : null,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: ArrowShape2D): Uint8Array {
        const proto = this.packObject(object);
        return ArrowShape2DProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): ArrowShape2D {
        const proto = ArrowShape2DProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 2410200)] = new ArrowShape2DProtoEncoder();

    class EllipseShape2DProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: EllipseShape2D): EllipseShape2DProto {
        const objectProto: Partial<EllipseShape2DProto> = { metatype: 2410400 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        if (object._position != null) {
          objectProto.position = object._position.pack(10);
        }
        if (object._offset != null) {
          objectProto.offset = object._offset.pack(10);
        }
        if (object._scale != null) {
          objectProto.scale = object._scale.pack(10);
        }
        if (object._rotation != null) {
          objectProto.rotation = object._rotation.pack(10);
        }
        if (object._skew != null) {
          objectProto.skew = object._skew.pack(10);
        }
        if (object._origin != null) {
          objectProto.origin = object._origin.pack(10);
        }
        if (object._anchor != null) {
          objectProto.anchor = Number(object._anchor) as any;
        }
        if (object._stroke != null) {
          objectProto.stroke = object._stroke.pack(10);
        }
        return objectProto as EllipseShape2DProto;
      }

      unpackObject(objectProto: EllipseShape2DProto, _session: Session | null): EllipseShape2D {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Stroke = STRUCT_CLASS_BY_TYPE[2101100] as typeof Stroke;
        const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
        const _Offset2 = STRUCT_CLASS_BY_TYPE[2400020] as typeof Offset2;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[2410400] as typeof EllipseShape2D)({
          stroke: objectProto.stroke != undefined ? _Stroke.unpack(10, objectProto.stroke, _session) as Stroke : null,
          position: objectProto.position != undefined ? _Vector2.unpack(10, objectProto.position, _session) as Vector2 : null,
          offset: objectProto.offset != undefined ? _Offset2.unpack(10, objectProto.offset, _session) as Offset2 : null,
          scale: objectProto.scale != undefined ? _Vector2.unpack(10, objectProto.scale, _session) as Vector2 : null,
          rotation: objectProto.rotation != undefined ? _Vector2.unpack(10, objectProto.rotation, _session) as Vector2 : null,
          skew: objectProto.skew != undefined ? _Vector2.unpack(10, objectProto.skew, _session) as Vector2 : null,
          origin: objectProto.origin != undefined ? _Vector2.unpack(10, objectProto.origin, _session) as Vector2 : null,
          anchor: objectProto.anchor != undefined ? Number(objectProto.anchor) as any : null,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: EllipseShape2D): Uint8Array {
        const proto = this.packObject(object);
        return EllipseShape2DProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): EllipseShape2D {
        const proto = EllipseShape2DProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 2410400)] = new EllipseShape2DProtoEncoder();

    class LineShape2DProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: LineShape2D): LineShape2DProto {
        const objectProto: Partial<LineShape2DProto> = { metatype: 2410100 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        if (object._position != null) {
          objectProto.position = object._position.pack(10);
        }
        if (object._offset != null) {
          objectProto.offset = object._offset.pack(10);
        }
        if (object._scale != null) {
          objectProto.scale = object._scale.pack(10);
        }
        if (object._rotation != null) {
          objectProto.rotation = object._rotation.pack(10);
        }
        if (object._skew != null) {
          objectProto.skew = object._skew.pack(10);
        }
        if (object._origin != null) {
          objectProto.origin = object._origin.pack(10);
        }
        if (object._anchor != null) {
          objectProto.anchor = Number(object._anchor) as any;
        }
        if (object._stroke != null) {
          objectProto.stroke = object._stroke.pack(10);
        }
        objectProto.start = object._start.pack(10);
        objectProto.end = object._end.pack(10);
        return objectProto as LineShape2DProto;
      }

      unpackObject(objectProto: LineShape2DProto, _session: Session | null): LineShape2D {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Stroke = STRUCT_CLASS_BY_TYPE[2101100] as typeof Stroke;
        const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
        const _Offset2 = STRUCT_CLASS_BY_TYPE[2400020] as typeof Offset2;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[2410100] as typeof LineShape2D)({
          start: _Vector2.unpack(10, objectProto.start, _session) as Vector2,
          end: _Vector2.unpack(10, objectProto.end, _session) as Vector2,
          stroke: objectProto.stroke != undefined ? _Stroke.unpack(10, objectProto.stroke, _session) as Stroke : null,
          position: objectProto.position != undefined ? _Vector2.unpack(10, objectProto.position, _session) as Vector2 : null,
          offset: objectProto.offset != undefined ? _Offset2.unpack(10, objectProto.offset, _session) as Offset2 : null,
          scale: objectProto.scale != undefined ? _Vector2.unpack(10, objectProto.scale, _session) as Vector2 : null,
          rotation: objectProto.rotation != undefined ? _Vector2.unpack(10, objectProto.rotation, _session) as Vector2 : null,
          skew: objectProto.skew != undefined ? _Vector2.unpack(10, objectProto.skew, _session) as Vector2 : null,
          origin: objectProto.origin != undefined ? _Vector2.unpack(10, objectProto.origin, _session) as Vector2 : null,
          anchor: objectProto.anchor != undefined ? Number(objectProto.anchor) as any : null,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: LineShape2D): Uint8Array {
        const proto = this.packObject(object);
        return LineShape2DProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): LineShape2D {
        const proto = LineShape2DProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 2410100)] = new LineShape2DProtoEncoder();

    class PathShape2DProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: PathShape2D): PathShape2DProto {
        const objectProto: Partial<PathShape2DProto> = { metatype: 2410600 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        if (object._position != null) {
          objectProto.position = object._position.pack(10);
        }
        if (object._offset != null) {
          objectProto.offset = object._offset.pack(10);
        }
        if (object._scale != null) {
          objectProto.scale = object._scale.pack(10);
        }
        if (object._rotation != null) {
          objectProto.rotation = object._rotation.pack(10);
        }
        if (object._skew != null) {
          objectProto.skew = object._skew.pack(10);
        }
        if (object._origin != null) {
          objectProto.origin = object._origin.pack(10);
        }
        if (object._anchor != null) {
          objectProto.anchor = Number(object._anchor) as any;
        }
        if (object._stroke != null) {
          objectProto.stroke = object._stroke.pack(10);
        }
        if (object._points) {
          const packedPoints: any[] = [];
          for (const item of object._points) {
            packedPoints.push(item.pack(10));
          }
          objectProto.points = packedPoints;
        }
        return objectProto as PathShape2DProto;
      }

      unpackObject(objectProto: PathShape2DProto, _session: Session | null): PathShape2D {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Stroke = STRUCT_CLASS_BY_TYPE[2101100] as typeof Stroke;
        const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
        const _Offset2 = STRUCT_CLASS_BY_TYPE[2400020] as typeof Offset2;
        const unpackedPoints: any[] = [];
        if (objectProto.points) {
          for (const item of objectProto.points) {
            unpackedPoints.push(_Vector2.unpack(10, item, _session) as Vector2);
          }
        }
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[2410600] as typeof PathShape2D)({
          points: unpackedPoints,
          stroke: objectProto.stroke != undefined ? _Stroke.unpack(10, objectProto.stroke, _session) as Stroke : null,
          position: objectProto.position != undefined ? _Vector2.unpack(10, objectProto.position, _session) as Vector2 : null,
          offset: objectProto.offset != undefined ? _Offset2.unpack(10, objectProto.offset, _session) as Offset2 : null,
          scale: objectProto.scale != undefined ? _Vector2.unpack(10, objectProto.scale, _session) as Vector2 : null,
          rotation: objectProto.rotation != undefined ? _Vector2.unpack(10, objectProto.rotation, _session) as Vector2 : null,
          skew: objectProto.skew != undefined ? _Vector2.unpack(10, objectProto.skew, _session) as Vector2 : null,
          origin: objectProto.origin != undefined ? _Vector2.unpack(10, objectProto.origin, _session) as Vector2 : null,
          anchor: objectProto.anchor != undefined ? Number(objectProto.anchor) as any : null,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: PathShape2D): Uint8Array {
        const proto = this.packObject(object);
        return PathShape2DProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): PathShape2D {
        const proto = PathShape2DProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 2410600)] = new PathShape2DProtoEncoder();

    class PolygonShape2DProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: PolygonShape2D): PolygonShape2DProto {
        const objectProto: Partial<PolygonShape2DProto> = { metatype: 2410500 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        if (object._position != null) {
          objectProto.position = object._position.pack(10);
        }
        if (object._offset != null) {
          objectProto.offset = object._offset.pack(10);
        }
        if (object._scale != null) {
          objectProto.scale = object._scale.pack(10);
        }
        if (object._rotation != null) {
          objectProto.rotation = object._rotation.pack(10);
        }
        if (object._skew != null) {
          objectProto.skew = object._skew.pack(10);
        }
        if (object._origin != null) {
          objectProto.origin = object._origin.pack(10);
        }
        if (object._anchor != null) {
          objectProto.anchor = Number(object._anchor) as any;
        }
        if (object._stroke != null) {
          objectProto.stroke = object._stroke.pack(10);
        }
        if (object._points) {
          const packedPoints: any[] = [];
          for (const item of object._points) {
            packedPoints.push(item.pack(10));
          }
          objectProto.points = packedPoints;
        }
        return objectProto as PolygonShape2DProto;
      }

      unpackObject(objectProto: PolygonShape2DProto, _session: Session | null): PolygonShape2D {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Stroke = STRUCT_CLASS_BY_TYPE[2101100] as typeof Stroke;
        const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
        const _Offset2 = STRUCT_CLASS_BY_TYPE[2400020] as typeof Offset2;
        const unpackedPoints: any[] = [];
        if (objectProto.points) {
          for (const item of objectProto.points) {
            unpackedPoints.push(_Vector2.unpack(10, item, _session) as Vector2);
          }
        }
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[2410500] as typeof PolygonShape2D)({
          points: unpackedPoints,
          stroke: objectProto.stroke != undefined ? _Stroke.unpack(10, objectProto.stroke, _session) as Stroke : null,
          position: objectProto.position != undefined ? _Vector2.unpack(10, objectProto.position, _session) as Vector2 : null,
          offset: objectProto.offset != undefined ? _Offset2.unpack(10, objectProto.offset, _session) as Offset2 : null,
          scale: objectProto.scale != undefined ? _Vector2.unpack(10, objectProto.scale, _session) as Vector2 : null,
          rotation: objectProto.rotation != undefined ? _Vector2.unpack(10, objectProto.rotation, _session) as Vector2 : null,
          skew: objectProto.skew != undefined ? _Vector2.unpack(10, objectProto.skew, _session) as Vector2 : null,
          origin: objectProto.origin != undefined ? _Vector2.unpack(10, objectProto.origin, _session) as Vector2 : null,
          anchor: objectProto.anchor != undefined ? Number(objectProto.anchor) as any : null,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: PolygonShape2D): Uint8Array {
        const proto = this.packObject(object);
        return PolygonShape2DProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): PolygonShape2D {
        const proto = PolygonShape2DProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 2410500)] = new PolygonShape2DProtoEncoder();

    class RectangleShape2DProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: RectangleShape2D): RectangleShape2DProto {
        const objectProto: Partial<RectangleShape2DProto> = { metatype: 2410300 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        if (object._position != null) {
          objectProto.position = object._position.pack(10);
        }
        if (object._offset != null) {
          objectProto.offset = object._offset.pack(10);
        }
        if (object._scale != null) {
          objectProto.scale = object._scale.pack(10);
        }
        if (object._rotation != null) {
          objectProto.rotation = object._rotation.pack(10);
        }
        if (object._skew != null) {
          objectProto.skew = object._skew.pack(10);
        }
        if (object._origin != null) {
          objectProto.origin = object._origin.pack(10);
        }
        if (object._anchor != null) {
          objectProto.anchor = Number(object._anchor) as any;
        }
        if (object._stroke != null) {
          objectProto.stroke = object._stroke.pack(10);
        }
        if (object._width != null) {
          objectProto.width = object._width.pack(10);
        }
        if (object._height != null) {
          objectProto.height = object._height.pack(10);
        }
        return objectProto as RectangleShape2DProto;
      }

      unpackObject(objectProto: RectangleShape2DProto, _session: Session | null): RectangleShape2D {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Stroke = STRUCT_CLASS_BY_TYPE[2101100] as typeof Stroke;
        const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
        const _Offset2 = STRUCT_CLASS_BY_TYPE[2400020] as typeof Offset2;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[2410300] as typeof RectangleShape2D)({
          width: objectProto.width != undefined ? _Vector2.unpack(10, objectProto.width, _session) as Vector2 : null,
          height: objectProto.height != undefined ? _Vector2.unpack(10, objectProto.height, _session) as Vector2 : null,
          stroke: objectProto.stroke != undefined ? _Stroke.unpack(10, objectProto.stroke, _session) as Stroke : null,
          position: objectProto.position != undefined ? _Vector2.unpack(10, objectProto.position, _session) as Vector2 : null,
          offset: objectProto.offset != undefined ? _Offset2.unpack(10, objectProto.offset, _session) as Offset2 : null,
          scale: objectProto.scale != undefined ? _Vector2.unpack(10, objectProto.scale, _session) as Vector2 : null,
          rotation: objectProto.rotation != undefined ? _Vector2.unpack(10, objectProto.rotation, _session) as Vector2 : null,
          skew: objectProto.skew != undefined ? _Vector2.unpack(10, objectProto.skew, _session) as Vector2 : null,
          origin: objectProto.origin != undefined ? _Vector2.unpack(10, objectProto.origin, _session) as Vector2 : null,
          anchor: objectProto.anchor != undefined ? Number(objectProto.anchor) as any : null,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: RectangleShape2D): Uint8Array {
        const proto = this.packObject(object);
        return RectangleShape2DProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): RectangleShape2D {
        const proto = RectangleShape2DProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 2410300)] = new RectangleShape2DProtoEncoder();

    class DatabaseProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Database): DatabaseProto {
        const objectProto: Partial<DatabaseProto> = { metatype: 1000000 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        objectProto.type = Number(object._type) as any;
        if (object._icon != null) {
          objectProto.icon = object._icon.pack(10);
        }
        if (object._status != null) {
          objectProto.status = Number(object._status) as any;
        }
        if (object._region != null) {
          objectProto.region = Number(object._region) as any;
        }
        if (object._galaxyName != null) {
          objectProto.galaxyName = object._galaxyName;
        }
        objectProto.externalName = object._externalName;
        if (object._customSchemaName != null) {
          objectProto.customSchemaName = object._customSchemaName;
        }
        objectProto.tenancy = Number(object._tenancy) as any;
        if (object._connectionUrl != null) {
          objectProto.connectionUrl = object._connectionUrl;
        }
        return objectProto as DatabaseProto;
      }

      unpackObject(objectProto: DatabaseProto, _session: Session | null): Database {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[1000000] as typeof Database)({
          type: Number(objectProto.type) as any,
          icon: objectProto.icon != undefined ? _Icon.unpack(10, objectProto.icon, _session) as Icon : null,
          galaxyName: objectProto.galaxyName != undefined ? objectProto.galaxyName : null,
          externalName: objectProto.externalName,
          customSchemaName: objectProto.customSchemaName != undefined ? objectProto.customSchemaName : null,
          tenancy: Number(objectProto.tenancy) as any,
          connectionUrl: objectProto.connectionUrl != undefined ? objectProto.connectionUrl : null,
          status: objectProto.status != undefined ? Number(objectProto.status) as any : null,
          region: objectProto.region != undefined ? Number(objectProto.region) as any : null,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: Database): Uint8Array {
        const proto = this.packObject(object);
        return DatabaseProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Database {
        const proto = DatabaseProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1000000)] = new DatabaseProtoEncoder();

    class MachineProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Machine): MachineProto {
        const objectProto: Partial<MachineProto> = { metatype: 1001000 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        objectProto.type = Number(object._type) as any;
        if (object._status != null) {
          objectProto.status = Number(object._status) as any;
        }
        if (object._region != null) {
          objectProto.region = Number(object._region) as any;
        }
        objectProto.version = object._version;
        if (object._externalName != null) {
          objectProto.externalName = object._externalName;
        }
        if (object._externalId != null) {
          objectProto.externalId = object._externalId;
        }
        if (object._imageId != null) {
          objectProto.imageId = object._imageId;
        }
        if (object._grpcUrl != null) {
          objectProto.grpcUrl = object._grpcUrl;
        }
        if (object._vncUrl != null) {
          objectProto.vncUrl = object._vncUrl;
        }
        if (object._clientPtr != null) {
          objectProto.clientPtr = object._clientPtr.pack(10);
        }
        objectProto.cpu = object._cpu;
        objectProto.ram = object._ram;
        objectProto.width = object._width;
        objectProto.height = object._height;
        objectProto.isHeadless = object._isHeadless;
        return objectProto as MachineProto;
      }

      unpackObject(objectProto: MachineProto, _session: Session | null): Machine {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[1001000] as typeof Machine)({
          type: Number(objectProto.type) as any,
          version: objectProto.version,
          externalName: objectProto.externalName != undefined ? objectProto.externalName : null,
          externalId: objectProto.externalId != undefined ? objectProto.externalId : null,
          imageId: objectProto.imageId != undefined ? objectProto.imageId : null,
          grpcUrl: objectProto.grpcUrl != undefined ? objectProto.grpcUrl : null,
          vncUrl: objectProto.vncUrl != undefined ? objectProto.vncUrl : null,
          client: objectProto.clientPtr != undefined ? _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference : null,
          cpu: objectProto.cpu,
          ram: objectProto.ram,
          width: Number(objectProto.width),
          height: Number(objectProto.height),
          isHeadless: objectProto.isHeadless,
          status: objectProto.status != undefined ? Number(objectProto.status) as any : null,
          region: objectProto.region != undefined ? Number(objectProto.region) as any : null,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: Machine): Uint8Array {
        const proto = this.packObject(object);
        return MachineProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Machine {
        const proto = MachineProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1001000)] = new MachineProtoEncoder();

    class CopyEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: CopyEvent): CopyEventProto {
        const objectProto: Partial<CopyEventProto> = { metatype: 2000501 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        if (object.nodePtr != null) {
          objectProto.nodePtr = object.nodePtr.pack(10);
        }
        return objectProto as CopyEventProto;
      }

      unpackObject(objectProto: CopyEventProto, _session: Session | null): CopyEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[2000501] as typeof CopyEvent)({
          node: objectProto.nodePtr != undefined ? _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference : null,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: CopyEvent): Uint8Array {
        const proto = this.packObject(object);
        return CopyEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): CopyEvent {
        const proto = CopyEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 2000501)] = new CopyEventProtoEncoder();

    class CutEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: CutEvent): CutEventProto {
        const objectProto: Partial<CutEventProto> = { metatype: 2000502 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        if (object.nodePtr != null) {
          objectProto.nodePtr = object.nodePtr.pack(10);
        }
        return objectProto as CutEventProto;
      }

      unpackObject(objectProto: CutEventProto, _session: Session | null): CutEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[2000502] as typeof CutEvent)({
          node: objectProto.nodePtr != undefined ? _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference : null,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: CutEvent): Uint8Array {
        const proto = this.packObject(object);
        return CutEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): CutEvent {
        const proto = CutEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 2000502)] = new CutEventProtoEncoder();

    class PasteEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: PasteEvent): PasteEventProto {
        const objectProto: Partial<PasteEventProto> = { metatype: 2000503 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        if (object.nodePtr != null) {
          objectProto.nodePtr = object.nodePtr.pack(10);
        }
        return objectProto as PasteEventProto;
      }

      unpackObject(objectProto: PasteEventProto, _session: Session | null): PasteEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[2000503] as typeof PasteEvent)({
          node: objectProto.nodePtr != undefined ? _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference : null,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: PasteEvent): Uint8Array {
        const proto = this.packObject(object);
        return PasteEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): PasteEvent {
        const proto = PasteEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 2000503)] = new PasteEventProtoEncoder();

    class DragStartEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: DragStartEvent): DragStartEventProto {
        const objectProto: Partial<DragStartEventProto> = { metatype: 2000401 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        if (object.nodePtr != null) {
          objectProto.nodePtr = object.nodePtr.pack(10);
        }
        objectProto.position = object.position.pack(10);
        return objectProto as DragStartEventProto;
      }

      unpackObject(objectProto: DragStartEventProto, _session: Session | null): DragStartEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
        return new (NODE_CLASS_BY_TYPE[2000401] as typeof DragStartEvent)({
          position: _Vector2.unpack(10, objectProto.position, _session) as Vector2,
          node: objectProto.nodePtr != undefined ? _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference : null,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: DragStartEvent): Uint8Array {
        const proto = this.packObject(object);
        return DragStartEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): DragStartEvent {
        const proto = DragStartEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 2000401)] = new DragStartEventProtoEncoder();

    class DragEndEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: DragEndEvent): DragEndEventProto {
        const objectProto: Partial<DragEndEventProto> = { metatype: 2000402 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        if (object.nodePtr != null) {
          objectProto.nodePtr = object.nodePtr.pack(10);
        }
        objectProto.position = object.position.pack(10);
        return objectProto as DragEndEventProto;
      }

      unpackObject(objectProto: DragEndEventProto, _session: Session | null): DragEndEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
        return new (NODE_CLASS_BY_TYPE[2000402] as typeof DragEndEvent)({
          position: _Vector2.unpack(10, objectProto.position, _session) as Vector2,
          node: objectProto.nodePtr != undefined ? _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference : null,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: DragEndEvent): Uint8Array {
        const proto = this.packObject(object);
        return DragEndEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): DragEndEvent {
        const proto = DragEndEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 2000402)] = new DragEndEventProtoEncoder();

    class DragOverEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: DragOverEvent): DragOverEventProto {
        const objectProto: Partial<DragOverEventProto> = { metatype: 2000403 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        if (object.nodePtr != null) {
          objectProto.nodePtr = object.nodePtr.pack(10);
        }
        objectProto.position = object.position.pack(10);
        return objectProto as DragOverEventProto;
      }

      unpackObject(objectProto: DragOverEventProto, _session: Session | null): DragOverEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
        return new (NODE_CLASS_BY_TYPE[2000403] as typeof DragOverEvent)({
          position: _Vector2.unpack(10, objectProto.position, _session) as Vector2,
          node: objectProto.nodePtr != undefined ? _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference : null,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: DragOverEvent): Uint8Array {
        const proto = this.packObject(object);
        return DragOverEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): DragOverEvent {
        const proto = DragOverEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 2000403)] = new DragOverEventProtoEncoder();

    class DragEnterEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: DragEnterEvent): DragEnterEventProto {
        const objectProto: Partial<DragEnterEventProto> = { metatype: 2000404 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        if (object.nodePtr != null) {
          objectProto.nodePtr = object.nodePtr.pack(10);
        }
        objectProto.position = object.position.pack(10);
        return objectProto as DragEnterEventProto;
      }

      unpackObject(objectProto: DragEnterEventProto, _session: Session | null): DragEnterEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
        return new (NODE_CLASS_BY_TYPE[2000404] as typeof DragEnterEvent)({
          position: _Vector2.unpack(10, objectProto.position, _session) as Vector2,
          node: objectProto.nodePtr != undefined ? _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference : null,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: DragEnterEvent): Uint8Array {
        const proto = this.packObject(object);
        return DragEnterEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): DragEnterEvent {
        const proto = DragEnterEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 2000404)] = new DragEnterEventProtoEncoder();

    class DragLeaveEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: DragLeaveEvent): DragLeaveEventProto {
        const objectProto: Partial<DragLeaveEventProto> = { metatype: 2000405 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        if (object.nodePtr != null) {
          objectProto.nodePtr = object.nodePtr.pack(10);
        }
        objectProto.position = object.position.pack(10);
        return objectProto as DragLeaveEventProto;
      }

      unpackObject(objectProto: DragLeaveEventProto, _session: Session | null): DragLeaveEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
        return new (NODE_CLASS_BY_TYPE[2000405] as typeof DragLeaveEvent)({
          position: _Vector2.unpack(10, objectProto.position, _session) as Vector2,
          node: objectProto.nodePtr != undefined ? _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference : null,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: DragLeaveEvent): Uint8Array {
        const proto = this.packObject(object);
        return DragLeaveEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): DragLeaveEvent {
        const proto = DragLeaveEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 2000405)] = new DragLeaveEventProtoEncoder();

    class DropEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: DropEvent): DropEventProto {
        const objectProto: Partial<DropEventProto> = { metatype: 2000406 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        if (object.nodePtr != null) {
          objectProto.nodePtr = object.nodePtr.pack(10);
        }
        objectProto.position = object.position.pack(10);
        return objectProto as DropEventProto;
      }

      unpackObject(objectProto: DropEventProto, _session: Session | null): DropEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
        return new (NODE_CLASS_BY_TYPE[2000406] as typeof DropEvent)({
          position: _Vector2.unpack(10, objectProto.position, _session) as Vector2,
          node: objectProto.nodePtr != undefined ? _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference : null,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: DropEvent): Uint8Array {
        const proto = this.packObject(object);
        return DropEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): DropEvent {
        const proto = DropEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 2000406)] = new DropEventProtoEncoder();

    class FocusInEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: FocusInEvent): FocusInEventProto {
        const objectProto: Partial<FocusInEventProto> = { metatype: 2000601 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        if (object.nodePtr != null) {
          objectProto.nodePtr = object.nodePtr.pack(10);
        }
        return objectProto as FocusInEventProto;
      }

      unpackObject(objectProto: FocusInEventProto, _session: Session | null): FocusInEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[2000601] as typeof FocusInEvent)({
          node: objectProto.nodePtr != undefined ? _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference : null,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: FocusInEvent): Uint8Array {
        const proto = this.packObject(object);
        return FocusInEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): FocusInEvent {
        const proto = FocusInEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 2000601)] = new FocusInEventProtoEncoder();

    class FocusOutEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: FocusOutEvent): FocusOutEventProto {
        const objectProto: Partial<FocusOutEventProto> = { metatype: 2000602 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        if (object.nodePtr != null) {
          objectProto.nodePtr = object.nodePtr.pack(10);
        }
        return objectProto as FocusOutEventProto;
      }

      unpackObject(objectProto: FocusOutEventProto, _session: Session | null): FocusOutEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[2000602] as typeof FocusOutEvent)({
          node: objectProto.nodePtr != undefined ? _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference : null,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: FocusOutEvent): Uint8Array {
        const proto = this.packObject(object);
        return FocusOutEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): FocusOutEvent {
        const proto = FocusOutEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 2000602)] = new FocusOutEventProtoEncoder();

    class KeyDownEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: KeyDownEvent): KeyDownEventProto {
        const objectProto: Partial<KeyDownEventProto> = { metatype: 2000301 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        if (object.nodePtr != null) {
          objectProto.nodePtr = object.nodePtr.pack(10);
        }
        objectProto.key = object.key;
        objectProto.code = object.code;
        objectProto.isRepeat = object.isRepeat;
        objectProto.isRedacted = object.isRedacted;
        objectProto.shiftKey = object.shiftKey;
        objectProto.altKey = object.altKey;
        objectProto.ctrlKey = object.ctrlKey;
        objectProto.metaKey = object.metaKey;
        return objectProto as KeyDownEventProto;
      }

      unpackObject(objectProto: KeyDownEventProto, _session: Session | null): KeyDownEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[2000301] as typeof KeyDownEvent)({
          key: objectProto.key,
          code: objectProto.code,
          isRepeat: objectProto.isRepeat,
          isRedacted: objectProto.isRedacted,
          shiftKey: objectProto.shiftKey,
          altKey: objectProto.altKey,
          ctrlKey: objectProto.ctrlKey,
          metaKey: objectProto.metaKey,
          node: objectProto.nodePtr != undefined ? _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference : null,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: KeyDownEvent): Uint8Array {
        const proto = this.packObject(object);
        return KeyDownEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): KeyDownEvent {
        const proto = KeyDownEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 2000301)] = new KeyDownEventProtoEncoder();

    class KeyUpEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: KeyUpEvent): KeyUpEventProto {
        const objectProto: Partial<KeyUpEventProto> = { metatype: 2000302 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        if (object.nodePtr != null) {
          objectProto.nodePtr = object.nodePtr.pack(10);
        }
        objectProto.key = object.key;
        objectProto.code = object.code;
        objectProto.isRepeat = object.isRepeat;
        objectProto.isRedacted = object.isRedacted;
        objectProto.shiftKey = object.shiftKey;
        objectProto.altKey = object.altKey;
        objectProto.ctrlKey = object.ctrlKey;
        objectProto.metaKey = object.metaKey;
        return objectProto as KeyUpEventProto;
      }

      unpackObject(objectProto: KeyUpEventProto, _session: Session | null): KeyUpEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[2000302] as typeof KeyUpEvent)({
          key: objectProto.key,
          code: objectProto.code,
          isRepeat: objectProto.isRepeat,
          isRedacted: objectProto.isRedacted,
          shiftKey: objectProto.shiftKey,
          altKey: objectProto.altKey,
          ctrlKey: objectProto.ctrlKey,
          metaKey: objectProto.metaKey,
          node: objectProto.nodePtr != undefined ? _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference : null,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: KeyUpEvent): Uint8Array {
        const proto = this.packObject(object);
        return KeyUpEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): KeyUpEvent {
        const proto = KeyUpEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 2000302)] = new KeyUpEventProtoEncoder();

    class KeyPressEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: KeyPressEvent): KeyPressEventProto {
        const objectProto: Partial<KeyPressEventProto> = { metatype: 2000303 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        if (object.nodePtr != null) {
          objectProto.nodePtr = object.nodePtr.pack(10);
        }
        objectProto.key = object.key;
        objectProto.code = object.code;
        objectProto.isRepeat = object.isRepeat;
        objectProto.isRedacted = object.isRedacted;
        objectProto.shiftKey = object.shiftKey;
        objectProto.altKey = object.altKey;
        objectProto.ctrlKey = object.ctrlKey;
        objectProto.metaKey = object.metaKey;
        return objectProto as KeyPressEventProto;
      }

      unpackObject(objectProto: KeyPressEventProto, _session: Session | null): KeyPressEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[2000303] as typeof KeyPressEvent)({
          key: objectProto.key,
          code: objectProto.code,
          isRepeat: objectProto.isRepeat,
          isRedacted: objectProto.isRedacted,
          shiftKey: objectProto.shiftKey,
          altKey: objectProto.altKey,
          ctrlKey: objectProto.ctrlKey,
          metaKey: objectProto.metaKey,
          node: objectProto.nodePtr != undefined ? _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference : null,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: KeyPressEvent): Uint8Array {
        const proto = this.packObject(object);
        return KeyPressEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): KeyPressEvent {
        const proto = KeyPressEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 2000303)] = new KeyPressEventProtoEncoder();

    class PointerDownEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: PointerDownEvent): PointerDownEventProto {
        const objectProto: Partial<PointerDownEventProto> = { metatype: 2000101 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        if (object.nodePtr != null) {
          objectProto.nodePtr = object.nodePtr.pack(10);
        }
        objectProto.position = object.position.pack(10);
        if (object.pressure != null) {
          objectProto.pressure = object.pressure;
        }
        objectProto.shiftKey = object.shiftKey;
        objectProto.altKey = object.altKey;
        objectProto.ctrlKey = object.ctrlKey;
        objectProto.metaKey = object.metaKey;
        return objectProto as PointerDownEventProto;
      }

      unpackObject(objectProto: PointerDownEventProto, _session: Session | null): PointerDownEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
        return new (NODE_CLASS_BY_TYPE[2000101] as typeof PointerDownEvent)({
          position: _Vector2.unpack(10, objectProto.position, _session) as Vector2,
          pressure: objectProto.pressure != undefined ? objectProto.pressure : null,
          shiftKey: objectProto.shiftKey,
          altKey: objectProto.altKey,
          ctrlKey: objectProto.ctrlKey,
          metaKey: objectProto.metaKey,
          node: objectProto.nodePtr != undefined ? _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference : null,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: PointerDownEvent): Uint8Array {
        const proto = this.packObject(object);
        return PointerDownEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): PointerDownEvent {
        const proto = PointerDownEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 2000101)] = new PointerDownEventProtoEncoder();

    class PointerUpEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: PointerUpEvent): PointerUpEventProto {
        const objectProto: Partial<PointerUpEventProto> = { metatype: 2000102 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        if (object.nodePtr != null) {
          objectProto.nodePtr = object.nodePtr.pack(10);
        }
        objectProto.position = object.position.pack(10);
        if (object.pressure != null) {
          objectProto.pressure = object.pressure;
        }
        objectProto.shiftKey = object.shiftKey;
        objectProto.altKey = object.altKey;
        objectProto.ctrlKey = object.ctrlKey;
        objectProto.metaKey = object.metaKey;
        return objectProto as PointerUpEventProto;
      }

      unpackObject(objectProto: PointerUpEventProto, _session: Session | null): PointerUpEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
        return new (NODE_CLASS_BY_TYPE[2000102] as typeof PointerUpEvent)({
          position: _Vector2.unpack(10, objectProto.position, _session) as Vector2,
          pressure: objectProto.pressure != undefined ? objectProto.pressure : null,
          shiftKey: objectProto.shiftKey,
          altKey: objectProto.altKey,
          ctrlKey: objectProto.ctrlKey,
          metaKey: objectProto.metaKey,
          node: objectProto.nodePtr != undefined ? _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference : null,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: PointerUpEvent): Uint8Array {
        const proto = this.packObject(object);
        return PointerUpEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): PointerUpEvent {
        const proto = PointerUpEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 2000102)] = new PointerUpEventProtoEncoder();

    class PointerMoveEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: PointerMoveEvent): PointerMoveEventProto {
        const objectProto: Partial<PointerMoveEventProto> = { metatype: 2000103 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        if (object.nodePtr != null) {
          objectProto.nodePtr = object.nodePtr.pack(10);
        }
        objectProto.position = object.position.pack(10);
        if (object.pressure != null) {
          objectProto.pressure = object.pressure;
        }
        objectProto.shiftKey = object.shiftKey;
        objectProto.altKey = object.altKey;
        objectProto.ctrlKey = object.ctrlKey;
        objectProto.metaKey = object.metaKey;
        return objectProto as PointerMoveEventProto;
      }

      unpackObject(objectProto: PointerMoveEventProto, _session: Session | null): PointerMoveEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
        return new (NODE_CLASS_BY_TYPE[2000103] as typeof PointerMoveEvent)({
          position: _Vector2.unpack(10, objectProto.position, _session) as Vector2,
          pressure: objectProto.pressure != undefined ? objectProto.pressure : null,
          shiftKey: objectProto.shiftKey,
          altKey: objectProto.altKey,
          ctrlKey: objectProto.ctrlKey,
          metaKey: objectProto.metaKey,
          node: objectProto.nodePtr != undefined ? _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference : null,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: PointerMoveEvent): Uint8Array {
        const proto = this.packObject(object);
        return PointerMoveEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): PointerMoveEvent {
        const proto = PointerMoveEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 2000103)] = new PointerMoveEventProtoEncoder();

    class PointerEnterEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: PointerEnterEvent): PointerEnterEventProto {
        const objectProto: Partial<PointerEnterEventProto> = { metatype: 2000104 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        if (object.nodePtr != null) {
          objectProto.nodePtr = object.nodePtr.pack(10);
        }
        objectProto.position = object.position.pack(10);
        if (object.pressure != null) {
          objectProto.pressure = object.pressure;
        }
        objectProto.shiftKey = object.shiftKey;
        objectProto.altKey = object.altKey;
        objectProto.ctrlKey = object.ctrlKey;
        objectProto.metaKey = object.metaKey;
        return objectProto as PointerEnterEventProto;
      }

      unpackObject(objectProto: PointerEnterEventProto, _session: Session | null): PointerEnterEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
        return new (NODE_CLASS_BY_TYPE[2000104] as typeof PointerEnterEvent)({
          position: _Vector2.unpack(10, objectProto.position, _session) as Vector2,
          pressure: objectProto.pressure != undefined ? objectProto.pressure : null,
          shiftKey: objectProto.shiftKey,
          altKey: objectProto.altKey,
          ctrlKey: objectProto.ctrlKey,
          metaKey: objectProto.metaKey,
          node: objectProto.nodePtr != undefined ? _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference : null,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: PointerEnterEvent): Uint8Array {
        const proto = this.packObject(object);
        return PointerEnterEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): PointerEnterEvent {
        const proto = PointerEnterEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 2000104)] = new PointerEnterEventProtoEncoder();

    class PointerOverEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: PointerOverEvent): PointerOverEventProto {
        const objectProto: Partial<PointerOverEventProto> = { metatype: 2000105 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        if (object.nodePtr != null) {
          objectProto.nodePtr = object.nodePtr.pack(10);
        }
        objectProto.position = object.position.pack(10);
        if (object.pressure != null) {
          objectProto.pressure = object.pressure;
        }
        objectProto.shiftKey = object.shiftKey;
        objectProto.altKey = object.altKey;
        objectProto.ctrlKey = object.ctrlKey;
        objectProto.metaKey = object.metaKey;
        return objectProto as PointerOverEventProto;
      }

      unpackObject(objectProto: PointerOverEventProto, _session: Session | null): PointerOverEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
        return new (NODE_CLASS_BY_TYPE[2000105] as typeof PointerOverEvent)({
          position: _Vector2.unpack(10, objectProto.position, _session) as Vector2,
          pressure: objectProto.pressure != undefined ? objectProto.pressure : null,
          shiftKey: objectProto.shiftKey,
          altKey: objectProto.altKey,
          ctrlKey: objectProto.ctrlKey,
          metaKey: objectProto.metaKey,
          node: objectProto.nodePtr != undefined ? _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference : null,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: PointerOverEvent): Uint8Array {
        const proto = this.packObject(object);
        return PointerOverEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): PointerOverEvent {
        const proto = PointerOverEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 2000105)] = new PointerOverEventProtoEncoder();

    class PointerLeaveEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: PointerLeaveEvent): PointerLeaveEventProto {
        const objectProto: Partial<PointerLeaveEventProto> = { metatype: 2000106 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        if (object.nodePtr != null) {
          objectProto.nodePtr = object.nodePtr.pack(10);
        }
        objectProto.position = object.position.pack(10);
        if (object.pressure != null) {
          objectProto.pressure = object.pressure;
        }
        objectProto.shiftKey = object.shiftKey;
        objectProto.altKey = object.altKey;
        objectProto.ctrlKey = object.ctrlKey;
        objectProto.metaKey = object.metaKey;
        return objectProto as PointerLeaveEventProto;
      }

      unpackObject(objectProto: PointerLeaveEventProto, _session: Session | null): PointerLeaveEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
        return new (NODE_CLASS_BY_TYPE[2000106] as typeof PointerLeaveEvent)({
          position: _Vector2.unpack(10, objectProto.position, _session) as Vector2,
          pressure: objectProto.pressure != undefined ? objectProto.pressure : null,
          shiftKey: objectProto.shiftKey,
          altKey: objectProto.altKey,
          ctrlKey: objectProto.ctrlKey,
          metaKey: objectProto.metaKey,
          node: objectProto.nodePtr != undefined ? _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference : null,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: PointerLeaveEvent): Uint8Array {
        const proto = this.packObject(object);
        return PointerLeaveEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): PointerLeaveEvent {
        const proto = PointerLeaveEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 2000106)] = new PointerLeaveEventProtoEncoder();

    class PointerLongPressEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: PointerLongPressEvent): PointerLongPressEventProto {
        const objectProto: Partial<PointerLongPressEventProto> = { metatype: 2000107 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        if (object.nodePtr != null) {
          objectProto.nodePtr = object.nodePtr.pack(10);
        }
        objectProto.position = object.position.pack(10);
        if (object.pressure != null) {
          objectProto.pressure = object.pressure;
        }
        objectProto.shiftKey = object.shiftKey;
        objectProto.altKey = object.altKey;
        objectProto.ctrlKey = object.ctrlKey;
        objectProto.metaKey = object.metaKey;
        return objectProto as PointerLongPressEventProto;
      }

      unpackObject(objectProto: PointerLongPressEventProto, _session: Session | null): PointerLongPressEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
        return new (NODE_CLASS_BY_TYPE[2000107] as typeof PointerLongPressEvent)({
          position: _Vector2.unpack(10, objectProto.position, _session) as Vector2,
          pressure: objectProto.pressure != undefined ? objectProto.pressure : null,
          shiftKey: objectProto.shiftKey,
          altKey: objectProto.altKey,
          ctrlKey: objectProto.ctrlKey,
          metaKey: objectProto.metaKey,
          node: objectProto.nodePtr != undefined ? _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference : null,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: PointerLongPressEvent): Uint8Array {
        const proto = this.packObject(object);
        return PointerLongPressEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): PointerLongPressEvent {
        const proto = PointerLongPressEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 2000107)] = new PointerLongPressEventProtoEncoder();

    class SingleClickEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: SingleClickEvent): SingleClickEventProto {
        const objectProto: Partial<SingleClickEventProto> = { metatype: 2000202 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        if (object.nodePtr != null) {
          objectProto.nodePtr = object.nodePtr.pack(10);
        }
        objectProto.position = object.position.pack(10);
        if (object.pressure != null) {
          objectProto.pressure = object.pressure;
        }
        objectProto.shiftKey = object.shiftKey;
        objectProto.altKey = object.altKey;
        objectProto.ctrlKey = object.ctrlKey;
        objectProto.metaKey = object.metaKey;
        objectProto.button = Number(object.button) as any;
        return objectProto as SingleClickEventProto;
      }

      unpackObject(objectProto: SingleClickEventProto, _session: Session | null): SingleClickEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
        return new (NODE_CLASS_BY_TYPE[2000202] as typeof SingleClickEvent)({
          button: Number(objectProto.button) as any,
          position: _Vector2.unpack(10, objectProto.position, _session) as Vector2,
          pressure: objectProto.pressure != undefined ? objectProto.pressure : null,
          shiftKey: objectProto.shiftKey,
          altKey: objectProto.altKey,
          ctrlKey: objectProto.ctrlKey,
          metaKey: objectProto.metaKey,
          node: objectProto.nodePtr != undefined ? _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference : null,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: SingleClickEvent): Uint8Array {
        const proto = this.packObject(object);
        return SingleClickEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): SingleClickEvent {
        const proto = SingleClickEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 2000202)] = new SingleClickEventProtoEncoder();

    class DoubleClickEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: DoubleClickEvent): DoubleClickEventProto {
        const objectProto: Partial<DoubleClickEventProto> = { metatype: 2000203 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        if (object.nodePtr != null) {
          objectProto.nodePtr = object.nodePtr.pack(10);
        }
        objectProto.position = object.position.pack(10);
        if (object.pressure != null) {
          objectProto.pressure = object.pressure;
        }
        objectProto.shiftKey = object.shiftKey;
        objectProto.altKey = object.altKey;
        objectProto.ctrlKey = object.ctrlKey;
        objectProto.metaKey = object.metaKey;
        objectProto.button = Number(object.button) as any;
        return objectProto as DoubleClickEventProto;
      }

      unpackObject(objectProto: DoubleClickEventProto, _session: Session | null): DoubleClickEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
        return new (NODE_CLASS_BY_TYPE[2000203] as typeof DoubleClickEvent)({
          button: Number(objectProto.button) as any,
          position: _Vector2.unpack(10, objectProto.position, _session) as Vector2,
          pressure: objectProto.pressure != undefined ? objectProto.pressure : null,
          shiftKey: objectProto.shiftKey,
          altKey: objectProto.altKey,
          ctrlKey: objectProto.ctrlKey,
          metaKey: objectProto.metaKey,
          node: objectProto.nodePtr != undefined ? _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference : null,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: DoubleClickEvent): Uint8Array {
        const proto = this.packObject(object);
        return DoubleClickEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): DoubleClickEvent {
        const proto = DoubleClickEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 2000203)] = new DoubleClickEventProtoEncoder();

    class TripleClickEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: TripleClickEvent): TripleClickEventProto {
        const objectProto: Partial<TripleClickEventProto> = { metatype: 2000204 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        if (object.nodePtr != null) {
          objectProto.nodePtr = object.nodePtr.pack(10);
        }
        objectProto.position = object.position.pack(10);
        if (object.pressure != null) {
          objectProto.pressure = object.pressure;
        }
        objectProto.shiftKey = object.shiftKey;
        objectProto.altKey = object.altKey;
        objectProto.ctrlKey = object.ctrlKey;
        objectProto.metaKey = object.metaKey;
        objectProto.button = Number(object.button) as any;
        return objectProto as TripleClickEventProto;
      }

      unpackObject(objectProto: TripleClickEventProto, _session: Session | null): TripleClickEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
        return new (NODE_CLASS_BY_TYPE[2000204] as typeof TripleClickEvent)({
          button: Number(objectProto.button) as any,
          position: _Vector2.unpack(10, objectProto.position, _session) as Vector2,
          pressure: objectProto.pressure != undefined ? objectProto.pressure : null,
          shiftKey: objectProto.shiftKey,
          altKey: objectProto.altKey,
          ctrlKey: objectProto.ctrlKey,
          metaKey: objectProto.metaKey,
          node: objectProto.nodePtr != undefined ? _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference : null,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: TripleClickEvent): Uint8Array {
        const proto = this.packObject(object);
        return TripleClickEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): TripleClickEvent {
        const proto = TripleClickEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 2000204)] = new TripleClickEventProtoEncoder();

    class WheelEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: WheelEvent): WheelEventProto {
        const objectProto: Partial<WheelEventProto> = { metatype: 2000210 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        if (object.nodePtr != null) {
          objectProto.nodePtr = object.nodePtr.pack(10);
        }
        objectProto.position = object.position.pack(10);
        if (object.pressure != null) {
          objectProto.pressure = object.pressure;
        }
        objectProto.shiftKey = object.shiftKey;
        objectProto.altKey = object.altKey;
        objectProto.ctrlKey = object.ctrlKey;
        objectProto.metaKey = object.metaKey;
        objectProto.button = Number(object.button) as any;
        objectProto.delta = object.delta.pack(10);
        return objectProto as WheelEventProto;
      }

      unpackObject(objectProto: WheelEventProto, _session: Session | null): WheelEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
        return new (NODE_CLASS_BY_TYPE[2000210] as typeof WheelEvent)({
          delta: _Vector2.unpack(10, objectProto.delta, _session) as Vector2,
          button: Number(objectProto.button) as any,
          position: _Vector2.unpack(10, objectProto.position, _session) as Vector2,
          pressure: objectProto.pressure != undefined ? objectProto.pressure : null,
          shiftKey: objectProto.shiftKey,
          altKey: objectProto.altKey,
          ctrlKey: objectProto.ctrlKey,
          metaKey: objectProto.metaKey,
          node: objectProto.nodePtr != undefined ? _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference : null,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: WheelEvent): Uint8Array {
        const proto = this.packObject(object);
        return WheelEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): WheelEvent {
        const proto = WheelEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 2000210)] = new WheelEventProtoEncoder();

    class ScriptProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Script): ScriptProto {
        const objectProto: Partial<ScriptProto> = { metatype: 700000 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        objectProto.code = object._code;
        return objectProto as ScriptProto;
      }

      unpackObject(objectProto: ScriptProto, _session: Session | null): Script {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[700000] as typeof Script)({
          code: objectProto.code,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: Script): Uint8Array {
        const proto = this.packObject(object);
        return ScriptProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Script {
        const proto = ScriptProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 700000)] = new ScriptProtoEncoder();

    class ServiceProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Service): ServiceProto {
        const objectProto: Partial<ServiceProto> = { metatype: 10300 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        if (object._icon != null) {
          objectProto.icon = object._icon.pack(10);
        }
        return objectProto as ServiceProto;
      }

      unpackObject(objectProto: ServiceProto, _session: Session | null): Service {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[10300] as typeof Service)({
          icon: objectProto.icon != undefined ? _Icon.unpack(10, objectProto.icon, _session) as Icon : null,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: Service): Uint8Array {
        const proto = this.packObject(object);
        return ServiceProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Service {
        const proto = ServiceProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 10300)] = new ServiceProtoEncoder();

    class TimerStartedEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: TimerStartedEvent): TimerStartedEventProto {
        const objectProto: Partial<TimerStartedEventProto> = { metatype: 705102 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        objectProto.nodePtr = object.nodePtr.pack(10);
        return objectProto as TimerStartedEventProto;
      }

      unpackObject(objectProto: TimerStartedEventProto, _session: Session | null): TimerStartedEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[705102] as typeof TimerStartedEvent)({
          node: _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: TimerStartedEvent): Uint8Array {
        const proto = this.packObject(object);
        return TimerStartedEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): TimerStartedEvent {
        const proto = TimerStartedEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 705102)] = new TimerStartedEventProtoEncoder();

    class TimerPausedEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: TimerPausedEvent): TimerPausedEventProto {
        const objectProto: Partial<TimerPausedEventProto> = { metatype: 705103 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        objectProto.nodePtr = object.nodePtr.pack(10);
        return objectProto as TimerPausedEventProto;
      }

      unpackObject(objectProto: TimerPausedEventProto, _session: Session | null): TimerPausedEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[705103] as typeof TimerPausedEvent)({
          node: _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: TimerPausedEvent): Uint8Array {
        const proto = this.packObject(object);
        return TimerPausedEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): TimerPausedEvent {
        const proto = TimerPausedEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 705103)] = new TimerPausedEventProtoEncoder();

    class TimerResumedEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: TimerResumedEvent): TimerResumedEventProto {
        const objectProto: Partial<TimerResumedEventProto> = { metatype: 705104 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        objectProto.nodePtr = object.nodePtr.pack(10);
        return objectProto as TimerResumedEventProto;
      }

      unpackObject(objectProto: TimerResumedEventProto, _session: Session | null): TimerResumedEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[705104] as typeof TimerResumedEvent)({
          node: _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: TimerResumedEvent): Uint8Array {
        const proto = this.packObject(object);
        return TimerResumedEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): TimerResumedEvent {
        const proto = TimerResumedEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 705104)] = new TimerResumedEventProtoEncoder();

    class TimerCompletedEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: TimerCompletedEvent): TimerCompletedEventProto {
        const objectProto: Partial<TimerCompletedEventProto> = { metatype: 705105 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        objectProto.nodePtr = object.nodePtr.pack(10);
        return objectProto as TimerCompletedEventProto;
      }

      unpackObject(objectProto: TimerCompletedEventProto, _session: Session | null): TimerCompletedEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[705105] as typeof TimerCompletedEvent)({
          node: _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: TimerCompletedEvent): Uint8Array {
        const proto = this.packObject(object);
        return TimerCompletedEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): TimerCompletedEvent {
        const proto = TimerCompletedEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 705105)] = new TimerCompletedEventProtoEncoder();

    class TimerCancelledEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: TimerCancelledEvent): TimerCancelledEventProto {
        const objectProto: Partial<TimerCancelledEventProto> = { metatype: 705106 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        objectProto.nodePtr = object.nodePtr.pack(10);
        return objectProto as TimerCancelledEventProto;
      }

      unpackObject(objectProto: TimerCancelledEventProto, _session: Session | null): TimerCancelledEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[705106] as typeof TimerCancelledEvent)({
          node: _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: TimerCancelledEvent): Uint8Array {
        const proto = this.packObject(object);
        return TimerCancelledEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): TimerCancelledEvent {
        const proto = TimerCancelledEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 705106)] = new TimerCancelledEventProtoEncoder();

    class TimerProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Timer): TimerProto {
        const objectProto: Partial<TimerProto> = { metatype: 705100 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        objectProto.type = Number(object._type) as any;
        if (object._schedule != null) {
          objectProto.schedule = object._schedule.pack(10);
        }
        return objectProto as TimerProto;
      }

      unpackObject(objectProto: TimerProto, _session: Session | null): Timer {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Schedule = STRUCT_CLASS_BY_TYPE[700001] as typeof Schedule;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[705100] as typeof Timer)({
          type: Number(objectProto.type) as any,
          schedule: objectProto.schedule != undefined ? _Schedule.unpack(10, objectProto.schedule, _session) as Schedule : null,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: Timer): Uint8Array {
        const proto = this.packObject(object);
        return TimerProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Timer {
        const proto = TimerProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 705100)] = new TimerProtoEncoder();

    class TriggerProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Trigger): TriggerProto {
        const objectProto: Partial<TriggerProto> = { metatype: 705000 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        if (object._icon != null) {
          objectProto.icon = object._icon.pack(10);
        }
        if (object._event != null) {
          objectProto.event = object._event.pack(10);
        }
        if (object._where != null) {
          objectProto.where = object._where.pack(10);
        }
        if (object._targetPtr != null) {
          objectProto.targetPtr = object._targetPtr.pack(10);
        }
        if (object._arguments) {
          objectProto.arguments = {} as any;
          for (const [key, value] of Object.entries(object._arguments) ) {
            objectProto.arguments![String(key)] = value.pack(10);
          }
        }
        return objectProto as TriggerProto;
      }

      unpackObject(objectProto: TriggerProto, _session: Session | null): Trigger {
        const _NodeDefinitionReference = STRUCT_CLASS_BY_TYPE[13] as typeof NodeDefinitionReference;
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _Condition = STRUCT_CLASS_BY_TYPE[204] as typeof Condition;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
        const unpackedArguments = {} as any;
        if (objectProto.arguments) {
          for (const [key, value] of Object.entries(objectProto.arguments)) {
            unpackedArguments.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[705000] as typeof Trigger)({
          icon: objectProto.icon != undefined ? _Icon.unpack(10, objectProto.icon, _session) as Icon : null,
          event: objectProto.event != undefined ? _NodeDefinitionReference.unpack(10, objectProto.event, _session) as NodeDefinitionReference : null,
          where: objectProto.where != undefined ? _Condition.unpack(10, objectProto.where, _session) as Condition : null,
          target: objectProto.targetPtr != undefined ? _NodeReference.unpack(10, objectProto.targetPtr, _session) as NodeReference : null,
          arguments: unpackedArguments,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: Trigger): Uint8Array {
        const proto = this.packObject(object);
        return TriggerProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Trigger {
        const proto = TriggerProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 705000)] = new TriggerProtoEncoder();

    class GaugeMetricProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: GaugeMetric): GaugeMetricProto {
        const objectProto: Partial<GaugeMetricProto> = { metatype: 1200000 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        if (object._icon != null) {
          objectProto.icon = object._icon.pack(10);
        }
        return objectProto as GaugeMetricProto;
      }

      unpackObject(objectProto: GaugeMetricProto, _session: Session | null): GaugeMetric {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[1200000] as typeof GaugeMetric)({
          icon: objectProto.icon != undefined ? _Icon.unpack(10, objectProto.icon, _session) as Icon : null,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: GaugeMetric): Uint8Array {
        const proto = this.packObject(object);
        return GaugeMetricProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): GaugeMetric {
        const proto = GaugeMetricProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1200000)] = new GaugeMetricProtoEncoder();

    class GaugeMeasurementEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: GaugeMeasurementEvent): GaugeMeasurementEventProto {
        const objectProto: Partial<GaugeMeasurementEventProto> = { metatype: 1200001 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.definitionPtr = object.definitionPtr.pack(10);
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        if (object.nodePtr != null) {
          objectProto.nodePtr = object.nodePtr.pack(10);
        }
        return objectProto as GaugeMeasurementEventProto;
      }

      unpackObject(objectProto: GaugeMeasurementEventProto, _session: Session | null): GaugeMeasurementEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[1200001] as typeof GaugeMeasurementEvent)({
          definition: _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          node: objectProto.nodePtr != undefined ? _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: GaugeMeasurementEvent): Uint8Array {
        const proto = this.packObject(object);
        return GaugeMeasurementEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): GaugeMeasurementEvent {
        const proto = GaugeMeasurementEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1200001)] = new GaugeMeasurementEventProtoEncoder();

    class CounterMetricProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: CounterMetric): CounterMetricProto {
        const objectProto: Partial<CounterMetricProto> = { metatype: 1200100 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        if (object._icon != null) {
          objectProto.icon = object._icon.pack(10);
        }
        return objectProto as CounterMetricProto;
      }

      unpackObject(objectProto: CounterMetricProto, _session: Session | null): CounterMetric {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[1200100] as typeof CounterMetric)({
          icon: objectProto.icon != undefined ? _Icon.unpack(10, objectProto.icon, _session) as Icon : null,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: CounterMetric): Uint8Array {
        const proto = this.packObject(object);
        return CounterMetricProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): CounterMetric {
        const proto = CounterMetricProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1200100)] = new CounterMetricProtoEncoder();

    class CounterMeasurementEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: CounterMeasurementEvent): CounterMeasurementEventProto {
        const objectProto: Partial<CounterMeasurementEventProto> = { metatype: 1200101 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.definitionPtr = object.definitionPtr.pack(10);
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        if (object.nodePtr != null) {
          objectProto.nodePtr = object.nodePtr.pack(10);
        }
        return objectProto as CounterMeasurementEventProto;
      }

      unpackObject(objectProto: CounterMeasurementEventProto, _session: Session | null): CounterMeasurementEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[1200101] as typeof CounterMeasurementEvent)({
          definition: _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          node: objectProto.nodePtr != undefined ? _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: CounterMeasurementEvent): Uint8Array {
        const proto = this.packObject(object);
        return CounterMeasurementEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): CounterMeasurementEvent {
        const proto = CounterMeasurementEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1200101)] = new CounterMeasurementEventProtoEncoder();

    class HistogramMetricProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: HistogramMetric): HistogramMetricProto {
        const objectProto: Partial<HistogramMetricProto> = { metatype: 1200200 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        if (object._icon != null) {
          objectProto.icon = object._icon.pack(10);
        }
        return objectProto as HistogramMetricProto;
      }

      unpackObject(objectProto: HistogramMetricProto, _session: Session | null): HistogramMetric {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[1200200] as typeof HistogramMetric)({
          icon: objectProto.icon != undefined ? _Icon.unpack(10, objectProto.icon, _session) as Icon : null,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: HistogramMetric): Uint8Array {
        const proto = this.packObject(object);
        return HistogramMetricProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): HistogramMetric {
        const proto = HistogramMetricProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1200200)] = new HistogramMetricProtoEncoder();

    class HistogramMeasurementEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: HistogramMeasurementEvent): HistogramMeasurementEventProto {
        const objectProto: Partial<HistogramMeasurementEventProto> = { metatype: 1200201 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.definitionPtr = object.definitionPtr.pack(10);
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        if (object.nodePtr != null) {
          objectProto.nodePtr = object.nodePtr.pack(10);
        }
        return objectProto as HistogramMeasurementEventProto;
      }

      unpackObject(objectProto: HistogramMeasurementEventProto, _session: Session | null): HistogramMeasurementEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[1200201] as typeof HistogramMeasurementEvent)({
          definition: _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          node: objectProto.nodePtr != undefined ? _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: HistogramMeasurementEvent): Uint8Array {
        const proto = this.packObject(object);
        return HistogramMeasurementEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): HistogramMeasurementEvent {
        const proto = HistogramMeasurementEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1200201)] = new HistogramMeasurementEventProtoEncoder();

    class LayerProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Layer): LayerProto {
        const objectProto: Partial<LayerProto> = { metatype: 1700300 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        objectProto.type = Number(object._type) as any;
        if (object._icon != null) {
          objectProto.icon = object._icon.pack(10);
        }
        if (object._isVisible != null) {
          objectProto.isVisible = object._isVisible;
        }
        if (object._opacity != null) {
          objectProto.opacity = object._opacity;
        }
        return objectProto as LayerProto;
      }

      unpackObject(objectProto: LayerProto, _session: Session | null): Layer {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[1700300] as typeof Layer)({
          type: Number(objectProto.type) as any,
          icon: objectProto.icon != undefined ? _Icon.unpack(10, objectProto.icon, _session) as Icon : null,
          isVisible: objectProto.isVisible != undefined ? objectProto.isVisible : null,
          opacity: objectProto.opacity != undefined ? objectProto.opacity : null,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: Layer): Uint8Array {
        const proto = this.packObject(object);
        return LayerProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Layer {
        const proto = LayerProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1700300)] = new LayerProtoEncoder();

    class FrameViewProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: FrameView): FrameViewProto {
        const objectProto: Partial<FrameViewProto> = { metatype: 1800200 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        if (object._position != null) {
          objectProto.position = object._position.pack(10);
        }
        if (object._offset != null) {
          objectProto.offset = object._offset.pack(10);
        }
        if (object._scale != null) {
          objectProto.scale = object._scale.pack(10);
        }
        if (object._rotation != null) {
          objectProto.rotation = object._rotation.pack(10);
        }
        if (object._skew != null) {
          objectProto.skew = object._skew.pack(10);
        }
        if (object._origin != null) {
          objectProto.origin = object._origin.pack(10);
        }
        if (object._anchor != null) {
          objectProto.anchor = Number(object._anchor) as any;
        }
        if (object._width != null) {
          objectProto.width = object._width.pack(10);
        }
        if (object._height != null) {
          objectProto.height = object._height.pack(10);
        }
        if (object._minWidth != null) {
          objectProto.minWidth = object._minWidth.pack(10);
        }
        if (object._minHeight != null) {
          objectProto.minHeight = object._minHeight.pack(10);
        }
        if (object._maxWidth != null) {
          objectProto.maxWidth = object._maxWidth.pack(10);
        }
        if (object._maxHeight != null) {
          objectProto.maxHeight = object._maxHeight.pack(10);
        }
        if (object._isVisible != null) {
          objectProto.isVisible = object._isVisible;
        }
        if (object._opacity != null) {
          objectProto.opacity = object._opacity;
        }
        if (object._fill != null) {
          objectProto.fill = object._fill.pack(10);
        }
        if (object._shadow != null) {
          objectProto.shadow = object._shadow.pack(10);
        }
        if (object._border != null) {
          objectProto.border = object._border.pack(10);
        }
        if (object._radius != null) {
          objectProto.radius = object._radius.pack(10);
        }
        if (object._layout != null) {
          objectProto.layout = Number(object._layout) as any;
        }
        if (object._direction != null) {
          objectProto.direction = Number(object._direction) as any;
        }
        if (object._distribute != null) {
          objectProto.distribute = Number(object._distribute) as any;
        }
        if (object._align != null) {
          objectProto.align = Number(object._align) as any;
        }
        if (object._gap != null) {
          objectProto.gap = object._gap.pack(10);
        }
        if (object._padding != null) {
          objectProto.padding = object._padding.pack(10);
        }
        if (object._grid != null) {
          objectProto.grid = object._grid.pack(10);
        }
        if (object._gridSpan != null) {
          objectProto.gridSpan = object._gridSpan.pack(10);
        }
        if (object._aspectRatio != null) {
          objectProto.aspectRatio = object._aspectRatio;
        }
        if (object._isWrap != null) {
          objectProto.isWrap = object._isWrap;
        }
        return objectProto as FrameViewProto;
      }

      unpackObject(objectProto: FrameViewProto, _session: Session | null): FrameView {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Length = STRUCT_CLASS_BY_TYPE[1800001] as typeof Length;
        const _Fill = STRUCT_CLASS_BY_TYPE[2100400] as typeof Fill;
        const _Border = STRUCT_CLASS_BY_TYPE[2100600] as typeof Border;
        const _Shadow = STRUCT_CLASS_BY_TYPE[2100700] as typeof Shadow;
        const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
        const _Offset2 = STRUCT_CLASS_BY_TYPE[2400020] as typeof Offset2;
        const _Grid2 = STRUCT_CLASS_BY_TYPE[2400021] as typeof Grid2;
        const _GridSpan2 = STRUCT_CLASS_BY_TYPE[2400022] as typeof GridSpan2;
        const _Inset2 = STRUCT_CLASS_BY_TYPE[2400023] as typeof Inset2;
        const _Corner2 = STRUCT_CLASS_BY_TYPE[2400024] as typeof Corner2;
        const _Axis2 = STRUCT_CLASS_BY_TYPE[2400025] as typeof Axis2;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[1800200] as typeof FrameView)({
          layout: objectProto.layout != undefined ? Number(objectProto.layout) as any : null,
          direction: objectProto.direction != undefined ? Number(objectProto.direction) as any : null,
          distribute: objectProto.distribute != undefined ? Number(objectProto.distribute) as any : null,
          align: objectProto.align != undefined ? Number(objectProto.align) as any : null,
          gap: objectProto.gap != undefined ? _Axis2.unpack(10, objectProto.gap, _session) as Axis2 : null,
          padding: objectProto.padding != undefined ? _Inset2.unpack(10, objectProto.padding, _session) as Inset2 : null,
          grid: objectProto.grid != undefined ? _Grid2.unpack(10, objectProto.grid, _session) as Grid2 : null,
          gridSpan: objectProto.gridSpan != undefined ? _GridSpan2.unpack(10, objectProto.gridSpan, _session) as GridSpan2 : null,
          aspectRatio: objectProto.aspectRatio != undefined ? objectProto.aspectRatio : null,
          isWrap: objectProto.isWrap != undefined ? objectProto.isWrap : null,
          width: objectProto.width != undefined ? _Length.unpack(10, objectProto.width, _session) as Length : null,
          height: objectProto.height != undefined ? _Length.unpack(10, objectProto.height, _session) as Length : null,
          minWidth: objectProto.minWidth != undefined ? _Length.unpack(10, objectProto.minWidth, _session) as Length : null,
          minHeight: objectProto.minHeight != undefined ? _Length.unpack(10, objectProto.minHeight, _session) as Length : null,
          maxWidth: objectProto.maxWidth != undefined ? _Length.unpack(10, objectProto.maxWidth, _session) as Length : null,
          maxHeight: objectProto.maxHeight != undefined ? _Length.unpack(10, objectProto.maxHeight, _session) as Length : null,
          isVisible: objectProto.isVisible != undefined ? objectProto.isVisible : null,
          opacity: objectProto.opacity != undefined ? objectProto.opacity : null,
          fill: objectProto.fill != undefined ? _Fill.unpack(10, objectProto.fill, _session) as Fill : null,
          shadow: objectProto.shadow != undefined ? _Shadow.unpack(10, objectProto.shadow, _session) as Shadow : null,
          border: objectProto.border != undefined ? _Border.unpack(10, objectProto.border, _session) as Border : null,
          radius: objectProto.radius != undefined ? _Corner2.unpack(10, objectProto.radius, _session) as Corner2 : null,
          position: objectProto.position != undefined ? _Vector2.unpack(10, objectProto.position, _session) as Vector2 : null,
          offset: objectProto.offset != undefined ? _Offset2.unpack(10, objectProto.offset, _session) as Offset2 : null,
          scale: objectProto.scale != undefined ? _Vector2.unpack(10, objectProto.scale, _session) as Vector2 : null,
          rotation: objectProto.rotation != undefined ? _Vector2.unpack(10, objectProto.rotation, _session) as Vector2 : null,
          skew: objectProto.skew != undefined ? _Vector2.unpack(10, objectProto.skew, _session) as Vector2 : null,
          origin: objectProto.origin != undefined ? _Vector2.unpack(10, objectProto.origin, _session) as Vector2 : null,
          anchor: objectProto.anchor != undefined ? Number(objectProto.anchor) as any : null,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: FrameView): Uint8Array {
        const proto = this.packObject(object);
        return FrameViewProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): FrameView {
        const proto = FrameViewProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1800200)] = new FrameViewProtoEncoder();

    class LabelViewProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: LabelView): LabelViewProto {
        const objectProto: Partial<LabelViewProto> = { metatype: 1800300 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        if (object._position != null) {
          objectProto.position = object._position.pack(10);
        }
        if (object._offset != null) {
          objectProto.offset = object._offset.pack(10);
        }
        if (object._scale != null) {
          objectProto.scale = object._scale.pack(10);
        }
        if (object._rotation != null) {
          objectProto.rotation = object._rotation.pack(10);
        }
        if (object._skew != null) {
          objectProto.skew = object._skew.pack(10);
        }
        if (object._origin != null) {
          objectProto.origin = object._origin.pack(10);
        }
        if (object._anchor != null) {
          objectProto.anchor = Number(object._anchor) as any;
        }
        if (object._width != null) {
          objectProto.width = object._width.pack(10);
        }
        if (object._height != null) {
          objectProto.height = object._height.pack(10);
        }
        if (object._minWidth != null) {
          objectProto.minWidth = object._minWidth.pack(10);
        }
        if (object._minHeight != null) {
          objectProto.minHeight = object._minHeight.pack(10);
        }
        if (object._maxWidth != null) {
          objectProto.maxWidth = object._maxWidth.pack(10);
        }
        if (object._maxHeight != null) {
          objectProto.maxHeight = object._maxHeight.pack(10);
        }
        if (object._isVisible != null) {
          objectProto.isVisible = object._isVisible;
        }
        if (object._opacity != null) {
          objectProto.opacity = object._opacity;
        }
        if (object._fill != null) {
          objectProto.fill = object._fill.pack(10);
        }
        if (object._shadow != null) {
          objectProto.shadow = object._shadow.pack(10);
        }
        if (object._border != null) {
          objectProto.border = object._border.pack(10);
        }
        if (object._radius != null) {
          objectProto.radius = object._radius.pack(10);
        }
        if (object._layout != null) {
          objectProto.layout = Number(object._layout) as any;
        }
        if (object._direction != null) {
          objectProto.direction = Number(object._direction) as any;
        }
        if (object._distribute != null) {
          objectProto.distribute = Number(object._distribute) as any;
        }
        if (object._align != null) {
          objectProto.align = Number(object._align) as any;
        }
        if (object._gap != null) {
          objectProto.gap = object._gap.pack(10);
        }
        if (object._padding != null) {
          objectProto.padding = object._padding.pack(10);
        }
        if (object._grid != null) {
          objectProto.grid = object._grid.pack(10);
        }
        if (object._gridSpan != null) {
          objectProto.gridSpan = object._gridSpan.pack(10);
        }
        if (object._aspectRatio != null) {
          objectProto.aspectRatio = object._aspectRatio;
        }
        if (object._isWrap != null) {
          objectProto.isWrap = object._isWrap;
        }
        return objectProto as LabelViewProto;
      }

      unpackObject(objectProto: LabelViewProto, _session: Session | null): LabelView {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Length = STRUCT_CLASS_BY_TYPE[1800001] as typeof Length;
        const _Fill = STRUCT_CLASS_BY_TYPE[2100400] as typeof Fill;
        const _Border = STRUCT_CLASS_BY_TYPE[2100600] as typeof Border;
        const _Shadow = STRUCT_CLASS_BY_TYPE[2100700] as typeof Shadow;
        const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
        const _Offset2 = STRUCT_CLASS_BY_TYPE[2400020] as typeof Offset2;
        const _Grid2 = STRUCT_CLASS_BY_TYPE[2400021] as typeof Grid2;
        const _GridSpan2 = STRUCT_CLASS_BY_TYPE[2400022] as typeof GridSpan2;
        const _Inset2 = STRUCT_CLASS_BY_TYPE[2400023] as typeof Inset2;
        const _Corner2 = STRUCT_CLASS_BY_TYPE[2400024] as typeof Corner2;
        const _Axis2 = STRUCT_CLASS_BY_TYPE[2400025] as typeof Axis2;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[1800300] as typeof LabelView)({
          layout: objectProto.layout != undefined ? Number(objectProto.layout) as any : null,
          direction: objectProto.direction != undefined ? Number(objectProto.direction) as any : null,
          distribute: objectProto.distribute != undefined ? Number(objectProto.distribute) as any : null,
          align: objectProto.align != undefined ? Number(objectProto.align) as any : null,
          gap: objectProto.gap != undefined ? _Axis2.unpack(10, objectProto.gap, _session) as Axis2 : null,
          padding: objectProto.padding != undefined ? _Inset2.unpack(10, objectProto.padding, _session) as Inset2 : null,
          grid: objectProto.grid != undefined ? _Grid2.unpack(10, objectProto.grid, _session) as Grid2 : null,
          gridSpan: objectProto.gridSpan != undefined ? _GridSpan2.unpack(10, objectProto.gridSpan, _session) as GridSpan2 : null,
          aspectRatio: objectProto.aspectRatio != undefined ? objectProto.aspectRatio : null,
          isWrap: objectProto.isWrap != undefined ? objectProto.isWrap : null,
          width: objectProto.width != undefined ? _Length.unpack(10, objectProto.width, _session) as Length : null,
          height: objectProto.height != undefined ? _Length.unpack(10, objectProto.height, _session) as Length : null,
          minWidth: objectProto.minWidth != undefined ? _Length.unpack(10, objectProto.minWidth, _session) as Length : null,
          minHeight: objectProto.minHeight != undefined ? _Length.unpack(10, objectProto.minHeight, _session) as Length : null,
          maxWidth: objectProto.maxWidth != undefined ? _Length.unpack(10, objectProto.maxWidth, _session) as Length : null,
          maxHeight: objectProto.maxHeight != undefined ? _Length.unpack(10, objectProto.maxHeight, _session) as Length : null,
          isVisible: objectProto.isVisible != undefined ? objectProto.isVisible : null,
          opacity: objectProto.opacity != undefined ? objectProto.opacity : null,
          fill: objectProto.fill != undefined ? _Fill.unpack(10, objectProto.fill, _session) as Fill : null,
          shadow: objectProto.shadow != undefined ? _Shadow.unpack(10, objectProto.shadow, _session) as Shadow : null,
          border: objectProto.border != undefined ? _Border.unpack(10, objectProto.border, _session) as Border : null,
          radius: objectProto.radius != undefined ? _Corner2.unpack(10, objectProto.radius, _session) as Corner2 : null,
          position: objectProto.position != undefined ? _Vector2.unpack(10, objectProto.position, _session) as Vector2 : null,
          offset: objectProto.offset != undefined ? _Offset2.unpack(10, objectProto.offset, _session) as Offset2 : null,
          scale: objectProto.scale != undefined ? _Vector2.unpack(10, objectProto.scale, _session) as Vector2 : null,
          rotation: objectProto.rotation != undefined ? _Vector2.unpack(10, objectProto.rotation, _session) as Vector2 : null,
          skew: objectProto.skew != undefined ? _Vector2.unpack(10, objectProto.skew, _session) as Vector2 : null,
          origin: objectProto.origin != undefined ? _Vector2.unpack(10, objectProto.origin, _session) as Vector2 : null,
          anchor: objectProto.anchor != undefined ? Number(objectProto.anchor) as any : null,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: LabelView): Uint8Array {
        const proto = this.packObject(object);
        return LabelViewProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): LabelView {
        const proto = LabelViewProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1800300)] = new LabelViewProtoEncoder();

    class NumberInputViewProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: NumberInputView): NumberInputViewProto {
        const objectProto: Partial<NumberInputViewProto> = { metatype: 1810100 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        if (object._position != null) {
          objectProto.position = object._position.pack(10);
        }
        if (object._offset != null) {
          objectProto.offset = object._offset.pack(10);
        }
        if (object._scale != null) {
          objectProto.scale = object._scale.pack(10);
        }
        if (object._rotation != null) {
          objectProto.rotation = object._rotation.pack(10);
        }
        if (object._skew != null) {
          objectProto.skew = object._skew.pack(10);
        }
        if (object._origin != null) {
          objectProto.origin = object._origin.pack(10);
        }
        if (object._anchor != null) {
          objectProto.anchor = Number(object._anchor) as any;
        }
        if (object._width != null) {
          objectProto.width = object._width.pack(10);
        }
        if (object._height != null) {
          objectProto.height = object._height.pack(10);
        }
        if (object._minWidth != null) {
          objectProto.minWidth = object._minWidth.pack(10);
        }
        if (object._minHeight != null) {
          objectProto.minHeight = object._minHeight.pack(10);
        }
        if (object._maxWidth != null) {
          objectProto.maxWidth = object._maxWidth.pack(10);
        }
        if (object._maxHeight != null) {
          objectProto.maxHeight = object._maxHeight.pack(10);
        }
        if (object._isVisible != null) {
          objectProto.isVisible = object._isVisible;
        }
        if (object._opacity != null) {
          objectProto.opacity = object._opacity;
        }
        if (object._fill != null) {
          objectProto.fill = object._fill.pack(10);
        }
        if (object._shadow != null) {
          objectProto.shadow = object._shadow.pack(10);
        }
        if (object._border != null) {
          objectProto.border = object._border.pack(10);
        }
        if (object._radius != null) {
          objectProto.radius = object._radius.pack(10);
        }
        if (object._value != null) {
          objectProto.value = object._value;
        }
        if (object._placeholder != null) {
          objectProto.placeholder = object._placeholder;
        }
        return objectProto as NumberInputViewProto;
      }

      unpackObject(objectProto: NumberInputViewProto, _session: Session | null): NumberInputView {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Length = STRUCT_CLASS_BY_TYPE[1800001] as typeof Length;
        const _Fill = STRUCT_CLASS_BY_TYPE[2100400] as typeof Fill;
        const _Border = STRUCT_CLASS_BY_TYPE[2100600] as typeof Border;
        const _Shadow = STRUCT_CLASS_BY_TYPE[2100700] as typeof Shadow;
        const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
        const _Offset2 = STRUCT_CLASS_BY_TYPE[2400020] as typeof Offset2;
        const _Corner2 = STRUCT_CLASS_BY_TYPE[2400024] as typeof Corner2;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[1810100] as typeof NumberInputView)({
          value: objectProto.value != undefined ? objectProto.value : null,
          placeholder: objectProto.placeholder != undefined ? objectProto.placeholder : null,
          width: objectProto.width != undefined ? _Length.unpack(10, objectProto.width, _session) as Length : null,
          height: objectProto.height != undefined ? _Length.unpack(10, objectProto.height, _session) as Length : null,
          minWidth: objectProto.minWidth != undefined ? _Length.unpack(10, objectProto.minWidth, _session) as Length : null,
          minHeight: objectProto.minHeight != undefined ? _Length.unpack(10, objectProto.minHeight, _session) as Length : null,
          maxWidth: objectProto.maxWidth != undefined ? _Length.unpack(10, objectProto.maxWidth, _session) as Length : null,
          maxHeight: objectProto.maxHeight != undefined ? _Length.unpack(10, objectProto.maxHeight, _session) as Length : null,
          isVisible: objectProto.isVisible != undefined ? objectProto.isVisible : null,
          opacity: objectProto.opacity != undefined ? objectProto.opacity : null,
          fill: objectProto.fill != undefined ? _Fill.unpack(10, objectProto.fill, _session) as Fill : null,
          shadow: objectProto.shadow != undefined ? _Shadow.unpack(10, objectProto.shadow, _session) as Shadow : null,
          border: objectProto.border != undefined ? _Border.unpack(10, objectProto.border, _session) as Border : null,
          radius: objectProto.radius != undefined ? _Corner2.unpack(10, objectProto.radius, _session) as Corner2 : null,
          position: objectProto.position != undefined ? _Vector2.unpack(10, objectProto.position, _session) as Vector2 : null,
          offset: objectProto.offset != undefined ? _Offset2.unpack(10, objectProto.offset, _session) as Offset2 : null,
          scale: objectProto.scale != undefined ? _Vector2.unpack(10, objectProto.scale, _session) as Vector2 : null,
          rotation: objectProto.rotation != undefined ? _Vector2.unpack(10, objectProto.rotation, _session) as Vector2 : null,
          skew: objectProto.skew != undefined ? _Vector2.unpack(10, objectProto.skew, _session) as Vector2 : null,
          origin: objectProto.origin != undefined ? _Vector2.unpack(10, objectProto.origin, _session) as Vector2 : null,
          anchor: objectProto.anchor != undefined ? Number(objectProto.anchor) as any : null,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: NumberInputView): Uint8Array {
        const proto = this.packObject(object);
        return NumberInputViewProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): NumberInputView {
        const proto = NumberInputViewProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1810100)] = new NumberInputViewProtoEncoder();

    class SliderInputViewProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: SliderInputView): SliderInputViewProto {
        const objectProto: Partial<SliderInputViewProto> = { metatype: 1810200 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        if (object._position != null) {
          objectProto.position = object._position.pack(10);
        }
        if (object._offset != null) {
          objectProto.offset = object._offset.pack(10);
        }
        if (object._scale != null) {
          objectProto.scale = object._scale.pack(10);
        }
        if (object._rotation != null) {
          objectProto.rotation = object._rotation.pack(10);
        }
        if (object._skew != null) {
          objectProto.skew = object._skew.pack(10);
        }
        if (object._origin != null) {
          objectProto.origin = object._origin.pack(10);
        }
        if (object._anchor != null) {
          objectProto.anchor = Number(object._anchor) as any;
        }
        if (object._width != null) {
          objectProto.width = object._width.pack(10);
        }
        if (object._height != null) {
          objectProto.height = object._height.pack(10);
        }
        if (object._minWidth != null) {
          objectProto.minWidth = object._minWidth.pack(10);
        }
        if (object._minHeight != null) {
          objectProto.minHeight = object._minHeight.pack(10);
        }
        if (object._maxWidth != null) {
          objectProto.maxWidth = object._maxWidth.pack(10);
        }
        if (object._maxHeight != null) {
          objectProto.maxHeight = object._maxHeight.pack(10);
        }
        if (object._isVisible != null) {
          objectProto.isVisible = object._isVisible;
        }
        if (object._opacity != null) {
          objectProto.opacity = object._opacity;
        }
        if (object._fill != null) {
          objectProto.fill = object._fill.pack(10);
        }
        if (object._shadow != null) {
          objectProto.shadow = object._shadow.pack(10);
        }
        if (object._border != null) {
          objectProto.border = object._border.pack(10);
        }
        if (object._radius != null) {
          objectProto.radius = object._radius.pack(10);
        }
        if (object._value != null) {
          objectProto.value = object._value;
        }
        if (object._minValue != null) {
          objectProto.minValue = object._minValue;
        }
        if (object._maxValue != null) {
          objectProto.maxValue = object._maxValue;
        }
        if (object._step != null) {
          objectProto.step = object._step;
        }
        return objectProto as SliderInputViewProto;
      }

      unpackObject(objectProto: SliderInputViewProto, _session: Session | null): SliderInputView {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Length = STRUCT_CLASS_BY_TYPE[1800001] as typeof Length;
        const _Fill = STRUCT_CLASS_BY_TYPE[2100400] as typeof Fill;
        const _Border = STRUCT_CLASS_BY_TYPE[2100600] as typeof Border;
        const _Shadow = STRUCT_CLASS_BY_TYPE[2100700] as typeof Shadow;
        const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
        const _Offset2 = STRUCT_CLASS_BY_TYPE[2400020] as typeof Offset2;
        const _Corner2 = STRUCT_CLASS_BY_TYPE[2400024] as typeof Corner2;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[1810200] as typeof SliderInputView)({
          value: objectProto.value != undefined ? objectProto.value : null,
          minValue: objectProto.minValue != undefined ? objectProto.minValue : null,
          maxValue: objectProto.maxValue != undefined ? objectProto.maxValue : null,
          step: objectProto.step != undefined ? objectProto.step : null,
          width: objectProto.width != undefined ? _Length.unpack(10, objectProto.width, _session) as Length : null,
          height: objectProto.height != undefined ? _Length.unpack(10, objectProto.height, _session) as Length : null,
          minWidth: objectProto.minWidth != undefined ? _Length.unpack(10, objectProto.minWidth, _session) as Length : null,
          minHeight: objectProto.minHeight != undefined ? _Length.unpack(10, objectProto.minHeight, _session) as Length : null,
          maxWidth: objectProto.maxWidth != undefined ? _Length.unpack(10, objectProto.maxWidth, _session) as Length : null,
          maxHeight: objectProto.maxHeight != undefined ? _Length.unpack(10, objectProto.maxHeight, _session) as Length : null,
          isVisible: objectProto.isVisible != undefined ? objectProto.isVisible : null,
          opacity: objectProto.opacity != undefined ? objectProto.opacity : null,
          fill: objectProto.fill != undefined ? _Fill.unpack(10, objectProto.fill, _session) as Fill : null,
          shadow: objectProto.shadow != undefined ? _Shadow.unpack(10, objectProto.shadow, _session) as Shadow : null,
          border: objectProto.border != undefined ? _Border.unpack(10, objectProto.border, _session) as Border : null,
          radius: objectProto.radius != undefined ? _Corner2.unpack(10, objectProto.radius, _session) as Corner2 : null,
          position: objectProto.position != undefined ? _Vector2.unpack(10, objectProto.position, _session) as Vector2 : null,
          offset: objectProto.offset != undefined ? _Offset2.unpack(10, objectProto.offset, _session) as Offset2 : null,
          scale: objectProto.scale != undefined ? _Vector2.unpack(10, objectProto.scale, _session) as Vector2 : null,
          rotation: objectProto.rotation != undefined ? _Vector2.unpack(10, objectProto.rotation, _session) as Vector2 : null,
          skew: objectProto.skew != undefined ? _Vector2.unpack(10, objectProto.skew, _session) as Vector2 : null,
          origin: objectProto.origin != undefined ? _Vector2.unpack(10, objectProto.origin, _session) as Vector2 : null,
          anchor: objectProto.anchor != undefined ? Number(objectProto.anchor) as any : null,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: SliderInputView): Uint8Array {
        const proto = this.packObject(object);
        return SliderInputViewProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): SliderInputView {
        const proto = SliderInputViewProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1810200)] = new SliderInputViewProtoEncoder();

    class SplitViewProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: SplitView): SplitViewProto {
        const objectProto: Partial<SplitViewProto> = { metatype: 1800400 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        if (object._position != null) {
          objectProto.position = object._position.pack(10);
        }
        if (object._offset != null) {
          objectProto.offset = object._offset.pack(10);
        }
        if (object._scale != null) {
          objectProto.scale = object._scale.pack(10);
        }
        if (object._rotation != null) {
          objectProto.rotation = object._rotation.pack(10);
        }
        if (object._skew != null) {
          objectProto.skew = object._skew.pack(10);
        }
        if (object._origin != null) {
          objectProto.origin = object._origin.pack(10);
        }
        if (object._anchor != null) {
          objectProto.anchor = Number(object._anchor) as any;
        }
        if (object._width != null) {
          objectProto.width = object._width.pack(10);
        }
        if (object._height != null) {
          objectProto.height = object._height.pack(10);
        }
        if (object._minWidth != null) {
          objectProto.minWidth = object._minWidth.pack(10);
        }
        if (object._minHeight != null) {
          objectProto.minHeight = object._minHeight.pack(10);
        }
        if (object._maxWidth != null) {
          objectProto.maxWidth = object._maxWidth.pack(10);
        }
        if (object._maxHeight != null) {
          objectProto.maxHeight = object._maxHeight.pack(10);
        }
        if (object._isVisible != null) {
          objectProto.isVisible = object._isVisible;
        }
        if (object._opacity != null) {
          objectProto.opacity = object._opacity;
        }
        if (object._fill != null) {
          objectProto.fill = object._fill.pack(10);
        }
        if (object._shadow != null) {
          objectProto.shadow = object._shadow.pack(10);
        }
        if (object._border != null) {
          objectProto.border = object._border.pack(10);
        }
        if (object._radius != null) {
          objectProto.radius = object._radius.pack(10);
        }
        if (object._layout != null) {
          objectProto.layout = Number(object._layout) as any;
        }
        if (object._direction != null) {
          objectProto.direction = Number(object._direction) as any;
        }
        if (object._distribute != null) {
          objectProto.distribute = Number(object._distribute) as any;
        }
        if (object._align != null) {
          objectProto.align = Number(object._align) as any;
        }
        if (object._gap != null) {
          objectProto.gap = object._gap.pack(10);
        }
        if (object._padding != null) {
          objectProto.padding = object._padding.pack(10);
        }
        if (object._grid != null) {
          objectProto.grid = object._grid.pack(10);
        }
        if (object._gridSpan != null) {
          objectProto.gridSpan = object._gridSpan.pack(10);
        }
        if (object._aspectRatio != null) {
          objectProto.aspectRatio = object._aspectRatio;
        }
        if (object._isWrap != null) {
          objectProto.isWrap = object._isWrap;
        }
        return objectProto as SplitViewProto;
      }

      unpackObject(objectProto: SplitViewProto, _session: Session | null): SplitView {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Length = STRUCT_CLASS_BY_TYPE[1800001] as typeof Length;
        const _Fill = STRUCT_CLASS_BY_TYPE[2100400] as typeof Fill;
        const _Border = STRUCT_CLASS_BY_TYPE[2100600] as typeof Border;
        const _Shadow = STRUCT_CLASS_BY_TYPE[2100700] as typeof Shadow;
        const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
        const _Offset2 = STRUCT_CLASS_BY_TYPE[2400020] as typeof Offset2;
        const _Grid2 = STRUCT_CLASS_BY_TYPE[2400021] as typeof Grid2;
        const _GridSpan2 = STRUCT_CLASS_BY_TYPE[2400022] as typeof GridSpan2;
        const _Inset2 = STRUCT_CLASS_BY_TYPE[2400023] as typeof Inset2;
        const _Corner2 = STRUCT_CLASS_BY_TYPE[2400024] as typeof Corner2;
        const _Axis2 = STRUCT_CLASS_BY_TYPE[2400025] as typeof Axis2;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[1800400] as typeof SplitView)({
          layout: objectProto.layout != undefined ? Number(objectProto.layout) as any : null,
          direction: objectProto.direction != undefined ? Number(objectProto.direction) as any : null,
          distribute: objectProto.distribute != undefined ? Number(objectProto.distribute) as any : null,
          align: objectProto.align != undefined ? Number(objectProto.align) as any : null,
          gap: objectProto.gap != undefined ? _Axis2.unpack(10, objectProto.gap, _session) as Axis2 : null,
          padding: objectProto.padding != undefined ? _Inset2.unpack(10, objectProto.padding, _session) as Inset2 : null,
          grid: objectProto.grid != undefined ? _Grid2.unpack(10, objectProto.grid, _session) as Grid2 : null,
          gridSpan: objectProto.gridSpan != undefined ? _GridSpan2.unpack(10, objectProto.gridSpan, _session) as GridSpan2 : null,
          aspectRatio: objectProto.aspectRatio != undefined ? objectProto.aspectRatio : null,
          isWrap: objectProto.isWrap != undefined ? objectProto.isWrap : null,
          width: objectProto.width != undefined ? _Length.unpack(10, objectProto.width, _session) as Length : null,
          height: objectProto.height != undefined ? _Length.unpack(10, objectProto.height, _session) as Length : null,
          minWidth: objectProto.minWidth != undefined ? _Length.unpack(10, objectProto.minWidth, _session) as Length : null,
          minHeight: objectProto.minHeight != undefined ? _Length.unpack(10, objectProto.minHeight, _session) as Length : null,
          maxWidth: objectProto.maxWidth != undefined ? _Length.unpack(10, objectProto.maxWidth, _session) as Length : null,
          maxHeight: objectProto.maxHeight != undefined ? _Length.unpack(10, objectProto.maxHeight, _session) as Length : null,
          isVisible: objectProto.isVisible != undefined ? objectProto.isVisible : null,
          opacity: objectProto.opacity != undefined ? objectProto.opacity : null,
          fill: objectProto.fill != undefined ? _Fill.unpack(10, objectProto.fill, _session) as Fill : null,
          shadow: objectProto.shadow != undefined ? _Shadow.unpack(10, objectProto.shadow, _session) as Shadow : null,
          border: objectProto.border != undefined ? _Border.unpack(10, objectProto.border, _session) as Border : null,
          radius: objectProto.radius != undefined ? _Corner2.unpack(10, objectProto.radius, _session) as Corner2 : null,
          position: objectProto.position != undefined ? _Vector2.unpack(10, objectProto.position, _session) as Vector2 : null,
          offset: objectProto.offset != undefined ? _Offset2.unpack(10, objectProto.offset, _session) as Offset2 : null,
          scale: objectProto.scale != undefined ? _Vector2.unpack(10, objectProto.scale, _session) as Vector2 : null,
          rotation: objectProto.rotation != undefined ? _Vector2.unpack(10, objectProto.rotation, _session) as Vector2 : null,
          skew: objectProto.skew != undefined ? _Vector2.unpack(10, objectProto.skew, _session) as Vector2 : null,
          origin: objectProto.origin != undefined ? _Vector2.unpack(10, objectProto.origin, _session) as Vector2 : null,
          anchor: objectProto.anchor != undefined ? Number(objectProto.anchor) as any : null,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: SplitView): Uint8Array {
        const proto = this.packObject(object);
        return SplitViewProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): SplitView {
        const proto = SplitViewProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1800400)] = new SplitViewProtoEncoder();

    class TextViewProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: TextView): TextViewProto {
        const objectProto: Partial<TextViewProto> = { metatype: 1805100 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        if (object._position != null) {
          objectProto.position = object._position.pack(10);
        }
        if (object._offset != null) {
          objectProto.offset = object._offset.pack(10);
        }
        if (object._scale != null) {
          objectProto.scale = object._scale.pack(10);
        }
        if (object._rotation != null) {
          objectProto.rotation = object._rotation.pack(10);
        }
        if (object._skew != null) {
          objectProto.skew = object._skew.pack(10);
        }
        if (object._origin != null) {
          objectProto.origin = object._origin.pack(10);
        }
        if (object._anchor != null) {
          objectProto.anchor = Number(object._anchor) as any;
        }
        if (object._width != null) {
          objectProto.width = object._width.pack(10);
        }
        if (object._height != null) {
          objectProto.height = object._height.pack(10);
        }
        if (object._minWidth != null) {
          objectProto.minWidth = object._minWidth.pack(10);
        }
        if (object._minHeight != null) {
          objectProto.minHeight = object._minHeight.pack(10);
        }
        if (object._maxWidth != null) {
          objectProto.maxWidth = object._maxWidth.pack(10);
        }
        if (object._maxHeight != null) {
          objectProto.maxHeight = object._maxHeight.pack(10);
        }
        if (object._isVisible != null) {
          objectProto.isVisible = object._isVisible;
        }
        if (object._opacity != null) {
          objectProto.opacity = object._opacity;
        }
        if (object._fill != null) {
          objectProto.fill = object._fill.pack(10);
        }
        if (object._shadow != null) {
          objectProto.shadow = object._shadow.pack(10);
        }
        if (object._border != null) {
          objectProto.border = object._border.pack(10);
        }
        if (object._radius != null) {
          objectProto.radius = object._radius.pack(10);
        }
        if (object._font != null) {
          objectProto.font = object._font.pack(10);
        }
        if (object._color != null) {
          objectProto.color = object._color.pack(10);
        }
        if (object._text != null) {
          objectProto.text = object._text.pack(10);
        }
        return objectProto as TextViewProto;
      }

      unpackObject(objectProto: TextViewProto, _session: Session | null): TextView {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Text = STRUCT_CLASS_BY_TYPE[400020] as typeof Text;
        const _Length = STRUCT_CLASS_BY_TYPE[1800001] as typeof Length;
        const _Fill = STRUCT_CLASS_BY_TYPE[2100400] as typeof Fill;
        const _Font = STRUCT_CLASS_BY_TYPE[2100500] as typeof Font;
        const _Border = STRUCT_CLASS_BY_TYPE[2100600] as typeof Border;
        const _Shadow = STRUCT_CLASS_BY_TYPE[2100700] as typeof Shadow;
        const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
        const _Offset2 = STRUCT_CLASS_BY_TYPE[2400020] as typeof Offset2;
        const _Corner2 = STRUCT_CLASS_BY_TYPE[2400024] as typeof Corner2;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[1805100] as typeof TextView)({
          text: objectProto.text != undefined ? _Text.unpack(10, objectProto.text, _session) as Text : null,
          font: objectProto.font != undefined ? _Font.unpack(10, objectProto.font, _session) as Font : null,
          color: objectProto.color != undefined ? _Fill.unpack(10, objectProto.color, _session) as Fill : null,
          width: objectProto.width != undefined ? _Length.unpack(10, objectProto.width, _session) as Length : null,
          height: objectProto.height != undefined ? _Length.unpack(10, objectProto.height, _session) as Length : null,
          minWidth: objectProto.minWidth != undefined ? _Length.unpack(10, objectProto.minWidth, _session) as Length : null,
          minHeight: objectProto.minHeight != undefined ? _Length.unpack(10, objectProto.minHeight, _session) as Length : null,
          maxWidth: objectProto.maxWidth != undefined ? _Length.unpack(10, objectProto.maxWidth, _session) as Length : null,
          maxHeight: objectProto.maxHeight != undefined ? _Length.unpack(10, objectProto.maxHeight, _session) as Length : null,
          isVisible: objectProto.isVisible != undefined ? objectProto.isVisible : null,
          opacity: objectProto.opacity != undefined ? objectProto.opacity : null,
          fill: objectProto.fill != undefined ? _Fill.unpack(10, objectProto.fill, _session) as Fill : null,
          shadow: objectProto.shadow != undefined ? _Shadow.unpack(10, objectProto.shadow, _session) as Shadow : null,
          border: objectProto.border != undefined ? _Border.unpack(10, objectProto.border, _session) as Border : null,
          radius: objectProto.radius != undefined ? _Corner2.unpack(10, objectProto.radius, _session) as Corner2 : null,
          position: objectProto.position != undefined ? _Vector2.unpack(10, objectProto.position, _session) as Vector2 : null,
          offset: objectProto.offset != undefined ? _Offset2.unpack(10, objectProto.offset, _session) as Offset2 : null,
          scale: objectProto.scale != undefined ? _Vector2.unpack(10, objectProto.scale, _session) as Vector2 : null,
          rotation: objectProto.rotation != undefined ? _Vector2.unpack(10, objectProto.rotation, _session) as Vector2 : null,
          skew: objectProto.skew != undefined ? _Vector2.unpack(10, objectProto.skew, _session) as Vector2 : null,
          origin: objectProto.origin != undefined ? _Vector2.unpack(10, objectProto.origin, _session) as Vector2 : null,
          anchor: objectProto.anchor != undefined ? Number(objectProto.anchor) as any : null,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: TextView): Uint8Array {
        const proto = this.packObject(object);
        return TextViewProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): TextView {
        const proto = TextViewProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1805100)] = new TextViewProtoEncoder();

    class SceneProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Scene): SceneProto {
        const objectProto: Partial<SceneProto> = { metatype: 1700200 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        if (object._icon != null) {
          objectProto.icon = object._icon.pack(10);
        }
        if (object._rootViewPtr != null) {
          objectProto.rootViewPtr = object._rootViewPtr.pack(10);
        }
        return objectProto as SceneProto;
      }

      unpackObject(objectProto: SceneProto, _session: Session | null): Scene {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[1700200] as typeof Scene)({
          rootView: objectProto.rootViewPtr != undefined ? _NodeReference.unpack(10, objectProto.rootViewPtr, _session) as NodeReference : null,
          icon: objectProto.icon != undefined ? _Icon.unpack(10, objectProto.icon, _session) as Icon : null,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: Scene): Uint8Array {
        const proto = this.packObject(object);
        return SceneProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Scene {
        const proto = SceneProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1700200)] = new SceneProtoEncoder();

    class StageProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Stage): StageProto {
        const objectProto: Partial<StageProto> = { metatype: 1700000 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        return objectProto as StageProto;
      }

      unpackObject(objectProto: StageProto, _session: Session | null): Stage {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[1700000] as typeof Stage)({
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: Stage): Uint8Array {
        const proto = this.packObject(object);
        return StageProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Stage {
        const proto = StageProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1700000)] = new StageProtoEncoder();

    class FollowProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Follow): FollowProto {
        const objectProto: Partial<FollowProto> = { metatype: 1400200 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        return objectProto as FollowProto;
      }

      unpackObject(objectProto: FollowProto, _session: Session | null): Follow {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[1400200] as typeof Follow)({
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: Follow): Uint8Array {
        const proto = this.packObject(object);
        return FollowProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Follow {
        const proto = FollowProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1400200)] = new FollowProtoEncoder();

    class FollowEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: FollowEvent): FollowEventProto {
        const objectProto: Partial<FollowEventProto> = { metatype: 1400201 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        objectProto.nodePtr = object.nodePtr.pack(10);
        return objectProto as FollowEventProto;
      }

      unpackObject(objectProto: FollowEventProto, _session: Session | null): FollowEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[1400201] as typeof FollowEvent)({
          node: _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: FollowEvent): Uint8Array {
        const proto = this.packObject(object);
        return FollowEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): FollowEvent {
        const proto = FollowEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1400201)] = new FollowEventProtoEncoder();

    class FollowAddedEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: FollowAddedEvent): FollowAddedEventProto {
        const objectProto: Partial<FollowAddedEventProto> = { metatype: 1400202 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        objectProto.nodePtr = object.nodePtr.pack(10);
        return objectProto as FollowAddedEventProto;
      }

      unpackObject(objectProto: FollowAddedEventProto, _session: Session | null): FollowAddedEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[1400202] as typeof FollowAddedEvent)({
          node: _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: FollowAddedEvent): Uint8Array {
        const proto = this.packObject(object);
        return FollowAddedEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): FollowAddedEvent {
        const proto = FollowAddedEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1400202)] = new FollowAddedEventProtoEncoder();

    class FollowRemovedEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: FollowRemovedEvent): FollowRemovedEventProto {
        const objectProto: Partial<FollowRemovedEventProto> = { metatype: 1400203 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        objectProto.nodePtr = object.nodePtr.pack(10);
        return objectProto as FollowRemovedEventProto;
      }

      unpackObject(objectProto: FollowRemovedEventProto, _session: Session | null): FollowRemovedEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[1400203] as typeof FollowRemovedEvent)({
          node: _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: FollowRemovedEvent): Uint8Array {
        const proto = this.packObject(object);
        return FollowRemovedEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): FollowRemovedEvent {
        const proto = FollowRemovedEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1400203)] = new FollowRemovedEventProtoEncoder();

    class NotificationSentEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: NotificationSentEvent): NotificationSentEventProto {
        const objectProto: Partial<NotificationSentEventProto> = { metatype: 1400502 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        objectProto.nodePtr = object.nodePtr.pack(10);
        return objectProto as NotificationSentEventProto;
      }

      unpackObject(objectProto: NotificationSentEventProto, _session: Session | null): NotificationSentEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[1400502] as typeof NotificationSentEvent)({
          node: _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: NotificationSentEvent): Uint8Array {
        const proto = this.packObject(object);
        return NotificationSentEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): NotificationSentEvent {
        const proto = NotificationSentEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1400502)] = new NotificationSentEventProtoEncoder();

    class NotificationRescindedEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: NotificationRescindedEvent): NotificationRescindedEventProto {
        const objectProto: Partial<NotificationRescindedEventProto> = { metatype: 1400503 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        objectProto.nodePtr = object.nodePtr.pack(10);
        return objectProto as NotificationRescindedEventProto;
      }

      unpackObject(objectProto: NotificationRescindedEventProto, _session: Session | null): NotificationRescindedEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[1400503] as typeof NotificationRescindedEvent)({
          node: _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: NotificationRescindedEvent): Uint8Array {
        const proto = this.packObject(object);
        return NotificationRescindedEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): NotificationRescindedEvent {
        const proto = NotificationRescindedEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1400503)] = new NotificationRescindedEventProtoEncoder();

    class NotificationReadEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: NotificationReadEvent): NotificationReadEventProto {
        const objectProto: Partial<NotificationReadEventProto> = { metatype: 1400504 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        objectProto.nodePtr = object.nodePtr.pack(10);
        return objectProto as NotificationReadEventProto;
      }

      unpackObject(objectProto: NotificationReadEventProto, _session: Session | null): NotificationReadEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[1400504] as typeof NotificationReadEvent)({
          node: _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: NotificationReadEvent): Uint8Array {
        const proto = this.packObject(object);
        return NotificationReadEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): NotificationReadEvent {
        const proto = NotificationReadEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1400504)] = new NotificationReadEventProtoEncoder();

    class NotificationDismissedEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: NotificationDismissedEvent): NotificationDismissedEventProto {
        const objectProto: Partial<NotificationDismissedEventProto> = { metatype: 1400505 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        objectProto.nodePtr = object.nodePtr.pack(10);
        return objectProto as NotificationDismissedEventProto;
      }

      unpackObject(objectProto: NotificationDismissedEventProto, _session: Session | null): NotificationDismissedEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[1400505] as typeof NotificationDismissedEvent)({
          node: _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: NotificationDismissedEvent): Uint8Array {
        const proto = this.packObject(object);
        return NotificationDismissedEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): NotificationDismissedEvent {
        const proto = NotificationDismissedEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1400505)] = new NotificationDismissedEventProtoEncoder();

    class NotificationExpiredEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: NotificationExpiredEvent): NotificationExpiredEventProto {
        const objectProto: Partial<NotificationExpiredEventProto> = { metatype: 1400506 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        objectProto.nodePtr = object.nodePtr.pack(10);
        return objectProto as NotificationExpiredEventProto;
      }

      unpackObject(objectProto: NotificationExpiredEventProto, _session: Session | null): NotificationExpiredEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[1400506] as typeof NotificationExpiredEvent)({
          node: _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: NotificationExpiredEvent): Uint8Array {
        const proto = this.packObject(object);
        return NotificationExpiredEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): NotificationExpiredEvent {
        const proto = NotificationExpiredEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1400506)] = new NotificationExpiredEventProtoEncoder();

    class NotificationProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Notification): NotificationProto {
        const objectProto: Partial<NotificationProto> = { metatype: 1400500 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        objectProto.title = object._title;
        objectProto.status = Number(object._status) as any;
        if (object._text != null) {
          objectProto.text = object._text.pack(10);
        }
        return objectProto as NotificationProto;
      }

      unpackObject(objectProto: NotificationProto, _session: Session | null): Notification {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Text = STRUCT_CLASS_BY_TYPE[400020] as typeof Text;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[1400500] as typeof Notification)({
          title: objectProto.title,
          status: Number(objectProto.status) as any,
          text: objectProto.text != undefined ? _Text.unpack(10, objectProto.text, _session) as Text : null,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: Notification): Uint8Array {
        const proto = this.packObject(object);
        return NotificationProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Notification {
        const proto = NotificationProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1400500)] = new NotificationProtoEncoder();

    class ReactionProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Reaction): ReactionProto {
        const objectProto: Partial<ReactionProto> = { metatype: 1400000 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        objectProto.content = object._content;
        return objectProto as ReactionProto;
      }

      unpackObject(objectProto: ReactionProto, _session: Session | null): Reaction {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[1400000] as typeof Reaction)({
          content: objectProto.content,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: Reaction): Uint8Array {
        const proto = this.packObject(object);
        return ReactionProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Reaction {
        const proto = ReactionProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1400000)] = new ReactionProtoEncoder();

    class ReactionEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: ReactionEvent): ReactionEventProto {
        const objectProto: Partial<ReactionEventProto> = { metatype: 1400001 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        objectProto.nodePtr = object.nodePtr.pack(10);
        objectProto.content = object.content;
        return objectProto as ReactionEventProto;
      }

      unpackObject(objectProto: ReactionEventProto, _session: Session | null): ReactionEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[1400001] as typeof ReactionEvent)({
          node: _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference,
          content: objectProto.content,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: ReactionEvent): Uint8Array {
        const proto = this.packObject(object);
        return ReactionEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): ReactionEvent {
        const proto = ReactionEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1400001)] = new ReactionEventProtoEncoder();

    class ReactionAddedEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: ReactionAddedEvent): ReactionAddedEventProto {
        const objectProto: Partial<ReactionAddedEventProto> = { metatype: 1400002 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        objectProto.nodePtr = object.nodePtr.pack(10);
        objectProto.content = object.content;
        return objectProto as ReactionAddedEventProto;
      }

      unpackObject(objectProto: ReactionAddedEventProto, _session: Session | null): ReactionAddedEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[1400002] as typeof ReactionAddedEvent)({
          node: _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference,
          content: objectProto.content,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: ReactionAddedEvent): Uint8Array {
        const proto = this.packObject(object);
        return ReactionAddedEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): ReactionAddedEvent {
        const proto = ReactionAddedEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1400002)] = new ReactionAddedEventProtoEncoder();

    class ReactionRemovedEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: ReactionRemovedEvent): ReactionRemovedEventProto {
        const objectProto: Partial<ReactionRemovedEventProto> = { metatype: 1400003 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        objectProto.nodePtr = object.nodePtr.pack(10);
        objectProto.content = object.content;
        return objectProto as ReactionRemovedEventProto;
      }

      unpackObject(objectProto: ReactionRemovedEventProto, _session: Session | null): ReactionRemovedEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[1400003] as typeof ReactionRemovedEvent)({
          node: _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference,
          content: objectProto.content,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: ReactionRemovedEvent): Uint8Array {
        const proto = this.packObject(object);
        return ReactionRemovedEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): ReactionRemovedEvent {
        const proto = ReactionRemovedEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1400003)] = new ReactionRemovedEventProtoEncoder();

    class StarProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Star): StarProto {
        const objectProto: Partial<StarProto> = { metatype: 1400100 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        return objectProto as StarProto;
      }

      unpackObject(objectProto: StarProto, _session: Session | null): Star {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[1400100] as typeof Star)({
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: Star): Uint8Array {
        const proto = this.packObject(object);
        return StarProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Star {
        const proto = StarProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1400100)] = new StarProtoEncoder();

    class StarEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: StarEvent): StarEventProto {
        const objectProto: Partial<StarEventProto> = { metatype: 1400101 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        objectProto.nodePtr = object.nodePtr.pack(10);
        return objectProto as StarEventProto;
      }

      unpackObject(objectProto: StarEventProto, _session: Session | null): StarEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[1400101] as typeof StarEvent)({
          node: _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: StarEvent): Uint8Array {
        const proto = this.packObject(object);
        return StarEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): StarEvent {
        const proto = StarEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1400101)] = new StarEventProtoEncoder();

    class StarAddedEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: StarAddedEvent): StarAddedEventProto {
        const objectProto: Partial<StarAddedEventProto> = { metatype: 1400102 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        objectProto.nodePtr = object.nodePtr.pack(10);
        return objectProto as StarAddedEventProto;
      }

      unpackObject(objectProto: StarAddedEventProto, _session: Session | null): StarAddedEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[1400102] as typeof StarAddedEvent)({
          node: _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: StarAddedEvent): Uint8Array {
        const proto = this.packObject(object);
        return StarAddedEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): StarAddedEvent {
        const proto = StarAddedEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1400102)] = new StarAddedEventProtoEncoder();

    class StarRemovedEventProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: StarRemovedEvent): StarRemovedEventProto {
        const objectProto: Partial<StarRemovedEventProto> = { metatype: 1400103 };
        objectProto.id = String(object.id);
        objectProto.spacePtr = object.spacePtr.pack(10);
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.causedByPtr != null) {
          objectProto.causedByPtr = object.causedByPtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.clientPtr = object.clientPtr.pack(10);
        objectProto.clientNonce = String(object.clientNonce);
        objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
        objectProto.clientEpoch = object.clientEpoch;
        objectProto.status = Number(object.status) as any;
        objectProto.nodePtr = object.nodePtr.pack(10);
        return objectProto as StarRemovedEventProto;
      }

      unpackObject(objectProto: StarRemovedEventProto, _session: Session | null): StarRemovedEvent {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (NODE_CLASS_BY_TYPE[1400103] as typeof StarRemovedEvent)({
          node: _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          causedBy: objectProto.causedByPtr != undefined ? _NodeReference.unpack(10, objectProto.causedByPtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          client: _NodeReference.unpack(10, objectProto.clientPtr, _session) as NodeReference,
          clientNonce: String(objectProto.clientNonce),
          clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
          clientEpoch: Number(objectProto.clientEpoch),
          status: Number(objectProto.status) as any,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: StarRemovedEvent): Uint8Array {
        const proto = this.packObject(object);
        return StarRemovedEventProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): StarRemovedEvent {
        const proto = StarRemovedEventProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 1400103)] = new StarRemovedEventProtoEncoder();

    class FolderProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Folder): FolderProto {
        const objectProto: Partial<FolderProto> = { metatype: 240000 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        objectProto.type = Number(object._type) as any;
        if (object._icon != null) {
          objectProto.icon = object._icon.pack(10);
        }
        if (object._slug != null) {
          objectProto.slug = object._slug;
        }
        if (object._mainScenePtr != null) {
          objectProto.mainScenePtr = object._mainScenePtr.pack(10);
        }
        return objectProto as FolderProto;
      }

      unpackObject(objectProto: FolderProto, _session: Session | null): Folder {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[240000] as typeof Folder)({
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          type: Number(objectProto.type) as any,
          icon: objectProto.icon != undefined ? _Icon.unpack(10, objectProto.icon, _session) as Icon : null,
          slug: objectProto.slug != undefined ? objectProto.slug : null,
          mainScene: objectProto.mainScenePtr != undefined ? _NodeReference.unpack(10, objectProto.mainScenePtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: Folder): Uint8Array {
        const proto = this.packObject(object);
        return FolderProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Folder {
        const proto = FolderProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 240000)] = new FolderProtoEncoder();

    class ClientProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Client): ClientProto {
        const objectProto: Partial<ClientProto> = { metatype: 121300 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._browserVersion != null) {
          objectProto.browserVersion = object._browserVersion;
        }
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        objectProto.type = Number(object._type) as any;
        if (object._machinePtr != null) {
          objectProto.machinePtr = object._machinePtr.pack(10);
        }
        if (object._userPtr != null) {
          objectProto.userPtr = object._userPtr.pack(10);
        }
        if (object._accessToken != null) {
          objectProto.accessToken = object._accessToken;
        }
        if (object._seenAt != null) {
          objectProto.seenAt = packProtoTimestamp(object._seenAt);
        }
        if (object._loggedInAt != null) {
          objectProto.loggedInAt = packProtoTimestamp(object._loggedInAt);
        }
        if (object._deviceType != null) {
          objectProto.deviceType = object._deviceType;
        }
        if (object._deviceName != null) {
          objectProto.deviceName = object._deviceName;
        }
        if (object._operatingSystem != null) {
          objectProto.operatingSystem = object._operatingSystem;
        }
        if (object._browserName != null) {
          objectProto.browserName = object._browserName;
        }
        return objectProto as ClientProto;
      }

      unpackObject(objectProto: ClientProto, _session: Session | null): Client {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[121300] as typeof Client)({
          type: Number(objectProto.type) as any,
          machine: objectProto.machinePtr != undefined ? _NodeReference.unpack(10, objectProto.machinePtr, _session) as NodeReference : null,
          user: objectProto.userPtr != undefined ? _NodeReference.unpack(10, objectProto.userPtr, _session) as NodeReference : null,
          accessToken: objectProto.accessToken != undefined ? objectProto.accessToken : null,
          seenAt: objectProto.seenAt != undefined ? unpackProtoTimestamp(objectProto.seenAt!) : null,
          loggedInAt: objectProto.loggedInAt != undefined ? unpackProtoTimestamp(objectProto.loggedInAt!) : null,
          deviceType: objectProto.deviceType != undefined ? objectProto.deviceType : null,
          deviceName: objectProto.deviceName != undefined ? objectProto.deviceName : null,
          operatingSystem: objectProto.operatingSystem != undefined ? objectProto.operatingSystem : null,
          browserName: objectProto.browserName != undefined ? objectProto.browserName : null,
          browserVersion: objectProto.browserVersion != undefined ? objectProto.browserVersion : null,
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: Client): Uint8Array {
        const proto = this.packObject(object);
        return ClientProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Client {
        const proto = ClientProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 121300)] = new ClientProtoEncoder();

    class HandleProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Handle): HandleProto {
        const objectProto: Partial<HandleProto> = { metatype: 100200 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        objectProto.slug = object._slug;
        return objectProto as HandleProto;
      }

      unpackObject(objectProto: HandleProto, _session: Session | null): Handle {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[100200] as typeof Handle)({
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          slug: objectProto.slug,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: Handle): Uint8Array {
        const proto = this.packObject(object);
        return HandleProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Handle {
        const proto = HandleProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 100200)] = new HandleProtoEncoder();

    class OrganizationProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Organization): OrganizationProto {
        const objectProto: Partial<OrganizationProto> = { metatype: 122000 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        objectProto.slug = object._slug;
        objectProto.status = Number(object._status) as any;
        if (object._handlePtr != null) {
          objectProto.handlePtr = object._handlePtr.pack(10);
        }
        return objectProto as OrganizationProto;
      }

      unpackObject(objectProto: OrganizationProto, _session: Session | null): Organization {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[122000] as typeof Organization)({
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          slug: objectProto.slug,
          status: Number(objectProto.status) as any,
          handle: objectProto.handlePtr != undefined ? _NodeReference.unpack(10, objectProto.handlePtr, _session) as NodeReference : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: Organization): Uint8Array {
        const proto = this.packObject(object);
        return OrganizationProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Organization {
        const proto = OrganizationProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 122000)] = new OrganizationProtoEncoder();

    class TeamProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Team): TeamProto {
        const objectProto: Partial<TeamProto> = { metatype: 122100 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        objectProto.slug = object._slug;
        return objectProto as TeamProto;
      }

      unpackObject(objectProto: TeamProto, _session: Session | null): Team {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[122100] as typeof Team)({
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          slug: objectProto.slug,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: Team): Uint8Array {
        const proto = this.packObject(object);
        return TeamProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Team {
        const proto = TeamProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 122100)] = new TeamProtoEncoder();

    class UserProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: User): UserProto {
        const objectProto: Partial<UserProto> = { metatype: 121000 };
        objectProto.id = String(object.id);
        if (object.parentPtr != null) {
          objectProto.parentPtr = object.parentPtr.pack(10);
        }
        objectProto.spacePtr = object.spacePtr.pack(10);
        objectProto.materialization = Number(object.materialization) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        objectProto.branchPtr = object.branchPtr.pack(10);
        objectProto.snapshotPtr = object.snapshotPtr.pack(10);
        if (object.precededByPtr != null) {
          objectProto.precededByPtr = object.precededByPtr.pack(10);
        }
        if (object.instancePtr != null) {
          objectProto.instancePtr = object.instancePtr.pack(10);
        }
        objectProto.createdAt = packProtoTimestamp(object.createdAt);
        objectProto.createdEpoch = object.createdEpoch;
        objectProto.createdByPtr = object.createdByPtr.pack(10);
        objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
        objectProto.updatedEpoch = object.updatedEpoch;
        objectProto.updatedByPtr = object.updatedByPtr.pack(10);
        if (object.deletedAt != null) {
          objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
        }
        if (object._ownedByPtr != null) {
          objectProto.ownedByPtr = object._ownedByPtr.pack(10);
        }
        objectProto.name = object._name;
        objectProto.orderKey = object.orderKey;
        if (object._customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object._customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        if (object._scriptPtr != null) {
          objectProto.scriptPtr = object._scriptPtr.pack(10);
        }
        if (object.isExtensible != null) {
          objectProto.isExtensible = object.isExtensible;
        }
        if (object.sourcePtr != null) {
          objectProto.sourcePtr = object.sourcePtr.pack(10);
        }
        if (object._key != null) {
          objectProto.key = object._key;
        }
        objectProto.slug = object._slug;
        objectProto.status = Number(object._status) as any;
        if (object._lastLoggedInAt != null) {
          objectProto.lastLoggedInAt = packProtoTimestamp(object._lastLoggedInAt);
        }
        objectProto.isStaff = object._isStaff;
        if (object._handlePtr != null) {
          objectProto.handlePtr = object._handlePtr.pack(10);
        }
        if (object._email != null) {
          objectProto.email = object._email;
        }
        if (object._passwordSalt != null) {
          objectProto.passwordSalt = object._passwordSalt;
        }
        if (object._passwordHash != null) {
          objectProto.passwordHash = object._passwordHash;
        }
        return objectProto as UserProto;
      }

      unpackObject(objectProto: UserProto, _session: Session | null): User {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (NODE_CLASS_BY_TYPE[121000] as typeof User)({
          parent: objectProto.parentPtr != undefined ? _NodeReference.unpack(10, objectProto.parentPtr, _session) as NodeReference : null,
          slug: objectProto.slug,
          status: Number(objectProto.status) as any,
          lastLoggedInAt: objectProto.lastLoggedInAt != undefined ? unpackProtoTimestamp(objectProto.lastLoggedInAt!) : null,
          isStaff: objectProto.isStaff,
          handle: objectProto.handlePtr != undefined ? _NodeReference.unpack(10, objectProto.handlePtr, _session) as NodeReference : null,
          email: objectProto.email != undefined ? objectProto.email : null,
          passwordSalt: objectProto.passwordSalt != undefined ? objectProto.passwordSalt : null,
          passwordHash: objectProto.passwordHash != undefined ? objectProto.passwordHash : null,
          materialization: Number(objectProto.materialization) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          branch: _NodeReference.unpack(10, objectProto.branchPtr, _session) as NodeReference,
          snapshot: _NodeReference.unpack(10, objectProto.snapshotPtr, _session) as NodeReference,
          precededBy: objectProto.precededByPtr != undefined ? _NodeReference.unpack(10, objectProto.precededByPtr, _session) as NodeReference : null,
          instance: objectProto.instancePtr != undefined ? _NodeReference.unpack(10, objectProto.instancePtr, _session) as NodeReference : null,
          createdAt: unpackProtoTimestamp(objectProto.createdAt!),
          createdEpoch: Number(objectProto.createdEpoch),
          createdBy: _NodeReference.unpack(10, objectProto.createdByPtr, _session) as NodeReference,
          updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
          updatedEpoch: Number(objectProto.updatedEpoch),
          updatedBy: _NodeReference.unpack(10, objectProto.updatedByPtr, _session) as NodeReference,
          deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
          ownedBy: objectProto.ownedByPtr != undefined ? _NodeReference.unpack(10, objectProto.ownedByPtr, _session) as NodeReference : null,
          name: objectProto.name,
          orderKey: objectProto.orderKey,
          customValues: unpackedCustomValues,
          script: objectProto.scriptPtr != undefined ? _NodeReference.unpack(10, objectProto.scriptPtr, _session) as NodeReference : null,
          isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
          source: objectProto.sourcePtr != undefined ? _NodeReference.unpack(10, objectProto.sourcePtr, _session) as NodeReference : null,
          key: objectProto.key != undefined ? objectProto.key : null,
          id: String(objectProto.id),
          space: _NodeReference.unpack(10, objectProto.spacePtr, _session) as NodeReference,
          _session,
        });
      }

      packObjectBytes(object: User): Uint8Array {
        const proto = this.packObject(object);
        return UserProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): User {
        const proto = UserProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(1, 121000)] = new UserProtoEncoder();

    class NodeDefinitionReferenceProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: NodeDefinitionReference): NodeDefinitionReferenceProto {
        const objectProto: Partial<NodeDefinitionReferenceProto> = { metatype: 13 };
        objectProto.type = Number(object.type) as any;
        objectProto.nodeType = Number(object.nodeType) as any;
        if (object.definitionPtr != null) {
          objectProto.definitionPtr = object.definitionPtr.pack(10);
        }
        return objectProto as NodeDefinitionReferenceProto;
      }

      unpackObject(objectProto: NodeDefinitionReferenceProto, _session: Session | null): NodeDefinitionReference {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (STRUCT_CLASS_BY_TYPE[13] as typeof NodeDefinitionReference)({
          type: Number(objectProto.type) as any,
          nodeType: Number(objectProto.nodeType) as any,
          definition: objectProto.definitionPtr != undefined ? _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference : null,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: NodeDefinitionReference): Uint8Array {
        const proto = this.packObject(object);
        return NodeDefinitionReferenceProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): NodeDefinitionReference {
        const proto = NodeDefinitionReferenceProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 13)] = new NodeDefinitionReferenceProtoEncoder();

    class ObjectDefinitionReferenceProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: ObjectDefinitionReference): ObjectDefinitionReferenceProto {
        const objectProto: Partial<ObjectDefinitionReferenceProto> = { metatype: 11 };
        objectProto.type = Number(object.type) as any;
        if (object.nodeType != null) {
          objectProto.nodeType = Number(object.nodeType) as any;
        }
        if (object.traitType != null) {
          objectProto.traitType = Number(object.traitType) as any;
        }
        if (object.structType != null) {
          objectProto.structType = Number(object.structType) as any;
        }
        if (object.customDefinitionPtr != null) {
          objectProto.customDefinitionPtr = object.customDefinitionPtr.pack(10);
        }
        return objectProto as ObjectDefinitionReferenceProto;
      }

      unpackObject(objectProto: ObjectDefinitionReferenceProto, _session: Session | null): ObjectDefinitionReference {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (STRUCT_CLASS_BY_TYPE[11] as typeof ObjectDefinitionReference)({
          type: Number(objectProto.type) as any,
          nodeType: objectProto.nodeType != undefined ? Number(objectProto.nodeType) as any : null,
          traitType: objectProto.traitType != undefined ? Number(objectProto.traitType) as any : null,
          structType: objectProto.structType != undefined ? Number(objectProto.structType) as any : null,
          customDefinition: objectProto.customDefinitionPtr != undefined ? _NodeReference.unpack(10, objectProto.customDefinitionPtr, _session) as NodeReference : null,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: ObjectDefinitionReference): Uint8Array {
        const proto = this.packObject(object);
        return ObjectDefinitionReferenceProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): ObjectDefinitionReference {
        const proto = ObjectDefinitionReferenceProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 11)] = new ObjectDefinitionReferenceProtoEncoder();

    class StructDefinitionReferenceProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: StructDefinitionReference): StructDefinitionReferenceProto {
        const objectProto: Partial<StructDefinitionReferenceProto> = { metatype: 16 };
        objectProto.type = Number(object.type) as any;
        if (object.structType != null) {
          objectProto.structType = Number(object.structType) as any;
        }
        objectProto.definitionPtr = object.definitionPtr.pack(10);
        return objectProto as StructDefinitionReferenceProto;
      }

      unpackObject(objectProto: StructDefinitionReferenceProto, _session: Session | null): StructDefinitionReference {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (STRUCT_CLASS_BY_TYPE[16] as typeof StructDefinitionReference)({
          type: Number(objectProto.type) as any,
          structType: objectProto.structType != undefined ? Number(objectProto.structType) as any : null,
          definition: _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: StructDefinitionReference): Uint8Array {
        const proto = this.packObject(object);
        return StructDefinitionReferenceProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): StructDefinitionReference {
        const proto = StructDefinitionReferenceProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 16)] = new StructDefinitionReferenceProtoEncoder();

    class PropertyReferenceProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: PropertyReference): PropertyReferenceProto {
        const objectProto: Partial<PropertyReferenceProto> = { metatype: 1001 };
        objectProto.type = Number(object.type) as any;
        if (object.nodeType != null) {
          objectProto.nodeType = Number(object.nodeType) as any;
        }
        if (object.traitType != null) {
          objectProto.traitType = Number(object.traitType) as any;
        }
        if (object.structType != null) {
          objectProto.structType = Number(object.structType) as any;
        }
        if (object.id != null) {
          objectProto.id = object.id;
        }
        if (object.customPropertyPtr != null) {
          objectProto.customPropertyPtr = object.customPropertyPtr.pack(10);
        }
        return objectProto as PropertyReferenceProto;
      }

      unpackObject(objectProto: PropertyReferenceProto, _session: Session | null): PropertyReference {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (STRUCT_CLASS_BY_TYPE[1001] as typeof PropertyReference)({
          type: Number(objectProto.type) as any,
          nodeType: objectProto.nodeType != undefined ? Number(objectProto.nodeType) as any : null,
          traitType: objectProto.traitType != undefined ? Number(objectProto.traitType) as any : null,
          structType: objectProto.structType != undefined ? Number(objectProto.structType) as any : null,
          id: objectProto.id != undefined ? Number(objectProto.id) : null,
          customProperty: objectProto.customPropertyPtr != undefined ? _NodeReference.unpack(10, objectProto.customPropertyPtr, _session) as NodeReference : null,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: PropertyReference): Uint8Array {
        const proto = this.packObject(object);
        return PropertyReferenceProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): PropertyReference {
        const proto = PropertyReferenceProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 1001)] = new PropertyReferenceProtoEncoder();

    class NodeReferenceProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: NodeReference): NodeReferenceProto {
        const objectProto: Partial<NodeReferenceProto> = { metatype: 1000 };
        objectProto.type = Number(object.type) as any;
        objectProto.id = String(object.id);
        objectProto.spaceId = String(object.spaceId);
        if (object.definitionId != null) {
          objectProto.definitionId = String(object.definitionId);
        }
        objectProto.branchId = String(object.branchId);
        objectProto.snapshotId = String(object.snapshotId);
        if (object.storeKey != null) {
          objectProto.storeKey = Number(object.storeKey) as any;
        }
        return objectProto as NodeReferenceProto;
      }

      unpackObject(objectProto: NodeReferenceProto, _session: Session | null): NodeReference {

        return new (STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference)({
          type: Number(objectProto.type) as any,
          id: String(objectProto.id),
          spaceId: String(objectProto.spaceId),
          definitionId: objectProto.definitionId != undefined ? String(objectProto.definitionId) : null,
          branchId: String(objectProto.branchId),
          snapshotId: String(objectProto.snapshotId),
          storeKey: objectProto.storeKey != undefined ? Number(objectProto.storeKey) as any : null,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: NodeReference): Uint8Array {
        const proto = this.packObject(object);
        return NodeReferenceProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): NodeReference {
        const proto = NodeReferenceProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 1000)] = new NodeReferenceProtoEncoder();

    class NodeDefinitionProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: NodeDefinition): NodeDefinitionProto {
        const objectProto: Partial<NodeDefinitionProto> = { metatype: 12 };
        objectProto.id = object.id;
        objectProto.type = Number(object.type) as any;
        objectProto.name = object.name;
        if (object.icon != null) {
          objectProto.icon = object.icon.pack(10);
        }
        if (object.description != null) {
          objectProto.description = object.description;
        }
        if (object.taggings) {
          const packedTaggings: any[] = [];
          for (const item of object.taggings) {
            packedTaggings.push(item);
          }
          objectProto.taggings = packedTaggings;
        }
        objectProto.isAbstract = object.isAbstract;
        objectProto.isExtensible = object.isExtensible;
        objectProto.isFinal = object.isFinal;
        objectProto.isFrozen = object.isFrozen;
        if (object.properties) {
          const packedProperties: any[] = [];
          for (const item of object.properties) {
            packedProperties.push(item.pack(10));
          }
          objectProto.properties = packedProperties;
        }
        if (object.indexes) {
          const packedIndexes: any[] = [];
          for (const item of object.indexes) {
            packedIndexes.push(item.pack(10));
          }
          objectProto.indexes = packedIndexes;
        }
        if (object.constraints) {
          const packedConstraints: any[] = [];
          for (const item of object.constraints) {
            packedConstraints.push(item.pack(10));
          }
          objectProto.constraints = packedConstraints;
        }
        if (object.permissions) {
          const packedPermissions: any[] = [];
          for (const item of object.permissions) {
            packedPermissions.push(item.pack(10));
          }
          objectProto.permissions = packedPermissions;
        }
        if (object.methods) {
          const packedMethods: any[] = [];
          for (const item of object.methods) {
            packedMethods.push(item.pack(10));
          }
          objectProto.methods = packedMethods;
        }
        if (object.actions) {
          const packedActions: any[] = [];
          for (const item of object.actions) {
            packedActions.push(item.pack(10));
          }
          objectProto.actions = packedActions;
        }
        if (object.constants) {
          const packedConstants: any[] = [];
          for (const item of object.constants) {
            packedConstants.push(item.pack(10));
          }
          objectProto.constants = packedConstants;
        }
        if (object.baseType != null) {
          objectProto.baseType = Number(object.baseType) as any;
        }
        if (object.extendedBy) {
          const packedExtendedBy: any[] = [];
          for (const item of object.extendedBy) {
            packedExtendedBy.push(Number(item) as any);
          }
          objectProto.extendedBy = packedExtendedBy;
        }
        if (object.inherits) {
          const packedInherits: any[] = [];
          for (const item of object.inherits) {
            packedInherits.push(Number(item) as any);
          }
          objectProto.inherits = packedInherits;
        }
        if (object.inheritedBy) {
          const packedInheritedBy: any[] = [];
          for (const item of object.inheritedBy) {
            packedInheritedBy.push(Number(item) as any);
          }
          objectProto.inheritedBy = packedInheritedBy;
        }
        if (object.traits) {
          const packedTraits: any[] = [];
          for (const item of object.traits) {
            packedTraits.push(Number(item) as any);
          }
          objectProto.traits = packedTraits;
        }
        if (object.selfTraits) {
          const packedSelfTraits: any[] = [];
          for (const item of object.selfTraits) {
            packedSelfTraits.push(Number(item) as any);
          }
          objectProto.selfTraits = packedSelfTraits;
        }
        if (object.eventTypes) {
          const packedEventTypes: any[] = [];
          for (const item of object.eventTypes) {
            packedEventTypes.push(Number(item) as any);
          }
          objectProto.eventTypes = packedEventTypes;
        }
        if (object.selfEventTypes) {
          const packedSelfEventTypes: any[] = [];
          for (const item of object.selfEventTypes) {
            packedSelfEventTypes.push(Number(item) as any);
          }
          objectProto.selfEventTypes = packedSelfEventTypes;
        }
        if (object.enumTypes) {
          const packedEnumTypes: any[] = [];
          for (const item of object.enumTypes) {
            packedEnumTypes.push(Number(item) as any);
          }
          objectProto.enumTypes = packedEnumTypes;
        }
        if (object.selfEnumTypes) {
          const packedSelfEnumTypes: any[] = [];
          for (const item of object.selfEnumTypes) {
            packedSelfEnumTypes.push(Number(item) as any);
          }
          objectProto.selfEnumTypes = packedSelfEnumTypes;
        }
        if (object.parentTypes) {
          const packedParentTypes: any[] = [];
          for (const item of object.parentTypes) {
            packedParentTypes.push(Number(item) as any);
          }
          objectProto.parentTypes = packedParentTypes;
        }
        if (object.childTypes) {
          const packedChildTypes: any[] = [];
          for (const item of object.childTypes) {
            packedChildTypes.push(Number(item) as any);
          }
          objectProto.childTypes = packedChildTypes;
        }
        if (object.ancestorTypes) {
          const packedAncestorTypes: any[] = [];
          for (const item of object.ancestorTypes) {
            packedAncestorTypes.push(Number(item) as any);
          }
          objectProto.ancestorTypes = packedAncestorTypes;
        }
        if (object.descendantTypes) {
          const packedDescendantTypes: any[] = [];
          for (const item of object.descendantTypes) {
            packedDescendantTypes.push(Number(item) as any);
          }
          objectProto.descendantTypes = packedDescendantTypes;
        }
        if (object.expectedParentTypes) {
          const packedExpectedParentTypes: any[] = [];
          for (const item of object.expectedParentTypes) {
            packedExpectedParentTypes.push(Number(item) as any);
          }
          objectProto.expectedParentTypes = packedExpectedParentTypes;
        }
        if (object.expectedChildTypes) {
          const packedExpectedChildTypes: any[] = [];
          for (const item of object.expectedChildTypes) {
            packedExpectedChildTypes.push(Number(item) as any);
          }
          objectProto.expectedChildTypes = packedExpectedChildTypes;
        }
        if (object.expectedAncestorTypes) {
          const packedExpectedAncestorTypes: any[] = [];
          for (const item of object.expectedAncestorTypes) {
            packedExpectedAncestorTypes.push(Number(item) as any);
          }
          objectProto.expectedAncestorTypes = packedExpectedAncestorTypes;
        }
        if (object.expectedDescendantTypes) {
          const packedExpectedDescendantTypes: any[] = [];
          for (const item of object.expectedDescendantTypes) {
            packedExpectedDescendantTypes.push(Number(item) as any);
          }
          objectProto.expectedDescendantTypes = packedExpectedDescendantTypes;
        }
        if (object.domain != null) {
          objectProto.domain = Number(object.domain) as any;
        }
        return objectProto as NodeDefinitionProto;
      }

      unpackObject(objectProto: NodeDefinitionProto, _session: Session | null): NodeDefinition {
        const _PropertyDefinition = STRUCT_CLASS_BY_TYPE[18] as typeof PropertyDefinition;
        const _ConstantDefinition = STRUCT_CLASS_BY_TYPE[19] as typeof ConstantDefinition;
        const _IndexDefinition = STRUCT_CLASS_BY_TYPE[30100] as typeof IndexDefinition;
        const _ConstraintDefinition = STRUCT_CLASS_BY_TYPE[30200] as typeof ConstraintDefinition;
        const _MethodDefinition = STRUCT_CLASS_BY_TYPE[40000] as typeof MethodDefinition;
        const _ActionDefinition = STRUCT_CLASS_BY_TYPE[40100] as typeof ActionDefinition;
        const _PermissionDefinition = STRUCT_CLASS_BY_TYPE[50000] as typeof PermissionDefinition;
        const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
        const unpackedProperties: any[] = [];
        if (objectProto.properties) {
          for (const item of objectProto.properties) {
            unpackedProperties.push(_PropertyDefinition.unpack(10, item, _session) as PropertyDefinition);
          }
        }
        const unpackedIndexes: any[] = [];
        if (objectProto.indexes) {
          for (const item of objectProto.indexes) {
            unpackedIndexes.push(_IndexDefinition.unpack(10, item, _session) as IndexDefinition);
          }
        }
        const unpackedConstraints: any[] = [];
        if (objectProto.constraints) {
          for (const item of objectProto.constraints) {
            unpackedConstraints.push(_ConstraintDefinition.unpack(10, item, _session) as ConstraintDefinition);
          }
        }
        const unpackedPermissions: any[] = [];
        if (objectProto.permissions) {
          for (const item of objectProto.permissions) {
            unpackedPermissions.push(_PermissionDefinition.unpack(10, item, _session) as PermissionDefinition);
          }
        }
        const unpackedMethods: any[] = [];
        if (objectProto.methods) {
          for (const item of objectProto.methods) {
            unpackedMethods.push(_MethodDefinition.unpack(10, item, _session) as MethodDefinition);
          }
        }
        const unpackedActions: any[] = [];
        if (objectProto.actions) {
          for (const item of objectProto.actions) {
            unpackedActions.push(_ActionDefinition.unpack(10, item, _session) as ActionDefinition);
          }
        }
        const unpackedConstants: any[] = [];
        if (objectProto.constants) {
          for (const item of objectProto.constants) {
            unpackedConstants.push(_ConstantDefinition.unpack(10, item, _session) as ConstantDefinition);
          }
        }
        const unpackedExtendedBy: any[] = [];
        if (objectProto.extendedBy) {
          for (const item of objectProto.extendedBy) {
            unpackedExtendedBy.push(Number(item) as any);
          }
        }
        const unpackedInherits: any[] = [];
        if (objectProto.inherits) {
          for (const item of objectProto.inherits) {
            unpackedInherits.push(Number(item) as any);
          }
        }
        const unpackedInheritedBy: any[] = [];
        if (objectProto.inheritedBy) {
          for (const item of objectProto.inheritedBy) {
            unpackedInheritedBy.push(Number(item) as any);
          }
        }
        const unpackedTraits: any[] = [];
        if (objectProto.traits) {
          for (const item of objectProto.traits) {
            unpackedTraits.push(Number(item) as any);
          }
        }
        const unpackedSelfTraits: any[] = [];
        if (objectProto.selfTraits) {
          for (const item of objectProto.selfTraits) {
            unpackedSelfTraits.push(Number(item) as any);
          }
        }
        const unpackedEventTypes: any[] = [];
        if (objectProto.eventTypes) {
          for (const item of objectProto.eventTypes) {
            unpackedEventTypes.push(Number(item) as any);
          }
        }
        const unpackedSelfEventTypes: any[] = [];
        if (objectProto.selfEventTypes) {
          for (const item of objectProto.selfEventTypes) {
            unpackedSelfEventTypes.push(Number(item) as any);
          }
        }
        const unpackedEnumTypes: any[] = [];
        if (objectProto.enumTypes) {
          for (const item of objectProto.enumTypes) {
            unpackedEnumTypes.push(Number(item) as any);
          }
        }
        const unpackedSelfEnumTypes: any[] = [];
        if (objectProto.selfEnumTypes) {
          for (const item of objectProto.selfEnumTypes) {
            unpackedSelfEnumTypes.push(Number(item) as any);
          }
        }
        const unpackedParentTypes: any[] = [];
        if (objectProto.parentTypes) {
          for (const item of objectProto.parentTypes) {
            unpackedParentTypes.push(Number(item) as any);
          }
        }
        const unpackedChildTypes: any[] = [];
        if (objectProto.childTypes) {
          for (const item of objectProto.childTypes) {
            unpackedChildTypes.push(Number(item) as any);
          }
        }
        const unpackedAncestorTypes: any[] = [];
        if (objectProto.ancestorTypes) {
          for (const item of objectProto.ancestorTypes) {
            unpackedAncestorTypes.push(Number(item) as any);
          }
        }
        const unpackedDescendantTypes: any[] = [];
        if (objectProto.descendantTypes) {
          for (const item of objectProto.descendantTypes) {
            unpackedDescendantTypes.push(Number(item) as any);
          }
        }
        const unpackedExpectedParentTypes: any[] = [];
        if (objectProto.expectedParentTypes) {
          for (const item of objectProto.expectedParentTypes) {
            unpackedExpectedParentTypes.push(Number(item) as any);
          }
        }
        const unpackedExpectedChildTypes: any[] = [];
        if (objectProto.expectedChildTypes) {
          for (const item of objectProto.expectedChildTypes) {
            unpackedExpectedChildTypes.push(Number(item) as any);
          }
        }
        const unpackedExpectedAncestorTypes: any[] = [];
        if (objectProto.expectedAncestorTypes) {
          for (const item of objectProto.expectedAncestorTypes) {
            unpackedExpectedAncestorTypes.push(Number(item) as any);
          }
        }
        const unpackedExpectedDescendantTypes: any[] = [];
        if (objectProto.expectedDescendantTypes) {
          for (const item of objectProto.expectedDescendantTypes) {
            unpackedExpectedDescendantTypes.push(Number(item) as any);
          }
        }
        const unpackedTaggings: any[] = [];
        if (objectProto.taggings) {
          for (const item of objectProto.taggings) {
            unpackedTaggings.push(Number(item));
          }
        }
        return new (STRUCT_CLASS_BY_TYPE[12] as typeof NodeDefinition)({
          type: Number(objectProto.type) as any,
          isAbstract: objectProto.isAbstract,
          isExtensible: objectProto.isExtensible,
          isFinal: objectProto.isFinal,
          isFrozen: objectProto.isFrozen,
          properties: unpackedProperties,
          indexes: unpackedIndexes,
          constraints: unpackedConstraints,
          permissions: unpackedPermissions,
          methods: unpackedMethods,
          actions: unpackedActions,
          constants: unpackedConstants,
          baseType: objectProto.baseType != undefined ? Number(objectProto.baseType) as any : null,
          extendedBy: unpackedExtendedBy,
          inherits: unpackedInherits,
          inheritedBy: unpackedInheritedBy,
          traits: unpackedTraits,
          selfTraits: unpackedSelfTraits,
          eventTypes: unpackedEventTypes,
          selfEventTypes: unpackedSelfEventTypes,
          enumTypes: unpackedEnumTypes,
          selfEnumTypes: unpackedSelfEnumTypes,
          parentTypes: unpackedParentTypes,
          childTypes: unpackedChildTypes,
          ancestorTypes: unpackedAncestorTypes,
          descendantTypes: unpackedDescendantTypes,
          expectedParentTypes: unpackedExpectedParentTypes,
          expectedChildTypes: unpackedExpectedChildTypes,
          expectedAncestorTypes: unpackedExpectedAncestorTypes,
          expectedDescendantTypes: unpackedExpectedDescendantTypes,
          domain: objectProto.domain != undefined ? Number(objectProto.domain) as any : null,
          id: Number(objectProto.id),
          name: objectProto.name,
          icon: objectProto.icon != undefined ? _Icon.unpack(10, objectProto.icon, _session) as Icon : null,
          description: objectProto.description != undefined ? objectProto.description : null,
          taggings: unpackedTaggings,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: NodeDefinition): Uint8Array {
        const proto = this.packObject(object);
        return NodeDefinitionProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): NodeDefinition {
        const proto = NodeDefinitionProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 12)] = new NodeDefinitionProtoEncoder();

    class TraitDefinitionProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: TraitDefinition): TraitDefinitionProto {
        const objectProto: Partial<TraitDefinitionProto> = { metatype: 14 };
        objectProto.id = object.id;
        objectProto.type = Number(object.type) as any;
        objectProto.name = object.name;
        if (object.icon != null) {
          objectProto.icon = object.icon.pack(10);
        }
        if (object.description != null) {
          objectProto.description = object.description;
        }
        if (object.taggings) {
          const packedTaggings: any[] = [];
          for (const item of object.taggings) {
            packedTaggings.push(item);
          }
          objectProto.taggings = packedTaggings;
        }
        objectProto.alias = object.alias;
        objectProto.isExtensible = object.isExtensible;
        if (object.permissions) {
          const packedPermissions: any[] = [];
          for (const item of object.permissions) {
            packedPermissions.push(item.pack(10));
          }
          objectProto.permissions = packedPermissions;
        }
        if (object.selfTraits) {
          const packedSelfTraits: any[] = [];
          for (const item of object.selfTraits) {
            packedSelfTraits.push(Number(item) as any);
          }
          objectProto.selfTraits = packedSelfTraits;
        }
        if (object.traits) {
          const packedTraits: any[] = [];
          for (const item of object.traits) {
            packedTraits.push(Number(item) as any);
          }
          objectProto.traits = packedTraits;
        }
        if (object.eventTypes) {
          const packedEventTypes: any[] = [];
          for (const item of object.eventTypes) {
            packedEventTypes.push(Number(item) as any);
          }
          objectProto.eventTypes = packedEventTypes;
        }
        if (object.selfEventTypes) {
          const packedSelfEventTypes: any[] = [];
          for (const item of object.selfEventTypes) {
            packedSelfEventTypes.push(Number(item) as any);
          }
          objectProto.selfEventTypes = packedSelfEventTypes;
        }
        if (object.enumTypes) {
          const packedEnumTypes: any[] = [];
          for (const item of object.enumTypes) {
            packedEnumTypes.push(Number(item) as any);
          }
          objectProto.enumTypes = packedEnumTypes;
        }
        if (object.selfEnumTypes) {
          const packedSelfEnumTypes: any[] = [];
          for (const item of object.selfEnumTypes) {
            packedSelfEnumTypes.push(Number(item) as any);
          }
          objectProto.selfEnumTypes = packedSelfEnumTypes;
        }
        return objectProto as TraitDefinitionProto;
      }

      unpackObject(objectProto: TraitDefinitionProto, _session: Session | null): TraitDefinition {
        const _PermissionDefinition = STRUCT_CLASS_BY_TYPE[50000] as typeof PermissionDefinition;
        const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
        const unpackedPermissions: any[] = [];
        if (objectProto.permissions) {
          for (const item of objectProto.permissions) {
            unpackedPermissions.push(_PermissionDefinition.unpack(10, item, _session) as PermissionDefinition);
          }
        }
        const unpackedSelfTraits: any[] = [];
        if (objectProto.selfTraits) {
          for (const item of objectProto.selfTraits) {
            unpackedSelfTraits.push(Number(item) as any);
          }
        }
        const unpackedTraits: any[] = [];
        if (objectProto.traits) {
          for (const item of objectProto.traits) {
            unpackedTraits.push(Number(item) as any);
          }
        }
        const unpackedEventTypes: any[] = [];
        if (objectProto.eventTypes) {
          for (const item of objectProto.eventTypes) {
            unpackedEventTypes.push(Number(item) as any);
          }
        }
        const unpackedSelfEventTypes: any[] = [];
        if (objectProto.selfEventTypes) {
          for (const item of objectProto.selfEventTypes) {
            unpackedSelfEventTypes.push(Number(item) as any);
          }
        }
        const unpackedEnumTypes: any[] = [];
        if (objectProto.enumTypes) {
          for (const item of objectProto.enumTypes) {
            unpackedEnumTypes.push(Number(item) as any);
          }
        }
        const unpackedSelfEnumTypes: any[] = [];
        if (objectProto.selfEnumTypes) {
          for (const item of objectProto.selfEnumTypes) {
            unpackedSelfEnumTypes.push(Number(item) as any);
          }
        }
        const unpackedTaggings: any[] = [];
        if (objectProto.taggings) {
          for (const item of objectProto.taggings) {
            unpackedTaggings.push(Number(item));
          }
        }
        return new (STRUCT_CLASS_BY_TYPE[14] as typeof TraitDefinition)({
          type: Number(objectProto.type) as any,
          alias: objectProto.alias,
          isExtensible: objectProto.isExtensible,
          permissions: unpackedPermissions,
          selfTraits: unpackedSelfTraits,
          traits: unpackedTraits,
          eventTypes: unpackedEventTypes,
          selfEventTypes: unpackedSelfEventTypes,
          enumTypes: unpackedEnumTypes,
          selfEnumTypes: unpackedSelfEnumTypes,
          id: Number(objectProto.id),
          name: objectProto.name,
          icon: objectProto.icon != undefined ? _Icon.unpack(10, objectProto.icon, _session) as Icon : null,
          description: objectProto.description != undefined ? objectProto.description : null,
          taggings: unpackedTaggings,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: TraitDefinition): Uint8Array {
        const proto = this.packObject(object);
        return TraitDefinitionProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): TraitDefinition {
        const proto = TraitDefinitionProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 14)] = new TraitDefinitionProtoEncoder();

    class StructDefinitionProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: StructDefinition): StructDefinitionProto {
        const objectProto: Partial<StructDefinitionProto> = { metatype: 15 };
        objectProto.id = object.id;
        objectProto.type = Number(object.type) as any;
        objectProto.name = object.name;
        if (object.icon != null) {
          objectProto.icon = object.icon.pack(10);
        }
        if (object.description != null) {
          objectProto.description = object.description;
        }
        if (object.taggings) {
          const packedTaggings: any[] = [];
          for (const item of object.taggings) {
            packedTaggings.push(item);
          }
          objectProto.taggings = packedTaggings;
        }
        objectProto.isFrozen = object.isFrozen;
        objectProto.isAbstract = object.isAbstract;
        objectProto.isExtensible = object.isExtensible;
        if (object.properties) {
          const packedProperties: any[] = [];
          for (const item of object.properties) {
            packedProperties.push(item.pack(10));
          }
          objectProto.properties = packedProperties;
        }
        if (object.methods) {
          const packedMethods: any[] = [];
          for (const item of object.methods) {
            packedMethods.push(item.pack(10));
          }
          objectProto.methods = packedMethods;
        }
        if (object.actions) {
          const packedActions: any[] = [];
          for (const item of object.actions) {
            packedActions.push(item.pack(10));
          }
          objectProto.actions = packedActions;
        }
        if (object.constants) {
          const packedConstants: any[] = [];
          for (const item of object.constants) {
            packedConstants.push(item.pack(10));
          }
          objectProto.constants = packedConstants;
        }
        if (object.tags) {
          const packedTags: any[] = [];
          for (const item of object.tags) {
            packedTags.push(item.pack(10));
          }
          objectProto.tags = packedTags;
        }
        if (object.baseType != null) {
          objectProto.baseType = Number(object.baseType) as any;
        }
        if (object.extendedBy) {
          const packedExtendedBy: any[] = [];
          for (const item of object.extendedBy) {
            packedExtendedBy.push(Number(item) as any);
          }
          objectProto.extendedBy = packedExtendedBy;
        }
        if (object.inherits) {
          const packedInherits: any[] = [];
          for (const item of object.inherits) {
            packedInherits.push(Number(item) as any);
          }
          objectProto.inherits = packedInherits;
        }
        if (object.inheritedBy) {
          const packedInheritedBy: any[] = [];
          for (const item of object.inheritedBy) {
            packedInheritedBy.push(Number(item) as any);
          }
          objectProto.inheritedBy = packedInheritedBy;
        }
        if (object.enumTypes) {
          const packedEnumTypes: any[] = [];
          for (const item of object.enumTypes) {
            packedEnumTypes.push(Number(item) as any);
          }
          objectProto.enumTypes = packedEnumTypes;
        }
        if (object.selfEnumTypes) {
          const packedSelfEnumTypes: any[] = [];
          for (const item of object.selfEnumTypes) {
            packedSelfEnumTypes.push(Number(item) as any);
          }
          objectProto.selfEnumTypes = packedSelfEnumTypes;
        }
        return objectProto as StructDefinitionProto;
      }

      unpackObject(objectProto: StructDefinitionProto, _session: Session | null): StructDefinition {
        const _PropertyDefinition = STRUCT_CLASS_BY_TYPE[18] as typeof PropertyDefinition;
        const _ConstantDefinition = STRUCT_CLASS_BY_TYPE[19] as typeof ConstantDefinition;
        const _TagDefinition = STRUCT_CLASS_BY_TYPE[21] as typeof TagDefinition;
        const _MethodDefinition = STRUCT_CLASS_BY_TYPE[40000] as typeof MethodDefinition;
        const _ActionDefinition = STRUCT_CLASS_BY_TYPE[40100] as typeof ActionDefinition;
        const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
        const unpackedProperties: any[] = [];
        if (objectProto.properties) {
          for (const item of objectProto.properties) {
            unpackedProperties.push(_PropertyDefinition.unpack(10, item, _session) as PropertyDefinition);
          }
        }
        const unpackedMethods: any[] = [];
        if (objectProto.methods) {
          for (const item of objectProto.methods) {
            unpackedMethods.push(_MethodDefinition.unpack(10, item, _session) as MethodDefinition);
          }
        }
        const unpackedActions: any[] = [];
        if (objectProto.actions) {
          for (const item of objectProto.actions) {
            unpackedActions.push(_ActionDefinition.unpack(10, item, _session) as ActionDefinition);
          }
        }
        const unpackedConstants: any[] = [];
        if (objectProto.constants) {
          for (const item of objectProto.constants) {
            unpackedConstants.push(_ConstantDefinition.unpack(10, item, _session) as ConstantDefinition);
          }
        }
        const unpackedTags: any[] = [];
        if (objectProto.tags) {
          for (const item of objectProto.tags) {
            unpackedTags.push(_TagDefinition.unpack(10, item, _session) as TagDefinition);
          }
        }
        const unpackedExtendedBy: any[] = [];
        if (objectProto.extendedBy) {
          for (const item of objectProto.extendedBy) {
            unpackedExtendedBy.push(Number(item) as any);
          }
        }
        const unpackedInherits: any[] = [];
        if (objectProto.inherits) {
          for (const item of objectProto.inherits) {
            unpackedInherits.push(Number(item) as any);
          }
        }
        const unpackedInheritedBy: any[] = [];
        if (objectProto.inheritedBy) {
          for (const item of objectProto.inheritedBy) {
            unpackedInheritedBy.push(Number(item) as any);
          }
        }
        const unpackedEnumTypes: any[] = [];
        if (objectProto.enumTypes) {
          for (const item of objectProto.enumTypes) {
            unpackedEnumTypes.push(Number(item) as any);
          }
        }
        const unpackedSelfEnumTypes: any[] = [];
        if (objectProto.selfEnumTypes) {
          for (const item of objectProto.selfEnumTypes) {
            unpackedSelfEnumTypes.push(Number(item) as any);
          }
        }
        const unpackedTaggings: any[] = [];
        if (objectProto.taggings) {
          for (const item of objectProto.taggings) {
            unpackedTaggings.push(Number(item));
          }
        }
        return new (STRUCT_CLASS_BY_TYPE[15] as typeof StructDefinition)({
          type: Number(objectProto.type) as any,
          isFrozen: objectProto.isFrozen,
          isAbstract: objectProto.isAbstract,
          isExtensible: objectProto.isExtensible,
          properties: unpackedProperties,
          methods: unpackedMethods,
          actions: unpackedActions,
          constants: unpackedConstants,
          tags: unpackedTags,
          baseType: objectProto.baseType != undefined ? Number(objectProto.baseType) as any : null,
          extendedBy: unpackedExtendedBy,
          inherits: unpackedInherits,
          inheritedBy: unpackedInheritedBy,
          enumTypes: unpackedEnumTypes,
          selfEnumTypes: unpackedSelfEnumTypes,
          id: Number(objectProto.id),
          name: objectProto.name,
          icon: objectProto.icon != undefined ? _Icon.unpack(10, objectProto.icon, _session) as Icon : null,
          description: objectProto.description != undefined ? objectProto.description : null,
          taggings: unpackedTaggings,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: StructDefinition): Uint8Array {
        const proto = this.packObject(object);
        return StructDefinitionProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): StructDefinition {
        const proto = StructDefinitionProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 15)] = new StructDefinitionProtoEncoder();

    class EnumDefinitionProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: EnumDefinition): EnumDefinitionProto {
        const objectProto: Partial<EnumDefinitionProto> = { metatype: 17 };
        objectProto.id = object.id;
        objectProto.type = Number(object.type) as any;
        objectProto.name = object.name;
        if (object.icon != null) {
          objectProto.icon = object.icon.pack(10);
        }
        if (object.description != null) {
          objectProto.description = object.description;
        }
        if (object.options) {
          const packedOptions: any[] = [];
          for (const item of object.options) {
            packedOptions.push(item.pack(10));
          }
          objectProto.options = packedOptions;
        }
        if (object.taggings) {
          const packedTaggings: any[] = [];
          for (const item of object.taggings) {
            packedTaggings.push(item);
          }
          objectProto.taggings = packedTaggings;
        }
        return objectProto as EnumDefinitionProto;
      }

      unpackObject(objectProto: EnumDefinitionProto, _session: Session | null): EnumDefinition {
        const _OptionDefinition = STRUCT_CLASS_BY_TYPE[20] as typeof OptionDefinition;
        const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
        const unpackedOptions: any[] = [];
        if (objectProto.options) {
          for (const item of objectProto.options) {
            unpackedOptions.push(_OptionDefinition.unpack(10, item, _session) as OptionDefinition);
          }
        }
        const unpackedTaggings: any[] = [];
        if (objectProto.taggings) {
          for (const item of objectProto.taggings) {
            unpackedTaggings.push(Number(item));
          }
        }
        return new (STRUCT_CLASS_BY_TYPE[17] as typeof EnumDefinition)({
          type: Number(objectProto.type) as any,
          options: unpackedOptions,
          id: Number(objectProto.id),
          name: objectProto.name,
          icon: objectProto.icon != undefined ? _Icon.unpack(10, objectProto.icon, _session) as Icon : null,
          description: objectProto.description != undefined ? objectProto.description : null,
          taggings: unpackedTaggings,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: EnumDefinition): Uint8Array {
        const proto = this.packObject(object);
        return EnumDefinitionProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): EnumDefinition {
        const proto = EnumDefinitionProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 17)] = new EnumDefinitionProtoEncoder();

    class PropertyDefinitionProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: PropertyDefinition): PropertyDefinitionProto {
        const objectProto: Partial<PropertyDefinitionProto> = { metatype: 18 };
        objectProto.id = object.id;
        objectProto.type = Number(object.type) as any;
        objectProto.name = object.name;
        if (object.icon != null) {
          objectProto.icon = object.icon.pack(10);
        }
        if (object.description != null) {
          objectProto.description = object.description;
        }
        objectProto.object = object.object.pack(10);
        objectProto.originalObject = object.originalObject.pack(10);
        if (object.taggings) {
          const packedTaggings: any[] = [];
          for (const item of object.taggings) {
            packedTaggings.push(item);
          }
          objectProto.taggings = packedTaggings;
        }
        objectProto.cardinality = Number(object.cardinality) as any;
        objectProto.scalarType = Number(object.scalarType) as any;
        if (object.primitiveType != null) {
          objectProto.primitiveType = Number(object.primitiveType) as any;
        }
        if (object.enumType != null) {
          objectProto.enumType = Number(object.enumType) as any;
        }
        if (object.nodeTypes) {
          const packedNodeTypes: any[] = [];
          for (const item of object.nodeTypes) {
            packedNodeTypes.push(Number(item) as any);
          }
          objectProto.nodeTypes = packedNodeTypes;
        }
        if (object.structType != null) {
          objectProto.structType = Number(object.structType) as any;
        }
        if (object.keyType != null) {
          objectProto.keyType = object.keyType.pack(10);
        }
        if (object.value != null) {
          objectProto.value = object.value.pack(10);
        }
        if (object.valueFactory != null) {
          objectProto.valueFactory = Number(object.valueFactory) as any;
        }
        if (object.collectionConstraint != null) {
          objectProto.collectionConstraint = object.collectionConstraint.pack(10);
        }
        if (object.stringConstraint != null) {
          objectProto.stringConstraint = object.stringConstraint.pack(10);
        }
        if (object.numberConstraint != null) {
          objectProto.numberConstraint = object.numberConstraint.pack(10);
        }
        if (object.edgeType != null) {
          objectProto.edgeType = Number(object.edgeType) as any;
        }
        if (object.cascade != null) {
          objectProto.cascade = Number(object.cascade) as any;
        }
        objectProto.isRequired = object.isRequired;
        objectProto.isUnique = object.isUnique;
        objectProto.isReadonly = object.isReadonly;
        objectProto.isMain = object.isMain;
        objectProto.isWired = object.isWired;
        objectProto.isStored = object.isStored;
        objectProto.isRepr = object.isRepr;
        objectProto.isHash = object.isHash;
        objectProto.isEq = object.isEq;
        objectProto.isInternal = object.isInternal;
        return objectProto as PropertyDefinitionProto;
      }

      unpackObject(objectProto: PropertyDefinitionProto, _session: Session | null): PropertyDefinition {
        const _ObjectDefinitionReference = STRUCT_CLASS_BY_TYPE[11] as typeof ObjectDefinitionReference;
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _Type = STRUCT_CLASS_BY_TYPE[101] as typeof Type;
        const _NumberConstraint = STRUCT_CLASS_BY_TYPE[110] as typeof NumberConstraint;
        const _StringConstraint = STRUCT_CLASS_BY_TYPE[111] as typeof StringConstraint;
        const _CollectionConstraint = STRUCT_CLASS_BY_TYPE[112] as typeof CollectionConstraint;
        const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
        const unpackedNodeTypes: any[] = [];
        if (objectProto.nodeTypes) {
          for (const item of objectProto.nodeTypes) {
            unpackedNodeTypes.push(Number(item) as any);
          }
        }
        const unpackedTaggings: any[] = [];
        if (objectProto.taggings) {
          for (const item of objectProto.taggings) {
            unpackedTaggings.push(Number(item));
          }
        }
        return new (STRUCT_CLASS_BY_TYPE[18] as typeof PropertyDefinition)({
          type: Number(objectProto.type) as any,
          object: _ObjectDefinitionReference.unpack(10, objectProto.object, _session) as ObjectDefinitionReference,
          originalObject: _ObjectDefinitionReference.unpack(10, objectProto.originalObject, _session) as ObjectDefinitionReference,
          cardinality: Number(objectProto.cardinality) as any,
          scalarType: Number(objectProto.scalarType) as any,
          primitiveType: objectProto.primitiveType != undefined ? Number(objectProto.primitiveType) as any : null,
          enumType: objectProto.enumType != undefined ? Number(objectProto.enumType) as any : null,
          nodeTypes: unpackedNodeTypes,
          structType: objectProto.structType != undefined ? Number(objectProto.structType) as any : null,
          keyType: objectProto.keyType != undefined ? _Type.unpack(10, objectProto.keyType, _session) as Type : null,
          value: objectProto.value != undefined ? _Value.unpack(10, objectProto.value, _session) as Value : null,
          valueFactory: objectProto.valueFactory != undefined ? Number(objectProto.valueFactory) as any : null,
          collectionConstraint: objectProto.collectionConstraint != undefined ? _CollectionConstraint.unpack(10, objectProto.collectionConstraint, _session) as CollectionConstraint : null,
          stringConstraint: objectProto.stringConstraint != undefined ? _StringConstraint.unpack(10, objectProto.stringConstraint, _session) as StringConstraint : null,
          numberConstraint: objectProto.numberConstraint != undefined ? _NumberConstraint.unpack(10, objectProto.numberConstraint, _session) as NumberConstraint : null,
          edgeType: objectProto.edgeType != undefined ? Number(objectProto.edgeType) as any : null,
          cascade: objectProto.cascade != undefined ? Number(objectProto.cascade) as any : null,
          isRequired: objectProto.isRequired,
          isUnique: objectProto.isUnique,
          isReadonly: objectProto.isReadonly,
          isMain: objectProto.isMain,
          isWired: objectProto.isWired,
          isStored: objectProto.isStored,
          isRepr: objectProto.isRepr,
          isHash: objectProto.isHash,
          isEq: objectProto.isEq,
          isInternal: objectProto.isInternal,
          id: Number(objectProto.id),
          name: objectProto.name,
          icon: objectProto.icon != undefined ? _Icon.unpack(10, objectProto.icon, _session) as Icon : null,
          description: objectProto.description != undefined ? objectProto.description : null,
          taggings: unpackedTaggings,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: PropertyDefinition): Uint8Array {
        const proto = this.packObject(object);
        return PropertyDefinitionProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): PropertyDefinition {
        const proto = PropertyDefinitionProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 18)] = new PropertyDefinitionProtoEncoder();

    class OptionDefinitionProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: OptionDefinition): OptionDefinitionProto {
        const objectProto: Partial<OptionDefinitionProto> = { metatype: 20 };
        objectProto.id = object.id;
        objectProto.type = Number(object.type) as any;
        objectProto.name = object.name;
        if (object.icon != null) {
          objectProto.icon = object.icon.pack(10);
        }
        if (object.description != null) {
          objectProto.description = object.description;
        }
        if (object.taggings) {
          const packedTaggings: any[] = [];
          for (const item of object.taggings) {
            packedTaggings.push(item);
          }
          objectProto.taggings = packedTaggings;
        }
        return objectProto as OptionDefinitionProto;
      }

      unpackObject(objectProto: OptionDefinitionProto, _session: Session | null): OptionDefinition {
        const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
        const unpackedTaggings: any[] = [];
        if (objectProto.taggings) {
          for (const item of objectProto.taggings) {
            unpackedTaggings.push(Number(item));
          }
        }
        return new (STRUCT_CLASS_BY_TYPE[20] as typeof OptionDefinition)({
          type: Number(objectProto.type) as any,
          id: Number(objectProto.id),
          name: objectProto.name,
          icon: objectProto.icon != undefined ? _Icon.unpack(10, objectProto.icon, _session) as Icon : null,
          description: objectProto.description != undefined ? objectProto.description : null,
          taggings: unpackedTaggings,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: OptionDefinition): Uint8Array {
        const proto = this.packObject(object);
        return OptionDefinitionProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): OptionDefinition {
        const proto = OptionDefinitionProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 20)] = new OptionDefinitionProtoEncoder();

    class ConstantDefinitionProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: ConstantDefinition): ConstantDefinitionProto {
        const objectProto: Partial<ConstantDefinitionProto> = { metatype: 19 };
        objectProto.id = object.id;
        objectProto.name = object.name;
        if (object.icon != null) {
          objectProto.icon = object.icon.pack(10);
        }
        if (object.description != null) {
          objectProto.description = object.description;
        }
        if (object.taggings) {
          const packedTaggings: any[] = [];
          for (const item of object.taggings) {
            packedTaggings.push(item);
          }
          objectProto.taggings = packedTaggings;
        }
        objectProto.value = object.value.pack(10);
        return objectProto as ConstantDefinitionProto;
      }

      unpackObject(objectProto: ConstantDefinitionProto, _session: Session | null): ConstantDefinition {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
        const unpackedTaggings: any[] = [];
        if (objectProto.taggings) {
          for (const item of objectProto.taggings) {
            unpackedTaggings.push(Number(item));
          }
        }
        return new (STRUCT_CLASS_BY_TYPE[19] as typeof ConstantDefinition)({
          value: _Value.unpack(10, objectProto.value, _session) as Value,
          id: Number(objectProto.id),
          name: objectProto.name,
          icon: objectProto.icon != undefined ? _Icon.unpack(10, objectProto.icon, _session) as Icon : null,
          description: objectProto.description != undefined ? objectProto.description : null,
          taggings: unpackedTaggings,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: ConstantDefinition): Uint8Array {
        const proto = this.packObject(object);
        return ConstantDefinitionProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): ConstantDefinition {
        const proto = ConstantDefinitionProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 19)] = new ConstantDefinitionProtoEncoder();

    class TagDefinitionProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: TagDefinition): TagDefinitionProto {
        const objectProto: Partial<TagDefinitionProto> = { metatype: 21 };
        objectProto.id = object.id;
        objectProto.name = object.name;
        if (object.icon != null) {
          objectProto.icon = object.icon.pack(10);
        }
        if (object.description != null) {
          objectProto.description = object.description;
        }
        if (object.taggings) {
          const packedTaggings: any[] = [];
          for (const item of object.taggings) {
            packedTaggings.push(item);
          }
          objectProto.taggings = packedTaggings;
        }
        return objectProto as TagDefinitionProto;
      }

      unpackObject(objectProto: TagDefinitionProto, _session: Session | null): TagDefinition {
        const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
        const unpackedTaggings: any[] = [];
        if (objectProto.taggings) {
          for (const item of objectProto.taggings) {
            unpackedTaggings.push(Number(item));
          }
        }
        return new (STRUCT_CLASS_BY_TYPE[21] as typeof TagDefinition)({
          id: Number(objectProto.id),
          name: objectProto.name,
          icon: objectProto.icon != undefined ? _Icon.unpack(10, objectProto.icon, _session) as Icon : null,
          description: objectProto.description != undefined ? objectProto.description : null,
          taggings: unpackedTaggings,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: TagDefinition): Uint8Array {
        const proto = this.packObject(object);
        return TagDefinitionProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): TagDefinition {
        const proto = TagDefinitionProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 21)] = new TagDefinitionProtoEncoder();

    class IndexDefinitionProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: IndexDefinition): IndexDefinitionProto {
        const objectProto: Partial<IndexDefinitionProto> = { metatype: 30100 };
        objectProto.id = object.id;
        objectProto.type = Number(object.type) as any;
        objectProto.name = object.name;
        if (object.icon != null) {
          objectProto.icon = object.icon.pack(10);
        }
        if (object.description != null) {
          objectProto.description = object.description;
        }
        if (object.properties) {
          const packedProperties: any[] = [];
          for (const item of object.properties) {
            packedProperties.push(item.pack(10));
          }
          objectProto.properties = packedProperties;
        }
        if (object.cover) {
          const packedCover: any[] = [];
          for (const item of object.cover) {
            packedCover.push(item.pack(10));
          }
          objectProto.cover = packedCover;
        }
        if (object.taggings) {
          const packedTaggings: any[] = [];
          for (const item of object.taggings) {
            packedTaggings.push(item);
          }
          objectProto.taggings = packedTaggings;
        }
        return objectProto as IndexDefinitionProto;
      }

      unpackObject(objectProto: IndexDefinitionProto, _session: Session | null): IndexDefinition {
        const _PropertyReference = STRUCT_CLASS_BY_TYPE[1001] as typeof PropertyReference;
        const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
        const unpackedProperties: any[] = [];
        if (objectProto.properties) {
          for (const item of objectProto.properties) {
            unpackedProperties.push(_PropertyReference.unpack(10, item, _session) as PropertyReference);
          }
        }
        const unpackedCover: any[] = [];
        if (objectProto.cover) {
          for (const item of objectProto.cover) {
            unpackedCover.push(_PropertyReference.unpack(10, item, _session) as PropertyReference);
          }
        }
        const unpackedTaggings: any[] = [];
        if (objectProto.taggings) {
          for (const item of objectProto.taggings) {
            unpackedTaggings.push(Number(item));
          }
        }
        return new (STRUCT_CLASS_BY_TYPE[30100] as typeof IndexDefinition)({
          type: Number(objectProto.type) as any,
          properties: unpackedProperties,
          cover: unpackedCover,
          id: Number(objectProto.id),
          name: objectProto.name,
          icon: objectProto.icon != undefined ? _Icon.unpack(10, objectProto.icon, _session) as Icon : null,
          description: objectProto.description != undefined ? objectProto.description : null,
          taggings: unpackedTaggings,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: IndexDefinition): Uint8Array {
        const proto = this.packObject(object);
        return IndexDefinitionProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): IndexDefinition {
        const proto = IndexDefinitionProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 30100)] = new IndexDefinitionProtoEncoder();

    class ConstraintDefinitionProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: ConstraintDefinition): ConstraintDefinitionProto {
        const objectProto: Partial<ConstraintDefinitionProto> = { metatype: 30200 };
        objectProto.id = object.id;
        objectProto.type = Number(object.type) as any;
        objectProto.name = object.name;
        if (object.icon != null) {
          objectProto.icon = object.icon.pack(10);
        }
        if (object.description != null) {
          objectProto.description = object.description;
        }
        if (object.properties) {
          const packedProperties: any[] = [];
          for (const item of object.properties) {
            packedProperties.push(item.pack(10));
          }
          objectProto.properties = packedProperties;
        }
        if (object.taggings) {
          const packedTaggings: any[] = [];
          for (const item of object.taggings) {
            packedTaggings.push(item);
          }
          objectProto.taggings = packedTaggings;
        }
        return objectProto as ConstraintDefinitionProto;
      }

      unpackObject(objectProto: ConstraintDefinitionProto, _session: Session | null): ConstraintDefinition {
        const _PropertyReference = STRUCT_CLASS_BY_TYPE[1001] as typeof PropertyReference;
        const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
        const unpackedProperties: any[] = [];
        if (objectProto.properties) {
          for (const item of objectProto.properties) {
            unpackedProperties.push(_PropertyReference.unpack(10, item, _session) as PropertyReference);
          }
        }
        const unpackedTaggings: any[] = [];
        if (objectProto.taggings) {
          for (const item of objectProto.taggings) {
            unpackedTaggings.push(Number(item));
          }
        }
        return new (STRUCT_CLASS_BY_TYPE[30200] as typeof ConstraintDefinition)({
          type: Number(objectProto.type) as any,
          properties: unpackedProperties,
          id: Number(objectProto.id),
          name: objectProto.name,
          icon: objectProto.icon != undefined ? _Icon.unpack(10, objectProto.icon, _session) as Icon : null,
          description: objectProto.description != undefined ? objectProto.description : null,
          taggings: unpackedTaggings,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: ConstraintDefinition): Uint8Array {
        const proto = this.packObject(object);
        return ConstraintDefinitionProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): ConstraintDefinition {
        const proto = ConstraintDefinitionProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 30200)] = new ConstraintDefinitionProtoEncoder();

    class PermissionDefinitionProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: PermissionDefinition): PermissionDefinitionProto {
        const objectProto: Partial<PermissionDefinitionProto> = { metatype: 50000 };
        objectProto.id = object.id;
        objectProto.name = object.name;
        if (object.icon != null) {
          objectProto.icon = object.icon.pack(10);
        }
        if (object.description != null) {
          objectProto.description = object.description;
        }
        if (object.taggings) {
          const packedTaggings: any[] = [];
          for (const item of object.taggings) {
            packedTaggings.push(item);
          }
          objectProto.taggings = packedTaggings;
        }
        return objectProto as PermissionDefinitionProto;
      }

      unpackObject(objectProto: PermissionDefinitionProto, _session: Session | null): PermissionDefinition {
        const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
        const unpackedTaggings: any[] = [];
        if (objectProto.taggings) {
          for (const item of objectProto.taggings) {
            unpackedTaggings.push(Number(item));
          }
        }
        return new (STRUCT_CLASS_BY_TYPE[50000] as typeof PermissionDefinition)({
          id: Number(objectProto.id),
          name: objectProto.name,
          icon: objectProto.icon != undefined ? _Icon.unpack(10, objectProto.icon, _session) as Icon : null,
          description: objectProto.description != undefined ? objectProto.description : null,
          taggings: unpackedTaggings,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: PermissionDefinition): Uint8Array {
        const proto = this.packObject(object);
        return PermissionDefinitionProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): PermissionDefinition {
        const proto = PermissionDefinitionProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 50000)] = new PermissionDefinitionProtoEncoder();

    class MethodDefinitionProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: MethodDefinition): MethodDefinitionProto {
        const objectProto: Partial<MethodDefinitionProto> = { metatype: 40000 };
        objectProto.id = object.id;
        objectProto.type = Number(object.type) as any;
        objectProto.name = object.name;
        if (object.icon != null) {
          objectProto.icon = object.icon.pack(10);
        }
        if (object.description != null) {
          objectProto.description = object.description;
        }
        if (object.properties) {
          const packedProperties: any[] = [];
          for (const item of object.properties) {
            packedProperties.push(item.pack(10));
          }
          objectProto.properties = packedProperties;
        }
        if (object.taggings) {
          const packedTaggings: any[] = [];
          for (const item of object.taggings) {
            packedTaggings.push(item);
          }
          objectProto.taggings = packedTaggings;
        }
        objectProto.cardinality = Number(object.cardinality) as any;
        if (object.platforms) {
          const packedPlatforms: any[] = [];
          for (const item of object.platforms) {
            packedPlatforms.push(Number(item) as any);
          }
          objectProto.platforms = packedPlatforms;
        }
        if (object.languages) {
          const packedLanguages: any[] = [];
          for (const item of object.languages) {
            packedLanguages.push(Number(item) as any);
          }
          objectProto.languages = packedLanguages;
        }
        return objectProto as MethodDefinitionProto;
      }

      unpackObject(objectProto: MethodDefinitionProto, _session: Session | null): MethodDefinition {
        const _PropertyDefinition = STRUCT_CLASS_BY_TYPE[18] as typeof PropertyDefinition;
        const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
        const unpackedProperties: any[] = [];
        if (objectProto.properties) {
          for (const item of objectProto.properties) {
            unpackedProperties.push(_PropertyDefinition.unpack(10, item, _session) as PropertyDefinition);
          }
        }
        const unpackedPlatforms: any[] = [];
        if (objectProto.platforms) {
          for (const item of objectProto.platforms) {
            unpackedPlatforms.push(Number(item) as any);
          }
        }
        const unpackedLanguages: any[] = [];
        if (objectProto.languages) {
          for (const item of objectProto.languages) {
            unpackedLanguages.push(Number(item) as any);
          }
        }
        const unpackedTaggings: any[] = [];
        if (objectProto.taggings) {
          for (const item of objectProto.taggings) {
            unpackedTaggings.push(Number(item));
          }
        }
        return new (STRUCT_CLASS_BY_TYPE[40000] as typeof MethodDefinition)({
          type: Number(objectProto.type) as any,
          properties: unpackedProperties,
          cardinality: Number(objectProto.cardinality) as any,
          platforms: unpackedPlatforms,
          languages: unpackedLanguages,
          id: Number(objectProto.id),
          name: objectProto.name,
          icon: objectProto.icon != undefined ? _Icon.unpack(10, objectProto.icon, _session) as Icon : null,
          description: objectProto.description != undefined ? objectProto.description : null,
          taggings: unpackedTaggings,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: MethodDefinition): Uint8Array {
        const proto = this.packObject(object);
        return MethodDefinitionProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): MethodDefinition {
        const proto = MethodDefinitionProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 40000)] = new MethodDefinitionProtoEncoder();

    class ActionDefinitionProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: ActionDefinition): ActionDefinitionProto {
        const objectProto: Partial<ActionDefinitionProto> = { metatype: 40100 };
        objectProto.id = object.id;
        objectProto.type = Number(object.type) as any;
        objectProto.name = object.name;
        if (object.icon != null) {
          objectProto.icon = object.icon.pack(10);
        }
        if (object.description != null) {
          objectProto.description = object.description;
        }
        if (object.properties) {
          const packedProperties: any[] = [];
          for (const item of object.properties) {
            packedProperties.push(item.pack(10));
          }
          objectProto.properties = packedProperties;
        }
        if (object.taggings) {
          const packedTaggings: any[] = [];
          for (const item of object.taggings) {
            packedTaggings.push(item);
          }
          objectProto.taggings = packedTaggings;
        }
        objectProto.cardinality = Number(object.cardinality) as any;
        if (object.platforms) {
          const packedPlatforms: any[] = [];
          for (const item of object.platforms) {
            packedPlatforms.push(Number(item) as any);
          }
          objectProto.platforms = packedPlatforms;
        }
        if (object.languages) {
          const packedLanguages: any[] = [];
          for (const item of object.languages) {
            packedLanguages.push(Number(item) as any);
          }
          objectProto.languages = packedLanguages;
        }
        return objectProto as ActionDefinitionProto;
      }

      unpackObject(objectProto: ActionDefinitionProto, _session: Session | null): ActionDefinition {
        const _PropertyDefinition = STRUCT_CLASS_BY_TYPE[18] as typeof PropertyDefinition;
        const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
        const unpackedProperties: any[] = [];
        if (objectProto.properties) {
          for (const item of objectProto.properties) {
            unpackedProperties.push(_PropertyDefinition.unpack(10, item, _session) as PropertyDefinition);
          }
        }
        const unpackedPlatforms: any[] = [];
        if (objectProto.platforms) {
          for (const item of objectProto.platforms) {
            unpackedPlatforms.push(Number(item) as any);
          }
        }
        const unpackedLanguages: any[] = [];
        if (objectProto.languages) {
          for (const item of objectProto.languages) {
            unpackedLanguages.push(Number(item) as any);
          }
        }
        const unpackedTaggings: any[] = [];
        if (objectProto.taggings) {
          for (const item of objectProto.taggings) {
            unpackedTaggings.push(Number(item));
          }
        }
        return new (STRUCT_CLASS_BY_TYPE[40100] as typeof ActionDefinition)({
          type: Number(objectProto.type) as any,
          properties: unpackedProperties,
          cardinality: Number(objectProto.cardinality) as any,
          platforms: unpackedPlatforms,
          languages: unpackedLanguages,
          id: Number(objectProto.id),
          name: objectProto.name,
          icon: objectProto.icon != undefined ? _Icon.unpack(10, objectProto.icon, _session) as Icon : null,
          description: objectProto.description != undefined ? objectProto.description : null,
          taggings: unpackedTaggings,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: ActionDefinition): Uint8Array {
        const proto = this.packObject(object);
        return ActionDefinitionProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): ActionDefinition {
        const proto = ActionDefinitionProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 40100)] = new ActionDefinitionProtoEncoder();

    class IconProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Icon): IconProto {
        const objectProto: Partial<IconProto> = { metatype: 400031 };
        objectProto.type = Number(object.type) as any;
        if (object.emoji != null) {
          objectProto.emoji = object.emoji;
        }
        if (object.faName != null) {
          objectProto.faName = object.faName;
        }
        if (object.vscName != null) {
          objectProto.vscName = object.vscName;
        }
        if (object.filePtr != null) {
          objectProto.filePtr = object.filePtr.pack(10);
        }
        if (object.fileUrl != null) {
          objectProto.fileUrl = object.fileUrl;
        }
        if (object.color != null) {
          objectProto.color = object.color.pack(10);
        }
        return objectProto as IconProto;
      }

      unpackObject(objectProto: IconProto, _session: Session | null): Icon {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Color = STRUCT_CLASS_BY_TYPE[2100300] as typeof Color;
        return new (STRUCT_CLASS_BY_TYPE[400031] as typeof Icon)({
          type: Number(objectProto.type) as any,
          emoji: objectProto.emoji != undefined ? objectProto.emoji : null,
          faName: objectProto.faName != undefined ? objectProto.faName : null,
          vscName: objectProto.vscName != undefined ? objectProto.vscName : null,
          file: objectProto.filePtr != undefined ? _NodeReference.unpack(10, objectProto.filePtr, _session) as NodeReference : null,
          fileUrl: objectProto.fileUrl != undefined ? objectProto.fileUrl : null,
          color: objectProto.color != undefined ? _Color.unpack(10, objectProto.color, _session) as Color : null,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: Icon): Uint8Array {
        const proto = this.packObject(object);
        return IconProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Icon {
        const proto = IconProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 400031)] = new IconProtoEncoder();

    class MigrationDefinitionProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: MigrationDefinition): MigrationDefinitionProto {
        const objectProto: Partial<MigrationDefinitionProto> = { metatype: 31000 };
        objectProto.id = object.id;
        objectProto.type = Number(object.type) as any;
        objectProto.name = object.name;
        if (object.icon != null) {
          objectProto.icon = object.icon.pack(10);
        }
        if (object.description != null) {
          objectProto.description = object.description;
        }
        if (object.taggings) {
          const packedTaggings: any[] = [];
          for (const item of object.taggings) {
            packedTaggings.push(item);
          }
          objectProto.taggings = packedTaggings;
        }
        return objectProto as MigrationDefinitionProto;
      }

      unpackObject(objectProto: MigrationDefinitionProto, _session: Session | null): MigrationDefinition {
        const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
        const unpackedTaggings: any[] = [];
        if (objectProto.taggings) {
          for (const item of objectProto.taggings) {
            unpackedTaggings.push(Number(item));
          }
        }
        return new (STRUCT_CLASS_BY_TYPE[31000] as typeof MigrationDefinition)({
          type: Number(objectProto.type) as any,
          id: Number(objectProto.id),
          name: objectProto.name,
          icon: objectProto.icon != undefined ? _Icon.unpack(10, objectProto.icon, _session) as Icon : null,
          description: objectProto.description != undefined ? objectProto.description : null,
          taggings: unpackedTaggings,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: MigrationDefinition): Uint8Array {
        const proto = this.packObject(object);
        return MigrationDefinitionProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): MigrationDefinition {
        const proto = MigrationDefinitionProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 31000)] = new MigrationDefinitionProtoEncoder();

    class MigrationOperationDefinitionProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: MigrationOperationDefinition): MigrationOperationDefinitionProto {
        const objectProto: Partial<MigrationOperationDefinitionProto> = { metatype: 31100 };
        objectProto.id = object.id;
        objectProto.name = object.name;
        if (object.icon != null) {
          objectProto.icon = object.icon.pack(10);
        }
        if (object.description != null) {
          objectProto.description = object.description;
        }
        if (object.taggings) {
          const packedTaggings: any[] = [];
          for (const item of object.taggings) {
            packedTaggings.push(item);
          }
          objectProto.taggings = packedTaggings;
        }
        return objectProto as MigrationOperationDefinitionProto;
      }

      unpackObject(objectProto: MigrationOperationDefinitionProto, _session: Session | null): MigrationOperationDefinition {
        const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
        const unpackedTaggings: any[] = [];
        if (objectProto.taggings) {
          for (const item of objectProto.taggings) {
            unpackedTaggings.push(Number(item));
          }
        }
        return new (STRUCT_CLASS_BY_TYPE[31100] as typeof MigrationOperationDefinition)({
          id: Number(objectProto.id),
          name: objectProto.name,
          icon: objectProto.icon != undefined ? _Icon.unpack(10, objectProto.icon, _session) as Icon : null,
          description: objectProto.description != undefined ? objectProto.description : null,
          taggings: unpackedTaggings,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: MigrationOperationDefinition): Uint8Array {
        const proto = this.packObject(object);
        return MigrationOperationDefinitionProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): MigrationOperationDefinition {
        const proto = MigrationOperationDefinitionProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 31100)] = new MigrationOperationDefinitionProtoEncoder();

    class StringConstraintProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: StringConstraint): StringConstraintProto {
        const objectProto: Partial<StringConstraintProto> = { metatype: 111 };
        if (object.format != null) {
          objectProto.format = Number(object.format) as any;
        }
        if (object.regex != null) {
          objectProto.regex = object.regex;
        }
        if (object.startsWith != null) {
          objectProto.startsWith = object.startsWith;
        }
        if (object.endsWith != null) {
          objectProto.endsWith = object.endsWith;
        }
        return objectProto as StringConstraintProto;
      }

      unpackObject(objectProto: StringConstraintProto, _session: Session | null): StringConstraint {

        return new (STRUCT_CLASS_BY_TYPE[111] as typeof StringConstraint)({
          format: objectProto.format != undefined ? Number(objectProto.format) as any : null,
          regex: objectProto.regex != undefined ? objectProto.regex : null,
          startsWith: objectProto.startsWith != undefined ? objectProto.startsWith : null,
          endsWith: objectProto.endsWith != undefined ? objectProto.endsWith : null,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: StringConstraint): Uint8Array {
        const proto = this.packObject(object);
        return StringConstraintProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): StringConstraint {
        const proto = StringConstraintProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 111)] = new StringConstraintProtoEncoder();

    class NumberConstraintProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: NumberConstraint): NumberConstraintProto {
        const objectProto: Partial<NumberConstraintProto> = { metatype: 110 };
        if (object.format != null) {
          objectProto.format = Number(object.format) as any;
        }
        if (object.minValue != null) {
          objectProto.minValue = object.minValue;
        }
        if (object.maxValue != null) {
          objectProto.maxValue = object.maxValue;
        }
        if (object.stepValue != null) {
          objectProto.stepValue = object.stepValue;
        }
        if (object.precision != null) {
          objectProto.precision = object.precision;
        }
        if (object.scale != null) {
          objectProto.scale = object.scale;
        }
        return objectProto as NumberConstraintProto;
      }

      unpackObject(objectProto: NumberConstraintProto, _session: Session | null): NumberConstraint {

        return new (STRUCT_CLASS_BY_TYPE[110] as typeof NumberConstraint)({
          format: objectProto.format != undefined ? Number(objectProto.format) as any : null,
          minValue: objectProto.minValue != undefined ? objectProto.minValue : null,
          maxValue: objectProto.maxValue != undefined ? objectProto.maxValue : null,
          stepValue: objectProto.stepValue != undefined ? objectProto.stepValue : null,
          precision: objectProto.precision != undefined ? Number(objectProto.precision) : null,
          scale: objectProto.scale != undefined ? Number(objectProto.scale) : null,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: NumberConstraint): Uint8Array {
        const proto = this.packObject(object);
        return NumberConstraintProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): NumberConstraint {
        const proto = NumberConstraintProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 110)] = new NumberConstraintProtoEncoder();

    class CollectionConstraintProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: CollectionConstraint): CollectionConstraintProto {
        const objectProto: Partial<CollectionConstraintProto> = { metatype: 112 };
        if (object.minLength != null) {
          objectProto.minLength = object.minLength;
        }
        if (object.maxLength != null) {
          objectProto.maxLength = object.maxLength;
        }
        return objectProto as CollectionConstraintProto;
      }

      unpackObject(objectProto: CollectionConstraintProto, _session: Session | null): CollectionConstraint {

        return new (STRUCT_CLASS_BY_TYPE[112] as typeof CollectionConstraint)({
          minLength: objectProto.minLength != undefined ? Number(objectProto.minLength) : null,
          maxLength: objectProto.maxLength != undefined ? Number(objectProto.maxLength) : null,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: CollectionConstraint): Uint8Array {
        const proto = this.packObject(object);
        return CollectionConstraintProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): CollectionConstraint {
        const proto = CollectionConstraintProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 112)] = new CollectionConstraintProtoEncoder();

    class TypeProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Type): TypeProto {
        const objectProto: Partial<TypeProto> = { metatype: 101 };
        if (object.name != null) {
          objectProto.name = object.name;
        }
        objectProto.cardinality = Number(object.cardinality) as any;
        objectProto.scalarType = Number(object.scalarType) as any;
        if (object.primitiveType != null) {
          objectProto.primitiveType = Number(object.primitiveType) as any;
        }
        if (object.enumType != null) {
          objectProto.enumType = Number(object.enumType) as any;
        }
        if (object.nodeTypes) {
          const packedNodeTypes: any[] = [];
          for (const item of object.nodeTypes) {
            packedNodeTypes.push(Number(item) as any);
          }
          objectProto.nodeTypes = packedNodeTypes;
        }
        if (object.structType != null) {
          objectProto.structType = Number(object.structType) as any;
        }
        if (object.keyType != null) {
          objectProto.keyType = object.keyType.pack(10);
        }
        if (object.value != null) {
          objectProto.value = object.value.pack(10);
        }
        if (object.valueFactory != null) {
          objectProto.valueFactory = Number(object.valueFactory) as any;
        }
        if (object.collectionConstraint != null) {
          objectProto.collectionConstraint = object.collectionConstraint.pack(10);
        }
        if (object.stringConstraint != null) {
          objectProto.stringConstraint = object.stringConstraint.pack(10);
        }
        if (object.numberConstraint != null) {
          objectProto.numberConstraint = object.numberConstraint.pack(10);
        }
        if (object.isRequired != null) {
          objectProto.isRequired = object.isRequired;
        }
        if (object.isMain != null) {
          objectProto.isMain = object.isMain;
        }
        return objectProto as TypeProto;
      }

      unpackObject(objectProto: TypeProto, _session: Session | null): Type {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _Type = STRUCT_CLASS_BY_TYPE[101] as typeof Type;
        const _NumberConstraint = STRUCT_CLASS_BY_TYPE[110] as typeof NumberConstraint;
        const _StringConstraint = STRUCT_CLASS_BY_TYPE[111] as typeof StringConstraint;
        const _CollectionConstraint = STRUCT_CLASS_BY_TYPE[112] as typeof CollectionConstraint;
        const unpackedNodeTypes: any[] = [];
        if (objectProto.nodeTypes) {
          for (const item of objectProto.nodeTypes) {
            unpackedNodeTypes.push(Number(item) as any);
          }
        }
        return new (STRUCT_CLASS_BY_TYPE[101] as typeof Type)({
          name: objectProto.name != undefined ? objectProto.name : null,
          cardinality: Number(objectProto.cardinality) as any,
          scalarType: Number(objectProto.scalarType) as any,
          primitiveType: objectProto.primitiveType != undefined ? Number(objectProto.primitiveType) as any : null,
          enumType: objectProto.enumType != undefined ? Number(objectProto.enumType) as any : null,
          nodeTypes: unpackedNodeTypes,
          structType: objectProto.structType != undefined ? Number(objectProto.structType) as any : null,
          keyType: objectProto.keyType != undefined ? _Type.unpack(10, objectProto.keyType, _session) as Type : null,
          value: objectProto.value != undefined ? _Value.unpack(10, objectProto.value, _session) as Value : null,
          valueFactory: objectProto.valueFactory != undefined ? Number(objectProto.valueFactory) as any : null,
          collectionConstraint: objectProto.collectionConstraint != undefined ? _CollectionConstraint.unpack(10, objectProto.collectionConstraint, _session) as CollectionConstraint : null,
          stringConstraint: objectProto.stringConstraint != undefined ? _StringConstraint.unpack(10, objectProto.stringConstraint, _session) as StringConstraint : null,
          numberConstraint: objectProto.numberConstraint != undefined ? _NumberConstraint.unpack(10, objectProto.numberConstraint, _session) as NumberConstraint : null,
          isRequired: objectProto.isRequired != undefined ? objectProto.isRequired : null,
          isMain: objectProto.isMain != undefined ? objectProto.isMain : null,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: Type): Uint8Array {
        const proto = this.packObject(object);
        return TypeProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Type {
        const proto = TypeProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 101)] = new TypeProtoEncoder();

    class ValueProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Value): ValueProto {
        const objectProto: Partial<ValueProto> = { metatype: 100 };
        objectProto.type = object.type.pack(10);
        if (object.value != null) {
          objectProto.value = packProtoJson(object.value);
        }
        return objectProto as ValueProto;
      }

      unpackObject(objectProto: ValueProto, _session: Session | null): Value {
        const _Type = STRUCT_CLASS_BY_TYPE[101] as typeof Type;
        return new (STRUCT_CLASS_BY_TYPE[100] as typeof Value)({
          type: _Type.unpack(10, objectProto.type, _session) as Type,
          value: objectProto.value != undefined ? unpackProtoJson(objectProto.value!) : null,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: Value): Uint8Array {
        const proto = this.packObject(object);
        return ValueProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Value {
        const proto = ValueProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 100)] = new ValueProtoEncoder();

    class FunctionProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Function): FunctionProto {
        const objectProto: Partial<FunctionProto> = { metatype: 201 };
        objectProto.type = Number(object.type) as any;
        objectProto.left = object.left.pack(10);
        if (object.right != null) {
          objectProto.right = object.right.pack(10);
        }
        return objectProto as FunctionProto;
      }

      unpackObject(objectProto: FunctionProto, _session: Session | null): Function {
        const _Expression = STRUCT_CLASS_BY_TYPE[200] as typeof Expression;
        return new (STRUCT_CLASS_BY_TYPE[201] as typeof Function)({
          type: Number(objectProto.type) as any,
          left: _Expression.unpack(10, objectProto.left, _session) as Expression,
          right: objectProto.right != undefined ? _Expression.unpack(10, objectProto.right, _session) as Expression : null,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: Function): Uint8Array {
        const proto = this.packObject(object);
        return FunctionProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Function {
        const proto = FunctionProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 201)] = new FunctionProtoEncoder();

    class ConditionProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Condition): ConditionProto {
        const objectProto: Partial<ConditionProto> = { metatype: 204 };
        objectProto.type = Number(object.type) as any;
        objectProto.left = object.left.pack(10);
        if (object.right != null) {
          objectProto.right = object.right.pack(10);
        }
        return objectProto as ConditionProto;
      }

      unpackObject(objectProto: ConditionProto, _session: Session | null): Condition {
        const _Expression = STRUCT_CLASS_BY_TYPE[200] as typeof Expression;
        return new (STRUCT_CLASS_BY_TYPE[204] as typeof Condition)({
          type: Number(objectProto.type) as any,
          left: _Expression.unpack(10, objectProto.left, _session) as Expression,
          right: objectProto.right != undefined ? _Expression.unpack(10, objectProto.right, _session) as Expression : null,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: Condition): Uint8Array {
        const proto = this.packObject(object);
        return ConditionProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Condition {
        const proto = ConditionProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 204)] = new ConditionProtoEncoder();

    class AggregationProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Aggregation): AggregationProto {
        const objectProto: Partial<AggregationProto> = { metatype: 203 };
        objectProto.type = Number(object.type) as any;
        if (object.expression != null) {
          objectProto.expression = object.expression.pack(10);
        }
        return objectProto as AggregationProto;
      }

      unpackObject(objectProto: AggregationProto, _session: Session | null): Aggregation {
        const _Expression = STRUCT_CLASS_BY_TYPE[200] as typeof Expression;
        return new (STRUCT_CLASS_BY_TYPE[203] as typeof Aggregation)({
          type: Number(objectProto.type) as any,
          expression: objectProto.expression != undefined ? _Expression.unpack(10, objectProto.expression, _session) as Expression : null,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: Aggregation): Uint8Array {
        const proto = this.packObject(object);
        return AggregationProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Aggregation {
        const proto = AggregationProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 203)] = new AggregationProtoEncoder();

    class ExpressionProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Expression): ExpressionProto {
        const objectProto: Partial<ExpressionProto> = { metatype: 200 };
        objectProto.type = Number(object.type) as any;
        if (object.literal != null) {
          objectProto.literal = object.literal.pack(10);
        }
        if (object.attribute != null) {
          objectProto.attribute = object.attribute.pack(10);
        }
        if (object.condition != null) {
          objectProto.condition = object.condition.pack(10);
        }
        if (object.function != null) {
          objectProto.function = object.function.pack(10);
        }
        if (object.aggregation != null) {
          objectProto.aggregation = object.aggregation.pack(10);
        }
        return objectProto as ExpressionProto;
      }

      unpackObject(objectProto: ExpressionProto, _session: Session | null): Expression {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _Function = STRUCT_CLASS_BY_TYPE[201] as typeof Function;
        const _Aggregation = STRUCT_CLASS_BY_TYPE[203] as typeof Aggregation;
        const _Condition = STRUCT_CLASS_BY_TYPE[204] as typeof Condition;
        const _PropertyReference = STRUCT_CLASS_BY_TYPE[1001] as typeof PropertyReference;
        return new (STRUCT_CLASS_BY_TYPE[200] as typeof Expression)({
          type: Number(objectProto.type) as any,
          literal: objectProto.literal != undefined ? _Value.unpack(10, objectProto.literal, _session) as Value : null,
          attribute: objectProto.attribute != undefined ? _PropertyReference.unpack(10, objectProto.attribute, _session) as PropertyReference : null,
          condition: objectProto.condition != undefined ? _Condition.unpack(10, objectProto.condition, _session) as Condition : null,
          function: objectProto.function != undefined ? _Function.unpack(10, objectProto.function, _session) as Function : null,
          aggregation: objectProto.aggregation != undefined ? _Aggregation.unpack(10, objectProto.aggregation, _session) as Aggregation : null,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: Expression): Uint8Array {
        const proto = this.packObject(object);
        return ExpressionProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Expression {
        const proto = ExpressionProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 200)] = new ExpressionProtoEncoder();

    class SortProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Sort): SortProto {
        const objectProto: Partial<SortProto> = { metatype: 205 };
        objectProto.type = Number(object.type) as any;
        objectProto.by = object.by.pack(10);
        if (object.mode != null) {
          objectProto.mode = Number(object.mode) as any;
        }
        return objectProto as SortProto;
      }

      unpackObject(objectProto: SortProto, _session: Session | null): Sort {
        const _Expression = STRUCT_CLASS_BY_TYPE[200] as typeof Expression;
        return new (STRUCT_CLASS_BY_TYPE[205] as typeof Sort)({
          type: Number(objectProto.type) as any,
          by: _Expression.unpack(10, objectProto.by, _session) as Expression,
          mode: objectProto.mode != undefined ? Number(objectProto.mode) as any : null,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: Sort): Uint8Array {
        const proto = this.packObject(object);
        return SortProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Sort {
        const proto = SortProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 205)] = new SortProtoEncoder();

    class SelectProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Select): SelectProto {
        const objectProto: Partial<SelectProto> = { metatype: 206 };
        if (object.attributes) {
          const packedAttributes: any[] = [];
          for (const item of object.attributes) {
            packedAttributes.push(item.pack(10));
          }
          objectProto.attributes = packedAttributes;
        }
        return objectProto as SelectProto;
      }

      unpackObject(objectProto: SelectProto, _session: Session | null): Select {
        const _PropertyReference = STRUCT_CLASS_BY_TYPE[1001] as typeof PropertyReference;
        const unpackedAttributes: any[] = [];
        if (objectProto.attributes) {
          for (const item of objectProto.attributes) {
            unpackedAttributes.push(_PropertyReference.unpack(10, item, _session) as PropertyReference);
          }
        }
        return new (STRUCT_CLASS_BY_TYPE[206] as typeof Select)({
          attributes: unpackedAttributes,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: Select): Uint8Array {
        const proto = this.packObject(object);
        return SelectProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Select {
        const proto = SelectProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 206)] = new SelectProtoEncoder();

    class JoinProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Join): JoinProto {
        const objectProto: Partial<JoinProto> = { metatype: 202 };
        objectProto.type = Number(object.type) as any;
        objectProto.recursive = object.recursive;
        if (object.depth != null) {
          objectProto.depth = object.depth;
        }
        if (object.on != null) {
          objectProto.on = object.on.pack(10);
        }
        return objectProto as JoinProto;
      }

      unpackObject(objectProto: JoinProto, _session: Session | null): Join {
        const _Condition = STRUCT_CLASS_BY_TYPE[204] as typeof Condition;
        return new (STRUCT_CLASS_BY_TYPE[202] as typeof Join)({
          type: Number(objectProto.type) as any,
          recursive: objectProto.recursive,
          depth: objectProto.depth != undefined ? Number(objectProto.depth) : null,
          on: objectProto.on != undefined ? _Condition.unpack(10, objectProto.on, _session) as Condition : null,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: Join): Uint8Array {
        const proto = this.packObject(object);
        return JoinProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Join {
        const proto = JoinProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 202)] = new JoinProtoEncoder();

    class QueryProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Query): QueryProto {
        const objectProto: Partial<QueryProto> = { metatype: 300 };
        objectProto.id = String(object.id);
        objectProto.type = Number(object.type) as any;
        objectProto.domain = Number(object.domain) as any;
        objectProto.name = object.name;
        objectProto.definition = object.definition.pack(10);
        if (object.subqueries) {
          const packedSubqueries: any[] = [];
          for (const item of object.subqueries) {
            packedSubqueries.push(item.pack(10));
          }
          objectProto.subqueries = packedSubqueries;
        }
        if (object.join != null) {
          objectProto.join = object.join.pack(10);
        }
        if (object.select != null) {
          objectProto.select = object.select.pack(10);
        }
        if (object.where != null) {
          objectProto.where = object.where.pack(10);
        }
        if (object.having != null) {
          objectProto.having = object.having.pack(10);
        }
        if (object.groupBy) {
          const packedGroupBy: any[] = [];
          for (const item of object.groupBy) {
            packedGroupBy.push(item.pack(10));
          }
          objectProto.groupBy = packedGroupBy;
        }
        if (object.aggregation != null) {
          objectProto.aggregation = object.aggregation.pack(10);
        }
        if (object.sort) {
          const packedSort: any[] = [];
          for (const item of object.sort) {
            packedSort.push(item.pack(10));
          }
          objectProto.sort = packedSort;
        }
        if (object.limit != null) {
          objectProto.limit = object.limit;
        }
        if (object.offset != null) {
          objectProto.offset = object.offset;
        }
        return objectProto as QueryProto;
      }

      unpackObject(objectProto: QueryProto, _session: Session | null): Query {
        const _NodeDefinitionReference = STRUCT_CLASS_BY_TYPE[13] as typeof NodeDefinitionReference;
        const _Expression = STRUCT_CLASS_BY_TYPE[200] as typeof Expression;
        const _Join = STRUCT_CLASS_BY_TYPE[202] as typeof Join;
        const _Aggregation = STRUCT_CLASS_BY_TYPE[203] as typeof Aggregation;
        const _Condition = STRUCT_CLASS_BY_TYPE[204] as typeof Condition;
        const _Sort = STRUCT_CLASS_BY_TYPE[205] as typeof Sort;
        const _Select = STRUCT_CLASS_BY_TYPE[206] as typeof Select;
        const _Query = STRUCT_CLASS_BY_TYPE[300] as typeof Query;
        const unpackedSubqueries: any[] = [];
        if (objectProto.subqueries) {
          for (const item of objectProto.subqueries) {
            unpackedSubqueries.push(_Query.unpack(10, item, _session) as Query);
          }
        }
        const unpackedGroupBy: any[] = [];
        if (objectProto.groupBy) {
          for (const item of objectProto.groupBy) {
            unpackedGroupBy.push(_Expression.unpack(10, item, _session) as Expression);
          }
        }
        const unpackedSort: any[] = [];
        if (objectProto.sort) {
          for (const item of objectProto.sort) {
            unpackedSort.push(_Sort.unpack(10, item, _session) as Sort);
          }
        }
        return new (STRUCT_CLASS_BY_TYPE[300] as typeof Query)({
          id: String(objectProto.id),
          type: Number(objectProto.type) as any,
          domain: Number(objectProto.domain) as any,
          name: objectProto.name,
          definition: _NodeDefinitionReference.unpack(10, objectProto.definition, _session) as NodeDefinitionReference,
          subqueries: unpackedSubqueries,
          join: objectProto.join != undefined ? _Join.unpack(10, objectProto.join, _session) as Join : null,
          select: objectProto.select != undefined ? _Select.unpack(10, objectProto.select, _session) as Select : null,
          where: objectProto.where != undefined ? _Condition.unpack(10, objectProto.where, _session) as Condition : null,
          having: objectProto.having != undefined ? _Condition.unpack(10, objectProto.having, _session) as Condition : null,
          groupBy: unpackedGroupBy,
          aggregation: objectProto.aggregation != undefined ? _Aggregation.unpack(10, objectProto.aggregation, _session) as Aggregation : null,
          sort: unpackedSort,
          limit: objectProto.limit != undefined ? Number(objectProto.limit) : null,
          offset: objectProto.offset != undefined ? Number(objectProto.offset) : null,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: Query): Uint8Array {
        const proto = this.packObject(object);
        return QueryProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Query {
        const proto = QueryProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 300)] = new QueryProtoEncoder();

    class DatumProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Datum): DatumProto {
        const objectProto: Partial<DatumProto> = { metatype: 2 };
        objectProto.definitionPtr = object.definitionPtr.pack(10);
        if (object.customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object.customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        return objectProto as DatumProto;
      }

      unpackObject(objectProto: DatumProto, _session: Session | null): Datum {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (STRUCT_CLASS_BY_TYPE[2] as typeof Datum)({
          definition: _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference,
          customValues: unpackedCustomValues,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: Datum): Uint8Array {
        const proto = this.packObject(object);
        return DatumProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Datum {
        const proto = DatumProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 2)] = new DatumProtoEncoder();

    class DatumMutableProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: DatumMutable): DatumMutableProto {
        const objectProto: Partial<DatumMutableProto> = { metatype: 3 };
        objectProto.definitionPtr = object.definitionPtr.pack(10);
        if (object.customValues) {
          objectProto.customValues = {} as any;
          for (const [key, value] of Object.entries(object.customValues) ) {
            objectProto.customValues![String(key)] = value.pack(10);
          }
        }
        return objectProto as DatumMutableProto;
      }

      unpackObject(objectProto: DatumMutableProto, _session: Session | null): DatumMutable {
        const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const unpackedCustomValues = {} as any;
        if (objectProto.customValues) {
          for (const [key, value] of Object.entries(objectProto.customValues)) {
            unpackedCustomValues.set(String(key), _Value.unpack(10, (value as any), _session) as Value);
          }
        }
        return new (STRUCT_CLASS_BY_TYPE[3] as typeof DatumMutable)({
          definition: _NodeReference.unpack(10, objectProto.definitionPtr, _session) as NodeReference,
          customValues: unpackedCustomValues,
          _session,
        });
      }

      packObjectBytes(object: DatumMutable): Uint8Array {
        const proto = this.packObject(object);
        return DatumMutableProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): DatumMutable {
        const proto = DatumMutableProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 3)] = new DatumMutableProtoEncoder();

    class TextSpanProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: TextSpan): TextSpanProto {
        const objectProto: Partial<TextSpanProto> = { metatype: 400021 };
        objectProto.type = Number(object.type) as any;
        if (object.content != null) {
          objectProto.content = object.content;
        }
        if (object.nodePtr != null) {
          objectProto.nodePtr = object.nodePtr.pack(10);
        }
        if (object.url != null) {
          objectProto.url = object.url;
        }
        if (object.isBold != null) {
          objectProto.isBold = object.isBold;
        }
        if (object.isItalic != null) {
          objectProto.isItalic = object.isItalic;
        }
        if (object.isStrikethrough != null) {
          objectProto.isStrikethrough = object.isStrikethrough;
        }
        if (object.isUnderline != null) {
          objectProto.isUnderline = object.isUnderline;
        }
        if (object.isCode != null) {
          objectProto.isCode = object.isCode;
        }
        return objectProto as TextSpanProto;
      }

      unpackObject(objectProto: TextSpanProto, _session: Session | null): TextSpan {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (STRUCT_CLASS_BY_TYPE[400021] as typeof TextSpan)({
          type: Number(objectProto.type) as any,
          content: objectProto.content != undefined ? objectProto.content : null,
          node: objectProto.nodePtr != undefined ? _NodeReference.unpack(10, objectProto.nodePtr, _session) as NodeReference : null,
          url: objectProto.url != undefined ? objectProto.url : null,
          isBold: objectProto.isBold != undefined ? objectProto.isBold : null,
          isItalic: objectProto.isItalic != undefined ? objectProto.isItalic : null,
          isStrikethrough: objectProto.isStrikethrough != undefined ? objectProto.isStrikethrough : null,
          isUnderline: objectProto.isUnderline != undefined ? objectProto.isUnderline : null,
          isCode: objectProto.isCode != undefined ? objectProto.isCode : null,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: TextSpan): Uint8Array {
        const proto = this.packObject(object);
        return TextSpanProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): TextSpan {
        const proto = TextSpanProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 400021)] = new TextSpanProtoEncoder();

    class TextProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Text): TextProto {
        const objectProto: Partial<TextProto> = { metatype: 400020 };
        if (object.spans) {
          const packedSpans: any[] = [];
          for (const item of object.spans) {
            packedSpans.push(item.pack(10));
          }
          objectProto.spans = packedSpans;
        }
        if (object.isBold != null) {
          objectProto.isBold = object.isBold;
        }
        if (object.isItalic != null) {
          objectProto.isItalic = object.isItalic;
        }
        if (object.isStrikethrough != null) {
          objectProto.isStrikethrough = object.isStrikethrough;
        }
        if (object.isUnderline != null) {
          objectProto.isUnderline = object.isUnderline;
        }
        if (object.isCode != null) {
          objectProto.isCode = object.isCode;
        }
        return objectProto as TextProto;
      }

      unpackObject(objectProto: TextProto, _session: Session | null): Text {
        const _TextSpan = STRUCT_CLASS_BY_TYPE[400021] as typeof TextSpan;
        const unpackedSpans: any[] = [];
        if (objectProto.spans) {
          for (const item of objectProto.spans) {
            unpackedSpans.push(_TextSpan.unpack(10, item, _session) as TextSpan);
          }
        }
        return new (STRUCT_CLASS_BY_TYPE[400020] as typeof Text)({
          spans: unpackedSpans,
          isBold: objectProto.isBold != undefined ? objectProto.isBold : null,
          isItalic: objectProto.isItalic != undefined ? objectProto.isItalic : null,
          isStrikethrough: objectProto.isStrikethrough != undefined ? objectProto.isStrikethrough : null,
          isUnderline: objectProto.isUnderline != undefined ? objectProto.isUnderline : null,
          isCode: objectProto.isCode != undefined ? objectProto.isCode : null,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: Text): Uint8Array {
        const proto = this.packObject(object);
        return TextProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Text {
        const proto = TextProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 400020)] = new TextProtoEncoder();

    class ColorProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Color): ColorProto {
        const objectProto: Partial<ColorProto> = { metatype: 2100300 };
        objectProto.type = Number(object.type) as any;
        if (object.stylePtr != null) {
          objectProto.stylePtr = object.stylePtr.pack(10);
        }
        if (object.hue != null) {
          objectProto.hue = Number(object.hue) as any;
        }
        if (object.shade != null) {
          objectProto.shade = Number(object.shade) as any;
        }
        if (object.intent != null) {
          objectProto.intent = Number(object.intent) as any;
        }
        if (object.x != null) {
          objectProto.x = object.x;
        }
        if (object.y != null) {
          objectProto.y = object.y;
        }
        if (object.z != null) {
          objectProto.z = object.z;
        }
        if (object.alpha != null) {
          objectProto.alpha = object.alpha;
        }
        return objectProto as ColorProto;
      }

      unpackObject(objectProto: ColorProto, _session: Session | null): Color {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        return new (STRUCT_CLASS_BY_TYPE[2100300] as typeof Color)({
          type: Number(objectProto.type) as any,
          style: objectProto.stylePtr != undefined ? _NodeReference.unpack(10, objectProto.stylePtr, _session) as NodeReference : null,
          hue: objectProto.hue != undefined ? Number(objectProto.hue) as any : null,
          shade: objectProto.shade != undefined ? Number(objectProto.shade) as any : null,
          intent: objectProto.intent != undefined ? Number(objectProto.intent) as any : null,
          x: objectProto.x != undefined ? objectProto.x : null,
          y: objectProto.y != undefined ? objectProto.y : null,
          z: objectProto.z != undefined ? objectProto.z : null,
          alpha: objectProto.alpha != undefined ? objectProto.alpha : null,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: Color): Uint8Array {
        const proto = this.packObject(object);
        return ColorProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Color {
        const proto = ColorProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 2100300)] = new ColorProtoEncoder();

    class BorderProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Border): BorderProto {
        const objectProto: Partial<BorderProto> = { metatype: 2100600 };
        objectProto.type = Number(object.type) as any;
        if (object.color != null) {
          objectProto.color = object.color.pack(10);
        }
        if (object.width != null) {
          objectProto.width = object.width.pack(10);
        }
        if (object.stylePtr != null) {
          objectProto.stylePtr = object.stylePtr.pack(10);
        }
        return objectProto as BorderProto;
      }

      unpackObject(objectProto: BorderProto, _session: Session | null): Border {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Color = STRUCT_CLASS_BY_TYPE[2100300] as typeof Color;
        const _Inset2 = STRUCT_CLASS_BY_TYPE[2400023] as typeof Inset2;
        return new (STRUCT_CLASS_BY_TYPE[2100600] as typeof Border)({
          type: Number(objectProto.type) as any,
          color: objectProto.color != undefined ? _Color.unpack(10, objectProto.color, _session) as Color : null,
          width: objectProto.width != undefined ? _Inset2.unpack(10, objectProto.width, _session) as Inset2 : null,
          style: objectProto.stylePtr != undefined ? _NodeReference.unpack(10, objectProto.stylePtr, _session) as NodeReference : null,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: Border): Uint8Array {
        const proto = this.packObject(object);
        return BorderProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Border {
        const proto = BorderProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 2100600)] = new BorderProtoEncoder();

    class GradientStopProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: GradientStop): GradientStopProto {
        const objectProto: Partial<GradientStopProto> = { metatype: 2100801 };
        if (object.color != null) {
          objectProto.color = object.color.pack(10);
        }
        objectProto.position = object.position;
        return objectProto as GradientStopProto;
      }

      unpackObject(objectProto: GradientStopProto, _session: Session | null): GradientStop {
        const _Color = STRUCT_CLASS_BY_TYPE[2100300] as typeof Color;
        return new (STRUCT_CLASS_BY_TYPE[2100801] as typeof GradientStop)({
          color: objectProto.color != undefined ? _Color.unpack(10, objectProto.color, _session) as Color : null,
          position: objectProto.position,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: GradientStop): Uint8Array {
        const proto = this.packObject(object);
        return GradientStopProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): GradientStop {
        const proto = GradientStopProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 2100801)] = new GradientStopProtoEncoder();

    class GradientProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Gradient): GradientProto {
        const objectProto: Partial<GradientProto> = { metatype: 2100800 };
        objectProto.type = Number(object.type) as any;
        if (object.stylePtr != null) {
          objectProto.stylePtr = object.stylePtr.pack(10);
        }
        if (object.angle != null) {
          objectProto.angle = object.angle;
        }
        if (object.stops) {
          const packedStops: any[] = [];
          for (const item of object.stops) {
            packedStops.push(item.pack(10));
          }
          objectProto.stops = packedStops;
        }
        if (object.centerAnchor != null) {
          objectProto.centerAnchor = object.centerAnchor.pack(10);
        }
        return objectProto as GradientProto;
      }

      unpackObject(objectProto: GradientProto, _session: Session | null): Gradient {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _GradientStop = STRUCT_CLASS_BY_TYPE[2100801] as typeof GradientStop;
        const _Axis2 = STRUCT_CLASS_BY_TYPE[2400025] as typeof Axis2;
        const unpackedStops: any[] = [];
        if (objectProto.stops) {
          for (const item of objectProto.stops) {
            unpackedStops.push(_GradientStop.unpack(10, item, _session) as GradientStop);
          }
        }
        return new (STRUCT_CLASS_BY_TYPE[2100800] as typeof Gradient)({
          type: Number(objectProto.type) as any,
          style: objectProto.stylePtr != undefined ? _NodeReference.unpack(10, objectProto.stylePtr, _session) as NodeReference : null,
          angle: objectProto.angle != undefined ? objectProto.angle : null,
          stops: unpackedStops,
          centerAnchor: objectProto.centerAnchor != undefined ? _Axis2.unpack(10, objectProto.centerAnchor, _session) as Axis2 : null,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: Gradient): Uint8Array {
        const proto = this.packObject(object);
        return GradientProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Gradient {
        const proto = GradientProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 2100800)] = new GradientProtoEncoder();

    class FillProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Fill): FillProto {
        const objectProto: Partial<FillProto> = { metatype: 2100400 };
        objectProto.type = Number(object.type) as any;
        if (object.stylePtr != null) {
          objectProto.stylePtr = object.stylePtr.pack(10);
        }
        if (object.color != null) {
          objectProto.color = object.color.pack(10);
        }
        if (object.gradient != null) {
          objectProto.gradient = object.gradient.pack(10);
        }
        if (object.imagePtr != null) {
          objectProto.imagePtr = object.imagePtr.pack(10);
        }
        if (object.position != null) {
          objectProto.position = Number(object.position) as any;
        }
        if (object.size != null) {
          objectProto.size = Number(object.size) as any;
        }
        return objectProto as FillProto;
      }

      unpackObject(objectProto: FillProto, _session: Session | null): Fill {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Color = STRUCT_CLASS_BY_TYPE[2100300] as typeof Color;
        const _Gradient = STRUCT_CLASS_BY_TYPE[2100800] as typeof Gradient;
        return new (STRUCT_CLASS_BY_TYPE[2100400] as typeof Fill)({
          type: Number(objectProto.type) as any,
          style: objectProto.stylePtr != undefined ? _NodeReference.unpack(10, objectProto.stylePtr, _session) as NodeReference : null,
          color: objectProto.color != undefined ? _Color.unpack(10, objectProto.color, _session) as Color : null,
          gradient: objectProto.gradient != undefined ? _Gradient.unpack(10, objectProto.gradient, _session) as Gradient : null,
          image: objectProto.imagePtr != undefined ? _NodeReference.unpack(10, objectProto.imagePtr, _session) as NodeReference : null,
          position: objectProto.position != undefined ? Number(objectProto.position) as any : null,
          size: objectProto.size != undefined ? Number(objectProto.size) as any : null,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: Fill): Uint8Array {
        const proto = this.packObject(object);
        return FillProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Fill {
        const proto = FillProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 2100400)] = new FillProtoEncoder();

    class FontProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Font): FontProto {
        const objectProto: Partial<FontProto> = { metatype: 2100500 };
        objectProto.type = Number(object.type) as any;
        if (object.stylePtr != null) {
          objectProto.stylePtr = object.stylePtr.pack(10);
        }
        if (object.weight != null) {
          objectProto.weight = Number(object.weight) as any;
        }
        if (object.color != null) {
          objectProto.color = object.color.pack(10);
        }
        if (object.size != null) {
          objectProto.size = Number(object.size) as any;
        }
        if (object.align != null) {
          objectProto.align = Number(object.align) as any;
        }
        if (object.lineHeight != null) {
          objectProto.lineHeight = object.lineHeight.pack(10);
        }
        if (object.letterSpacing != null) {
          objectProto.letterSpacing = object.letterSpacing.pack(10);
        }
        if (object.decoration != null) {
          objectProto.decoration = Number(object.decoration) as any;
        }
        if (object.transform != null) {
          objectProto.transform = Number(object.transform) as any;
        }
        return objectProto as FontProto;
      }

      unpackObject(objectProto: FontProto, _session: Session | null): Font {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Length = STRUCT_CLASS_BY_TYPE[1800001] as typeof Length;
        const _Fill = STRUCT_CLASS_BY_TYPE[2100400] as typeof Fill;
        return new (STRUCT_CLASS_BY_TYPE[2100500] as typeof Font)({
          type: Number(objectProto.type) as any,
          style: objectProto.stylePtr != undefined ? _NodeReference.unpack(10, objectProto.stylePtr, _session) as NodeReference : null,
          weight: objectProto.weight != undefined ? Number(objectProto.weight) as any : null,
          color: objectProto.color != undefined ? _Fill.unpack(10, objectProto.color, _session) as Fill : null,
          size: objectProto.size != undefined ? Number(objectProto.size) as any : null,
          align: objectProto.align != undefined ? Number(objectProto.align) as any : null,
          lineHeight: objectProto.lineHeight != undefined ? _Length.unpack(10, objectProto.lineHeight, _session) as Length : null,
          letterSpacing: objectProto.letterSpacing != undefined ? _Length.unpack(10, objectProto.letterSpacing, _session) as Length : null,
          decoration: objectProto.decoration != undefined ? Number(objectProto.decoration) as any : null,
          transform: objectProto.transform != undefined ? Number(objectProto.transform) as any : null,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: Font): Uint8Array {
        const proto = this.packObject(object);
        return FontProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Font {
        const proto = FontProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 2100500)] = new FontProtoEncoder();

    class ShadowProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Shadow): ShadowProto {
        const objectProto: Partial<ShadowProto> = { metatype: 2100700 };
        objectProto.type = Number(object.type) as any;
        if (object.stylePtr != null) {
          objectProto.stylePtr = object.stylePtr.pack(10);
        }
        if (object.color != null) {
          objectProto.color = object.color.pack(10);
        }
        objectProto.position = Number(object.position) as any;
        if (object.offset != null) {
          objectProto.offset = object.offset.pack(10);
        }
        if (object.blur != null) {
          objectProto.blur = object.blur;
        }
        if (object.spread != null) {
          objectProto.spread = object.spread;
        }
        if (object.diffusion != null) {
          objectProto.diffusion = object.diffusion;
        }
        return objectProto as ShadowProto;
      }

      unpackObject(objectProto: ShadowProto, _session: Session | null): Shadow {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Color = STRUCT_CLASS_BY_TYPE[2100300] as typeof Color;
        const _Axis2 = STRUCT_CLASS_BY_TYPE[2400025] as typeof Axis2;
        return new (STRUCT_CLASS_BY_TYPE[2100700] as typeof Shadow)({
          type: Number(objectProto.type) as any,
          style: objectProto.stylePtr != undefined ? _NodeReference.unpack(10, objectProto.stylePtr, _session) as NodeReference : null,
          color: objectProto.color != undefined ? _Color.unpack(10, objectProto.color, _session) as Color : null,
          position: Number(objectProto.position) as any,
          offset: objectProto.offset != undefined ? _Axis2.unpack(10, objectProto.offset, _session) as Axis2 : null,
          blur: objectProto.blur != undefined ? Number(objectProto.blur) : null,
          spread: objectProto.spread != undefined ? Number(objectProto.spread) : null,
          diffusion: objectProto.diffusion != undefined ? objectProto.diffusion : null,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: Shadow): Uint8Array {
        const proto = this.packObject(object);
        return ShadowProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Shadow {
        const proto = ShadowProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 2100700)] = new ShadowProtoEncoder();

    class StrokeProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Stroke): StrokeProto {
        const objectProto: Partial<StrokeProto> = { metatype: 2101100 };
        objectProto.type = Number(object.type) as any;
        objectProto.size = object.size;
        objectProto.thinning = object.thinning;
        objectProto.smoothing = object.smoothing;
        objectProto.streamline = object.streamline;
        objectProto.easing = Number(object.easing) as any;
        if (object.color != null) {
          objectProto.color = object.color.pack(10);
        }
        if (object.start != null) {
          objectProto.start = object.start.pack(10);
        }
        if (object.end != null) {
          objectProto.end = object.end.pack(10);
        }
        return objectProto as StrokeProto;
      }

      unpackObject(objectProto: StrokeProto, _session: Session | null): Stroke {
        const _Color = STRUCT_CLASS_BY_TYPE[2100300] as typeof Color;
        const _StrokeCap = STRUCT_CLASS_BY_TYPE[2101101] as typeof StrokeCap;
        return new (STRUCT_CLASS_BY_TYPE[2101100] as typeof Stroke)({
          type: Number(objectProto.type) as any,
          size: Number(objectProto.size),
          thinning: objectProto.thinning,
          smoothing: objectProto.smoothing,
          streamline: objectProto.streamline,
          easing: Number(objectProto.easing) as any,
          color: objectProto.color != undefined ? _Color.unpack(10, objectProto.color, _session) as Color : null,
          start: objectProto.start != undefined ? _StrokeCap.unpack(10, objectProto.start, _session) as StrokeCap : null,
          end: objectProto.end != undefined ? _StrokeCap.unpack(10, objectProto.end, _session) as StrokeCap : null,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: Stroke): Uint8Array {
        const proto = this.packObject(object);
        return StrokeProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Stroke {
        const proto = StrokeProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 2101100)] = new StrokeProtoEncoder();

    class StrokeCapProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: StrokeCap): StrokeCapProto {
        const objectProto: Partial<StrokeCapProto> = { metatype: 2101101 };
        objectProto.cap = object.cap;
        objectProto.taper = object.taper;
        objectProto.easing = Number(object.easing) as any;
        return objectProto as StrokeCapProto;
      }

      unpackObject(objectProto: StrokeCapProto, _session: Session | null): StrokeCap {

        return new (STRUCT_CLASS_BY_TYPE[2101101] as typeof StrokeCap)({
          cap: objectProto.cap,
          taper: objectProto.taper,
          easing: Number(objectProto.easing) as any,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: StrokeCap): Uint8Array {
        const proto = this.packObject(object);
        return StrokeCapProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): StrokeCap {
        const proto = StrokeCapProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 2101101)] = new StrokeCapProtoEncoder();

    class StrokePointProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: StrokePoint): StrokePointProto {
        const objectProto: Partial<StrokePointProto> = { metatype: 2101103 };
        objectProto.point = object.point.pack(10);
        objectProto.originalPoint = object.originalPoint.pack(10);
        objectProto.pressure = object.pressure;
        objectProto.direction = object.direction.pack(10);
        objectProto.distance = object.distance;
        objectProto.runningLength = object.runningLength;
        objectProto.radius = object.radius;
        return objectProto as StrokePointProto;
      }

      unpackObject(objectProto: StrokePointProto, _session: Session | null): StrokePoint {
        const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
        return new (STRUCT_CLASS_BY_TYPE[2101103] as typeof StrokePoint)({
          point: _Vector2.unpack(10, objectProto.point, _session) as Vector2,
          originalPoint: _Vector2.unpack(10, objectProto.originalPoint, _session) as Vector2,
          pressure: objectProto.pressure,
          direction: _Vector2.unpack(10, objectProto.direction, _session) as Vector2,
          distance: objectProto.distance,
          runningLength: objectProto.runningLength,
          radius: objectProto.radius,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: StrokePoint): Uint8Array {
        const proto = this.packObject(object);
        return StrokePointProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): StrokePoint {
        const proto = StrokePointProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 2101103)] = new StrokePointProtoEncoder();

    class StrokePathProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: StrokePath): StrokePathProto {
        const objectProto: Partial<StrokePathProto> = { metatype: 2101102 };
        if (object.points) {
          const packedPoints: any[] = [];
          for (const item of object.points) {
            packedPoints.push(item.pack(10));
          }
          objectProto.points = packedPoints;
        }
        return objectProto as StrokePathProto;
      }

      unpackObject(objectProto: StrokePathProto, _session: Session | null): StrokePath {
        const _StrokePoint = STRUCT_CLASS_BY_TYPE[2101103] as typeof StrokePoint;
        const unpackedPoints: any[] = [];
        if (objectProto.points) {
          for (const item of objectProto.points) {
            unpackedPoints.push(_StrokePoint.unpack(10, item, _session) as StrokePoint);
          }
        }
        return new (STRUCT_CLASS_BY_TYPE[2101102] as typeof StrokePath)({
          points: unpackedPoints,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: StrokePath): Uint8Array {
        const proto = this.packObject(object);
        return StrokePathProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): StrokePath {
        const proto = StrokePathProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 2101102)] = new StrokePathProtoEncoder();

    class TransitionProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Transition): TransitionProto {
        const objectProto: Partial<TransitionProto> = { metatype: 2200000 };
        objectProto.type = Number(object.type) as any;
        if (object.stylePtr != null) {
          objectProto.stylePtr = object.stylePtr.pack(10);
        }
        if (object.delay != null) {
          objectProto.delay = object.delay;
        }
        if (object.duration != null) {
          objectProto.duration = object.duration;
        }
        if (object.ease) {
          const packedEase: any[] = [];
          for (const item of object.ease) {
            packedEase.push(item);
          }
          objectProto.ease = packedEase;
        }
        if (object.stiffness != null) {
          objectProto.stiffness = object.stiffness;
        }
        if (object.damping != null) {
          objectProto.damping = object.damping;
        }
        if (object.mass != null) {
          objectProto.mass = object.mass;
        }
        if (object.bounce != null) {
          objectProto.bounce = object.bounce;
        }
        if (object.springType != null) {
          objectProto.springType = Number(object.springType) as any;
        }
        return objectProto as TransitionProto;
      }

      unpackObject(objectProto: TransitionProto, _session: Session | null): Transition {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const unpackedEase: any[] = [];
        if (objectProto.ease) {
          for (const item of objectProto.ease) {
            unpackedEase.push(item);
          }
        }
        return new (STRUCT_CLASS_BY_TYPE[2200000] as typeof Transition)({
          type: Number(objectProto.type) as any,
          style: objectProto.stylePtr != undefined ? _NodeReference.unpack(10, objectProto.stylePtr, _session) as NodeReference : null,
          delay: objectProto.delay != undefined ? objectProto.delay : null,
          duration: objectProto.duration != undefined ? objectProto.duration : null,
          ease: unpackedEase,
          stiffness: objectProto.stiffness != undefined ? objectProto.stiffness : null,
          damping: objectProto.damping != undefined ? objectProto.damping : null,
          mass: objectProto.mass != undefined ? objectProto.mass : null,
          bounce: objectProto.bounce != undefined ? objectProto.bounce : null,
          springType: objectProto.springType != undefined ? Number(objectProto.springType) as any : null,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: Transition): Uint8Array {
        const proto = this.packObject(object);
        return TransitionProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Transition {
        const proto = TransitionProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 2200000)] = new TransitionProtoEncoder();

    class EffectProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Effect): EffectProto {
        const objectProto: Partial<EffectProto> = { metatype: 2200100 };
        objectProto.type = Number(object.type) as any;
        if (object.stylePtr != null) {
          objectProto.stylePtr = object.stylePtr.pack(10);
        }
        if (object.opacity != null) {
          objectProto.opacity = object.opacity;
        }
        if (object.offset != null) {
          objectProto.offset = object.offset.pack(10);
        }
        if (object.scale != null) {
          objectProto.scale = object.scale;
        }
        if (object.rotate != null) {
          objectProto.rotate = object.rotate.pack(10);
        }
        if (object.skew != null) {
          objectProto.skew = object.skew.pack(10);
        }
        if (object.perspective != null) {
          objectProto.perspective = object.perspective;
        }
        if (object.delay != null) {
          objectProto.delay = packProtoDuration(object.delay);
        }
        if (object.duration != null) {
          objectProto.duration = object.duration;
        }
        if (object.threshold != null) {
          objectProto.threshold = object.threshold;
        }
        if (object.once != null) {
          objectProto.once = object.once;
        }
        if (object.repeat != null) {
          objectProto.repeat = Number(object.repeat) as any;
        }
        if (object.split != null) {
          objectProto.split = Number(object.split) as any;
        }
        if (object.offscreen != null) {
          objectProto.offscreen = Number(object.offscreen) as any;
        }
        if (object.transition != null) {
          objectProto.transition = object.transition.pack(10);
        }
        return objectProto as EffectProto;
      }

      unpackObject(objectProto: EffectProto, _session: Session | null): Effect {
        const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
        const _Transition = STRUCT_CLASS_BY_TYPE[2200000] as typeof Transition;
        const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
        const _Axis3 = STRUCT_CLASS_BY_TYPE[2400026] as typeof Axis3;
        return new (STRUCT_CLASS_BY_TYPE[2200100] as typeof Effect)({
          type: Number(objectProto.type) as any,
          style: objectProto.stylePtr != undefined ? _NodeReference.unpack(10, objectProto.stylePtr, _session) as NodeReference : null,
          opacity: objectProto.opacity != undefined ? objectProto.opacity : null,
          offset: objectProto.offset != undefined ? _Vector2.unpack(10, objectProto.offset, _session) as Vector2 : null,
          scale: objectProto.scale != undefined ? objectProto.scale : null,
          rotate: objectProto.rotate != undefined ? _Axis3.unpack(10, objectProto.rotate, _session) as Axis3 : null,
          skew: objectProto.skew != undefined ? _Vector2.unpack(10, objectProto.skew, _session) as Vector2 : null,
          perspective: objectProto.perspective != undefined ? objectProto.perspective : null,
          delay: objectProto.delay != undefined ? unpackProtoDuration(objectProto.delay!) : null,
          duration: objectProto.duration != undefined ? objectProto.duration : null,
          threshold: objectProto.threshold != undefined ? objectProto.threshold : null,
          once: objectProto.once != undefined ? objectProto.once : null,
          repeat: objectProto.repeat != undefined ? Number(objectProto.repeat) as any : null,
          split: objectProto.split != undefined ? Number(objectProto.split) as any : null,
          offscreen: objectProto.offscreen != undefined ? Number(objectProto.offscreen) as any : null,
          transition: objectProto.transition != undefined ? _Transition.unpack(10, objectProto.transition, _session) as Transition : null,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: Effect): Uint8Array {
        const proto = this.packObject(object);
        return EffectProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Effect {
        const proto = EffectProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 2200100)] = new EffectProtoEncoder();

    class Arrow2DProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Arrow2D): Arrow2DProto {
        const objectProto: Partial<Arrow2DProto> = { metatype: 2411200 };
        objectProto.startType = Number(object.startType) as any;
        objectProto.start = object.start.pack(10);
        objectProto.endType = Number(object.endType) as any;
        objectProto.end = object.end.pack(10);
        return objectProto as Arrow2DProto;
      }

      unpackObject(objectProto: Arrow2DProto, _session: Session | null): Arrow2D {
        const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
        return new (STRUCT_CLASS_BY_TYPE[2411200] as typeof Arrow2D)({
          startType: Number(objectProto.startType) as any,
          start: _Vector2.unpack(10, objectProto.start, _session) as Vector2,
          endType: Number(objectProto.endType) as any,
          end: _Vector2.unpack(10, objectProto.end, _session) as Vector2,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: Arrow2D): Uint8Array {
        const proto = this.packObject(object);
        return Arrow2DProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Arrow2D {
        const proto = Arrow2DProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 2411200)] = new Arrow2DProtoEncoder();

    class Ellipse2DProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Ellipse2D): Ellipse2DProto {
        const objectProto: Partial<Ellipse2DProto> = { metatype: 2411400 };
        if (object.stroke != null) {
          objectProto.stroke = object.stroke.pack(10);
        }
        return objectProto as Ellipse2DProto;
      }

      unpackObject(objectProto: Ellipse2DProto, _session: Session | null): Ellipse2D {
        const _Stroke = STRUCT_CLASS_BY_TYPE[2101100] as typeof Stroke;
        return new (STRUCT_CLASS_BY_TYPE[2411400] as typeof Ellipse2D)({
          stroke: objectProto.stroke != undefined ? _Stroke.unpack(10, objectProto.stroke, _session) as Stroke : null,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: Ellipse2D): Uint8Array {
        const proto = this.packObject(object);
        return Ellipse2DProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Ellipse2D {
        const proto = Ellipse2DProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 2411400)] = new Ellipse2DProtoEncoder();

    class Line2DProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Line2D): Line2DProto {
        const objectProto: Partial<Line2DProto> = { metatype: 2411100 };
        if (object.stroke != null) {
          objectProto.stroke = object.stroke.pack(10);
        }
        objectProto.start = object.start.pack(10);
        objectProto.end = object.end.pack(10);
        return objectProto as Line2DProto;
      }

      unpackObject(objectProto: Line2DProto, _session: Session | null): Line2D {
        const _Stroke = STRUCT_CLASS_BY_TYPE[2101100] as typeof Stroke;
        const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
        return new (STRUCT_CLASS_BY_TYPE[2411100] as typeof Line2D)({
          stroke: objectProto.stroke != undefined ? _Stroke.unpack(10, objectProto.stroke, _session) as Stroke : null,
          start: _Vector2.unpack(10, objectProto.start, _session) as Vector2,
          end: _Vector2.unpack(10, objectProto.end, _session) as Vector2,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: Line2D): Uint8Array {
        const proto = this.packObject(object);
        return Line2DProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Line2D {
        const proto = Line2DProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 2411100)] = new Line2DProtoEncoder();

    class Path2DProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Path2D): Path2DProto {
        const objectProto: Partial<Path2DProto> = { metatype: 2411600 };
        if (object.stroke != null) {
          objectProto.stroke = object.stroke.pack(10);
        }
        if (object.points) {
          const packedPoints: any[] = [];
          for (const item of object.points) {
            packedPoints.push(item.pack(10));
          }
          objectProto.points = packedPoints;
        }
        return objectProto as Path2DProto;
      }

      unpackObject(objectProto: Path2DProto, _session: Session | null): Path2D {
        const _Stroke = STRUCT_CLASS_BY_TYPE[2101100] as typeof Stroke;
        const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
        const unpackedPoints: any[] = [];
        if (objectProto.points) {
          for (const item of objectProto.points) {
            unpackedPoints.push(_Vector2.unpack(10, item, _session) as Vector2);
          }
        }
        return new (STRUCT_CLASS_BY_TYPE[2411600] as typeof Path2D)({
          stroke: objectProto.stroke != undefined ? _Stroke.unpack(10, objectProto.stroke, _session) as Stroke : null,
          points: unpackedPoints,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: Path2D): Uint8Array {
        const proto = this.packObject(object);
        return Path2DProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Path2D {
        const proto = Path2DProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 2411600)] = new Path2DProtoEncoder();

    class Polygon2DProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Polygon2D): Polygon2DProto {
        const objectProto: Partial<Polygon2DProto> = { metatype: 2411500 };
        if (object.stroke != null) {
          objectProto.stroke = object.stroke.pack(10);
        }
        if (object.points) {
          const packedPoints: any[] = [];
          for (const item of object.points) {
            packedPoints.push(item.pack(10));
          }
          objectProto.points = packedPoints;
        }
        return objectProto as Polygon2DProto;
      }

      unpackObject(objectProto: Polygon2DProto, _session: Session | null): Polygon2D {
        const _Stroke = STRUCT_CLASS_BY_TYPE[2101100] as typeof Stroke;
        const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
        const unpackedPoints: any[] = [];
        if (objectProto.points) {
          for (const item of objectProto.points) {
            unpackedPoints.push(_Vector2.unpack(10, item, _session) as Vector2);
          }
        }
        return new (STRUCT_CLASS_BY_TYPE[2411500] as typeof Polygon2D)({
          stroke: objectProto.stroke != undefined ? _Stroke.unpack(10, objectProto.stroke, _session) as Stroke : null,
          points: unpackedPoints,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: Polygon2D): Uint8Array {
        const proto = this.packObject(object);
        return Polygon2DProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Polygon2D {
        const proto = Polygon2DProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 2411500)] = new Polygon2DProtoEncoder();

    class Vector2ProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Vector2): Vector2Proto {
        const objectProto: Partial<Vector2Proto> = { metatype: 2400000 };
        objectProto.x = object.x;
        objectProto.y = object.y;
        return objectProto as Vector2Proto;
      }

      unpackObject(objectProto: Vector2Proto, _session: Session | null): Vector2 {

        return new (STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2)({
          x: objectProto.x,
          y: objectProto.y,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: Vector2): Uint8Array {
        const proto = this.packObject(object);
        return Vector2Proto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Vector2 {
        const proto = Vector2Proto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 2400000)] = new Vector2ProtoEncoder();

    class Vector3ProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Vector3): Vector3Proto {
        const objectProto: Partial<Vector3Proto> = { metatype: 2400002 };
        objectProto.x = object.x;
        objectProto.y = object.y;
        objectProto.z = object.z;
        return objectProto as Vector3Proto;
      }

      unpackObject(objectProto: Vector3Proto, _session: Session | null): Vector3 {

        return new (STRUCT_CLASS_BY_TYPE[2400002] as typeof Vector3)({
          x: objectProto.x,
          y: objectProto.y,
          z: objectProto.z,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: Vector3): Uint8Array {
        const proto = this.packObject(object);
        return Vector3Proto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Vector3 {
        const proto = Vector3Proto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 2400002)] = new Vector3ProtoEncoder();

    class Vector4ProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Vector4): Vector4Proto {
        const objectProto: Partial<Vector4Proto> = { metatype: 2400004 };
        objectProto.x = object.x;
        objectProto.y = object.y;
        objectProto.z = object.z;
        objectProto.w = object.w;
        return objectProto as Vector4Proto;
      }

      unpackObject(objectProto: Vector4Proto, _session: Session | null): Vector4 {

        return new (STRUCT_CLASS_BY_TYPE[2400004] as typeof Vector4)({
          x: objectProto.x,
          y: objectProto.y,
          z: objectProto.z,
          w: objectProto.w,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: Vector4): Uint8Array {
        const proto = this.packObject(object);
        return Vector4Proto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Vector4 {
        const proto = Vector4Proto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 2400004)] = new Vector4ProtoEncoder();

    class Vector2iProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Vector2i): Vector2iProto {
        const objectProto: Partial<Vector2iProto> = { metatype: 2400001 };
        objectProto.x = object.x;
        objectProto.y = object.y;
        return objectProto as Vector2iProto;
      }

      unpackObject(objectProto: Vector2iProto, _session: Session | null): Vector2i {

        return new (STRUCT_CLASS_BY_TYPE[2400001] as typeof Vector2i)({
          x: Number(objectProto.x),
          y: Number(objectProto.y),
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: Vector2i): Uint8Array {
        const proto = this.packObject(object);
        return Vector2iProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Vector2i {
        const proto = Vector2iProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 2400001)] = new Vector2iProtoEncoder();

    class Vector3iProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Vector3i): Vector3iProto {
        const objectProto: Partial<Vector3iProto> = { metatype: 2400003 };
        objectProto.x = object.x;
        objectProto.y = object.y;
        objectProto.z = object.z;
        return objectProto as Vector3iProto;
      }

      unpackObject(objectProto: Vector3iProto, _session: Session | null): Vector3i {

        return new (STRUCT_CLASS_BY_TYPE[2400003] as typeof Vector3i)({
          x: Number(objectProto.x),
          y: Number(objectProto.y),
          z: Number(objectProto.z),
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: Vector3i): Uint8Array {
        const proto = this.packObject(object);
        return Vector3iProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Vector3i {
        const proto = Vector3iProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 2400003)] = new Vector3iProtoEncoder();

    class Vector4iProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Vector4i): Vector4iProto {
        const objectProto: Partial<Vector4iProto> = { metatype: 2400005 };
        objectProto.x = object.x;
        objectProto.y = object.y;
        objectProto.z = object.z;
        objectProto.w = object.w;
        return objectProto as Vector4iProto;
      }

      unpackObject(objectProto: Vector4iProto, _session: Session | null): Vector4i {

        return new (STRUCT_CLASS_BY_TYPE[2400005] as typeof Vector4i)({
          x: Number(objectProto.x),
          y: Number(objectProto.y),
          z: Number(objectProto.z),
          w: Number(objectProto.w),
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: Vector4i): Uint8Array {
        const proto = this.packObject(object);
        return Vector4iProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Vector4i {
        const proto = Vector4iProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 2400005)] = new Vector4iProtoEncoder();

    class QuaternionProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Quaternion): QuaternionProto {
        const objectProto: Partial<QuaternionProto> = { metatype: 2400010 };
        objectProto.x = object.x;
        objectProto.y = object.y;
        objectProto.z = object.z;
        objectProto.w = object.w;
        return objectProto as QuaternionProto;
      }

      unpackObject(objectProto: QuaternionProto, _session: Session | null): Quaternion {

        return new (STRUCT_CLASS_BY_TYPE[2400010] as typeof Quaternion)({
          x: objectProto.x,
          y: objectProto.y,
          z: objectProto.z,
          w: objectProto.w,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: Quaternion): Uint8Array {
        const proto = this.packObject(object);
        return QuaternionProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Quaternion {
        const proto = QuaternionProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 2400010)] = new QuaternionProtoEncoder();

    class Rectangle2DProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Rectangle2D): Rectangle2DProto {
        const objectProto: Partial<Rectangle2DProto> = { metatype: 2411300 };
        if (object.stroke != null) {
          objectProto.stroke = object.stroke.pack(10);
        }
        if (object.width != null) {
          objectProto.width = object.width.pack(10);
        }
        if (object.height != null) {
          objectProto.height = object.height.pack(10);
        }
        return objectProto as Rectangle2DProto;
      }

      unpackObject(objectProto: Rectangle2DProto, _session: Session | null): Rectangle2D {
        const _Stroke = STRUCT_CLASS_BY_TYPE[2101100] as typeof Stroke;
        const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
        return new (STRUCT_CLASS_BY_TYPE[2411300] as typeof Rectangle2D)({
          stroke: objectProto.stroke != undefined ? _Stroke.unpack(10, objectProto.stroke, _session) as Stroke : null,
          width: objectProto.width != undefined ? _Vector2.unpack(10, objectProto.width, _session) as Vector2 : null,
          height: objectProto.height != undefined ? _Vector2.unpack(10, objectProto.height, _session) as Vector2 : null,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: Rectangle2D): Uint8Array {
        const proto = this.packObject(object);
        return Rectangle2DProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Rectangle2D {
        const proto = Rectangle2DProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 2411300)] = new Rectangle2DProtoEncoder();

    class LengthProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Length): LengthProto {
        const objectProto: Partial<LengthProto> = { metatype: 1800001 };
        objectProto.unit = Number(object.unit) as any;
        objectProto.value = object.value;
        return objectProto as LengthProto;
      }

      unpackObject(objectProto: LengthProto, _session: Session | null): Length {

        return new (STRUCT_CLASS_BY_TYPE[1800001] as typeof Length)({
          unit: Number(objectProto.unit) as any,
          value: objectProto.value,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: Length): Uint8Array {
        const proto = this.packObject(object);
        return LengthProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Length {
        const proto = LengthProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 1800001)] = new LengthProtoEncoder();

    class Offset2ProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Offset2): Offset2Proto {
        const objectProto: Partial<Offset2Proto> = { metatype: 2400020 };
        objectProto.type = Number(object.type) as any;
        if (object.top != null) {
          objectProto.top = object.top.pack(10);
        }
        if (object.left != null) {
          objectProto.left = object.left.pack(10);
        }
        if (object.width != null) {
          objectProto.width = object.width.pack(10);
        }
        if (object.height != null) {
          objectProto.height = object.height.pack(10);
        }
        return objectProto as Offset2Proto;
      }

      unpackObject(objectProto: Offset2Proto, _session: Session | null): Offset2 {
        const _Length = STRUCT_CLASS_BY_TYPE[1800001] as typeof Length;
        return new (STRUCT_CLASS_BY_TYPE[2400020] as typeof Offset2)({
          type: Number(objectProto.type) as any,
          top: objectProto.top != undefined ? _Length.unpack(10, objectProto.top, _session) as Length : null,
          left: objectProto.left != undefined ? _Length.unpack(10, objectProto.left, _session) as Length : null,
          width: objectProto.width != undefined ? _Length.unpack(10, objectProto.width, _session) as Length : null,
          height: objectProto.height != undefined ? _Length.unpack(10, objectProto.height, _session) as Length : null,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: Offset2): Uint8Array {
        const proto = this.packObject(object);
        return Offset2Proto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Offset2 {
        const proto = Offset2Proto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 2400020)] = new Offset2ProtoEncoder();

    class Inset2ProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Inset2): Inset2Proto {
        const objectProto: Partial<Inset2Proto> = { metatype: 2400023 };
        objectProto.base = object.base;
        if (object.top != null) {
          objectProto.top = object.top;
        }
        if (object.left != null) {
          objectProto.left = object.left;
        }
        if (object.right != null) {
          objectProto.right = object.right;
        }
        if (object.bottom != null) {
          objectProto.bottom = object.bottom;
        }
        return objectProto as Inset2Proto;
      }

      unpackObject(objectProto: Inset2Proto, _session: Session | null): Inset2 {

        return new (STRUCT_CLASS_BY_TYPE[2400023] as typeof Inset2)({
          base: Number(objectProto.base),
          top: objectProto.top != undefined ? Number(objectProto.top) : null,
          left: objectProto.left != undefined ? Number(objectProto.left) : null,
          right: objectProto.right != undefined ? Number(objectProto.right) : null,
          bottom: objectProto.bottom != undefined ? Number(objectProto.bottom) : null,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: Inset2): Uint8Array {
        const proto = this.packObject(object);
        return Inset2Proto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Inset2 {
        const proto = Inset2Proto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 2400023)] = new Inset2ProtoEncoder();

    class Corner2ProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Corner2): Corner2Proto {
        const objectProto: Partial<Corner2Proto> = { metatype: 2400024 };
        objectProto.base = object.base;
        if (object.topLeft != null) {
          objectProto.topLeft = object.topLeft;
        }
        if (object.topRight != null) {
          objectProto.topRight = object.topRight;
        }
        if (object.bottomLeft != null) {
          objectProto.bottomLeft = object.bottomLeft;
        }
        if (object.bottomRight != null) {
          objectProto.bottomRight = object.bottomRight;
        }
        return objectProto as Corner2Proto;
      }

      unpackObject(objectProto: Corner2Proto, _session: Session | null): Corner2 {

        return new (STRUCT_CLASS_BY_TYPE[2400024] as typeof Corner2)({
          base: Number(objectProto.base),
          topLeft: objectProto.topLeft != undefined ? Number(objectProto.topLeft) : null,
          topRight: objectProto.topRight != undefined ? Number(objectProto.topRight) : null,
          bottomLeft: objectProto.bottomLeft != undefined ? Number(objectProto.bottomLeft) : null,
          bottomRight: objectProto.bottomRight != undefined ? Number(objectProto.bottomRight) : null,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: Corner2): Uint8Array {
        const proto = this.packObject(object);
        return Corner2Proto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Corner2 {
        const proto = Corner2Proto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 2400024)] = new Corner2ProtoEncoder();

    class Axis2ProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Axis2): Axis2Proto {
        const objectProto: Partial<Axis2Proto> = { metatype: 2400025 };
        objectProto.base = object.base;
        if (object.x != null) {
          objectProto.x = object.x;
        }
        if (object.y != null) {
          objectProto.y = object.y;
        }
        return objectProto as Axis2Proto;
      }

      unpackObject(objectProto: Axis2Proto, _session: Session | null): Axis2 {

        return new (STRUCT_CLASS_BY_TYPE[2400025] as typeof Axis2)({
          base: objectProto.base,
          x: objectProto.x != undefined ? objectProto.x : null,
          y: objectProto.y != undefined ? objectProto.y : null,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: Axis2): Uint8Array {
        const proto = this.packObject(object);
        return Axis2Proto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Axis2 {
        const proto = Axis2Proto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 2400025)] = new Axis2ProtoEncoder();

    class Axis3ProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Axis3): Axis3Proto {
        const objectProto: Partial<Axis3Proto> = { metatype: 2400026 };
        objectProto.base = object.base;
        if (object.x != null) {
          objectProto.x = object.x;
        }
        if (object.y != null) {
          objectProto.y = object.y;
        }
        if (object.z != null) {
          objectProto.z = object.z;
        }
        return objectProto as Axis3Proto;
      }

      unpackObject(objectProto: Axis3Proto, _session: Session | null): Axis3 {

        return new (STRUCT_CLASS_BY_TYPE[2400026] as typeof Axis3)({
          base: objectProto.base,
          x: objectProto.x != undefined ? objectProto.x : null,
          y: objectProto.y != undefined ? objectProto.y : null,
          z: objectProto.z != undefined ? objectProto.z : null,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: Axis3): Uint8Array {
        const proto = this.packObject(object);
        return Axis3Proto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Axis3 {
        const proto = Axis3Proto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 2400026)] = new Axis3ProtoEncoder();

    class Grid2ProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Grid2): Grid2Proto {
        const objectProto: Partial<Grid2Proto> = { metatype: 2400021 };
        objectProto.columns = object.columns;
        objectProto.rows = object.rows;
        if (object.columnWidth != null) {
          objectProto.columnWidth = object.columnWidth.pack(10);
        }
        if (object.columnMinWidth != null) {
          objectProto.columnMinWidth = object.columnMinWidth.pack(10);
        }
        if (object.rowHeight != null) {
          objectProto.rowHeight = object.rowHeight.pack(10);
        }
        return objectProto as Grid2Proto;
      }

      unpackObject(objectProto: Grid2Proto, _session: Session | null): Grid2 {
        const _Length = STRUCT_CLASS_BY_TYPE[1800001] as typeof Length;
        return new (STRUCT_CLASS_BY_TYPE[2400021] as typeof Grid2)({
          columns: Number(objectProto.columns),
          rows: Number(objectProto.rows),
          columnWidth: objectProto.columnWidth != undefined ? _Length.unpack(10, objectProto.columnWidth, _session) as Length : null,
          columnMinWidth: objectProto.columnMinWidth != undefined ? _Length.unpack(10, objectProto.columnMinWidth, _session) as Length : null,
          rowHeight: objectProto.rowHeight != undefined ? _Length.unpack(10, objectProto.rowHeight, _session) as Length : null,
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: Grid2): Uint8Array {
        const proto = this.packObject(object);
        return Grid2Proto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Grid2 {
        const proto = Grid2Proto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 2400021)] = new Grid2ProtoEncoder();

    class GridSpan2ProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: GridSpan2): GridSpan2Proto {
        const objectProto: Partial<GridSpan2Proto> = { metatype: 2400022 };
        objectProto.columns = object.columns;
        objectProto.rows = object.rows;
        return objectProto as GridSpan2Proto;
      }

      unpackObject(objectProto: GridSpan2Proto, _session: Session | null): GridSpan2 {

        return new (STRUCT_CLASS_BY_TYPE[2400022] as typeof GridSpan2)({
          columns: Number(objectProto.columns),
          rows: Number(objectProto.rows),
          _packedCache: [{ encoding: 10, isBytes: false, packed: objectProto }],
          _session,
        });
      }

      packObjectBytes(object: GridSpan2): Uint8Array {
        const proto = this.packObject(object);
        return GridSpan2Proto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): GridSpan2 {
        const proto = GridSpan2Proto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 2400022)] = new GridSpan2ProtoEncoder();

    class ScheduleProtoEncoder implements _ProtoObjectEncoder {
      packObject(object: Schedule): ScheduleProto {
        const objectProto: Partial<ScheduleProto> = { metatype: 700001 };
        objectProto.frequency = Number(object.frequency) as any;
        objectProto.interval = object.interval;
        if (object.start != null) {
          objectProto.start = packProtoTimestamp(object.start);
        }
        if (object.end != null) {
          objectProto.end = packProtoTimestamp(object.end);
        }
        if (object.count != null) {
          objectProto.count = object.count;
        }
        if (object.weekStart != null) {
          objectProto.weekStart = Number(object.weekStart) as any;
        }
        if (object.bySetPos) {
          const packedBySetPos: any[] = [];
          for (const item of object.bySetPos) {
            packedBySetPos.push(item);
          }
          objectProto.bySetPos = packedBySetPos;
        }
        if (object.byMonth) {
          const packedByMonth: any[] = [];
          for (const item of object.byMonth) {
            packedByMonth.push(Number(item) as any);
          }
          objectProto.byMonth = packedByMonth;
        }
        if (object.byMonthDay) {
          const packedByMonthDay: any[] = [];
          for (const item of object.byMonthDay) {
            packedByMonthDay.push(item);
          }
          objectProto.byMonthDay = packedByMonthDay;
        }
        if (object.byYearDay) {
          const packedByYearDay: any[] = [];
          for (const item of object.byYearDay) {
            packedByYearDay.push(item);
          }
          objectProto.byYearDay = packedByYearDay;
        }
        if (object.byEaster) {
          const packedByEaster: any[] = [];
          for (const item of object.byEaster) {
            packedByEaster.push(item);
          }
          objectProto.byEaster = packedByEaster;
        }
        if (object.byWeekNo) {
          const packedByWeekNo: any[] = [];
          for (const item of object.byWeekNo) {
            packedByWeekNo.push(item);
          }
          objectProto.byWeekNo = packedByWeekNo;
        }
        if (object.byWeekDay) {
          const packedByWeekDay: any[] = [];
          for (const item of object.byWeekDay) {
            packedByWeekDay.push(Number(item) as any);
          }
          objectProto.byWeekDay = packedByWeekDay;
        }
        if (object.byHour) {
          const packedByHour: any[] = [];
          for (const item of object.byHour) {
            packedByHour.push(item);
          }
          objectProto.byHour = packedByHour;
        }
        if (object.byMinute) {
          const packedByMinute: any[] = [];
          for (const item of object.byMinute) {
            packedByMinute.push(item);
          }
          objectProto.byMinute = packedByMinute;
        }
        if (object.bySecond) {
          const packedBySecond: any[] = [];
          for (const item of object.bySecond) {
            packedBySecond.push(item);
          }
          objectProto.bySecond = packedBySecond;
        }
        return objectProto as ScheduleProto;
      }

      unpackObject(objectProto: ScheduleProto, _session: Session | null): Schedule {

        const unpackedBySetPos: any[] = [];
        if (objectProto.bySetPos) {
          for (const item of objectProto.bySetPos) {
            unpackedBySetPos.push(Number(item));
          }
        }
        const unpackedByMonth: any[] = [];
        if (objectProto.byMonth) {
          for (const item of objectProto.byMonth) {
            unpackedByMonth.push(Number(item) as any);
          }
        }
        const unpackedByMonthDay: any[] = [];
        if (objectProto.byMonthDay) {
          for (const item of objectProto.byMonthDay) {
            unpackedByMonthDay.push(Number(item));
          }
        }
        const unpackedByYearDay: any[] = [];
        if (objectProto.byYearDay) {
          for (const item of objectProto.byYearDay) {
            unpackedByYearDay.push(Number(item));
          }
        }
        const unpackedByEaster: any[] = [];
        if (objectProto.byEaster) {
          for (const item of objectProto.byEaster) {
            unpackedByEaster.push(Number(item));
          }
        }
        const unpackedByWeekNo: any[] = [];
        if (objectProto.byWeekNo) {
          for (const item of objectProto.byWeekNo) {
            unpackedByWeekNo.push(Number(item));
          }
        }
        const unpackedByWeekDay: any[] = [];
        if (objectProto.byWeekDay) {
          for (const item of objectProto.byWeekDay) {
            unpackedByWeekDay.push(Number(item) as any);
          }
        }
        const unpackedByHour: any[] = [];
        if (objectProto.byHour) {
          for (const item of objectProto.byHour) {
            unpackedByHour.push(Number(item));
          }
        }
        const unpackedByMinute: any[] = [];
        if (objectProto.byMinute) {
          for (const item of objectProto.byMinute) {
            unpackedByMinute.push(Number(item));
          }
        }
        const unpackedBySecond: any[] = [];
        if (objectProto.bySecond) {
          for (const item of objectProto.bySecond) {
            unpackedBySecond.push(Number(item));
          }
        }
        return new (STRUCT_CLASS_BY_TYPE[700001] as typeof Schedule)({
          frequency: Number(objectProto.frequency) as any,
          interval: Number(objectProto.interval),
          start: objectProto.start != undefined ? unpackProtoTimestamp(objectProto.start!) : null,
          end: objectProto.end != undefined ? unpackProtoTimestamp(objectProto.end!) : null,
          count: objectProto.count != undefined ? Number(objectProto.count) : null,
          weekStart: objectProto.weekStart != undefined ? Number(objectProto.weekStart) as any : null,
          bySetPos: unpackedBySetPos,
          byMonth: unpackedByMonth,
          byMonthDay: unpackedByMonthDay,
          byYearDay: unpackedByYearDay,
          byEaster: unpackedByEaster,
          byWeekNo: unpackedByWeekNo,
          byWeekDay: unpackedByWeekDay,
          byHour: unpackedByHour,
          byMinute: unpackedByMinute,
          bySecond: unpackedBySecond,
          _session,
        });
      }

      packObjectBytes(object: Schedule): Uint8Array {
        const proto = this.packObject(object);
        return ScheduleProto.toBinary(proto);
      }

      unpackObjectBytes(objectBytes: Uint8Array, _session: Session | null): Schedule {
        const proto = ScheduleProto.fromBinary(objectBytes);
        return this.unpackObject(proto, _session);
      }
    }

    PROTO_OBJECT_ENCODERS[getObjectKey(2, 700001)] = new ScheduleProtoEncoder();
}

loadEncoders();
