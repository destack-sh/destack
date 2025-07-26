import {
  CSON_OBJECT_ENCODERS,
  type CsonObjectEncoder,
  getObjectKey,
} from "@destack/encoder/cson/core";
import type {
  Action,
  ActionDefinition,
  Aggregation,
  Arrow2D,
  ArrowShape2D,
  Axis2,
  Axis3,
  Border,
  BorderStyle,
  Branch,
  CheckedType,
  Client,
  CollectionConstraint,
  Color,
  ColorStyle,
  Condition,
  ConstantDefinition,
  Constraint,
  ConstraintDefinition,
  CopyEvent,
  Corner2,
  CounterMeasurementEvent,
  CounterMetric,
  CustomEnum,
  CustomEvent,
  CustomOption,
  CustomProperty,
  CustomStruct,
  CutEvent,
  Database,
  DoubleClickEvent,
  DragEndEvent,
  DragEnterEvent,
  DragLeaveEvent,
  DragOverEvent,
  DragStartEvent,
  DropEvent,
  EditEvent,
  Effect,
  EffectStyle,
  Ellipse2D,
  EllipseShape2D,
  Entitlement,
  EntitlementExpiredEvent,
  EntitlementGrantedEvent,
  EntitlementRequestedEvent,
  EntitlementRevokedEvent,
  EnumDefinition,
  Environment,
  Expression,
  File,
  Fill,
  FillStyle,
  FocusInEvent,
  FocusOutEvent,
  Folder,
  Follow,
  FollowAddedEvent,
  FollowEvent,
  FollowRemovedEvent,
  Font,
  FontStyle,
  FrameView,
  Function,
  GaugeMeasurementEvent,
  GaugeMetric,
  Gradient,
  GradientStop,
  GradientStyle,
  Grid2,
  GridSpan2,
  Handle,
  HistogramMeasurementEvent,
  HistogramMetric,
  Icon,
  Index,
  IndexDefinition,
  Inset2,
  Invite,
  InviteAcceptedEvent,
  InviteRejectedEvent,
  InviteRescindedEvent,
  InviteSentEvent,
  Join,
  KeyDownEvent,
  KeyPressEvent,
  KeyUpEvent,
  LabelView,
  Layer,
  Length,
  Line2D,
  LineShape2D,
  LogEvent,
  Machine,
  Membership,
  MembershipJoinedEvent,
  MembershipLeftEvent,
  Method,
  MethodDefinition,
  Migration,
  MigrationDefinition,
  MigrationOperation,
  MigrationOperationDefinition,
  NodeDefinition,
  NodeDefinitionReference,
  NodeReference,
  Notification,
  NotificationDismissedEvent,
  NotificationExpiredEvent,
  NotificationReadEvent,
  NotificationRescindedEvent,
  NotificationSentEvent,
  NumberConstraint,
  NumberInputView,
  ObjectDefinitionReference,
  Offset2,
  OptionDefinition,
  Organization,
  Palette,
  PasteEvent,
  Path2D,
  PathShape2D,
  Permission,
  PermissionDefinition,
  PointerDownEvent,
  PointerEnterEvent,
  PointerLeaveEvent,
  PointerLongPressEvent,
  PointerMoveEvent,
  PointerOverEvent,
  PointerUpEvent,
  Polygon2D,
  PolygonShape2D,
  PropertyDefinition,
  PropertyReference,
  Quaternion,
  Query,
  Reaction,
  ReactionAddedEvent,
  ReactionEvent,
  ReactionRemovedEvent,
  Rectangle2D,
  RectangleShape2D,
  Role,
  RoleAssignedEvent,
  RoleUnassignedEvent,
  RunCompletedEvent,
  RunFailedEvent,
  RunPausedEvent,
  RunPauseRequestedEvent,
  RunResumedEvent,
  RunResumeRequestedEvent,
  RunStartedEvent,
  RunStopRequestedEvent,
  Sanction,
  SanctionExpiredEvent,
  SanctionGrantedEvent,
  SanctionRequestedEvent,
  SanctionRevokedEvent,
  Scene,
  Schedule,
  Script,
  Select,
  Service,
  Session,
  Shadow,
  ShadowStyle,
  SingleClickEvent,
  SliderInputView,
  Snapshot,
  Sort,
  Space,
  SpanEvent,
  SplitView,
  Stage,
  Star,
  StarAddedEvent,
  StarEvent,
  StarRemovedEvent,
  StringConstraint,
  Stroke,
  StrokeCap,
  StrokePath,
  StrokePoint,
  StrokeStyle,
  StructDefinition,
  StructDefinitionReference,
  Tag,
  TagDefinition,
  Tagging,
  Team,
  Text,
  TextSpan,
  TextView,
  Theme,
  Timer,
  TimerCancelledEvent,
  TimerCompletedEvent,
  TimerPausedEvent,
  TimerResumedEvent,
  TimerStartedEvent,
  TraitDefinition,
  Transition,
  TransitionStyle,
  Trigger,
  TripleClickEvent,
  Type,
  User,
  Value,
  Vector2,
  Vector2i,
  Vector3,
  Vector3i,
  Vector4,
  Vector4i,
  WheelEvent,
} from "@destack/language";
import { NODE_CLASS_BY_TYPE, STRUCT_CLASS_BY_TYPE } from "@destack/language/registry";
import {
  base64Decode,
  base64Encode,
  timedeltaFromISOFormat,
  timedeltaToISOFormat,
} from "@destack/utils";
import { Temporal } from "temporal-polyfill";
export const CSON_ENCODERS: { [key: string]: CsonObjectEncoder } = {};
let loaded = false;
export function loadEncoders(): void {
  if (loaded) {
    return;
  }
  loaded = true;

  class TagCsonEncoder implements CsonObjectEncoder {
    packObject(object: Tag): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 12000;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      if (object._icon != null) {
        objectCson["102"] = object._icon.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Tag {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
      const iconValue = objectCson["102"];
      const unpackedIcon =
        iconValue != undefined ? (_Icon.unpack(2, iconValue, _session) as Icon) : undefined;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[12000] as typeof Tag)({
        icon: unpackedIcon,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 12000)] = new TagCsonEncoder();

  class TaggingCsonEncoder implements CsonObjectEncoder {
    packObject(object: Tagging): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 12100;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      objectCson["110"] = object._tagPtr.pack(2);
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Tagging {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[12100] as typeof Tagging)({
        tag: _NodeReference.unpack(2, objectCson["110"], _session) as NodeReference,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 12100)] = new TaggingCsonEncoder();

  class CustomEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: CustomEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 20000;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      if (object._icon != null) {
        objectCson["102"] = object._icon.pack(2);
      }
      if (object._baseType != null) {
        objectCson["110"] = object._baseType.pack(2);
      }
      const packedSelfTraits: any[] = [];
      for (const item of object._selfTraits) {
        packedSelfTraits.push(item.pack(2));
      }
      objectCson["111"] = packedSelfTraits;
      objectCson["112"] = object._isAbstract;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): CustomEvent {
      const _NodeDefinitionReference = STRUCT_CLASS_BY_TYPE[13] as typeof NodeDefinitionReference;
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
      const iconValue = objectCson["102"];
      const unpackedIcon =
        iconValue != undefined ? (_Icon.unpack(2, iconValue, _session) as Icon) : undefined;
      const baseTypeValue = objectCson["110"];
      const unpackedBaseType =
        baseTypeValue != undefined
          ? (_NodeDefinitionReference.unpack(2, baseTypeValue, _session) as NodeDefinitionReference)
          : undefined;
      const unpackedSelfTraits: any[] = [];
      for (const item of objectCson["111"]) {
        unpackedSelfTraits.push(
          _NodeDefinitionReference.unpack(2, item, _session) as NodeDefinitionReference,
        );
      }
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[20000] as typeof CustomEvent)({
        icon: unpackedIcon,
        baseType: unpackedBaseType,
        selfTraits: unpackedSelfTraits,
        isAbstract: Boolean(objectCson["112"]),
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 20000)] = new CustomEventCsonEncoder();

  class EditEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: EditEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 90100;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      objectCson["100"] = object.type;
      objectCson["101"] = object.nodePtr.pack(2);
      if (object.operation != null) {
        objectCson["102"] = object.operation;
      }
      if (object.propertyId != null) {
        objectCson["103"] = object.propertyId;
      }
      if (object.customPropertyPtr != null) {
        objectCson["104"] = object.customPropertyPtr.pack(2);
      }
      if (object.key != null) {
        objectCson["105"] = object.key.pack(2);
      }
      if (object.value != null) {
        objectCson["110"] = object.value.pack(2);
      }
      if (object.reverseOperation != null) {
        objectCson["202"] = object.reverseOperation;
      }
      if (object.reverseValue != null) {
        objectCson["210"] = object.reverseValue.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): EditEvent {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const operationValue = objectCson["102"];
      const unpackedOperation = operationValue != undefined ? Number(operationValue) : undefined;
      const propertyIdValue = objectCson["103"];
      const unpackedPropertyId = propertyIdValue != undefined ? Number(propertyIdValue) : undefined;
      const customPropertyPtrValue = objectCson["104"];
      const unpackedCustomPropertyPtr =
        customPropertyPtrValue != undefined
          ? (_NodeReference.unpack(2, customPropertyPtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["105"];
      const unpackedKey =
        keyValue != undefined ? (_Value.unpack(2, keyValue, _session) as Value) : undefined;
      const valueValue = objectCson["110"];
      const unpackedValue =
        valueValue != undefined ? (_Value.unpack(2, valueValue, _session) as Value) : undefined;
      const reverseOperationValue = objectCson["202"];
      const unpackedReverseOperation =
        reverseOperationValue != undefined ? Number(reverseOperationValue) : undefined;
      const reverseValueValue = objectCson["210"];
      const unpackedReverseValue =
        reverseValueValue != undefined
          ? (_Value.unpack(2, reverseValueValue, _session) as Value)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[90100] as typeof EditEvent)({
        type: Number(objectCson["100"]),
        node: _NodeReference.unpack(2, objectCson["101"], _session) as NodeReference,
        operation: unpackedOperation,
        propertyId: unpackedPropertyId,
        customProperty: unpackedCustomPropertyPtr,
        key: unpackedKey,
        value: unpackedValue,
        reverseOperation: unpackedReverseOperation,
        reverseValue: unpackedReverseValue,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 90100)] = new EditEventCsonEncoder();

  class PermissionCsonEncoder implements CsonObjectEncoder {
    packObject(object: Permission): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 50000;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Permission {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[50000] as typeof Permission)({
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 50000)] = new PermissionCsonEncoder();

  class MethodCsonEncoder implements CsonObjectEncoder {
    packObject(object: Method): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 40000;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      objectCson["100"] = object._type;
      if (object._text != null) {
        objectCson["104"] = object._text.pack(2);
      }
      objectCson["110"] = object._cardinality;
      const packedPlatforms: any[] = [];
      for (const item of object._platforms) {
        packedPlatforms.push(item);
      }
      objectCson["130"] = packedPlatforms;
      const packedLanguages: any[] = [];
      for (const item of object._languages) {
        packedLanguages.push(item);
      }
      objectCson["131"] = packedLanguages;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Method {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Text = STRUCT_CLASS_BY_TYPE[400020] as typeof Text;
      const textValue = objectCson["104"];
      const unpackedText =
        textValue != undefined ? (_Text.unpack(2, textValue, _session) as Text) : undefined;
      const unpackedPlatforms: any[] = [];
      for (const item of objectCson["130"]) {
        unpackedPlatforms.push(Number(item));
      }
      const unpackedLanguages: any[] = [];
      for (const item of objectCson["131"]) {
        unpackedLanguages.push(Number(item));
      }
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[40000] as typeof Method)({
        type: Number(objectCson["100"]),
        text: unpackedText,
        cardinality: Number(objectCson["110"]),
        platforms: unpackedPlatforms,
        languages: unpackedLanguages,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 40000)] = new MethodCsonEncoder();

  class ActionCsonEncoder implements CsonObjectEncoder {
    packObject(object: Action): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 40100;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      objectCson["100"] = object._type;
      if (object._text != null) {
        objectCson["104"] = object._text.pack(2);
      }
      objectCson["110"] = object._cardinality;
      const packedPlatforms: any[] = [];
      for (const item of object._platforms) {
        packedPlatforms.push(item);
      }
      objectCson["130"] = packedPlatforms;
      const packedLanguages: any[] = [];
      for (const item of object._languages) {
        packedLanguages.push(item);
      }
      objectCson["131"] = packedLanguages;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Action {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Text = STRUCT_CLASS_BY_TYPE[400020] as typeof Text;
      const textValue = objectCson["104"];
      const unpackedText =
        textValue != undefined ? (_Text.unpack(2, textValue, _session) as Text) : undefined;
      const unpackedPlatforms: any[] = [];
      for (const item of objectCson["130"]) {
        unpackedPlatforms.push(Number(item));
      }
      const unpackedLanguages: any[] = [];
      for (const item of objectCson["131"]) {
        unpackedLanguages.push(Number(item));
      }
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[40100] as typeof Action)({
        type: Number(objectCson["100"]),
        text: unpackedText,
        cardinality: Number(objectCson["110"]),
        platforms: unpackedPlatforms,
        languages: unpackedLanguages,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 40100)] = new ActionCsonEncoder();

  class CustomEnumCsonEncoder implements CsonObjectEncoder {
    packObject(object: CustomEnum): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 20300;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      if (object._icon != null) {
        objectCson["102"] = object._icon.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): CustomEnum {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
      const iconValue = objectCson["102"];
      const unpackedIcon =
        iconValue != undefined ? (_Icon.unpack(2, iconValue, _session) as Icon) : undefined;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[20300] as typeof CustomEnum)({
        icon: unpackedIcon,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 20300)] = new CustomEnumCsonEncoder();

  class CustomOptionCsonEncoder implements CsonObjectEncoder {
    packObject(object: CustomOption): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 20400;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      if (object._icon != null) {
        objectCson["102"] = object._icon.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): CustomOption {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const iconValue = objectCson["102"];
      const unpackedIcon =
        iconValue != undefined ? (_Icon.unpack(2, iconValue, _session) as Icon) : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[20400] as typeof CustomOption)({
        parent: unpackedParentPtr,
        icon: unpackedIcon,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 20400)] = new CustomOptionCsonEncoder();

  class IndexCsonEncoder implements CsonObjectEncoder {
    packObject(object: Index): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 30100;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      objectCson["100"] = object._type;
      const packedProperties: any[] = [];
      for (const item of object._properties) {
        packedProperties.push(item.pack(2));
      }
      objectCson["105"] = packedProperties;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Index {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _PropertyReference = STRUCT_CLASS_BY_TYPE[1001] as typeof PropertyReference;
      const unpackedProperties: any[] = [];
      for (const item of objectCson["105"]) {
        unpackedProperties.push(_PropertyReference.unpack(2, item, _session) as PropertyReference);
      }
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[30100] as typeof Index)({
        type: Number(objectCson["100"]),
        properties: unpackedProperties,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 30100)] = new IndexCsonEncoder();

  class ConstraintCsonEncoder implements CsonObjectEncoder {
    packObject(object: Constraint): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 30200;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      objectCson["100"] = object._type;
      const packedProperties: any[] = [];
      for (const item of object._properties) {
        packedProperties.push(item.pack(2));
      }
      objectCson["105"] = packedProperties;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Constraint {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _PropertyReference = STRUCT_CLASS_BY_TYPE[1001] as typeof PropertyReference;
      const unpackedProperties: any[] = [];
      for (const item of objectCson["105"]) {
        unpackedProperties.push(_PropertyReference.unpack(2, item, _session) as PropertyReference);
      }
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[30200] as typeof Constraint)({
        type: Number(objectCson["100"]),
        properties: unpackedProperties,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 30200)] = new ConstraintCsonEncoder();

  class MigrationCsonEncoder implements CsonObjectEncoder {
    packObject(object: Migration): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 31000;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      objectCson["100"] = object._type;
      if (object._description != null) {
        objectCson["103"] = object._description;
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Migration {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const descriptionValue = objectCson["103"];
      const unpackedDescription = descriptionValue != undefined ? descriptionValue : undefined;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[31000] as typeof Migration)({
        type: Number(objectCson["100"]),
        description: unpackedDescription,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 31000)] = new MigrationCsonEncoder();

  class MigrationOperationCsonEncoder implements CsonObjectEncoder {
    packObject(object: MigrationOperation): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 31100;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): MigrationOperation {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[31100] as typeof MigrationOperation)({
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 31100)] = new MigrationOperationCsonEncoder();

  class CustomPropertyCsonEncoder implements CsonObjectEncoder {
    packObject(object: CustomProperty): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 20200;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      objectCson["100"] = object._type.pack(2);
      if (object._icon != null) {
        objectCson["102"] = object._icon.pack(2);
      }
      objectCson["103"] = object._zone;
      if (object._edgeType != null) {
        objectCson["140"] = object._edgeType;
      }
      if (object._cascade != null) {
        objectCson["141"] = object._cascade;
      }
      if (object._isUnique != null) {
        objectCson["201"] = object._isUnique;
      }
      if (object._isReadonly != null) {
        objectCson["202"] = object._isReadonly;
      }
      if (object._isMain != null) {
        objectCson["203"] = object._isMain;
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): CustomProperty {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _CheckedType = STRUCT_CLASS_BY_TYPE[102] as typeof CheckedType;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
      const iconValue = objectCson["102"];
      const unpackedIcon =
        iconValue != undefined ? (_Icon.unpack(2, iconValue, _session) as Icon) : undefined;
      const edgeTypeValue = objectCson["140"];
      const unpackedEdgeType = edgeTypeValue != undefined ? Number(edgeTypeValue) : undefined;
      const cascadeValue = objectCson["141"];
      const unpackedCascade = cascadeValue != undefined ? Number(cascadeValue) : undefined;
      const isUniqueValue = objectCson["201"];
      const unpackedIsUnique = isUniqueValue != undefined ? Boolean(isUniqueValue) : undefined;
      const isReadonlyValue = objectCson["202"];
      const unpackedIsReadonly =
        isReadonlyValue != undefined ? Boolean(isReadonlyValue) : undefined;
      const isMainValue = objectCson["203"];
      const unpackedIsMain = isMainValue != undefined ? Boolean(isMainValue) : undefined;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[20200] as typeof CustomProperty)({
        type: _CheckedType.unpack(2, objectCson["100"], _session) as CheckedType,
        icon: unpackedIcon,
        zone: Number(objectCson["103"]),
        edgeType: unpackedEdgeType,
        cascade: unpackedCascade,
        isUnique: unpackedIsUnique,
        isReadonly: unpackedIsReadonly,
        isMain: unpackedIsMain,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 20200)] = new CustomPropertyCsonEncoder();

  class SpaceCsonEncoder implements CsonObjectEncoder {
    packObject(object: Space): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1100;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      objectCson["102"] = object._slug;
      if (object._handlePtr != null) {
        objectCson["111"] = object._handlePtr.pack(2);
      }
      objectCson["120"] = object._region;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Space {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const handlePtrValue = objectCson["111"];
      const unpackedHandlePtr =
        handlePtrValue != undefined
          ? (_NodeReference.unpack(2, handlePtrValue, _session) as NodeReference)
          : undefined;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[1100] as typeof Space)({
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        slug: objectCson["102"],
        handle: unpackedHandlePtr,
        region: Number(objectCson["120"]),
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1100)] = new SpaceCsonEncoder();

  class CustomStructCsonEncoder implements CsonObjectEncoder {
    packObject(object: CustomStruct): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 20100;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      if (object._icon != null) {
        objectCson["102"] = object._icon.pack(2);
      }
      if (object._baseType != null) {
        objectCson["110"] = object._baseType.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): CustomStruct {
      const _StructDefinitionReference =
        STRUCT_CLASS_BY_TYPE[16] as typeof StructDefinitionReference;
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
      const iconValue = objectCson["102"];
      const unpackedIcon =
        iconValue != undefined ? (_Icon.unpack(2, iconValue, _session) as Icon) : undefined;
      const baseTypeValue = objectCson["110"];
      const unpackedBaseType =
        baseTypeValue != undefined
          ? (_StructDefinitionReference.unpack(
              2,
              baseTypeValue,
              _session,
            ) as StructDefinitionReference)
          : undefined;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[20100] as typeof CustomStruct)({
        icon: unpackedIcon,
        baseType: unpackedBaseType,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 20100)] = new CustomStructCsonEncoder();

  class BranchCsonEncoder implements CsonObjectEncoder {
    packObject(object: Branch): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2000;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      objectCson["100"] = object._type;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Branch {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[2000] as typeof Branch)({
        parent: unpackedParentPtr,
        type: Number(objectCson["100"]),
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 2000)] = new BranchCsonEncoder();

  class SnapshotCsonEncoder implements CsonObjectEncoder {
    packObject(object: Snapshot): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2100;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      objectCson["100"] = object._type;
      objectCson["110"] = object._status;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Snapshot {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[2100] as typeof Snapshot)({
        parent: unpackedParentPtr,
        type: Number(objectCson["100"]),
        status: Number(objectCson["110"]),
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 2100)] = new SnapshotCsonEncoder();

  class EntitlementRequestedEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: EntitlementRequestedEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 360502;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      objectCson["101"] = object.nodePtr.pack(2);
      objectCson["110"] = object.targetPtr.pack(2);
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): EntitlementRequestedEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[360502] as typeof EntitlementRequestedEvent)({
        node: _NodeReference.unpack(2, objectCson["101"], _session) as NodeReference,
        target: _NodeReference.unpack(2, objectCson["110"], _session) as NodeReference,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 360502)] = new EntitlementRequestedEventCsonEncoder();

  class EntitlementGrantedEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: EntitlementGrantedEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 360503;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      objectCson["101"] = object.nodePtr.pack(2);
      objectCson["110"] = object.targetPtr.pack(2);
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): EntitlementGrantedEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[360503] as typeof EntitlementGrantedEvent)({
        node: _NodeReference.unpack(2, objectCson["101"], _session) as NodeReference,
        target: _NodeReference.unpack(2, objectCson["110"], _session) as NodeReference,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 360503)] = new EntitlementGrantedEventCsonEncoder();

  class EntitlementRevokedEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: EntitlementRevokedEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 360504;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      objectCson["101"] = object.nodePtr.pack(2);
      objectCson["110"] = object.targetPtr.pack(2);
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): EntitlementRevokedEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[360504] as typeof EntitlementRevokedEvent)({
        node: _NodeReference.unpack(2, objectCson["101"], _session) as NodeReference,
        target: _NodeReference.unpack(2, objectCson["110"], _session) as NodeReference,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 360504)] = new EntitlementRevokedEventCsonEncoder();

  class EntitlementExpiredEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: EntitlementExpiredEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 360505;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      objectCson["101"] = object.nodePtr.pack(2);
      objectCson["110"] = object.targetPtr.pack(2);
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): EntitlementExpiredEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[360505] as typeof EntitlementExpiredEvent)({
        node: _NodeReference.unpack(2, objectCson["101"], _session) as NodeReference,
        target: _NodeReference.unpack(2, objectCson["110"], _session) as NodeReference,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 360505)] = new EntitlementExpiredEventCsonEncoder();

  class EntitlementCsonEncoder implements CsonObjectEncoder {
    packObject(object: Entitlement): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 360500;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      objectCson["100"] = object._type;
      if (object._expiresAt != null) {
        objectCson["110"] = object._expiresAt.toString({ timeZoneName: "never" });
      }
      objectCson["111"] = object._targetPtr.pack(2);
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Entitlement {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const expiresAtValue = objectCson["110"];
      const unpackedExpiresAt =
        expiresAtValue != undefined
          ? Temporal.Instant.from(expiresAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[360500] as typeof Entitlement)({
        type: Number(objectCson["100"]),
        expiresAt: unpackedExpiresAt,
        target: _NodeReference.unpack(2, objectCson["111"], _session) as NodeReference,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 360500)] = new EntitlementCsonEncoder();

  class InviteSentEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: InviteSentEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 360102;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      objectCson["101"] = object.nodePtr.pack(2);
      objectCson["102"] = object.joinablePtr.pack(2);
      objectCson["103"] = object.memberPtr.pack(2);
      objectCson["110"] = object.rolePtr.pack(2);
      objectCson["111"] = object.roleType;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): InviteSentEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[360102] as typeof InviteSentEvent)({
        role: _NodeReference.unpack(2, objectCson["110"], _session) as NodeReference,
        roleType: Number(objectCson["111"]),
        node: _NodeReference.unpack(2, objectCson["101"], _session) as NodeReference,
        joinable: _NodeReference.unpack(2, objectCson["102"], _session) as NodeReference,
        member: _NodeReference.unpack(2, objectCson["103"], _session) as NodeReference,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 360102)] = new InviteSentEventCsonEncoder();

  class InviteRescindedEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: InviteRescindedEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 360103;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      objectCson["101"] = object.nodePtr.pack(2);
      objectCson["102"] = object.joinablePtr.pack(2);
      objectCson["103"] = object.memberPtr.pack(2);
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): InviteRescindedEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[360103] as typeof InviteRescindedEvent)({
        node: _NodeReference.unpack(2, objectCson["101"], _session) as NodeReference,
        joinable: _NodeReference.unpack(2, objectCson["102"], _session) as NodeReference,
        member: _NodeReference.unpack(2, objectCson["103"], _session) as NodeReference,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 360103)] = new InviteRescindedEventCsonEncoder();

  class InviteAcceptedEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: InviteAcceptedEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 360104;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      objectCson["101"] = object.nodePtr.pack(2);
      objectCson["102"] = object.joinablePtr.pack(2);
      objectCson["103"] = object.memberPtr.pack(2);
      objectCson["110"] = object.rolePtr.pack(2);
      objectCson["111"] = object.roleType;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): InviteAcceptedEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[360104] as typeof InviteAcceptedEvent)({
        role: _NodeReference.unpack(2, objectCson["110"], _session) as NodeReference,
        roleType: Number(objectCson["111"]),
        node: _NodeReference.unpack(2, objectCson["101"], _session) as NodeReference,
        joinable: _NodeReference.unpack(2, objectCson["102"], _session) as NodeReference,
        member: _NodeReference.unpack(2, objectCson["103"], _session) as NodeReference,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 360104)] = new InviteAcceptedEventCsonEncoder();

  class InviteRejectedEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: InviteRejectedEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 360105;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      objectCson["101"] = object.nodePtr.pack(2);
      objectCson["102"] = object.joinablePtr.pack(2);
      objectCson["103"] = object.memberPtr.pack(2);
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): InviteRejectedEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[360105] as typeof InviteRejectedEvent)({
        node: _NodeReference.unpack(2, objectCson["101"], _session) as NodeReference,
        joinable: _NodeReference.unpack(2, objectCson["102"], _session) as NodeReference,
        member: _NodeReference.unpack(2, objectCson["103"], _session) as NodeReference,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 360105)] = new InviteRejectedEventCsonEncoder();

  class InviteCsonEncoder implements CsonObjectEncoder {
    packObject(object: Invite): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 360100;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      objectCson["110"] = object._memberPtr.pack(2);
      if (object._rolePtr != null) {
        objectCson["111"] = object._rolePtr.pack(2);
      }
      if (object._roleType != null) {
        objectCson["112"] = object._roleType;
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Invite {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const rolePtrValue = objectCson["111"];
      const unpackedRolePtr =
        rolePtrValue != undefined
          ? (_NodeReference.unpack(2, rolePtrValue, _session) as NodeReference)
          : undefined;
      const roleTypeValue = objectCson["112"];
      const unpackedRoleType = roleTypeValue != undefined ? Number(roleTypeValue) : undefined;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[360100] as typeof Invite)({
        member: _NodeReference.unpack(2, objectCson["110"], _session) as NodeReference,
        role: unpackedRolePtr,
        roleType: unpackedRoleType,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 360100)] = new InviteCsonEncoder();

  class MembershipJoinedEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: MembershipJoinedEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 360002;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      objectCson["101"] = object.nodePtr.pack(2);
      objectCson["102"] = object.joinablePtr.pack(2);
      objectCson["103"] = object.memberPtr.pack(2);
      objectCson["110"] = object.rolePtr.pack(2);
      objectCson["111"] = object.roleType;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): MembershipJoinedEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[360002] as typeof MembershipJoinedEvent)({
        role: _NodeReference.unpack(2, objectCson["110"], _session) as NodeReference,
        roleType: Number(objectCson["111"]),
        node: _NodeReference.unpack(2, objectCson["101"], _session) as NodeReference,
        joinable: _NodeReference.unpack(2, objectCson["102"], _session) as NodeReference,
        member: _NodeReference.unpack(2, objectCson["103"], _session) as NodeReference,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 360002)] = new MembershipJoinedEventCsonEncoder();

  class MembershipLeftEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: MembershipLeftEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 360003;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      objectCson["101"] = object.nodePtr.pack(2);
      objectCson["102"] = object.joinablePtr.pack(2);
      objectCson["103"] = object.memberPtr.pack(2);
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): MembershipLeftEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[360003] as typeof MembershipLeftEvent)({
        node: _NodeReference.unpack(2, objectCson["101"], _session) as NodeReference,
        joinable: _NodeReference.unpack(2, objectCson["102"], _session) as NodeReference,
        member: _NodeReference.unpack(2, objectCson["103"], _session) as NodeReference,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 360003)] = new MembershipLeftEventCsonEncoder();

  class MembershipCsonEncoder implements CsonObjectEncoder {
    packObject(object: Membership): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 360000;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      objectCson["110"] = object._memberPtr.pack(2);
      if (object._rolePtr != null) {
        objectCson["111"] = object._rolePtr.pack(2);
      }
      if (object._roleType != null) {
        objectCson["112"] = object._roleType;
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Membership {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const rolePtrValue = objectCson["111"];
      const unpackedRolePtr =
        rolePtrValue != undefined
          ? (_NodeReference.unpack(2, rolePtrValue, _session) as NodeReference)
          : undefined;
      const roleTypeValue = objectCson["112"];
      const unpackedRoleType = roleTypeValue != undefined ? Number(roleTypeValue) : undefined;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[360000] as typeof Membership)({
        member: _NodeReference.unpack(2, objectCson["110"], _session) as NodeReference,
        role: unpackedRolePtr,
        roleType: unpackedRoleType,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 360000)] = new MembershipCsonEncoder();

  class RoleAssignedEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: RoleAssignedEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 360202;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      objectCson["101"] = object.nodePtr.pack(2);
      objectCson["110"] = object.actorPtr.pack(2);
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): RoleAssignedEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[360202] as typeof RoleAssignedEvent)({
        node: _NodeReference.unpack(2, objectCson["101"], _session) as NodeReference,
        actor: _NodeReference.unpack(2, objectCson["110"], _session) as NodeReference,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 360202)] = new RoleAssignedEventCsonEncoder();

  class RoleUnassignedEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: RoleUnassignedEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 360203;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      objectCson["101"] = object.nodePtr.pack(2);
      objectCson["110"] = object.actorPtr.pack(2);
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): RoleUnassignedEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[360203] as typeof RoleUnassignedEvent)({
        node: _NodeReference.unpack(2, objectCson["101"], _session) as NodeReference,
        actor: _NodeReference.unpack(2, objectCson["110"], _session) as NodeReference,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 360203)] = new RoleUnassignedEventCsonEncoder();

  class RoleCsonEncoder implements CsonObjectEncoder {
    packObject(object: Role): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 360200;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      objectCson["100"] = object._type;
      if (object._icon != null) {
        objectCson["102"] = object._icon.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Role {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
      const iconValue = objectCson["102"];
      const unpackedIcon =
        iconValue != undefined ? (_Icon.unpack(2, iconValue, _session) as Icon) : undefined;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[360200] as typeof Role)({
        type: Number(objectCson["100"]),
        icon: unpackedIcon,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 360200)] = new RoleCsonEncoder();

  class SanctionRequestedEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: SanctionRequestedEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 360402;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      objectCson["101"] = object.nodePtr.pack(2);
      objectCson["110"] = object.targetPtr.pack(2);
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): SanctionRequestedEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[360402] as typeof SanctionRequestedEvent)({
        node: _NodeReference.unpack(2, objectCson["101"], _session) as NodeReference,
        target: _NodeReference.unpack(2, objectCson["110"], _session) as NodeReference,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 360402)] = new SanctionRequestedEventCsonEncoder();

  class SanctionGrantedEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: SanctionGrantedEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 360403;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      objectCson["101"] = object.nodePtr.pack(2);
      objectCson["110"] = object.targetPtr.pack(2);
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): SanctionGrantedEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[360403] as typeof SanctionGrantedEvent)({
        node: _NodeReference.unpack(2, objectCson["101"], _session) as NodeReference,
        target: _NodeReference.unpack(2, objectCson["110"], _session) as NodeReference,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 360403)] = new SanctionGrantedEventCsonEncoder();

  class SanctionRevokedEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: SanctionRevokedEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 360404;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      objectCson["101"] = object.nodePtr.pack(2);
      objectCson["110"] = object.targetPtr.pack(2);
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): SanctionRevokedEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[360404] as typeof SanctionRevokedEvent)({
        node: _NodeReference.unpack(2, objectCson["101"], _session) as NodeReference,
        target: _NodeReference.unpack(2, objectCson["110"], _session) as NodeReference,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 360404)] = new SanctionRevokedEventCsonEncoder();

  class SanctionExpiredEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: SanctionExpiredEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 360405;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      objectCson["101"] = object.nodePtr.pack(2);
      objectCson["110"] = object.targetPtr.pack(2);
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): SanctionExpiredEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[360405] as typeof SanctionExpiredEvent)({
        node: _NodeReference.unpack(2, objectCson["101"], _session) as NodeReference,
        target: _NodeReference.unpack(2, objectCson["110"], _session) as NodeReference,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 360405)] = new SanctionExpiredEventCsonEncoder();

  class SanctionCsonEncoder implements CsonObjectEncoder {
    packObject(object: Sanction): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 360400;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      objectCson["100"] = object._type;
      if (object._expiresAt != null) {
        objectCson["110"] = object._expiresAt.toString({ timeZoneName: "never" });
      }
      objectCson["111"] = object._targetPtr.pack(2);
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Sanction {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const expiresAtValue = objectCson["110"];
      const unpackedExpiresAt =
        expiresAtValue != undefined
          ? Temporal.Instant.from(expiresAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[360400] as typeof Sanction)({
        type: Number(objectCson["100"]),
        expiresAt: unpackedExpiresAt,
        target: _NodeReference.unpack(2, objectCson["111"], _session) as NodeReference,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 360400)] = new SanctionCsonEncoder();

  class ColorStyleCsonEncoder implements CsonObjectEncoder {
    packObject(object: ColorStyle): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2100300;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      objectCson["100"] = object._type;
      if (object._hue != null) {
        objectCson["200"] = object._hue;
      }
      if (object._shade != null) {
        objectCson["201"] = object._shade;
      }
      if (object._intent != null) {
        objectCson["202"] = object._intent;
      }
      if (object._x != null) {
        objectCson["203"] = object._x;
      }
      if (object._y != null) {
        objectCson["204"] = object._y;
      }
      if (object._z != null) {
        objectCson["205"] = object._z;
      }
      if (object._alpha != null) {
        objectCson["206"] = object._alpha;
      }
      if (object._dark != null) {
        objectCson["207"] = object._dark.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): ColorStyle {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Color = STRUCT_CLASS_BY_TYPE[2100300] as typeof Color;
      const hueValue = objectCson["200"];
      const unpackedHue = hueValue != undefined ? Number(hueValue) : undefined;
      const shadeValue = objectCson["201"];
      const unpackedShade = shadeValue != undefined ? Number(shadeValue) : undefined;
      const intentValue = objectCson["202"];
      const unpackedIntent = intentValue != undefined ? Number(intentValue) : undefined;
      const xValue = objectCson["203"];
      const unpackedX = xValue != undefined ? Number(xValue) : undefined;
      const yValue = objectCson["204"];
      const unpackedY = yValue != undefined ? Number(yValue) : undefined;
      const zValue = objectCson["205"];
      const unpackedZ = zValue != undefined ? Number(zValue) : undefined;
      const alphaValue = objectCson["206"];
      const unpackedAlpha = alphaValue != undefined ? Number(alphaValue) : undefined;
      const darkValue = objectCson["207"];
      const unpackedDark =
        darkValue != undefined ? (_Color.unpack(2, darkValue, _session) as Color) : undefined;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[2100300] as typeof ColorStyle)({
        type: Number(objectCson["100"]),
        hue: unpackedHue,
        shade: unpackedShade,
        intent: unpackedIntent,
        x: unpackedX,
        y: unpackedY,
        z: unpackedZ,
        alpha: unpackedAlpha,
        dark: unpackedDark,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 2100300)] = new ColorStyleCsonEncoder();

  class BorderStyleCsonEncoder implements CsonObjectEncoder {
    packObject(object: BorderStyle): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2100600;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      objectCson["100"] = object._type;
      if (object._color != null) {
        objectCson["200"] = object._color.pack(2);
      }
      if (object._width != null) {
        objectCson["201"] = object._width.pack(2);
      }
      if (object._stylePtr != null) {
        objectCson["202"] = object._stylePtr.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): BorderStyle {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Color = STRUCT_CLASS_BY_TYPE[2100300] as typeof Color;
      const _Inset2 = STRUCT_CLASS_BY_TYPE[2400023] as typeof Inset2;
      const colorValue = objectCson["200"];
      const unpackedColor =
        colorValue != undefined ? (_Color.unpack(2, colorValue, _session) as Color) : undefined;
      const widthValue = objectCson["201"];
      const unpackedWidth =
        widthValue != undefined ? (_Inset2.unpack(2, widthValue, _session) as Inset2) : undefined;
      const stylePtrValue = objectCson["202"];
      const unpackedStylePtr =
        stylePtrValue != undefined
          ? (_NodeReference.unpack(2, stylePtrValue, _session) as NodeReference)
          : undefined;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[2100600] as typeof BorderStyle)({
        type: Number(objectCson["100"]),
        color: unpackedColor,
        width: unpackedWidth,
        style: unpackedStylePtr,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 2100600)] = new BorderStyleCsonEncoder();

  class GradientStyleCsonEncoder implements CsonObjectEncoder {
    packObject(object: GradientStyle): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2100800;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      objectCson["100"] = object._type;
      if (object._angle != null) {
        objectCson["102"] = object._angle;
      }
      const packedStops: any[] = [];
      for (const item of object._stops) {
        packedStops.push(item.pack(2));
      }
      objectCson["103"] = packedStops;
      if (object._centerAnchor != null) {
        objectCson["104"] = object._centerAnchor.pack(2);
      }
      if (object._dark != null) {
        objectCson["105"] = object._dark.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): GradientStyle {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Gradient = STRUCT_CLASS_BY_TYPE[2100800] as typeof Gradient;
      const _GradientStop = STRUCT_CLASS_BY_TYPE[2100801] as typeof GradientStop;
      const _Axis2 = STRUCT_CLASS_BY_TYPE[2400025] as typeof Axis2;
      const angleValue = objectCson["102"];
      const unpackedAngle = angleValue != undefined ? Number(angleValue) : undefined;
      const unpackedStops: any[] = [];
      for (const item of objectCson["103"]) {
        unpackedStops.push(_GradientStop.unpack(2, item, _session) as GradientStop);
      }
      const centerAnchorValue = objectCson["104"];
      const unpackedCenterAnchor =
        centerAnchorValue != undefined
          ? (_Axis2.unpack(2, centerAnchorValue, _session) as Axis2)
          : undefined;
      const darkValue = objectCson["105"];
      const unpackedDark =
        darkValue != undefined ? (_Gradient.unpack(2, darkValue, _session) as Gradient) : undefined;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[2100800] as typeof GradientStyle)({
        type: Number(objectCson["100"]),
        angle: unpackedAngle,
        stops: unpackedStops,
        centerAnchor: unpackedCenterAnchor,
        dark: unpackedDark,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 2100800)] = new GradientStyleCsonEncoder();

  class FillStyleCsonEncoder implements CsonObjectEncoder {
    packObject(object: FillStyle): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2100400;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      objectCson["100"] = object._type;
      if (object._color != null) {
        objectCson["200"] = object._color.pack(2);
      }
      if (object._gradient != null) {
        objectCson["201"] = object._gradient.pack(2);
      }
      if (object._imagePtr != null) {
        objectCson["202"] = object._imagePtr.pack(2);
      }
      if (object._position != null) {
        objectCson["203"] = object._position;
      }
      if (object._size != null) {
        objectCson["204"] = object._size;
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): FillStyle {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Color = STRUCT_CLASS_BY_TYPE[2100300] as typeof Color;
      const _Gradient = STRUCT_CLASS_BY_TYPE[2100800] as typeof Gradient;
      const colorValue = objectCson["200"];
      const unpackedColor =
        colorValue != undefined ? (_Color.unpack(2, colorValue, _session) as Color) : undefined;
      const gradientValue = objectCson["201"];
      const unpackedGradient =
        gradientValue != undefined
          ? (_Gradient.unpack(2, gradientValue, _session) as Gradient)
          : undefined;
      const imagePtrValue = objectCson["202"];
      const unpackedImagePtr =
        imagePtrValue != undefined
          ? (_NodeReference.unpack(2, imagePtrValue, _session) as NodeReference)
          : undefined;
      const positionValue = objectCson["203"];
      const unpackedPosition = positionValue != undefined ? Number(positionValue) : undefined;
      const sizeValue = objectCson["204"];
      const unpackedSize = sizeValue != undefined ? Number(sizeValue) : undefined;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[2100400] as typeof FillStyle)({
        type: Number(objectCson["100"]),
        color: unpackedColor,
        gradient: unpackedGradient,
        image: unpackedImagePtr,
        position: unpackedPosition,
        size: unpackedSize,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 2100400)] = new FillStyleCsonEncoder();

  class FontStyleCsonEncoder implements CsonObjectEncoder {
    packObject(object: FontStyle): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2100500;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      objectCson["100"] = object._type;
      if (object._weight != null) {
        objectCson["102"] = object._weight;
      }
      if (object._color != null) {
        objectCson["103"] = object._color.pack(2);
      }
      if (object._size != null) {
        objectCson["104"] = object._size;
      }
      if (object._align != null) {
        objectCson["105"] = object._align;
      }
      if (object._lineHeight != null) {
        objectCson["106"] = object._lineHeight.pack(2);
      }
      if (object._letterSpacing != null) {
        objectCson["107"] = object._letterSpacing.pack(2);
      }
      if (object._decoration != null) {
        objectCson["108"] = object._decoration;
      }
      if (object._transform != null) {
        objectCson["109"] = object._transform;
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): FontStyle {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Length = STRUCT_CLASS_BY_TYPE[1800001] as typeof Length;
      const _Fill = STRUCT_CLASS_BY_TYPE[2100400] as typeof Fill;
      const weightValue = objectCson["102"];
      const unpackedWeight = weightValue != undefined ? Number(weightValue) : undefined;
      const colorValue = objectCson["103"];
      const unpackedColor =
        colorValue != undefined ? (_Fill.unpack(2, colorValue, _session) as Fill) : undefined;
      const sizeValue = objectCson["104"];
      const unpackedSize = sizeValue != undefined ? Number(sizeValue) : undefined;
      const alignValue = objectCson["105"];
      const unpackedAlign = alignValue != undefined ? Number(alignValue) : undefined;
      const lineHeightValue = objectCson["106"];
      const unpackedLineHeight =
        lineHeightValue != undefined
          ? (_Length.unpack(2, lineHeightValue, _session) as Length)
          : undefined;
      const letterSpacingValue = objectCson["107"];
      const unpackedLetterSpacing =
        letterSpacingValue != undefined
          ? (_Length.unpack(2, letterSpacingValue, _session) as Length)
          : undefined;
      const decorationValue = objectCson["108"];
      const unpackedDecoration = decorationValue != undefined ? Number(decorationValue) : undefined;
      const transformValue = objectCson["109"];
      const unpackedTransform = transformValue != undefined ? Number(transformValue) : undefined;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[2100500] as typeof FontStyle)({
        type: Number(objectCson["100"]),
        weight: unpackedWeight,
        color: unpackedColor,
        size: unpackedSize,
        align: unpackedAlign,
        lineHeight: unpackedLineHeight,
        letterSpacing: unpackedLetterSpacing,
        decoration: unpackedDecoration,
        transform: unpackedTransform,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 2100500)] = new FontStyleCsonEncoder();

  class PaletteCsonEncoder implements CsonObjectEncoder {
    packObject(object: Palette): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2100100;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      if (object._icon != null) {
        objectCson["102"] = object._icon.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Palette {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
      const iconValue = objectCson["102"];
      const unpackedIcon =
        iconValue != undefined ? (_Icon.unpack(2, iconValue, _session) as Icon) : undefined;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[2100100] as typeof Palette)({
        icon: unpackedIcon,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 2100100)] = new PaletteCsonEncoder();

  class ShadowStyleCsonEncoder implements CsonObjectEncoder {
    packObject(object: ShadowStyle): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2100700;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      objectCson["100"] = object._type;
      if (object._color != null) {
        objectCson["200"] = object._color.pack(2);
      }
      objectCson["201"] = object._position;
      if (object._offset != null) {
        objectCson["202"] = object._offset.pack(2);
      }
      if (object._blur != null) {
        objectCson["203"] = object._blur;
      }
      if (object._spread != null) {
        objectCson["204"] = object._spread;
      }
      if (object._diffusion != null) {
        objectCson["205"] = object._diffusion;
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): ShadowStyle {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Color = STRUCT_CLASS_BY_TYPE[2100300] as typeof Color;
      const _Axis2 = STRUCT_CLASS_BY_TYPE[2400025] as typeof Axis2;
      const colorValue = objectCson["200"];
      const unpackedColor =
        colorValue != undefined ? (_Color.unpack(2, colorValue, _session) as Color) : undefined;
      const offsetValue = objectCson["202"];
      const unpackedOffset =
        offsetValue != undefined ? (_Axis2.unpack(2, offsetValue, _session) as Axis2) : undefined;
      const blurValue = objectCson["203"];
      const unpackedBlur = blurValue != undefined ? Number(blurValue) : undefined;
      const spreadValue = objectCson["204"];
      const unpackedSpread = spreadValue != undefined ? Number(spreadValue) : undefined;
      const diffusionValue = objectCson["205"];
      const unpackedDiffusion = diffusionValue != undefined ? Number(diffusionValue) : undefined;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[2100700] as typeof ShadowStyle)({
        type: Number(objectCson["100"]),
        color: unpackedColor,
        position: Number(objectCson["201"]),
        offset: unpackedOffset,
        blur: unpackedBlur,
        spread: unpackedSpread,
        diffusion: unpackedDiffusion,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 2100700)] = new ShadowStyleCsonEncoder();

  class StrokeStyleCsonEncoder implements CsonObjectEncoder {
    packObject(object: StrokeStyle): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2101100;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      objectCson["100"] = object._type;
      objectCson["200"] = object._size;
      objectCson["201"] = object._thinning;
      objectCson["202"] = object._smoothing;
      objectCson["203"] = object._streamline;
      objectCson["204"] = object._easing;
      if (object._start != null) {
        objectCson["205"] = object._start.pack(2);
      }
      if (object._end != null) {
        objectCson["206"] = object._end.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): StrokeStyle {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _StrokeCap = STRUCT_CLASS_BY_TYPE[2101101] as typeof StrokeCap;
      const startValue = objectCson["205"];
      const unpackedStart =
        startValue != undefined
          ? (_StrokeCap.unpack(2, startValue, _session) as StrokeCap)
          : undefined;
      const endValue = objectCson["206"];
      const unpackedEnd =
        endValue != undefined ? (_StrokeCap.unpack(2, endValue, _session) as StrokeCap) : undefined;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[2101100] as typeof StrokeStyle)({
        type: Number(objectCson["100"]),
        size: Number(objectCson["200"]),
        thinning: Number(objectCson["201"]),
        smoothing: Number(objectCson["202"]),
        streamline: Number(objectCson["203"]),
        easing: Number(objectCson["204"]),
        start: unpackedStart,
        end: unpackedEnd,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 2101100)] = new StrokeStyleCsonEncoder();

  class ThemeCsonEncoder implements CsonObjectEncoder {
    packObject(object: Theme): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2100000;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Theme {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[2100000] as typeof Theme)({
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 2100000)] = new ThemeCsonEncoder();

  class TransitionStyleCsonEncoder implements CsonObjectEncoder {
    packObject(object: TransitionStyle): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2200000;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      objectCson["100"] = object._type;
      if (object._delay != null) {
        objectCson["102"] = object._delay;
      }
      if (object._duration != null) {
        objectCson["103"] = object._duration;
      }
      const packedEase: any[] = [];
      for (const item of object._ease) {
        packedEase.push(item);
      }
      objectCson["104"] = packedEase;
      if (object._stiffness != null) {
        objectCson["105"] = object._stiffness;
      }
      if (object._damping != null) {
        objectCson["106"] = object._damping;
      }
      if (object._mass != null) {
        objectCson["107"] = object._mass;
      }
      if (object._bounce != null) {
        objectCson["108"] = object._bounce;
      }
      if (object._springType != null) {
        objectCson["109"] = object._springType;
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): TransitionStyle {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const delayValue = objectCson["102"];
      const unpackedDelay = delayValue != undefined ? Number(delayValue) : undefined;
      const durationValue = objectCson["103"];
      const unpackedDuration = durationValue != undefined ? Number(durationValue) : undefined;
      const unpackedEase: any[] = [];
      for (const item of objectCson["104"]) {
        unpackedEase.push(Number(item));
      }
      const stiffnessValue = objectCson["105"];
      const unpackedStiffness = stiffnessValue != undefined ? Number(stiffnessValue) : undefined;
      const dampingValue = objectCson["106"];
      const unpackedDamping = dampingValue != undefined ? Number(dampingValue) : undefined;
      const massValue = objectCson["107"];
      const unpackedMass = massValue != undefined ? Number(massValue) : undefined;
      const bounceValue = objectCson["108"];
      const unpackedBounce = bounceValue != undefined ? Number(bounceValue) : undefined;
      const springTypeValue = objectCson["109"];
      const unpackedSpringType = springTypeValue != undefined ? Number(springTypeValue) : undefined;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[2200000] as typeof TransitionStyle)({
        type: Number(objectCson["100"]),
        delay: unpackedDelay,
        duration: unpackedDuration,
        ease: unpackedEase,
        stiffness: unpackedStiffness,
        damping: unpackedDamping,
        mass: unpackedMass,
        bounce: unpackedBounce,
        springType: unpackedSpringType,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 2200000)] = new TransitionStyleCsonEncoder();

  class EffectStyleCsonEncoder implements CsonObjectEncoder {
    packObject(object: EffectStyle): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2200100;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      objectCson["100"] = object._type;
      if (object._opacity != null) {
        objectCson["200"] = object._opacity;
      }
      if (object._offset != null) {
        objectCson["201"] = object._offset.pack(2);
      }
      if (object._scale != null) {
        objectCson["202"] = object._scale;
      }
      if (object._rotate != null) {
        objectCson["203"] = object._rotate.pack(2);
      }
      if (object._skew != null) {
        objectCson["204"] = object._skew.pack(2);
      }
      if (object._perspective != null) {
        objectCson["205"] = object._perspective;
      }
      if (object._delay != null) {
        objectCson["206"] = timedeltaToISOFormat(object._delay);
      }
      if (object._duration != null) {
        objectCson["207"] = object._duration;
      }
      if (object._threshold != null) {
        objectCson["208"] = object._threshold;
      }
      if (object._once != null) {
        objectCson["209"] = object._once;
      }
      if (object._repeat != null) {
        objectCson["210"] = object._repeat;
      }
      if (object._split != null) {
        objectCson["211"] = object._split;
      }
      if (object._offscreen != null) {
        objectCson["212"] = object._offscreen;
      }
      if (object._transition != null) {
        objectCson["213"] = object._transition.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): EffectStyle {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Transition = STRUCT_CLASS_BY_TYPE[2200000] as typeof Transition;
      const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
      const _Axis3 = STRUCT_CLASS_BY_TYPE[2400026] as typeof Axis3;
      const opacityValue = objectCson["200"];
      const unpackedOpacity = opacityValue != undefined ? Number(opacityValue) : undefined;
      const offsetValue = objectCson["201"];
      const unpackedOffset =
        offsetValue != undefined
          ? (_Vector2.unpack(2, offsetValue, _session) as Vector2)
          : undefined;
      const scaleValue = objectCson["202"];
      const unpackedScale = scaleValue != undefined ? Number(scaleValue) : undefined;
      const rotateValue = objectCson["203"];
      const unpackedRotate =
        rotateValue != undefined ? (_Axis3.unpack(2, rotateValue, _session) as Axis3) : undefined;
      const skewValue = objectCson["204"];
      const unpackedSkew =
        skewValue != undefined ? (_Vector2.unpack(2, skewValue, _session) as Vector2) : undefined;
      const perspectiveValue = objectCson["205"];
      const unpackedPerspective =
        perspectiveValue != undefined ? Number(perspectiveValue) : undefined;
      const delayValue = objectCson["206"];
      const unpackedDelay =
        delayValue != undefined ? timedeltaFromISOFormat(delayValue) : undefined;
      const durationValue = objectCson["207"];
      const unpackedDuration = durationValue != undefined ? Number(durationValue) : undefined;
      const thresholdValue = objectCson["208"];
      const unpackedThreshold = thresholdValue != undefined ? Number(thresholdValue) : undefined;
      const onceValue = objectCson["209"];
      const unpackedOnce = onceValue != undefined ? Boolean(onceValue) : undefined;
      const repeatValue = objectCson["210"];
      const unpackedRepeat = repeatValue != undefined ? Number(repeatValue) : undefined;
      const splitValue = objectCson["211"];
      const unpackedSplit = splitValue != undefined ? Number(splitValue) : undefined;
      const offscreenValue = objectCson["212"];
      const unpackedOffscreen = offscreenValue != undefined ? Number(offscreenValue) : undefined;
      const transitionValue = objectCson["213"];
      const unpackedTransition =
        transitionValue != undefined
          ? (_Transition.unpack(2, transitionValue, _session) as Transition)
          : undefined;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[2200100] as typeof EffectStyle)({
        type: Number(objectCson["100"]),
        opacity: unpackedOpacity,
        offset: unpackedOffset,
        scale: unpackedScale,
        rotate: unpackedRotate,
        skew: unpackedSkew,
        perspective: unpackedPerspective,
        delay: unpackedDelay,
        duration: unpackedDuration,
        threshold: unpackedThreshold,
        once: unpackedOnce,
        repeat: unpackedRepeat,
        split: unpackedSplit,
        offscreen: unpackedOffscreen,
        transition: unpackedTransition,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 2200100)] = new EffectStyleCsonEncoder();

  class FileCsonEncoder implements CsonObjectEncoder {
    packObject(object: File): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 480000;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      objectCson["100"] = object._type;
      if (object._region != null) {
        objectCson["111"] = object._region;
      }
      if (object._mimeType != null) {
        objectCson["120"] = object._mimeType;
      }
      if (object._format != null) {
        objectCson["121"] = object._format;
      }
      if (object._size != null) {
        objectCson["122"] = object._size;
      }
      if (object._sha256 != null) {
        objectCson["123"] = object._sha256;
      }
      if (object._width != null) {
        objectCson["124"] = object._width;
      }
      if (object._height != null) {
        objectCson["125"] = object._height;
      }
      if (object._aspectRatio != null) {
        objectCson["126"] = object._aspectRatio;
      }
      if (object._codec != null) {
        objectCson["127"] = object._codec;
      }
      if (object._duration != null) {
        objectCson["128"] = timedeltaToISOFormat(object._duration);
      }
      if (object._url != null) {
        objectCson["130"] = object._url;
      }
      if (object._contentUrl != null) {
        objectCson["131"] = object._contentUrl;
      }
      if (object._thumbnailUrl != null) {
        objectCson["132"] = object._thumbnailUrl;
      }
      if (object._faviconUrl != null) {
        objectCson["133"] = object._faviconUrl;
      }
      if (object._thumbnailWidth != null) {
        objectCson["134"] = object._thumbnailWidth;
      }
      if (object._thumbnailHeight != null) {
        objectCson["135"] = object._thumbnailHeight;
      }
      if (object._content != null) {
        objectCson["136"] = base64Encode(object._content);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): File {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const mimeTypeValue = objectCson["120"];
      const unpackedMimeType = mimeTypeValue != undefined ? mimeTypeValue : undefined;
      const formatValue = objectCson["121"];
      const unpackedFormat = formatValue != undefined ? Number(formatValue) : undefined;
      const sizeValue = objectCson["122"];
      const unpackedSize = sizeValue != undefined ? Number(sizeValue) : undefined;
      const sha256Value = objectCson["123"];
      const unpackedSha256 = sha256Value != undefined ? sha256Value : undefined;
      const widthValue = objectCson["124"];
      const unpackedWidth = widthValue != undefined ? Number(widthValue) : undefined;
      const heightValue = objectCson["125"];
      const unpackedHeight = heightValue != undefined ? Number(heightValue) : undefined;
      const aspectRatioValue = objectCson["126"];
      const unpackedAspectRatio =
        aspectRatioValue != undefined ? Number(aspectRatioValue) : undefined;
      const codecValue = objectCson["127"];
      const unpackedCodec = codecValue != undefined ? codecValue : undefined;
      const durationValue = objectCson["128"];
      const unpackedDuration =
        durationValue != undefined ? timedeltaFromISOFormat(durationValue) : undefined;
      const urlValue = objectCson["130"];
      const unpackedUrl = urlValue != undefined ? urlValue : undefined;
      const contentUrlValue = objectCson["131"];
      const unpackedContentUrl = contentUrlValue != undefined ? contentUrlValue : undefined;
      const thumbnailUrlValue = objectCson["132"];
      const unpackedThumbnailUrl = thumbnailUrlValue != undefined ? thumbnailUrlValue : undefined;
      const faviconUrlValue = objectCson["133"];
      const unpackedFaviconUrl = faviconUrlValue != undefined ? faviconUrlValue : undefined;
      const thumbnailWidthValue = objectCson["134"];
      const unpackedThumbnailWidth =
        thumbnailWidthValue != undefined ? Number(thumbnailWidthValue) : undefined;
      const thumbnailHeightValue = objectCson["135"];
      const unpackedThumbnailHeight =
        thumbnailHeightValue != undefined ? Number(thumbnailHeightValue) : undefined;
      const contentValue = objectCson["136"];
      const unpackedContent = contentValue != undefined ? base64Decode(contentValue) : undefined;
      const regionValue = objectCson["111"];
      const unpackedRegion = regionValue != undefined ? Number(regionValue) : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[480000] as typeof File)({
        parent: unpackedParentPtr,
        type: Number(objectCson["100"]),
        mimeType: unpackedMimeType,
        format: unpackedFormat,
        size: unpackedSize,
        sha256: unpackedSha256,
        width: unpackedWidth,
        height: unpackedHeight,
        aspectRatio: unpackedAspectRatio,
        codec: unpackedCodec,
        duration: unpackedDuration,
        url: unpackedUrl,
        contentUrl: unpackedContentUrl,
        thumbnailUrl: unpackedThumbnailUrl,
        faviconUrl: unpackedFaviconUrl,
        thumbnailWidth: unpackedThumbnailWidth,
        thumbnailHeight: unpackedThumbnailHeight,
        content: unpackedContent,
        region: unpackedRegion,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 480000)] = new FileCsonEncoder();

  class EnvironmentCsonEncoder implements CsonObjectEncoder {
    packObject(object: Environment): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1100000;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      if (object._icon != null) {
        objectCson["102"] = object._icon.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Environment {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const iconValue = objectCson["102"];
      const unpackedIcon =
        iconValue != undefined ? (_Icon.unpack(2, iconValue, _session) as Icon) : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[1100000] as typeof Environment)({
        parent: unpackedParentPtr,
        icon: unpackedIcon,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1100000)] = new EnvironmentCsonEncoder();

  class LogEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: LogEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1110011;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      if (object.nodePtr != null) {
        objectCson["101"] = object.nodePtr.pack(2);
      }
      objectCson["110"] = object.content;
      const packedAttributes: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object.attributes)) {
        packedAttributes[String(key)] = value;
      }
      objectCson["111"] = packedAttributes;
      objectCson["112"] = object.level;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): LogEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const unpackedAttributes = {} as any;
      for (const [key, value] of Object.entries(objectCson["111"])) {
        unpackedAttributes[key] = value as any;
      }
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      const nodePtrValue = objectCson["101"];
      const unpackedNodePtr =
        nodePtrValue != undefined
          ? (_NodeReference.unpack(2, nodePtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[1110011] as typeof LogEvent)({
        content: objectCson["110"],
        attributes: unpackedAttributes,
        level: Number(objectCson["112"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        node: unpackedNodePtr,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1110011)] = new LogEventCsonEncoder();

  class RunStartedEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: RunStartedEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1110002;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      objectCson["101"] = object.nodePtr.pack(2);
      if (object.targetPtr != null) {
        objectCson["110"] = object.targetPtr.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): RunStartedEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const targetPtrValue = objectCson["110"];
      const unpackedTargetPtr =
        targetPtrValue != undefined
          ? (_NodeReference.unpack(2, targetPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[1110002] as typeof RunStartedEvent)({
        node: _NodeReference.unpack(2, objectCson["101"], _session) as NodeReference,
        target: unpackedTargetPtr,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1110002)] = new RunStartedEventCsonEncoder();

  class RunPauseRequestedEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: RunPauseRequestedEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1110003;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      objectCson["101"] = object.nodePtr.pack(2);
      if (object.targetPtr != null) {
        objectCson["110"] = object.targetPtr.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): RunPauseRequestedEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const targetPtrValue = objectCson["110"];
      const unpackedTargetPtr =
        targetPtrValue != undefined
          ? (_NodeReference.unpack(2, targetPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[1110003] as typeof RunPauseRequestedEvent)({
        node: _NodeReference.unpack(2, objectCson["101"], _session) as NodeReference,
        target: unpackedTargetPtr,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1110003)] = new RunPauseRequestedEventCsonEncoder();

  class RunPausedEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: RunPausedEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1110004;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      objectCson["101"] = object.nodePtr.pack(2);
      if (object.targetPtr != null) {
        objectCson["110"] = object.targetPtr.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): RunPausedEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const targetPtrValue = objectCson["110"];
      const unpackedTargetPtr =
        targetPtrValue != undefined
          ? (_NodeReference.unpack(2, targetPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[1110004] as typeof RunPausedEvent)({
        node: _NodeReference.unpack(2, objectCson["101"], _session) as NodeReference,
        target: unpackedTargetPtr,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1110004)] = new RunPausedEventCsonEncoder();

  class RunResumeRequestedEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: RunResumeRequestedEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1110005;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      objectCson["101"] = object.nodePtr.pack(2);
      if (object.targetPtr != null) {
        objectCson["110"] = object.targetPtr.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): RunResumeRequestedEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const targetPtrValue = objectCson["110"];
      const unpackedTargetPtr =
        targetPtrValue != undefined
          ? (_NodeReference.unpack(2, targetPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[1110005] as typeof RunResumeRequestedEvent)({
        node: _NodeReference.unpack(2, objectCson["101"], _session) as NodeReference,
        target: unpackedTargetPtr,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1110005)] = new RunResumeRequestedEventCsonEncoder();

  class RunResumedEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: RunResumedEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1110006;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      objectCson["101"] = object.nodePtr.pack(2);
      if (object.targetPtr != null) {
        objectCson["110"] = object.targetPtr.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): RunResumedEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const targetPtrValue = objectCson["110"];
      const unpackedTargetPtr =
        targetPtrValue != undefined
          ? (_NodeReference.unpack(2, targetPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[1110006] as typeof RunResumedEvent)({
        node: _NodeReference.unpack(2, objectCson["101"], _session) as NodeReference,
        target: unpackedTargetPtr,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1110006)] = new RunResumedEventCsonEncoder();

  class RunStopRequestedEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: RunStopRequestedEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1110007;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      objectCson["101"] = object.nodePtr.pack(2);
      if (object.targetPtr != null) {
        objectCson["110"] = object.targetPtr.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): RunStopRequestedEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const targetPtrValue = objectCson["110"];
      const unpackedTargetPtr =
        targetPtrValue != undefined
          ? (_NodeReference.unpack(2, targetPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[1110007] as typeof RunStopRequestedEvent)({
        node: _NodeReference.unpack(2, objectCson["101"], _session) as NodeReference,
        target: unpackedTargetPtr,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1110007)] = new RunStopRequestedEventCsonEncoder();

  class RunFailedEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: RunFailedEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1110008;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      objectCson["101"] = object.nodePtr.pack(2);
      if (object.targetPtr != null) {
        objectCson["110"] = object.targetPtr.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): RunFailedEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const targetPtrValue = objectCson["110"];
      const unpackedTargetPtr =
        targetPtrValue != undefined
          ? (_NodeReference.unpack(2, targetPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[1110008] as typeof RunFailedEvent)({
        node: _NodeReference.unpack(2, objectCson["101"], _session) as NodeReference,
        target: unpackedTargetPtr,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1110008)] = new RunFailedEventCsonEncoder();

  class RunCompletedEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: RunCompletedEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1110009;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      objectCson["101"] = object.nodePtr.pack(2);
      if (object.targetPtr != null) {
        objectCson["110"] = object.targetPtr.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): RunCompletedEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const targetPtrValue = objectCson["110"];
      const unpackedTargetPtr =
        targetPtrValue != undefined
          ? (_NodeReference.unpack(2, targetPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[1110009] as typeof RunCompletedEvent)({
        node: _NodeReference.unpack(2, objectCson["101"], _session) as NodeReference,
        target: unpackedTargetPtr,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1110009)] = new RunCompletedEventCsonEncoder();

  class SpanEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: SpanEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1110010;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      objectCson["101"] = object.nodePtr.pack(2);
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): SpanEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[1110010] as typeof SpanEvent)({
        node: _NodeReference.unpack(2, objectCson["101"], _session) as NodeReference,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1110010)] = new SpanEventCsonEncoder();

  class ArrowShape2DCsonEncoder implements CsonObjectEncoder {
    packObject(object: ArrowShape2D): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2410200;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      if (object._position != null) {
        objectCson["110"] = object._position.pack(2);
      }
      if (object._offset != null) {
        objectCson["111"] = object._offset.pack(2);
      }
      if (object._scale != null) {
        objectCson["112"] = object._scale.pack(2);
      }
      if (object._rotation != null) {
        objectCson["113"] = object._rotation.pack(2);
      }
      if (object._skew != null) {
        objectCson["114"] = object._skew.pack(2);
      }
      if (object._origin != null) {
        objectCson["115"] = object._origin.pack(2);
      }
      if (object._anchor != null) {
        objectCson["116"] = object._anchor;
      }
      if (object._stroke != null) {
        objectCson["180"] = object._stroke.pack(2);
      }
      objectCson["200"] = object._startType;
      objectCson["201"] = object._start.pack(2);
      objectCson["210"] = object._endType;
      objectCson["211"] = object._end.pack(2);
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): ArrowShape2D {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Stroke = STRUCT_CLASS_BY_TYPE[2101100] as typeof Stroke;
      const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
      const _Offset2 = STRUCT_CLASS_BY_TYPE[2400020] as typeof Offset2;
      const strokeValue = objectCson["180"];
      const unpackedStroke =
        strokeValue != undefined ? (_Stroke.unpack(2, strokeValue, _session) as Stroke) : undefined;
      const positionValue = objectCson["110"];
      const unpackedPosition =
        positionValue != undefined
          ? (_Vector2.unpack(2, positionValue, _session) as Vector2)
          : undefined;
      const offsetValue = objectCson["111"];
      const unpackedOffset =
        offsetValue != undefined
          ? (_Offset2.unpack(2, offsetValue, _session) as Offset2)
          : undefined;
      const scaleValue = objectCson["112"];
      const unpackedScale =
        scaleValue != undefined ? (_Vector2.unpack(2, scaleValue, _session) as Vector2) : undefined;
      const rotationValue = objectCson["113"];
      const unpackedRotation =
        rotationValue != undefined
          ? (_Vector2.unpack(2, rotationValue, _session) as Vector2)
          : undefined;
      const skewValue = objectCson["114"];
      const unpackedSkew =
        skewValue != undefined ? (_Vector2.unpack(2, skewValue, _session) as Vector2) : undefined;
      const originValue = objectCson["115"];
      const unpackedOrigin =
        originValue != undefined
          ? (_Vector2.unpack(2, originValue, _session) as Vector2)
          : undefined;
      const anchorValue = objectCson["116"];
      const unpackedAnchor = anchorValue != undefined ? Number(anchorValue) : undefined;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[2410200] as typeof ArrowShape2D)({
        startType: Number(objectCson["200"]),
        start: _Vector2.unpack(2, objectCson["201"], _session) as Vector2,
        endType: Number(objectCson["210"]),
        end: _Vector2.unpack(2, objectCson["211"], _session) as Vector2,
        stroke: unpackedStroke,
        position: unpackedPosition,
        offset: unpackedOffset,
        scale: unpackedScale,
        rotation: unpackedRotation,
        skew: unpackedSkew,
        origin: unpackedOrigin,
        anchor: unpackedAnchor,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 2410200)] = new ArrowShape2DCsonEncoder();

  class EllipseShape2DCsonEncoder implements CsonObjectEncoder {
    packObject(object: EllipseShape2D): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2410400;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      if (object._position != null) {
        objectCson["110"] = object._position.pack(2);
      }
      if (object._offset != null) {
        objectCson["111"] = object._offset.pack(2);
      }
      if (object._scale != null) {
        objectCson["112"] = object._scale.pack(2);
      }
      if (object._rotation != null) {
        objectCson["113"] = object._rotation.pack(2);
      }
      if (object._skew != null) {
        objectCson["114"] = object._skew.pack(2);
      }
      if (object._origin != null) {
        objectCson["115"] = object._origin.pack(2);
      }
      if (object._anchor != null) {
        objectCson["116"] = object._anchor;
      }
      if (object._stroke != null) {
        objectCson["180"] = object._stroke.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): EllipseShape2D {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Stroke = STRUCT_CLASS_BY_TYPE[2101100] as typeof Stroke;
      const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
      const _Offset2 = STRUCT_CLASS_BY_TYPE[2400020] as typeof Offset2;
      const strokeValue = objectCson["180"];
      const unpackedStroke =
        strokeValue != undefined ? (_Stroke.unpack(2, strokeValue, _session) as Stroke) : undefined;
      const positionValue = objectCson["110"];
      const unpackedPosition =
        positionValue != undefined
          ? (_Vector2.unpack(2, positionValue, _session) as Vector2)
          : undefined;
      const offsetValue = objectCson["111"];
      const unpackedOffset =
        offsetValue != undefined
          ? (_Offset2.unpack(2, offsetValue, _session) as Offset2)
          : undefined;
      const scaleValue = objectCson["112"];
      const unpackedScale =
        scaleValue != undefined ? (_Vector2.unpack(2, scaleValue, _session) as Vector2) : undefined;
      const rotationValue = objectCson["113"];
      const unpackedRotation =
        rotationValue != undefined
          ? (_Vector2.unpack(2, rotationValue, _session) as Vector2)
          : undefined;
      const skewValue = objectCson["114"];
      const unpackedSkew =
        skewValue != undefined ? (_Vector2.unpack(2, skewValue, _session) as Vector2) : undefined;
      const originValue = objectCson["115"];
      const unpackedOrigin =
        originValue != undefined
          ? (_Vector2.unpack(2, originValue, _session) as Vector2)
          : undefined;
      const anchorValue = objectCson["116"];
      const unpackedAnchor = anchorValue != undefined ? Number(anchorValue) : undefined;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[2410400] as typeof EllipseShape2D)({
        stroke: unpackedStroke,
        position: unpackedPosition,
        offset: unpackedOffset,
        scale: unpackedScale,
        rotation: unpackedRotation,
        skew: unpackedSkew,
        origin: unpackedOrigin,
        anchor: unpackedAnchor,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 2410400)] = new EllipseShape2DCsonEncoder();

  class LineShape2DCsonEncoder implements CsonObjectEncoder {
    packObject(object: LineShape2D): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2410100;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      if (object._position != null) {
        objectCson["110"] = object._position.pack(2);
      }
      if (object._offset != null) {
        objectCson["111"] = object._offset.pack(2);
      }
      if (object._scale != null) {
        objectCson["112"] = object._scale.pack(2);
      }
      if (object._rotation != null) {
        objectCson["113"] = object._rotation.pack(2);
      }
      if (object._skew != null) {
        objectCson["114"] = object._skew.pack(2);
      }
      if (object._origin != null) {
        objectCson["115"] = object._origin.pack(2);
      }
      if (object._anchor != null) {
        objectCson["116"] = object._anchor;
      }
      if (object._stroke != null) {
        objectCson["180"] = object._stroke.pack(2);
      }
      objectCson["200"] = object._start.pack(2);
      objectCson["210"] = object._end.pack(2);
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): LineShape2D {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Stroke = STRUCT_CLASS_BY_TYPE[2101100] as typeof Stroke;
      const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
      const _Offset2 = STRUCT_CLASS_BY_TYPE[2400020] as typeof Offset2;
      const strokeValue = objectCson["180"];
      const unpackedStroke =
        strokeValue != undefined ? (_Stroke.unpack(2, strokeValue, _session) as Stroke) : undefined;
      const positionValue = objectCson["110"];
      const unpackedPosition =
        positionValue != undefined
          ? (_Vector2.unpack(2, positionValue, _session) as Vector2)
          : undefined;
      const offsetValue = objectCson["111"];
      const unpackedOffset =
        offsetValue != undefined
          ? (_Offset2.unpack(2, offsetValue, _session) as Offset2)
          : undefined;
      const scaleValue = objectCson["112"];
      const unpackedScale =
        scaleValue != undefined ? (_Vector2.unpack(2, scaleValue, _session) as Vector2) : undefined;
      const rotationValue = objectCson["113"];
      const unpackedRotation =
        rotationValue != undefined
          ? (_Vector2.unpack(2, rotationValue, _session) as Vector2)
          : undefined;
      const skewValue = objectCson["114"];
      const unpackedSkew =
        skewValue != undefined ? (_Vector2.unpack(2, skewValue, _session) as Vector2) : undefined;
      const originValue = objectCson["115"];
      const unpackedOrigin =
        originValue != undefined
          ? (_Vector2.unpack(2, originValue, _session) as Vector2)
          : undefined;
      const anchorValue = objectCson["116"];
      const unpackedAnchor = anchorValue != undefined ? Number(anchorValue) : undefined;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[2410100] as typeof LineShape2D)({
        start: _Vector2.unpack(2, objectCson["200"], _session) as Vector2,
        end: _Vector2.unpack(2, objectCson["210"], _session) as Vector2,
        stroke: unpackedStroke,
        position: unpackedPosition,
        offset: unpackedOffset,
        scale: unpackedScale,
        rotation: unpackedRotation,
        skew: unpackedSkew,
        origin: unpackedOrigin,
        anchor: unpackedAnchor,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 2410100)] = new LineShape2DCsonEncoder();

  class PathShape2DCsonEncoder implements CsonObjectEncoder {
    packObject(object: PathShape2D): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2410600;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      if (object._position != null) {
        objectCson["110"] = object._position.pack(2);
      }
      if (object._offset != null) {
        objectCson["111"] = object._offset.pack(2);
      }
      if (object._scale != null) {
        objectCson["112"] = object._scale.pack(2);
      }
      if (object._rotation != null) {
        objectCson["113"] = object._rotation.pack(2);
      }
      if (object._skew != null) {
        objectCson["114"] = object._skew.pack(2);
      }
      if (object._origin != null) {
        objectCson["115"] = object._origin.pack(2);
      }
      if (object._anchor != null) {
        objectCson["116"] = object._anchor;
      }
      if (object._stroke != null) {
        objectCson["180"] = object._stroke.pack(2);
      }
      const packedPoints: any[] = [];
      for (const item of object._points) {
        packedPoints.push(item.pack(2));
      }
      objectCson["200"] = packedPoints;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): PathShape2D {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Stroke = STRUCT_CLASS_BY_TYPE[2101100] as typeof Stroke;
      const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
      const _Offset2 = STRUCT_CLASS_BY_TYPE[2400020] as typeof Offset2;
      const unpackedPoints: any[] = [];
      for (const item of objectCson["200"]) {
        unpackedPoints.push(_Vector2.unpack(2, item, _session) as Vector2);
      }
      const strokeValue = objectCson["180"];
      const unpackedStroke =
        strokeValue != undefined ? (_Stroke.unpack(2, strokeValue, _session) as Stroke) : undefined;
      const positionValue = objectCson["110"];
      const unpackedPosition =
        positionValue != undefined
          ? (_Vector2.unpack(2, positionValue, _session) as Vector2)
          : undefined;
      const offsetValue = objectCson["111"];
      const unpackedOffset =
        offsetValue != undefined
          ? (_Offset2.unpack(2, offsetValue, _session) as Offset2)
          : undefined;
      const scaleValue = objectCson["112"];
      const unpackedScale =
        scaleValue != undefined ? (_Vector2.unpack(2, scaleValue, _session) as Vector2) : undefined;
      const rotationValue = objectCson["113"];
      const unpackedRotation =
        rotationValue != undefined
          ? (_Vector2.unpack(2, rotationValue, _session) as Vector2)
          : undefined;
      const skewValue = objectCson["114"];
      const unpackedSkew =
        skewValue != undefined ? (_Vector2.unpack(2, skewValue, _session) as Vector2) : undefined;
      const originValue = objectCson["115"];
      const unpackedOrigin =
        originValue != undefined
          ? (_Vector2.unpack(2, originValue, _session) as Vector2)
          : undefined;
      const anchorValue = objectCson["116"];
      const unpackedAnchor = anchorValue != undefined ? Number(anchorValue) : undefined;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[2410600] as typeof PathShape2D)({
        points: unpackedPoints,
        stroke: unpackedStroke,
        position: unpackedPosition,
        offset: unpackedOffset,
        scale: unpackedScale,
        rotation: unpackedRotation,
        skew: unpackedSkew,
        origin: unpackedOrigin,
        anchor: unpackedAnchor,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 2410600)] = new PathShape2DCsonEncoder();

  class PolygonShape2DCsonEncoder implements CsonObjectEncoder {
    packObject(object: PolygonShape2D): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2410500;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      if (object._position != null) {
        objectCson["110"] = object._position.pack(2);
      }
      if (object._offset != null) {
        objectCson["111"] = object._offset.pack(2);
      }
      if (object._scale != null) {
        objectCson["112"] = object._scale.pack(2);
      }
      if (object._rotation != null) {
        objectCson["113"] = object._rotation.pack(2);
      }
      if (object._skew != null) {
        objectCson["114"] = object._skew.pack(2);
      }
      if (object._origin != null) {
        objectCson["115"] = object._origin.pack(2);
      }
      if (object._anchor != null) {
        objectCson["116"] = object._anchor;
      }
      if (object._stroke != null) {
        objectCson["180"] = object._stroke.pack(2);
      }
      const packedPoints: any[] = [];
      for (const item of object._points) {
        packedPoints.push(item.pack(2));
      }
      objectCson["210"] = packedPoints;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): PolygonShape2D {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Stroke = STRUCT_CLASS_BY_TYPE[2101100] as typeof Stroke;
      const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
      const _Offset2 = STRUCT_CLASS_BY_TYPE[2400020] as typeof Offset2;
      const unpackedPoints: any[] = [];
      for (const item of objectCson["210"]) {
        unpackedPoints.push(_Vector2.unpack(2, item, _session) as Vector2);
      }
      const strokeValue = objectCson["180"];
      const unpackedStroke =
        strokeValue != undefined ? (_Stroke.unpack(2, strokeValue, _session) as Stroke) : undefined;
      const positionValue = objectCson["110"];
      const unpackedPosition =
        positionValue != undefined
          ? (_Vector2.unpack(2, positionValue, _session) as Vector2)
          : undefined;
      const offsetValue = objectCson["111"];
      const unpackedOffset =
        offsetValue != undefined
          ? (_Offset2.unpack(2, offsetValue, _session) as Offset2)
          : undefined;
      const scaleValue = objectCson["112"];
      const unpackedScale =
        scaleValue != undefined ? (_Vector2.unpack(2, scaleValue, _session) as Vector2) : undefined;
      const rotationValue = objectCson["113"];
      const unpackedRotation =
        rotationValue != undefined
          ? (_Vector2.unpack(2, rotationValue, _session) as Vector2)
          : undefined;
      const skewValue = objectCson["114"];
      const unpackedSkew =
        skewValue != undefined ? (_Vector2.unpack(2, skewValue, _session) as Vector2) : undefined;
      const originValue = objectCson["115"];
      const unpackedOrigin =
        originValue != undefined
          ? (_Vector2.unpack(2, originValue, _session) as Vector2)
          : undefined;
      const anchorValue = objectCson["116"];
      const unpackedAnchor = anchorValue != undefined ? Number(anchorValue) : undefined;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[2410500] as typeof PolygonShape2D)({
        points: unpackedPoints,
        stroke: unpackedStroke,
        position: unpackedPosition,
        offset: unpackedOffset,
        scale: unpackedScale,
        rotation: unpackedRotation,
        skew: unpackedSkew,
        origin: unpackedOrigin,
        anchor: unpackedAnchor,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 2410500)] = new PolygonShape2DCsonEncoder();

  class RectangleShape2DCsonEncoder implements CsonObjectEncoder {
    packObject(object: RectangleShape2D): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2410300;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      if (object._position != null) {
        objectCson["110"] = object._position.pack(2);
      }
      if (object._offset != null) {
        objectCson["111"] = object._offset.pack(2);
      }
      if (object._scale != null) {
        objectCson["112"] = object._scale.pack(2);
      }
      if (object._rotation != null) {
        objectCson["113"] = object._rotation.pack(2);
      }
      if (object._skew != null) {
        objectCson["114"] = object._skew.pack(2);
      }
      if (object._origin != null) {
        objectCson["115"] = object._origin.pack(2);
      }
      if (object._anchor != null) {
        objectCson["116"] = object._anchor;
      }
      if (object._stroke != null) {
        objectCson["180"] = object._stroke.pack(2);
      }
      if (object._width != null) {
        objectCson["210"] = object._width.pack(2);
      }
      if (object._height != null) {
        objectCson["220"] = object._height.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): RectangleShape2D {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Stroke = STRUCT_CLASS_BY_TYPE[2101100] as typeof Stroke;
      const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
      const _Offset2 = STRUCT_CLASS_BY_TYPE[2400020] as typeof Offset2;
      const widthValue = objectCson["210"];
      const unpackedWidth =
        widthValue != undefined ? (_Vector2.unpack(2, widthValue, _session) as Vector2) : undefined;
      const heightValue = objectCson["220"];
      const unpackedHeight =
        heightValue != undefined
          ? (_Vector2.unpack(2, heightValue, _session) as Vector2)
          : undefined;
      const strokeValue = objectCson["180"];
      const unpackedStroke =
        strokeValue != undefined ? (_Stroke.unpack(2, strokeValue, _session) as Stroke) : undefined;
      const positionValue = objectCson["110"];
      const unpackedPosition =
        positionValue != undefined
          ? (_Vector2.unpack(2, positionValue, _session) as Vector2)
          : undefined;
      const offsetValue = objectCson["111"];
      const unpackedOffset =
        offsetValue != undefined
          ? (_Offset2.unpack(2, offsetValue, _session) as Offset2)
          : undefined;
      const scaleValue = objectCson["112"];
      const unpackedScale =
        scaleValue != undefined ? (_Vector2.unpack(2, scaleValue, _session) as Vector2) : undefined;
      const rotationValue = objectCson["113"];
      const unpackedRotation =
        rotationValue != undefined
          ? (_Vector2.unpack(2, rotationValue, _session) as Vector2)
          : undefined;
      const skewValue = objectCson["114"];
      const unpackedSkew =
        skewValue != undefined ? (_Vector2.unpack(2, skewValue, _session) as Vector2) : undefined;
      const originValue = objectCson["115"];
      const unpackedOrigin =
        originValue != undefined
          ? (_Vector2.unpack(2, originValue, _session) as Vector2)
          : undefined;
      const anchorValue = objectCson["116"];
      const unpackedAnchor = anchorValue != undefined ? Number(anchorValue) : undefined;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[2410300] as typeof RectangleShape2D)({
        width: unpackedWidth,
        height: unpackedHeight,
        stroke: unpackedStroke,
        position: unpackedPosition,
        offset: unpackedOffset,
        scale: unpackedScale,
        rotation: unpackedRotation,
        skew: unpackedSkew,
        origin: unpackedOrigin,
        anchor: unpackedAnchor,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 2410300)] = new RectangleShape2DCsonEncoder();

  class DatabaseCsonEncoder implements CsonObjectEncoder {
    packObject(object: Database): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1000000;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      objectCson["100"] = object._type;
      if (object._icon != null) {
        objectCson["102"] = object._icon.pack(2);
      }
      if (object._region != null) {
        objectCson["111"] = object._region;
      }
      if (object._galaxyName != null) {
        objectCson["200"] = object._galaxyName;
      }
      objectCson["201"] = object._externalName;
      if (object._customSchemaName != null) {
        objectCson["202"] = object._customSchemaName;
      }
      objectCson["203"] = object._tenancy;
      if (object._connectionUrl != null) {
        objectCson["204"] = object._connectionUrl;
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Database {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
      const iconValue = objectCson["102"];
      const unpackedIcon =
        iconValue != undefined ? (_Icon.unpack(2, iconValue, _session) as Icon) : undefined;
      const galaxyNameValue = objectCson["200"];
      const unpackedGalaxyName = galaxyNameValue != undefined ? galaxyNameValue : undefined;
      const customSchemaNameValue = objectCson["202"];
      const unpackedCustomSchemaName =
        customSchemaNameValue != undefined ? customSchemaNameValue : undefined;
      const connectionUrlValue = objectCson["204"];
      const unpackedConnectionUrl =
        connectionUrlValue != undefined ? connectionUrlValue : undefined;
      const regionValue = objectCson["111"];
      const unpackedRegion = regionValue != undefined ? Number(regionValue) : undefined;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[1000000] as typeof Database)({
        type: Number(objectCson["100"]),
        icon: unpackedIcon,
        galaxyName: unpackedGalaxyName,
        externalName: objectCson["201"],
        customSchemaName: unpackedCustomSchemaName,
        tenancy: Number(objectCson["203"]),
        connectionUrl: unpackedConnectionUrl,
        region: unpackedRegion,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1000000)] = new DatabaseCsonEncoder();

  class MachineCsonEncoder implements CsonObjectEncoder {
    packObject(object: Machine): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1001000;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      objectCson["100"] = object._type;
      if (object._region != null) {
        objectCson["111"] = object._region;
      }
      objectCson["120"] = object._version;
      if (object._externalName != null) {
        objectCson["121"] = object._externalName;
      }
      if (object._externalId != null) {
        objectCson["122"] = object._externalId;
      }
      if (object._imageId != null) {
        objectCson["123"] = object._imageId;
      }
      objectCson["130"] = object._cpu;
      objectCson["131"] = object._ram;
      objectCson["132"] = object._width;
      objectCson["133"] = object._height;
      objectCson["134"] = object._isHeadless;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Machine {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const externalNameValue = objectCson["121"];
      const unpackedExternalName = externalNameValue != undefined ? externalNameValue : undefined;
      const externalIdValue = objectCson["122"];
      const unpackedExternalId = externalIdValue != undefined ? externalIdValue : undefined;
      const imageIdValue = objectCson["123"];
      const unpackedImageId = imageIdValue != undefined ? imageIdValue : undefined;
      const regionValue = objectCson["111"];
      const unpackedRegion = regionValue != undefined ? Number(regionValue) : undefined;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[1001000] as typeof Machine)({
        type: Number(objectCson["100"]),
        version: objectCson["120"],
        externalName: unpackedExternalName,
        externalId: unpackedExternalId,
        imageId: unpackedImageId,
        cpu: Number(objectCson["130"]),
        ram: Number(objectCson["131"]),
        width: Number(objectCson["132"]),
        height: Number(objectCson["133"]),
        isHeadless: Boolean(objectCson["134"]),
        region: unpackedRegion,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1001000)] = new MachineCsonEncoder();

  class CopyEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: CopyEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2000501;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      if (object.nodePtr != null) {
        objectCson["101"] = object.nodePtr.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): CopyEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const nodePtrValue = objectCson["101"];
      const unpackedNodePtr =
        nodePtrValue != undefined
          ? (_NodeReference.unpack(2, nodePtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[2000501] as typeof CopyEvent)({
        node: unpackedNodePtr,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 2000501)] = new CopyEventCsonEncoder();

  class CutEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: CutEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2000502;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      if (object.nodePtr != null) {
        objectCson["101"] = object.nodePtr.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): CutEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const nodePtrValue = objectCson["101"];
      const unpackedNodePtr =
        nodePtrValue != undefined
          ? (_NodeReference.unpack(2, nodePtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[2000502] as typeof CutEvent)({
        node: unpackedNodePtr,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 2000502)] = new CutEventCsonEncoder();

  class PasteEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: PasteEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2000503;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      if (object.nodePtr != null) {
        objectCson["101"] = object.nodePtr.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): PasteEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const nodePtrValue = objectCson["101"];
      const unpackedNodePtr =
        nodePtrValue != undefined
          ? (_NodeReference.unpack(2, nodePtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[2000503] as typeof PasteEvent)({
        node: unpackedNodePtr,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 2000503)] = new PasteEventCsonEncoder();

  class DragStartEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: DragStartEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2000401;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      if (object.nodePtr != null) {
        objectCson["101"] = object.nodePtr.pack(2);
      }
      objectCson["110"] = object.position.pack(2);
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): DragStartEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
      const nodePtrValue = objectCson["101"];
      const unpackedNodePtr =
        nodePtrValue != undefined
          ? (_NodeReference.unpack(2, nodePtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[2000401] as typeof DragStartEvent)({
        position: _Vector2.unpack(2, objectCson["110"], _session) as Vector2,
        node: unpackedNodePtr,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 2000401)] = new DragStartEventCsonEncoder();

  class DragEndEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: DragEndEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2000402;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      if (object.nodePtr != null) {
        objectCson["101"] = object.nodePtr.pack(2);
      }
      objectCson["110"] = object.position.pack(2);
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): DragEndEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
      const nodePtrValue = objectCson["101"];
      const unpackedNodePtr =
        nodePtrValue != undefined
          ? (_NodeReference.unpack(2, nodePtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[2000402] as typeof DragEndEvent)({
        position: _Vector2.unpack(2, objectCson["110"], _session) as Vector2,
        node: unpackedNodePtr,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 2000402)] = new DragEndEventCsonEncoder();

  class DragOverEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: DragOverEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2000403;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      if (object.nodePtr != null) {
        objectCson["101"] = object.nodePtr.pack(2);
      }
      objectCson["110"] = object.position.pack(2);
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): DragOverEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
      const nodePtrValue = objectCson["101"];
      const unpackedNodePtr =
        nodePtrValue != undefined
          ? (_NodeReference.unpack(2, nodePtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[2000403] as typeof DragOverEvent)({
        position: _Vector2.unpack(2, objectCson["110"], _session) as Vector2,
        node: unpackedNodePtr,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 2000403)] = new DragOverEventCsonEncoder();

  class DragEnterEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: DragEnterEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2000404;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      if (object.nodePtr != null) {
        objectCson["101"] = object.nodePtr.pack(2);
      }
      objectCson["110"] = object.position.pack(2);
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): DragEnterEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
      const nodePtrValue = objectCson["101"];
      const unpackedNodePtr =
        nodePtrValue != undefined
          ? (_NodeReference.unpack(2, nodePtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[2000404] as typeof DragEnterEvent)({
        position: _Vector2.unpack(2, objectCson["110"], _session) as Vector2,
        node: unpackedNodePtr,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 2000404)] = new DragEnterEventCsonEncoder();

  class DragLeaveEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: DragLeaveEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2000405;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      if (object.nodePtr != null) {
        objectCson["101"] = object.nodePtr.pack(2);
      }
      objectCson["110"] = object.position.pack(2);
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): DragLeaveEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
      const nodePtrValue = objectCson["101"];
      const unpackedNodePtr =
        nodePtrValue != undefined
          ? (_NodeReference.unpack(2, nodePtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[2000405] as typeof DragLeaveEvent)({
        position: _Vector2.unpack(2, objectCson["110"], _session) as Vector2,
        node: unpackedNodePtr,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 2000405)] = new DragLeaveEventCsonEncoder();

  class DropEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: DropEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2000406;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      if (object.nodePtr != null) {
        objectCson["101"] = object.nodePtr.pack(2);
      }
      objectCson["110"] = object.position.pack(2);
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): DropEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
      const nodePtrValue = objectCson["101"];
      const unpackedNodePtr =
        nodePtrValue != undefined
          ? (_NodeReference.unpack(2, nodePtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[2000406] as typeof DropEvent)({
        position: _Vector2.unpack(2, objectCson["110"], _session) as Vector2,
        node: unpackedNodePtr,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 2000406)] = new DropEventCsonEncoder();

  class FocusInEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: FocusInEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2000601;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      if (object.nodePtr != null) {
        objectCson["101"] = object.nodePtr.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): FocusInEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const nodePtrValue = objectCson["101"];
      const unpackedNodePtr =
        nodePtrValue != undefined
          ? (_NodeReference.unpack(2, nodePtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[2000601] as typeof FocusInEvent)({
        node: unpackedNodePtr,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 2000601)] = new FocusInEventCsonEncoder();

  class FocusOutEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: FocusOutEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2000602;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      if (object.nodePtr != null) {
        objectCson["101"] = object.nodePtr.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): FocusOutEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const nodePtrValue = objectCson["101"];
      const unpackedNodePtr =
        nodePtrValue != undefined
          ? (_NodeReference.unpack(2, nodePtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[2000602] as typeof FocusOutEvent)({
        node: unpackedNodePtr,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 2000602)] = new FocusOutEventCsonEncoder();

  class KeyDownEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: KeyDownEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2000301;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      if (object.nodePtr != null) {
        objectCson["101"] = object.nodePtr.pack(2);
      }
      objectCson["110"] = object.key;
      objectCson["111"] = object.code;
      objectCson["112"] = object.isRepeat;
      objectCson["113"] = object.isRedacted;
      objectCson["120"] = object.shiftKey;
      objectCson["121"] = object.altKey;
      objectCson["122"] = object.ctrlKey;
      objectCson["123"] = object.metaKey;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): KeyDownEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const nodePtrValue = objectCson["101"];
      const unpackedNodePtr =
        nodePtrValue != undefined
          ? (_NodeReference.unpack(2, nodePtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[2000301] as typeof KeyDownEvent)({
        key: objectCson["110"],
        code: objectCson["111"],
        isRepeat: Boolean(objectCson["112"]),
        isRedacted: Boolean(objectCson["113"]),
        shiftKey: Boolean(objectCson["120"]),
        altKey: Boolean(objectCson["121"]),
        ctrlKey: Boolean(objectCson["122"]),
        metaKey: Boolean(objectCson["123"]),
        node: unpackedNodePtr,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 2000301)] = new KeyDownEventCsonEncoder();

  class KeyUpEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: KeyUpEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2000302;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      if (object.nodePtr != null) {
        objectCson["101"] = object.nodePtr.pack(2);
      }
      objectCson["110"] = object.key;
      objectCson["111"] = object.code;
      objectCson["112"] = object.isRepeat;
      objectCson["113"] = object.isRedacted;
      objectCson["120"] = object.shiftKey;
      objectCson["121"] = object.altKey;
      objectCson["122"] = object.ctrlKey;
      objectCson["123"] = object.metaKey;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): KeyUpEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const nodePtrValue = objectCson["101"];
      const unpackedNodePtr =
        nodePtrValue != undefined
          ? (_NodeReference.unpack(2, nodePtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[2000302] as typeof KeyUpEvent)({
        key: objectCson["110"],
        code: objectCson["111"],
        isRepeat: Boolean(objectCson["112"]),
        isRedacted: Boolean(objectCson["113"]),
        shiftKey: Boolean(objectCson["120"]),
        altKey: Boolean(objectCson["121"]),
        ctrlKey: Boolean(objectCson["122"]),
        metaKey: Boolean(objectCson["123"]),
        node: unpackedNodePtr,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 2000302)] = new KeyUpEventCsonEncoder();

  class KeyPressEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: KeyPressEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2000303;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      if (object.nodePtr != null) {
        objectCson["101"] = object.nodePtr.pack(2);
      }
      objectCson["110"] = object.key;
      objectCson["111"] = object.code;
      objectCson["112"] = object.isRepeat;
      objectCson["113"] = object.isRedacted;
      objectCson["120"] = object.shiftKey;
      objectCson["121"] = object.altKey;
      objectCson["122"] = object.ctrlKey;
      objectCson["123"] = object.metaKey;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): KeyPressEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const nodePtrValue = objectCson["101"];
      const unpackedNodePtr =
        nodePtrValue != undefined
          ? (_NodeReference.unpack(2, nodePtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[2000303] as typeof KeyPressEvent)({
        key: objectCson["110"],
        code: objectCson["111"],
        isRepeat: Boolean(objectCson["112"]),
        isRedacted: Boolean(objectCson["113"]),
        shiftKey: Boolean(objectCson["120"]),
        altKey: Boolean(objectCson["121"]),
        ctrlKey: Boolean(objectCson["122"]),
        metaKey: Boolean(objectCson["123"]),
        node: unpackedNodePtr,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 2000303)] = new KeyPressEventCsonEncoder();

  class PointerDownEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: PointerDownEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2000101;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      if (object.nodePtr != null) {
        objectCson["101"] = object.nodePtr.pack(2);
      }
      objectCson["110"] = object.position.pack(2);
      if (object.pressure != null) {
        objectCson["111"] = object.pressure;
      }
      objectCson["120"] = object.shiftKey;
      objectCson["121"] = object.altKey;
      objectCson["122"] = object.ctrlKey;
      objectCson["123"] = object.metaKey;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): PointerDownEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
      const pressureValue = objectCson["111"];
      const unpackedPressure = pressureValue != undefined ? Number(pressureValue) : undefined;
      const nodePtrValue = objectCson["101"];
      const unpackedNodePtr =
        nodePtrValue != undefined
          ? (_NodeReference.unpack(2, nodePtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[2000101] as typeof PointerDownEvent)({
        position: _Vector2.unpack(2, objectCson["110"], _session) as Vector2,
        pressure: unpackedPressure,
        shiftKey: Boolean(objectCson["120"]),
        altKey: Boolean(objectCson["121"]),
        ctrlKey: Boolean(objectCson["122"]),
        metaKey: Boolean(objectCson["123"]),
        node: unpackedNodePtr,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 2000101)] = new PointerDownEventCsonEncoder();

  class PointerUpEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: PointerUpEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2000102;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      if (object.nodePtr != null) {
        objectCson["101"] = object.nodePtr.pack(2);
      }
      objectCson["110"] = object.position.pack(2);
      if (object.pressure != null) {
        objectCson["111"] = object.pressure;
      }
      objectCson["120"] = object.shiftKey;
      objectCson["121"] = object.altKey;
      objectCson["122"] = object.ctrlKey;
      objectCson["123"] = object.metaKey;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): PointerUpEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
      const pressureValue = objectCson["111"];
      const unpackedPressure = pressureValue != undefined ? Number(pressureValue) : undefined;
      const nodePtrValue = objectCson["101"];
      const unpackedNodePtr =
        nodePtrValue != undefined
          ? (_NodeReference.unpack(2, nodePtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[2000102] as typeof PointerUpEvent)({
        position: _Vector2.unpack(2, objectCson["110"], _session) as Vector2,
        pressure: unpackedPressure,
        shiftKey: Boolean(objectCson["120"]),
        altKey: Boolean(objectCson["121"]),
        ctrlKey: Boolean(objectCson["122"]),
        metaKey: Boolean(objectCson["123"]),
        node: unpackedNodePtr,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 2000102)] = new PointerUpEventCsonEncoder();

  class PointerMoveEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: PointerMoveEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2000103;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      if (object.nodePtr != null) {
        objectCson["101"] = object.nodePtr.pack(2);
      }
      objectCson["110"] = object.position.pack(2);
      if (object.pressure != null) {
        objectCson["111"] = object.pressure;
      }
      objectCson["120"] = object.shiftKey;
      objectCson["121"] = object.altKey;
      objectCson["122"] = object.ctrlKey;
      objectCson["123"] = object.metaKey;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): PointerMoveEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
      const pressureValue = objectCson["111"];
      const unpackedPressure = pressureValue != undefined ? Number(pressureValue) : undefined;
      const nodePtrValue = objectCson["101"];
      const unpackedNodePtr =
        nodePtrValue != undefined
          ? (_NodeReference.unpack(2, nodePtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[2000103] as typeof PointerMoveEvent)({
        position: _Vector2.unpack(2, objectCson["110"], _session) as Vector2,
        pressure: unpackedPressure,
        shiftKey: Boolean(objectCson["120"]),
        altKey: Boolean(objectCson["121"]),
        ctrlKey: Boolean(objectCson["122"]),
        metaKey: Boolean(objectCson["123"]),
        node: unpackedNodePtr,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 2000103)] = new PointerMoveEventCsonEncoder();

  class PointerEnterEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: PointerEnterEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2000104;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      if (object.nodePtr != null) {
        objectCson["101"] = object.nodePtr.pack(2);
      }
      objectCson["110"] = object.position.pack(2);
      if (object.pressure != null) {
        objectCson["111"] = object.pressure;
      }
      objectCson["120"] = object.shiftKey;
      objectCson["121"] = object.altKey;
      objectCson["122"] = object.ctrlKey;
      objectCson["123"] = object.metaKey;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): PointerEnterEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
      const pressureValue = objectCson["111"];
      const unpackedPressure = pressureValue != undefined ? Number(pressureValue) : undefined;
      const nodePtrValue = objectCson["101"];
      const unpackedNodePtr =
        nodePtrValue != undefined
          ? (_NodeReference.unpack(2, nodePtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[2000104] as typeof PointerEnterEvent)({
        position: _Vector2.unpack(2, objectCson["110"], _session) as Vector2,
        pressure: unpackedPressure,
        shiftKey: Boolean(objectCson["120"]),
        altKey: Boolean(objectCson["121"]),
        ctrlKey: Boolean(objectCson["122"]),
        metaKey: Boolean(objectCson["123"]),
        node: unpackedNodePtr,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 2000104)] = new PointerEnterEventCsonEncoder();

  class PointerOverEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: PointerOverEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2000105;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      if (object.nodePtr != null) {
        objectCson["101"] = object.nodePtr.pack(2);
      }
      objectCson["110"] = object.position.pack(2);
      if (object.pressure != null) {
        objectCson["111"] = object.pressure;
      }
      objectCson["120"] = object.shiftKey;
      objectCson["121"] = object.altKey;
      objectCson["122"] = object.ctrlKey;
      objectCson["123"] = object.metaKey;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): PointerOverEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
      const pressureValue = objectCson["111"];
      const unpackedPressure = pressureValue != undefined ? Number(pressureValue) : undefined;
      const nodePtrValue = objectCson["101"];
      const unpackedNodePtr =
        nodePtrValue != undefined
          ? (_NodeReference.unpack(2, nodePtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[2000105] as typeof PointerOverEvent)({
        position: _Vector2.unpack(2, objectCson["110"], _session) as Vector2,
        pressure: unpackedPressure,
        shiftKey: Boolean(objectCson["120"]),
        altKey: Boolean(objectCson["121"]),
        ctrlKey: Boolean(objectCson["122"]),
        metaKey: Boolean(objectCson["123"]),
        node: unpackedNodePtr,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 2000105)] = new PointerOverEventCsonEncoder();

  class PointerLeaveEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: PointerLeaveEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2000106;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      if (object.nodePtr != null) {
        objectCson["101"] = object.nodePtr.pack(2);
      }
      objectCson["110"] = object.position.pack(2);
      if (object.pressure != null) {
        objectCson["111"] = object.pressure;
      }
      objectCson["120"] = object.shiftKey;
      objectCson["121"] = object.altKey;
      objectCson["122"] = object.ctrlKey;
      objectCson["123"] = object.metaKey;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): PointerLeaveEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
      const pressureValue = objectCson["111"];
      const unpackedPressure = pressureValue != undefined ? Number(pressureValue) : undefined;
      const nodePtrValue = objectCson["101"];
      const unpackedNodePtr =
        nodePtrValue != undefined
          ? (_NodeReference.unpack(2, nodePtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[2000106] as typeof PointerLeaveEvent)({
        position: _Vector2.unpack(2, objectCson["110"], _session) as Vector2,
        pressure: unpackedPressure,
        shiftKey: Boolean(objectCson["120"]),
        altKey: Boolean(objectCson["121"]),
        ctrlKey: Boolean(objectCson["122"]),
        metaKey: Boolean(objectCson["123"]),
        node: unpackedNodePtr,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 2000106)] = new PointerLeaveEventCsonEncoder();

  class PointerLongPressEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: PointerLongPressEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2000107;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      if (object.nodePtr != null) {
        objectCson["101"] = object.nodePtr.pack(2);
      }
      objectCson["110"] = object.position.pack(2);
      if (object.pressure != null) {
        objectCson["111"] = object.pressure;
      }
      objectCson["120"] = object.shiftKey;
      objectCson["121"] = object.altKey;
      objectCson["122"] = object.ctrlKey;
      objectCson["123"] = object.metaKey;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): PointerLongPressEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
      const pressureValue = objectCson["111"];
      const unpackedPressure = pressureValue != undefined ? Number(pressureValue) : undefined;
      const nodePtrValue = objectCson["101"];
      const unpackedNodePtr =
        nodePtrValue != undefined
          ? (_NodeReference.unpack(2, nodePtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[2000107] as typeof PointerLongPressEvent)({
        position: _Vector2.unpack(2, objectCson["110"], _session) as Vector2,
        pressure: unpackedPressure,
        shiftKey: Boolean(objectCson["120"]),
        altKey: Boolean(objectCson["121"]),
        ctrlKey: Boolean(objectCson["122"]),
        metaKey: Boolean(objectCson["123"]),
        node: unpackedNodePtr,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 2000107)] = new PointerLongPressEventCsonEncoder();

  class SingleClickEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: SingleClickEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2000202;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      if (object.nodePtr != null) {
        objectCson["101"] = object.nodePtr.pack(2);
      }
      objectCson["110"] = object.position.pack(2);
      if (object.pressure != null) {
        objectCson["111"] = object.pressure;
      }
      objectCson["120"] = object.shiftKey;
      objectCson["121"] = object.altKey;
      objectCson["122"] = object.ctrlKey;
      objectCson["123"] = object.metaKey;
      objectCson["130"] = object.button;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): SingleClickEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
      const pressureValue = objectCson["111"];
      const unpackedPressure = pressureValue != undefined ? Number(pressureValue) : undefined;
      const nodePtrValue = objectCson["101"];
      const unpackedNodePtr =
        nodePtrValue != undefined
          ? (_NodeReference.unpack(2, nodePtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[2000202] as typeof SingleClickEvent)({
        button: Number(objectCson["130"]),
        position: _Vector2.unpack(2, objectCson["110"], _session) as Vector2,
        pressure: unpackedPressure,
        shiftKey: Boolean(objectCson["120"]),
        altKey: Boolean(objectCson["121"]),
        ctrlKey: Boolean(objectCson["122"]),
        metaKey: Boolean(objectCson["123"]),
        node: unpackedNodePtr,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 2000202)] = new SingleClickEventCsonEncoder();

  class DoubleClickEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: DoubleClickEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2000203;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      if (object.nodePtr != null) {
        objectCson["101"] = object.nodePtr.pack(2);
      }
      objectCson["110"] = object.position.pack(2);
      if (object.pressure != null) {
        objectCson["111"] = object.pressure;
      }
      objectCson["120"] = object.shiftKey;
      objectCson["121"] = object.altKey;
      objectCson["122"] = object.ctrlKey;
      objectCson["123"] = object.metaKey;
      objectCson["130"] = object.button;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): DoubleClickEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
      const pressureValue = objectCson["111"];
      const unpackedPressure = pressureValue != undefined ? Number(pressureValue) : undefined;
      const nodePtrValue = objectCson["101"];
      const unpackedNodePtr =
        nodePtrValue != undefined
          ? (_NodeReference.unpack(2, nodePtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[2000203] as typeof DoubleClickEvent)({
        button: Number(objectCson["130"]),
        position: _Vector2.unpack(2, objectCson["110"], _session) as Vector2,
        pressure: unpackedPressure,
        shiftKey: Boolean(objectCson["120"]),
        altKey: Boolean(objectCson["121"]),
        ctrlKey: Boolean(objectCson["122"]),
        metaKey: Boolean(objectCson["123"]),
        node: unpackedNodePtr,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 2000203)] = new DoubleClickEventCsonEncoder();

  class TripleClickEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: TripleClickEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2000204;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      if (object.nodePtr != null) {
        objectCson["101"] = object.nodePtr.pack(2);
      }
      objectCson["110"] = object.position.pack(2);
      if (object.pressure != null) {
        objectCson["111"] = object.pressure;
      }
      objectCson["120"] = object.shiftKey;
      objectCson["121"] = object.altKey;
      objectCson["122"] = object.ctrlKey;
      objectCson["123"] = object.metaKey;
      objectCson["130"] = object.button;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): TripleClickEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
      const pressureValue = objectCson["111"];
      const unpackedPressure = pressureValue != undefined ? Number(pressureValue) : undefined;
      const nodePtrValue = objectCson["101"];
      const unpackedNodePtr =
        nodePtrValue != undefined
          ? (_NodeReference.unpack(2, nodePtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[2000204] as typeof TripleClickEvent)({
        button: Number(objectCson["130"]),
        position: _Vector2.unpack(2, objectCson["110"], _session) as Vector2,
        pressure: unpackedPressure,
        shiftKey: Boolean(objectCson["120"]),
        altKey: Boolean(objectCson["121"]),
        ctrlKey: Boolean(objectCson["122"]),
        metaKey: Boolean(objectCson["123"]),
        node: unpackedNodePtr,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 2000204)] = new TripleClickEventCsonEncoder();

  class WheelEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: WheelEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2000210;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      if (object.nodePtr != null) {
        objectCson["101"] = object.nodePtr.pack(2);
      }
      objectCson["110"] = object.position.pack(2);
      if (object.pressure != null) {
        objectCson["111"] = object.pressure;
      }
      objectCson["120"] = object.shiftKey;
      objectCson["121"] = object.altKey;
      objectCson["122"] = object.ctrlKey;
      objectCson["123"] = object.metaKey;
      objectCson["130"] = object.button;
      objectCson["140"] = object.delta.pack(2);
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): WheelEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
      const pressureValue = objectCson["111"];
      const unpackedPressure = pressureValue != undefined ? Number(pressureValue) : undefined;
      const nodePtrValue = objectCson["101"];
      const unpackedNodePtr =
        nodePtrValue != undefined
          ? (_NodeReference.unpack(2, nodePtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[2000210] as typeof WheelEvent)({
        delta: _Vector2.unpack(2, objectCson["140"], _session) as Vector2,
        button: Number(objectCson["130"]),
        position: _Vector2.unpack(2, objectCson["110"], _session) as Vector2,
        pressure: unpackedPressure,
        shiftKey: Boolean(objectCson["120"]),
        altKey: Boolean(objectCson["121"]),
        ctrlKey: Boolean(objectCson["122"]),
        metaKey: Boolean(objectCson["123"]),
        node: unpackedNodePtr,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 2000210)] = new WheelEventCsonEncoder();

  class ScriptCsonEncoder implements CsonObjectEncoder {
    packObject(object: Script): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 700000;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      objectCson["110"] = object._code;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Script {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[700000] as typeof Script)({
        code: objectCson["110"],
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 700000)] = new ScriptCsonEncoder();

  class ServiceCsonEncoder implements CsonObjectEncoder {
    packObject(object: Service): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 10300;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      if (object._icon != null) {
        objectCson["102"] = object._icon.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Service {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
      const iconValue = objectCson["102"];
      const unpackedIcon =
        iconValue != undefined ? (_Icon.unpack(2, iconValue, _session) as Icon) : undefined;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[10300] as typeof Service)({
        icon: unpackedIcon,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 10300)] = new ServiceCsonEncoder();

  class TimerStartedEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: TimerStartedEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 705102;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      objectCson["101"] = object.nodePtr.pack(2);
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): TimerStartedEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[705102] as typeof TimerStartedEvent)({
        node: _NodeReference.unpack(2, objectCson["101"], _session) as NodeReference,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 705102)] = new TimerStartedEventCsonEncoder();

  class TimerPausedEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: TimerPausedEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 705103;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      objectCson["101"] = object.nodePtr.pack(2);
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): TimerPausedEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[705103] as typeof TimerPausedEvent)({
        node: _NodeReference.unpack(2, objectCson["101"], _session) as NodeReference,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 705103)] = new TimerPausedEventCsonEncoder();

  class TimerResumedEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: TimerResumedEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 705104;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      objectCson["101"] = object.nodePtr.pack(2);
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): TimerResumedEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[705104] as typeof TimerResumedEvent)({
        node: _NodeReference.unpack(2, objectCson["101"], _session) as NodeReference,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 705104)] = new TimerResumedEventCsonEncoder();

  class TimerCompletedEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: TimerCompletedEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 705105;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      objectCson["101"] = object.nodePtr.pack(2);
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): TimerCompletedEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[705105] as typeof TimerCompletedEvent)({
        node: _NodeReference.unpack(2, objectCson["101"], _session) as NodeReference,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 705105)] = new TimerCompletedEventCsonEncoder();

  class TimerCancelledEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: TimerCancelledEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 705106;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      objectCson["101"] = object.nodePtr.pack(2);
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): TimerCancelledEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[705106] as typeof TimerCancelledEvent)({
        node: _NodeReference.unpack(2, objectCson["101"], _session) as NodeReference,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 705106)] = new TimerCancelledEventCsonEncoder();

  class TimerCsonEncoder implements CsonObjectEncoder {
    packObject(object: Timer): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 705100;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      objectCson["100"] = object._type;
      if (object._schedule != null) {
        objectCson["110"] = object._schedule.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Timer {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Schedule = STRUCT_CLASS_BY_TYPE[700001] as typeof Schedule;
      const scheduleValue = objectCson["110"];
      const unpackedSchedule =
        scheduleValue != undefined
          ? (_Schedule.unpack(2, scheduleValue, _session) as Schedule)
          : undefined;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[705100] as typeof Timer)({
        type: Number(objectCson["100"]),
        schedule: unpackedSchedule,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 705100)] = new TimerCsonEncoder();

  class TriggerCsonEncoder implements CsonObjectEncoder {
    packObject(object: Trigger): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 705000;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      if (object._icon != null) {
        objectCson["102"] = object._icon.pack(2);
      }
      if (object._event != null) {
        objectCson["110"] = object._event.pack(2);
      }
      if (object._where != null) {
        objectCson["111"] = object._where.pack(2);
      }
      if (object._targetPtr != null) {
        objectCson["120"] = object._targetPtr.pack(2);
      }
      const packedArguments: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._arguments)) {
        packedArguments[String(key)] = value.pack(2);
      }
      objectCson["121"] = packedArguments;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Trigger {
      const _NodeDefinitionReference = STRUCT_CLASS_BY_TYPE[13] as typeof NodeDefinitionReference;
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _Condition = STRUCT_CLASS_BY_TYPE[204] as typeof Condition;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
      const iconValue = objectCson["102"];
      const unpackedIcon =
        iconValue != undefined ? (_Icon.unpack(2, iconValue, _session) as Icon) : undefined;
      const eventValue = objectCson["110"];
      const unpackedEvent =
        eventValue != undefined
          ? (_NodeDefinitionReference.unpack(2, eventValue, _session) as NodeDefinitionReference)
          : undefined;
      const whereValue = objectCson["111"];
      const unpackedWhere =
        whereValue != undefined
          ? (_Condition.unpack(2, whereValue, _session) as Condition)
          : undefined;
      const targetPtrValue = objectCson["120"];
      const unpackedTargetPtr =
        targetPtrValue != undefined
          ? (_NodeReference.unpack(2, targetPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedArguments = {} as any;
      for (const [key, value] of Object.entries(objectCson["121"])) {
        unpackedArguments[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[705000] as typeof Trigger)({
        icon: unpackedIcon,
        event: unpackedEvent,
        where: unpackedWhere,
        target: unpackedTargetPtr,
        arguments: unpackedArguments,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 705000)] = new TriggerCsonEncoder();

  class GaugeMetricCsonEncoder implements CsonObjectEncoder {
    packObject(object: GaugeMetric): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1200000;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      if (object._icon != null) {
        objectCson["102"] = object._icon.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): GaugeMetric {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
      const iconValue = objectCson["102"];
      const unpackedIcon =
        iconValue != undefined ? (_Icon.unpack(2, iconValue, _session) as Icon) : undefined;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[1200000] as typeof GaugeMetric)({
        icon: unpackedIcon,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1200000)] = new GaugeMetricCsonEncoder();

  class GaugeMeasurementEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: GaugeMeasurementEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1200001;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["6"] = object.definitionPtr.pack(2);
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      if (object.nodePtr != null) {
        objectCson["101"] = object.nodePtr.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): GaugeMeasurementEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      const nodePtrValue = objectCson["101"];
      const unpackedNodePtr =
        nodePtrValue != undefined
          ? (_NodeReference.unpack(2, nodePtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[1200001] as typeof GaugeMeasurementEvent)({
        definition: _NodeReference.unpack(2, objectCson["6"], _session) as NodeReference,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        node: unpackedNodePtr,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1200001)] = new GaugeMeasurementEventCsonEncoder();

  class CounterMetricCsonEncoder implements CsonObjectEncoder {
    packObject(object: CounterMetric): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1200100;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      if (object._icon != null) {
        objectCson["102"] = object._icon.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): CounterMetric {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
      const iconValue = objectCson["102"];
      const unpackedIcon =
        iconValue != undefined ? (_Icon.unpack(2, iconValue, _session) as Icon) : undefined;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[1200100] as typeof CounterMetric)({
        icon: unpackedIcon,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1200100)] = new CounterMetricCsonEncoder();

  class CounterMeasurementEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: CounterMeasurementEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1200101;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["6"] = object.definitionPtr.pack(2);
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      if (object.nodePtr != null) {
        objectCson["101"] = object.nodePtr.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): CounterMeasurementEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      const nodePtrValue = objectCson["101"];
      const unpackedNodePtr =
        nodePtrValue != undefined
          ? (_NodeReference.unpack(2, nodePtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[1200101] as typeof CounterMeasurementEvent)({
        definition: _NodeReference.unpack(2, objectCson["6"], _session) as NodeReference,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        node: unpackedNodePtr,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1200101)] = new CounterMeasurementEventCsonEncoder();

  class HistogramMetricCsonEncoder implements CsonObjectEncoder {
    packObject(object: HistogramMetric): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1200200;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      if (object._icon != null) {
        objectCson["102"] = object._icon.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): HistogramMetric {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
      const iconValue = objectCson["102"];
      const unpackedIcon =
        iconValue != undefined ? (_Icon.unpack(2, iconValue, _session) as Icon) : undefined;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[1200200] as typeof HistogramMetric)({
        icon: unpackedIcon,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1200200)] = new HistogramMetricCsonEncoder();

  class HistogramMeasurementEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: HistogramMeasurementEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1200201;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["6"] = object.definitionPtr.pack(2);
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      if (object.nodePtr != null) {
        objectCson["101"] = object.nodePtr.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): HistogramMeasurementEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      const nodePtrValue = objectCson["101"];
      const unpackedNodePtr =
        nodePtrValue != undefined
          ? (_NodeReference.unpack(2, nodePtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[1200201] as typeof HistogramMeasurementEvent)({
        definition: _NodeReference.unpack(2, objectCson["6"], _session) as NodeReference,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        node: unpackedNodePtr,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1200201)] = new HistogramMeasurementEventCsonEncoder();

  class LayerCsonEncoder implements CsonObjectEncoder {
    packObject(object: Layer): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1700300;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      objectCson["100"] = object._type;
      if (object._icon != null) {
        objectCson["102"] = object._icon.pack(2);
      }
      if (object._isVisible != null) {
        objectCson["140"] = object._isVisible;
      }
      if (object._opacity != null) {
        objectCson["141"] = object._opacity;
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Layer {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
      const iconValue = objectCson["102"];
      const unpackedIcon =
        iconValue != undefined ? (_Icon.unpack(2, iconValue, _session) as Icon) : undefined;
      const isVisibleValue = objectCson["140"];
      const unpackedIsVisible = isVisibleValue != undefined ? Boolean(isVisibleValue) : undefined;
      const opacityValue = objectCson["141"];
      const unpackedOpacity = opacityValue != undefined ? Number(opacityValue) : undefined;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[1700300] as typeof Layer)({
        type: Number(objectCson["100"]),
        icon: unpackedIcon,
        isVisible: unpackedIsVisible,
        opacity: unpackedOpacity,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1700300)] = new LayerCsonEncoder();

  class FrameViewCsonEncoder implements CsonObjectEncoder {
    packObject(object: FrameView): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1800200;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      if (object._position != null) {
        objectCson["110"] = object._position.pack(2);
      }
      if (object._offset != null) {
        objectCson["111"] = object._offset.pack(2);
      }
      if (object._scale != null) {
        objectCson["112"] = object._scale.pack(2);
      }
      if (object._rotation != null) {
        objectCson["113"] = object._rotation.pack(2);
      }
      if (object._skew != null) {
        objectCson["114"] = object._skew.pack(2);
      }
      if (object._origin != null) {
        objectCson["115"] = object._origin.pack(2);
      }
      if (object._anchor != null) {
        objectCson["116"] = object._anchor;
      }
      if (object._width != null) {
        objectCson["120"] = object._width.pack(2);
      }
      if (object._height != null) {
        objectCson["121"] = object._height.pack(2);
      }
      if (object._minWidth != null) {
        objectCson["122"] = object._minWidth.pack(2);
      }
      if (object._minHeight != null) {
        objectCson["123"] = object._minHeight.pack(2);
      }
      if (object._maxWidth != null) {
        objectCson["124"] = object._maxWidth.pack(2);
      }
      if (object._maxHeight != null) {
        objectCson["125"] = object._maxHeight.pack(2);
      }
      if (object._isVisible != null) {
        objectCson["130"] = object._isVisible;
      }
      if (object._opacity != null) {
        objectCson["131"] = object._opacity;
      }
      if (object._fill != null) {
        objectCson["140"] = object._fill.pack(2);
      }
      if (object._shadow != null) {
        objectCson["141"] = object._shadow.pack(2);
      }
      if (object._border != null) {
        objectCson["142"] = object._border.pack(2);
      }
      if (object._radius != null) {
        objectCson["143"] = object._radius.pack(2);
      }
      if (object._layout != null) {
        objectCson["150"] = object._layout;
      }
      if (object._direction != null) {
        objectCson["151"] = object._direction;
      }
      if (object._distribute != null) {
        objectCson["152"] = object._distribute;
      }
      if (object._align != null) {
        objectCson["153"] = object._align;
      }
      if (object._gap != null) {
        objectCson["154"] = object._gap.pack(2);
      }
      if (object._padding != null) {
        objectCson["155"] = object._padding.pack(2);
      }
      if (object._grid != null) {
        objectCson["156"] = object._grid.pack(2);
      }
      if (object._gridSpan != null) {
        objectCson["157"] = object._gridSpan.pack(2);
      }
      if (object._aspectRatio != null) {
        objectCson["158"] = object._aspectRatio;
      }
      if (object._isWrap != null) {
        objectCson["159"] = object._isWrap;
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): FrameView {
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
      const layoutValue = objectCson["150"];
      const unpackedLayout = layoutValue != undefined ? Number(layoutValue) : undefined;
      const directionValue = objectCson["151"];
      const unpackedDirection = directionValue != undefined ? Number(directionValue) : undefined;
      const distributeValue = objectCson["152"];
      const unpackedDistribute = distributeValue != undefined ? Number(distributeValue) : undefined;
      const alignValue = objectCson["153"];
      const unpackedAlign = alignValue != undefined ? Number(alignValue) : undefined;
      const gapValue = objectCson["154"];
      const unpackedGap =
        gapValue != undefined ? (_Axis2.unpack(2, gapValue, _session) as Axis2) : undefined;
      const paddingValue = objectCson["155"];
      const unpackedPadding =
        paddingValue != undefined
          ? (_Inset2.unpack(2, paddingValue, _session) as Inset2)
          : undefined;
      const gridValue = objectCson["156"];
      const unpackedGrid =
        gridValue != undefined ? (_Grid2.unpack(2, gridValue, _session) as Grid2) : undefined;
      const gridSpanValue = objectCson["157"];
      const unpackedGridSpan =
        gridSpanValue != undefined
          ? (_GridSpan2.unpack(2, gridSpanValue, _session) as GridSpan2)
          : undefined;
      const aspectRatioValue = objectCson["158"];
      const unpackedAspectRatio =
        aspectRatioValue != undefined ? Number(aspectRatioValue) : undefined;
      const isWrapValue = objectCson["159"];
      const unpackedIsWrap = isWrapValue != undefined ? Boolean(isWrapValue) : undefined;
      const widthValue = objectCson["120"];
      const unpackedWidth =
        widthValue != undefined ? (_Length.unpack(2, widthValue, _session) as Length) : undefined;
      const heightValue = objectCson["121"];
      const unpackedHeight =
        heightValue != undefined ? (_Length.unpack(2, heightValue, _session) as Length) : undefined;
      const minWidthValue = objectCson["122"];
      const unpackedMinWidth =
        minWidthValue != undefined
          ? (_Length.unpack(2, minWidthValue, _session) as Length)
          : undefined;
      const minHeightValue = objectCson["123"];
      const unpackedMinHeight =
        minHeightValue != undefined
          ? (_Length.unpack(2, minHeightValue, _session) as Length)
          : undefined;
      const maxWidthValue = objectCson["124"];
      const unpackedMaxWidth =
        maxWidthValue != undefined
          ? (_Length.unpack(2, maxWidthValue, _session) as Length)
          : undefined;
      const maxHeightValue = objectCson["125"];
      const unpackedMaxHeight =
        maxHeightValue != undefined
          ? (_Length.unpack(2, maxHeightValue, _session) as Length)
          : undefined;
      const isVisibleValue = objectCson["130"];
      const unpackedIsVisible = isVisibleValue != undefined ? Boolean(isVisibleValue) : undefined;
      const opacityValue = objectCson["131"];
      const unpackedOpacity = opacityValue != undefined ? Number(opacityValue) : undefined;
      const fillValue = objectCson["140"];
      const unpackedFill =
        fillValue != undefined ? (_Fill.unpack(2, fillValue, _session) as Fill) : undefined;
      const shadowValue = objectCson["141"];
      const unpackedShadow =
        shadowValue != undefined ? (_Shadow.unpack(2, shadowValue, _session) as Shadow) : undefined;
      const borderValue = objectCson["142"];
      const unpackedBorder =
        borderValue != undefined ? (_Border.unpack(2, borderValue, _session) as Border) : undefined;
      const radiusValue = objectCson["143"];
      const unpackedRadius =
        radiusValue != undefined
          ? (_Corner2.unpack(2, radiusValue, _session) as Corner2)
          : undefined;
      const positionValue = objectCson["110"];
      const unpackedPosition =
        positionValue != undefined
          ? (_Vector2.unpack(2, positionValue, _session) as Vector2)
          : undefined;
      const offsetValue = objectCson["111"];
      const unpackedOffset =
        offsetValue != undefined
          ? (_Offset2.unpack(2, offsetValue, _session) as Offset2)
          : undefined;
      const scaleValue = objectCson["112"];
      const unpackedScale =
        scaleValue != undefined ? (_Vector2.unpack(2, scaleValue, _session) as Vector2) : undefined;
      const rotationValue = objectCson["113"];
      const unpackedRotation =
        rotationValue != undefined
          ? (_Vector2.unpack(2, rotationValue, _session) as Vector2)
          : undefined;
      const skewValue = objectCson["114"];
      const unpackedSkew =
        skewValue != undefined ? (_Vector2.unpack(2, skewValue, _session) as Vector2) : undefined;
      const originValue = objectCson["115"];
      const unpackedOrigin =
        originValue != undefined
          ? (_Vector2.unpack(2, originValue, _session) as Vector2)
          : undefined;
      const anchorValue = objectCson["116"];
      const unpackedAnchor = anchorValue != undefined ? Number(anchorValue) : undefined;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[1800200] as typeof FrameView)({
        layout: unpackedLayout,
        direction: unpackedDirection,
        distribute: unpackedDistribute,
        align: unpackedAlign,
        gap: unpackedGap,
        padding: unpackedPadding,
        grid: unpackedGrid,
        gridSpan: unpackedGridSpan,
        aspectRatio: unpackedAspectRatio,
        isWrap: unpackedIsWrap,
        width: unpackedWidth,
        height: unpackedHeight,
        minWidth: unpackedMinWidth,
        minHeight: unpackedMinHeight,
        maxWidth: unpackedMaxWidth,
        maxHeight: unpackedMaxHeight,
        isVisible: unpackedIsVisible,
        opacity: unpackedOpacity,
        fill: unpackedFill,
        shadow: unpackedShadow,
        border: unpackedBorder,
        radius: unpackedRadius,
        position: unpackedPosition,
        offset: unpackedOffset,
        scale: unpackedScale,
        rotation: unpackedRotation,
        skew: unpackedSkew,
        origin: unpackedOrigin,
        anchor: unpackedAnchor,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1800200)] = new FrameViewCsonEncoder();

  class LabelViewCsonEncoder implements CsonObjectEncoder {
    packObject(object: LabelView): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1800300;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      if (object._position != null) {
        objectCson["110"] = object._position.pack(2);
      }
      if (object._offset != null) {
        objectCson["111"] = object._offset.pack(2);
      }
      if (object._scale != null) {
        objectCson["112"] = object._scale.pack(2);
      }
      if (object._rotation != null) {
        objectCson["113"] = object._rotation.pack(2);
      }
      if (object._skew != null) {
        objectCson["114"] = object._skew.pack(2);
      }
      if (object._origin != null) {
        objectCson["115"] = object._origin.pack(2);
      }
      if (object._anchor != null) {
        objectCson["116"] = object._anchor;
      }
      if (object._width != null) {
        objectCson["120"] = object._width.pack(2);
      }
      if (object._height != null) {
        objectCson["121"] = object._height.pack(2);
      }
      if (object._minWidth != null) {
        objectCson["122"] = object._minWidth.pack(2);
      }
      if (object._minHeight != null) {
        objectCson["123"] = object._minHeight.pack(2);
      }
      if (object._maxWidth != null) {
        objectCson["124"] = object._maxWidth.pack(2);
      }
      if (object._maxHeight != null) {
        objectCson["125"] = object._maxHeight.pack(2);
      }
      if (object._isVisible != null) {
        objectCson["130"] = object._isVisible;
      }
      if (object._opacity != null) {
        objectCson["131"] = object._opacity;
      }
      if (object._fill != null) {
        objectCson["140"] = object._fill.pack(2);
      }
      if (object._shadow != null) {
        objectCson["141"] = object._shadow.pack(2);
      }
      if (object._border != null) {
        objectCson["142"] = object._border.pack(2);
      }
      if (object._radius != null) {
        objectCson["143"] = object._radius.pack(2);
      }
      if (object._layout != null) {
        objectCson["150"] = object._layout;
      }
      if (object._direction != null) {
        objectCson["151"] = object._direction;
      }
      if (object._distribute != null) {
        objectCson["152"] = object._distribute;
      }
      if (object._align != null) {
        objectCson["153"] = object._align;
      }
      if (object._gap != null) {
        objectCson["154"] = object._gap.pack(2);
      }
      if (object._padding != null) {
        objectCson["155"] = object._padding.pack(2);
      }
      if (object._grid != null) {
        objectCson["156"] = object._grid.pack(2);
      }
      if (object._gridSpan != null) {
        objectCson["157"] = object._gridSpan.pack(2);
      }
      if (object._aspectRatio != null) {
        objectCson["158"] = object._aspectRatio;
      }
      if (object._isWrap != null) {
        objectCson["159"] = object._isWrap;
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): LabelView {
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
      const layoutValue = objectCson["150"];
      const unpackedLayout = layoutValue != undefined ? Number(layoutValue) : undefined;
      const directionValue = objectCson["151"];
      const unpackedDirection = directionValue != undefined ? Number(directionValue) : undefined;
      const distributeValue = objectCson["152"];
      const unpackedDistribute = distributeValue != undefined ? Number(distributeValue) : undefined;
      const alignValue = objectCson["153"];
      const unpackedAlign = alignValue != undefined ? Number(alignValue) : undefined;
      const gapValue = objectCson["154"];
      const unpackedGap =
        gapValue != undefined ? (_Axis2.unpack(2, gapValue, _session) as Axis2) : undefined;
      const paddingValue = objectCson["155"];
      const unpackedPadding =
        paddingValue != undefined
          ? (_Inset2.unpack(2, paddingValue, _session) as Inset2)
          : undefined;
      const gridValue = objectCson["156"];
      const unpackedGrid =
        gridValue != undefined ? (_Grid2.unpack(2, gridValue, _session) as Grid2) : undefined;
      const gridSpanValue = objectCson["157"];
      const unpackedGridSpan =
        gridSpanValue != undefined
          ? (_GridSpan2.unpack(2, gridSpanValue, _session) as GridSpan2)
          : undefined;
      const aspectRatioValue = objectCson["158"];
      const unpackedAspectRatio =
        aspectRatioValue != undefined ? Number(aspectRatioValue) : undefined;
      const isWrapValue = objectCson["159"];
      const unpackedIsWrap = isWrapValue != undefined ? Boolean(isWrapValue) : undefined;
      const widthValue = objectCson["120"];
      const unpackedWidth =
        widthValue != undefined ? (_Length.unpack(2, widthValue, _session) as Length) : undefined;
      const heightValue = objectCson["121"];
      const unpackedHeight =
        heightValue != undefined ? (_Length.unpack(2, heightValue, _session) as Length) : undefined;
      const minWidthValue = objectCson["122"];
      const unpackedMinWidth =
        minWidthValue != undefined
          ? (_Length.unpack(2, minWidthValue, _session) as Length)
          : undefined;
      const minHeightValue = objectCson["123"];
      const unpackedMinHeight =
        minHeightValue != undefined
          ? (_Length.unpack(2, minHeightValue, _session) as Length)
          : undefined;
      const maxWidthValue = objectCson["124"];
      const unpackedMaxWidth =
        maxWidthValue != undefined
          ? (_Length.unpack(2, maxWidthValue, _session) as Length)
          : undefined;
      const maxHeightValue = objectCson["125"];
      const unpackedMaxHeight =
        maxHeightValue != undefined
          ? (_Length.unpack(2, maxHeightValue, _session) as Length)
          : undefined;
      const isVisibleValue = objectCson["130"];
      const unpackedIsVisible = isVisibleValue != undefined ? Boolean(isVisibleValue) : undefined;
      const opacityValue = objectCson["131"];
      const unpackedOpacity = opacityValue != undefined ? Number(opacityValue) : undefined;
      const fillValue = objectCson["140"];
      const unpackedFill =
        fillValue != undefined ? (_Fill.unpack(2, fillValue, _session) as Fill) : undefined;
      const shadowValue = objectCson["141"];
      const unpackedShadow =
        shadowValue != undefined ? (_Shadow.unpack(2, shadowValue, _session) as Shadow) : undefined;
      const borderValue = objectCson["142"];
      const unpackedBorder =
        borderValue != undefined ? (_Border.unpack(2, borderValue, _session) as Border) : undefined;
      const radiusValue = objectCson["143"];
      const unpackedRadius =
        radiusValue != undefined
          ? (_Corner2.unpack(2, radiusValue, _session) as Corner2)
          : undefined;
      const positionValue = objectCson["110"];
      const unpackedPosition =
        positionValue != undefined
          ? (_Vector2.unpack(2, positionValue, _session) as Vector2)
          : undefined;
      const offsetValue = objectCson["111"];
      const unpackedOffset =
        offsetValue != undefined
          ? (_Offset2.unpack(2, offsetValue, _session) as Offset2)
          : undefined;
      const scaleValue = objectCson["112"];
      const unpackedScale =
        scaleValue != undefined ? (_Vector2.unpack(2, scaleValue, _session) as Vector2) : undefined;
      const rotationValue = objectCson["113"];
      const unpackedRotation =
        rotationValue != undefined
          ? (_Vector2.unpack(2, rotationValue, _session) as Vector2)
          : undefined;
      const skewValue = objectCson["114"];
      const unpackedSkew =
        skewValue != undefined ? (_Vector2.unpack(2, skewValue, _session) as Vector2) : undefined;
      const originValue = objectCson["115"];
      const unpackedOrigin =
        originValue != undefined
          ? (_Vector2.unpack(2, originValue, _session) as Vector2)
          : undefined;
      const anchorValue = objectCson["116"];
      const unpackedAnchor = anchorValue != undefined ? Number(anchorValue) : undefined;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[1800300] as typeof LabelView)({
        layout: unpackedLayout,
        direction: unpackedDirection,
        distribute: unpackedDistribute,
        align: unpackedAlign,
        gap: unpackedGap,
        padding: unpackedPadding,
        grid: unpackedGrid,
        gridSpan: unpackedGridSpan,
        aspectRatio: unpackedAspectRatio,
        isWrap: unpackedIsWrap,
        width: unpackedWidth,
        height: unpackedHeight,
        minWidth: unpackedMinWidth,
        minHeight: unpackedMinHeight,
        maxWidth: unpackedMaxWidth,
        maxHeight: unpackedMaxHeight,
        isVisible: unpackedIsVisible,
        opacity: unpackedOpacity,
        fill: unpackedFill,
        shadow: unpackedShadow,
        border: unpackedBorder,
        radius: unpackedRadius,
        position: unpackedPosition,
        offset: unpackedOffset,
        scale: unpackedScale,
        rotation: unpackedRotation,
        skew: unpackedSkew,
        origin: unpackedOrigin,
        anchor: unpackedAnchor,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1800300)] = new LabelViewCsonEncoder();

  class NumberInputViewCsonEncoder implements CsonObjectEncoder {
    packObject(object: NumberInputView): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1810100;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      if (object._position != null) {
        objectCson["110"] = object._position.pack(2);
      }
      if (object._offset != null) {
        objectCson["111"] = object._offset.pack(2);
      }
      if (object._scale != null) {
        objectCson["112"] = object._scale.pack(2);
      }
      if (object._rotation != null) {
        objectCson["113"] = object._rotation.pack(2);
      }
      if (object._skew != null) {
        objectCson["114"] = object._skew.pack(2);
      }
      if (object._origin != null) {
        objectCson["115"] = object._origin.pack(2);
      }
      if (object._anchor != null) {
        objectCson["116"] = object._anchor;
      }
      if (object._width != null) {
        objectCson["120"] = object._width.pack(2);
      }
      if (object._height != null) {
        objectCson["121"] = object._height.pack(2);
      }
      if (object._minWidth != null) {
        objectCson["122"] = object._minWidth.pack(2);
      }
      if (object._minHeight != null) {
        objectCson["123"] = object._minHeight.pack(2);
      }
      if (object._maxWidth != null) {
        objectCson["124"] = object._maxWidth.pack(2);
      }
      if (object._maxHeight != null) {
        objectCson["125"] = object._maxHeight.pack(2);
      }
      if (object._isVisible != null) {
        objectCson["130"] = object._isVisible;
      }
      if (object._opacity != null) {
        objectCson["131"] = object._opacity;
      }
      if (object._fill != null) {
        objectCson["140"] = object._fill.pack(2);
      }
      if (object._shadow != null) {
        objectCson["141"] = object._shadow.pack(2);
      }
      if (object._border != null) {
        objectCson["142"] = object._border.pack(2);
      }
      if (object._radius != null) {
        objectCson["143"] = object._radius.pack(2);
      }
      if (object._value != null) {
        objectCson["250"] = object._value;
      }
      if (object._placeholder != null) {
        objectCson["251"] = object._placeholder;
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): NumberInputView {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Length = STRUCT_CLASS_BY_TYPE[1800001] as typeof Length;
      const _Fill = STRUCT_CLASS_BY_TYPE[2100400] as typeof Fill;
      const _Border = STRUCT_CLASS_BY_TYPE[2100600] as typeof Border;
      const _Shadow = STRUCT_CLASS_BY_TYPE[2100700] as typeof Shadow;
      const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
      const _Offset2 = STRUCT_CLASS_BY_TYPE[2400020] as typeof Offset2;
      const _Corner2 = STRUCT_CLASS_BY_TYPE[2400024] as typeof Corner2;
      const valueValue = objectCson["250"];
      const unpackedValue = valueValue != undefined ? Number(valueValue) : undefined;
      const placeholderValue = objectCson["251"];
      const unpackedPlaceholder = placeholderValue != undefined ? placeholderValue : undefined;
      const widthValue = objectCson["120"];
      const unpackedWidth =
        widthValue != undefined ? (_Length.unpack(2, widthValue, _session) as Length) : undefined;
      const heightValue = objectCson["121"];
      const unpackedHeight =
        heightValue != undefined ? (_Length.unpack(2, heightValue, _session) as Length) : undefined;
      const minWidthValue = objectCson["122"];
      const unpackedMinWidth =
        minWidthValue != undefined
          ? (_Length.unpack(2, minWidthValue, _session) as Length)
          : undefined;
      const minHeightValue = objectCson["123"];
      const unpackedMinHeight =
        minHeightValue != undefined
          ? (_Length.unpack(2, minHeightValue, _session) as Length)
          : undefined;
      const maxWidthValue = objectCson["124"];
      const unpackedMaxWidth =
        maxWidthValue != undefined
          ? (_Length.unpack(2, maxWidthValue, _session) as Length)
          : undefined;
      const maxHeightValue = objectCson["125"];
      const unpackedMaxHeight =
        maxHeightValue != undefined
          ? (_Length.unpack(2, maxHeightValue, _session) as Length)
          : undefined;
      const isVisibleValue = objectCson["130"];
      const unpackedIsVisible = isVisibleValue != undefined ? Boolean(isVisibleValue) : undefined;
      const opacityValue = objectCson["131"];
      const unpackedOpacity = opacityValue != undefined ? Number(opacityValue) : undefined;
      const fillValue = objectCson["140"];
      const unpackedFill =
        fillValue != undefined ? (_Fill.unpack(2, fillValue, _session) as Fill) : undefined;
      const shadowValue = objectCson["141"];
      const unpackedShadow =
        shadowValue != undefined ? (_Shadow.unpack(2, shadowValue, _session) as Shadow) : undefined;
      const borderValue = objectCson["142"];
      const unpackedBorder =
        borderValue != undefined ? (_Border.unpack(2, borderValue, _session) as Border) : undefined;
      const radiusValue = objectCson["143"];
      const unpackedRadius =
        radiusValue != undefined
          ? (_Corner2.unpack(2, radiusValue, _session) as Corner2)
          : undefined;
      const positionValue = objectCson["110"];
      const unpackedPosition =
        positionValue != undefined
          ? (_Vector2.unpack(2, positionValue, _session) as Vector2)
          : undefined;
      const offsetValue = objectCson["111"];
      const unpackedOffset =
        offsetValue != undefined
          ? (_Offset2.unpack(2, offsetValue, _session) as Offset2)
          : undefined;
      const scaleValue = objectCson["112"];
      const unpackedScale =
        scaleValue != undefined ? (_Vector2.unpack(2, scaleValue, _session) as Vector2) : undefined;
      const rotationValue = objectCson["113"];
      const unpackedRotation =
        rotationValue != undefined
          ? (_Vector2.unpack(2, rotationValue, _session) as Vector2)
          : undefined;
      const skewValue = objectCson["114"];
      const unpackedSkew =
        skewValue != undefined ? (_Vector2.unpack(2, skewValue, _session) as Vector2) : undefined;
      const originValue = objectCson["115"];
      const unpackedOrigin =
        originValue != undefined
          ? (_Vector2.unpack(2, originValue, _session) as Vector2)
          : undefined;
      const anchorValue = objectCson["116"];
      const unpackedAnchor = anchorValue != undefined ? Number(anchorValue) : undefined;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[1810100] as typeof NumberInputView)({
        value: unpackedValue,
        placeholder: unpackedPlaceholder,
        width: unpackedWidth,
        height: unpackedHeight,
        minWidth: unpackedMinWidth,
        minHeight: unpackedMinHeight,
        maxWidth: unpackedMaxWidth,
        maxHeight: unpackedMaxHeight,
        isVisible: unpackedIsVisible,
        opacity: unpackedOpacity,
        fill: unpackedFill,
        shadow: unpackedShadow,
        border: unpackedBorder,
        radius: unpackedRadius,
        position: unpackedPosition,
        offset: unpackedOffset,
        scale: unpackedScale,
        rotation: unpackedRotation,
        skew: unpackedSkew,
        origin: unpackedOrigin,
        anchor: unpackedAnchor,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1810100)] = new NumberInputViewCsonEncoder();

  class SliderInputViewCsonEncoder implements CsonObjectEncoder {
    packObject(object: SliderInputView): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1810200;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      if (object._position != null) {
        objectCson["110"] = object._position.pack(2);
      }
      if (object._offset != null) {
        objectCson["111"] = object._offset.pack(2);
      }
      if (object._scale != null) {
        objectCson["112"] = object._scale.pack(2);
      }
      if (object._rotation != null) {
        objectCson["113"] = object._rotation.pack(2);
      }
      if (object._skew != null) {
        objectCson["114"] = object._skew.pack(2);
      }
      if (object._origin != null) {
        objectCson["115"] = object._origin.pack(2);
      }
      if (object._anchor != null) {
        objectCson["116"] = object._anchor;
      }
      if (object._width != null) {
        objectCson["120"] = object._width.pack(2);
      }
      if (object._height != null) {
        objectCson["121"] = object._height.pack(2);
      }
      if (object._minWidth != null) {
        objectCson["122"] = object._minWidth.pack(2);
      }
      if (object._minHeight != null) {
        objectCson["123"] = object._minHeight.pack(2);
      }
      if (object._maxWidth != null) {
        objectCson["124"] = object._maxWidth.pack(2);
      }
      if (object._maxHeight != null) {
        objectCson["125"] = object._maxHeight.pack(2);
      }
      if (object._isVisible != null) {
        objectCson["130"] = object._isVisible;
      }
      if (object._opacity != null) {
        objectCson["131"] = object._opacity;
      }
      if (object._fill != null) {
        objectCson["140"] = object._fill.pack(2);
      }
      if (object._shadow != null) {
        objectCson["141"] = object._shadow.pack(2);
      }
      if (object._border != null) {
        objectCson["142"] = object._border.pack(2);
      }
      if (object._radius != null) {
        objectCson["143"] = object._radius.pack(2);
      }
      if (object._value != null) {
        objectCson["250"] = object._value;
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): SliderInputView {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Length = STRUCT_CLASS_BY_TYPE[1800001] as typeof Length;
      const _Fill = STRUCT_CLASS_BY_TYPE[2100400] as typeof Fill;
      const _Border = STRUCT_CLASS_BY_TYPE[2100600] as typeof Border;
      const _Shadow = STRUCT_CLASS_BY_TYPE[2100700] as typeof Shadow;
      const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
      const _Offset2 = STRUCT_CLASS_BY_TYPE[2400020] as typeof Offset2;
      const _Corner2 = STRUCT_CLASS_BY_TYPE[2400024] as typeof Corner2;
      const valueValue = objectCson["250"];
      const unpackedValue = valueValue != undefined ? Number(valueValue) : undefined;
      const widthValue = objectCson["120"];
      const unpackedWidth =
        widthValue != undefined ? (_Length.unpack(2, widthValue, _session) as Length) : undefined;
      const heightValue = objectCson["121"];
      const unpackedHeight =
        heightValue != undefined ? (_Length.unpack(2, heightValue, _session) as Length) : undefined;
      const minWidthValue = objectCson["122"];
      const unpackedMinWidth =
        minWidthValue != undefined
          ? (_Length.unpack(2, minWidthValue, _session) as Length)
          : undefined;
      const minHeightValue = objectCson["123"];
      const unpackedMinHeight =
        minHeightValue != undefined
          ? (_Length.unpack(2, minHeightValue, _session) as Length)
          : undefined;
      const maxWidthValue = objectCson["124"];
      const unpackedMaxWidth =
        maxWidthValue != undefined
          ? (_Length.unpack(2, maxWidthValue, _session) as Length)
          : undefined;
      const maxHeightValue = objectCson["125"];
      const unpackedMaxHeight =
        maxHeightValue != undefined
          ? (_Length.unpack(2, maxHeightValue, _session) as Length)
          : undefined;
      const isVisibleValue = objectCson["130"];
      const unpackedIsVisible = isVisibleValue != undefined ? Boolean(isVisibleValue) : undefined;
      const opacityValue = objectCson["131"];
      const unpackedOpacity = opacityValue != undefined ? Number(opacityValue) : undefined;
      const fillValue = objectCson["140"];
      const unpackedFill =
        fillValue != undefined ? (_Fill.unpack(2, fillValue, _session) as Fill) : undefined;
      const shadowValue = objectCson["141"];
      const unpackedShadow =
        shadowValue != undefined ? (_Shadow.unpack(2, shadowValue, _session) as Shadow) : undefined;
      const borderValue = objectCson["142"];
      const unpackedBorder =
        borderValue != undefined ? (_Border.unpack(2, borderValue, _session) as Border) : undefined;
      const radiusValue = objectCson["143"];
      const unpackedRadius =
        radiusValue != undefined
          ? (_Corner2.unpack(2, radiusValue, _session) as Corner2)
          : undefined;
      const positionValue = objectCson["110"];
      const unpackedPosition =
        positionValue != undefined
          ? (_Vector2.unpack(2, positionValue, _session) as Vector2)
          : undefined;
      const offsetValue = objectCson["111"];
      const unpackedOffset =
        offsetValue != undefined
          ? (_Offset2.unpack(2, offsetValue, _session) as Offset2)
          : undefined;
      const scaleValue = objectCson["112"];
      const unpackedScale =
        scaleValue != undefined ? (_Vector2.unpack(2, scaleValue, _session) as Vector2) : undefined;
      const rotationValue = objectCson["113"];
      const unpackedRotation =
        rotationValue != undefined
          ? (_Vector2.unpack(2, rotationValue, _session) as Vector2)
          : undefined;
      const skewValue = objectCson["114"];
      const unpackedSkew =
        skewValue != undefined ? (_Vector2.unpack(2, skewValue, _session) as Vector2) : undefined;
      const originValue = objectCson["115"];
      const unpackedOrigin =
        originValue != undefined
          ? (_Vector2.unpack(2, originValue, _session) as Vector2)
          : undefined;
      const anchorValue = objectCson["116"];
      const unpackedAnchor = anchorValue != undefined ? Number(anchorValue) : undefined;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[1810200] as typeof SliderInputView)({
        value: unpackedValue,
        width: unpackedWidth,
        height: unpackedHeight,
        minWidth: unpackedMinWidth,
        minHeight: unpackedMinHeight,
        maxWidth: unpackedMaxWidth,
        maxHeight: unpackedMaxHeight,
        isVisible: unpackedIsVisible,
        opacity: unpackedOpacity,
        fill: unpackedFill,
        shadow: unpackedShadow,
        border: unpackedBorder,
        radius: unpackedRadius,
        position: unpackedPosition,
        offset: unpackedOffset,
        scale: unpackedScale,
        rotation: unpackedRotation,
        skew: unpackedSkew,
        origin: unpackedOrigin,
        anchor: unpackedAnchor,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1810200)] = new SliderInputViewCsonEncoder();

  class SplitViewCsonEncoder implements CsonObjectEncoder {
    packObject(object: SplitView): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1800400;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      if (object._position != null) {
        objectCson["110"] = object._position.pack(2);
      }
      if (object._offset != null) {
        objectCson["111"] = object._offset.pack(2);
      }
      if (object._scale != null) {
        objectCson["112"] = object._scale.pack(2);
      }
      if (object._rotation != null) {
        objectCson["113"] = object._rotation.pack(2);
      }
      if (object._skew != null) {
        objectCson["114"] = object._skew.pack(2);
      }
      if (object._origin != null) {
        objectCson["115"] = object._origin.pack(2);
      }
      if (object._anchor != null) {
        objectCson["116"] = object._anchor;
      }
      if (object._width != null) {
        objectCson["120"] = object._width.pack(2);
      }
      if (object._height != null) {
        objectCson["121"] = object._height.pack(2);
      }
      if (object._minWidth != null) {
        objectCson["122"] = object._minWidth.pack(2);
      }
      if (object._minHeight != null) {
        objectCson["123"] = object._minHeight.pack(2);
      }
      if (object._maxWidth != null) {
        objectCson["124"] = object._maxWidth.pack(2);
      }
      if (object._maxHeight != null) {
        objectCson["125"] = object._maxHeight.pack(2);
      }
      if (object._isVisible != null) {
        objectCson["130"] = object._isVisible;
      }
      if (object._opacity != null) {
        objectCson["131"] = object._opacity;
      }
      if (object._fill != null) {
        objectCson["140"] = object._fill.pack(2);
      }
      if (object._shadow != null) {
        objectCson["141"] = object._shadow.pack(2);
      }
      if (object._border != null) {
        objectCson["142"] = object._border.pack(2);
      }
      if (object._radius != null) {
        objectCson["143"] = object._radius.pack(2);
      }
      if (object._layout != null) {
        objectCson["150"] = object._layout;
      }
      if (object._direction != null) {
        objectCson["151"] = object._direction;
      }
      if (object._distribute != null) {
        objectCson["152"] = object._distribute;
      }
      if (object._align != null) {
        objectCson["153"] = object._align;
      }
      if (object._gap != null) {
        objectCson["154"] = object._gap.pack(2);
      }
      if (object._padding != null) {
        objectCson["155"] = object._padding.pack(2);
      }
      if (object._grid != null) {
        objectCson["156"] = object._grid.pack(2);
      }
      if (object._gridSpan != null) {
        objectCson["157"] = object._gridSpan.pack(2);
      }
      if (object._aspectRatio != null) {
        objectCson["158"] = object._aspectRatio;
      }
      if (object._isWrap != null) {
        objectCson["159"] = object._isWrap;
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): SplitView {
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
      const layoutValue = objectCson["150"];
      const unpackedLayout = layoutValue != undefined ? Number(layoutValue) : undefined;
      const directionValue = objectCson["151"];
      const unpackedDirection = directionValue != undefined ? Number(directionValue) : undefined;
      const distributeValue = objectCson["152"];
      const unpackedDistribute = distributeValue != undefined ? Number(distributeValue) : undefined;
      const alignValue = objectCson["153"];
      const unpackedAlign = alignValue != undefined ? Number(alignValue) : undefined;
      const gapValue = objectCson["154"];
      const unpackedGap =
        gapValue != undefined ? (_Axis2.unpack(2, gapValue, _session) as Axis2) : undefined;
      const paddingValue = objectCson["155"];
      const unpackedPadding =
        paddingValue != undefined
          ? (_Inset2.unpack(2, paddingValue, _session) as Inset2)
          : undefined;
      const gridValue = objectCson["156"];
      const unpackedGrid =
        gridValue != undefined ? (_Grid2.unpack(2, gridValue, _session) as Grid2) : undefined;
      const gridSpanValue = objectCson["157"];
      const unpackedGridSpan =
        gridSpanValue != undefined
          ? (_GridSpan2.unpack(2, gridSpanValue, _session) as GridSpan2)
          : undefined;
      const aspectRatioValue = objectCson["158"];
      const unpackedAspectRatio =
        aspectRatioValue != undefined ? Number(aspectRatioValue) : undefined;
      const isWrapValue = objectCson["159"];
      const unpackedIsWrap = isWrapValue != undefined ? Boolean(isWrapValue) : undefined;
      const widthValue = objectCson["120"];
      const unpackedWidth =
        widthValue != undefined ? (_Length.unpack(2, widthValue, _session) as Length) : undefined;
      const heightValue = objectCson["121"];
      const unpackedHeight =
        heightValue != undefined ? (_Length.unpack(2, heightValue, _session) as Length) : undefined;
      const minWidthValue = objectCson["122"];
      const unpackedMinWidth =
        minWidthValue != undefined
          ? (_Length.unpack(2, minWidthValue, _session) as Length)
          : undefined;
      const minHeightValue = objectCson["123"];
      const unpackedMinHeight =
        minHeightValue != undefined
          ? (_Length.unpack(2, minHeightValue, _session) as Length)
          : undefined;
      const maxWidthValue = objectCson["124"];
      const unpackedMaxWidth =
        maxWidthValue != undefined
          ? (_Length.unpack(2, maxWidthValue, _session) as Length)
          : undefined;
      const maxHeightValue = objectCson["125"];
      const unpackedMaxHeight =
        maxHeightValue != undefined
          ? (_Length.unpack(2, maxHeightValue, _session) as Length)
          : undefined;
      const isVisibleValue = objectCson["130"];
      const unpackedIsVisible = isVisibleValue != undefined ? Boolean(isVisibleValue) : undefined;
      const opacityValue = objectCson["131"];
      const unpackedOpacity = opacityValue != undefined ? Number(opacityValue) : undefined;
      const fillValue = objectCson["140"];
      const unpackedFill =
        fillValue != undefined ? (_Fill.unpack(2, fillValue, _session) as Fill) : undefined;
      const shadowValue = objectCson["141"];
      const unpackedShadow =
        shadowValue != undefined ? (_Shadow.unpack(2, shadowValue, _session) as Shadow) : undefined;
      const borderValue = objectCson["142"];
      const unpackedBorder =
        borderValue != undefined ? (_Border.unpack(2, borderValue, _session) as Border) : undefined;
      const radiusValue = objectCson["143"];
      const unpackedRadius =
        radiusValue != undefined
          ? (_Corner2.unpack(2, radiusValue, _session) as Corner2)
          : undefined;
      const positionValue = objectCson["110"];
      const unpackedPosition =
        positionValue != undefined
          ? (_Vector2.unpack(2, positionValue, _session) as Vector2)
          : undefined;
      const offsetValue = objectCson["111"];
      const unpackedOffset =
        offsetValue != undefined
          ? (_Offset2.unpack(2, offsetValue, _session) as Offset2)
          : undefined;
      const scaleValue = objectCson["112"];
      const unpackedScale =
        scaleValue != undefined ? (_Vector2.unpack(2, scaleValue, _session) as Vector2) : undefined;
      const rotationValue = objectCson["113"];
      const unpackedRotation =
        rotationValue != undefined
          ? (_Vector2.unpack(2, rotationValue, _session) as Vector2)
          : undefined;
      const skewValue = objectCson["114"];
      const unpackedSkew =
        skewValue != undefined ? (_Vector2.unpack(2, skewValue, _session) as Vector2) : undefined;
      const originValue = objectCson["115"];
      const unpackedOrigin =
        originValue != undefined
          ? (_Vector2.unpack(2, originValue, _session) as Vector2)
          : undefined;
      const anchorValue = objectCson["116"];
      const unpackedAnchor = anchorValue != undefined ? Number(anchorValue) : undefined;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[1800400] as typeof SplitView)({
        layout: unpackedLayout,
        direction: unpackedDirection,
        distribute: unpackedDistribute,
        align: unpackedAlign,
        gap: unpackedGap,
        padding: unpackedPadding,
        grid: unpackedGrid,
        gridSpan: unpackedGridSpan,
        aspectRatio: unpackedAspectRatio,
        isWrap: unpackedIsWrap,
        width: unpackedWidth,
        height: unpackedHeight,
        minWidth: unpackedMinWidth,
        minHeight: unpackedMinHeight,
        maxWidth: unpackedMaxWidth,
        maxHeight: unpackedMaxHeight,
        isVisible: unpackedIsVisible,
        opacity: unpackedOpacity,
        fill: unpackedFill,
        shadow: unpackedShadow,
        border: unpackedBorder,
        radius: unpackedRadius,
        position: unpackedPosition,
        offset: unpackedOffset,
        scale: unpackedScale,
        rotation: unpackedRotation,
        skew: unpackedSkew,
        origin: unpackedOrigin,
        anchor: unpackedAnchor,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1800400)] = new SplitViewCsonEncoder();

  class TextViewCsonEncoder implements CsonObjectEncoder {
    packObject(object: TextView): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1805100;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      if (object._position != null) {
        objectCson["110"] = object._position.pack(2);
      }
      if (object._offset != null) {
        objectCson["111"] = object._offset.pack(2);
      }
      if (object._scale != null) {
        objectCson["112"] = object._scale.pack(2);
      }
      if (object._rotation != null) {
        objectCson["113"] = object._rotation.pack(2);
      }
      if (object._skew != null) {
        objectCson["114"] = object._skew.pack(2);
      }
      if (object._origin != null) {
        objectCson["115"] = object._origin.pack(2);
      }
      if (object._anchor != null) {
        objectCson["116"] = object._anchor;
      }
      if (object._width != null) {
        objectCson["120"] = object._width.pack(2);
      }
      if (object._height != null) {
        objectCson["121"] = object._height.pack(2);
      }
      if (object._minWidth != null) {
        objectCson["122"] = object._minWidth.pack(2);
      }
      if (object._minHeight != null) {
        objectCson["123"] = object._minHeight.pack(2);
      }
      if (object._maxWidth != null) {
        objectCson["124"] = object._maxWidth.pack(2);
      }
      if (object._maxHeight != null) {
        objectCson["125"] = object._maxHeight.pack(2);
      }
      if (object._isVisible != null) {
        objectCson["130"] = object._isVisible;
      }
      if (object._opacity != null) {
        objectCson["131"] = object._opacity;
      }
      if (object._fill != null) {
        objectCson["140"] = object._fill.pack(2);
      }
      if (object._shadow != null) {
        objectCson["141"] = object._shadow.pack(2);
      }
      if (object._border != null) {
        objectCson["142"] = object._border.pack(2);
      }
      if (object._radius != null) {
        objectCson["143"] = object._radius.pack(2);
      }
      if (object._font != null) {
        objectCson["201"] = object._font.pack(2);
      }
      if (object._color != null) {
        objectCson["202"] = object._color.pack(2);
      }
      if (object._text != null) {
        objectCson["250"] = object._text.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): TextView {
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
      const textValue = objectCson["250"];
      const unpackedText =
        textValue != undefined ? (_Text.unpack(2, textValue, _session) as Text) : undefined;
      const fontValue = objectCson["201"];
      const unpackedFont =
        fontValue != undefined ? (_Font.unpack(2, fontValue, _session) as Font) : undefined;
      const colorValue = objectCson["202"];
      const unpackedColor =
        colorValue != undefined ? (_Fill.unpack(2, colorValue, _session) as Fill) : undefined;
      const widthValue = objectCson["120"];
      const unpackedWidth =
        widthValue != undefined ? (_Length.unpack(2, widthValue, _session) as Length) : undefined;
      const heightValue = objectCson["121"];
      const unpackedHeight =
        heightValue != undefined ? (_Length.unpack(2, heightValue, _session) as Length) : undefined;
      const minWidthValue = objectCson["122"];
      const unpackedMinWidth =
        minWidthValue != undefined
          ? (_Length.unpack(2, minWidthValue, _session) as Length)
          : undefined;
      const minHeightValue = objectCson["123"];
      const unpackedMinHeight =
        minHeightValue != undefined
          ? (_Length.unpack(2, minHeightValue, _session) as Length)
          : undefined;
      const maxWidthValue = objectCson["124"];
      const unpackedMaxWidth =
        maxWidthValue != undefined
          ? (_Length.unpack(2, maxWidthValue, _session) as Length)
          : undefined;
      const maxHeightValue = objectCson["125"];
      const unpackedMaxHeight =
        maxHeightValue != undefined
          ? (_Length.unpack(2, maxHeightValue, _session) as Length)
          : undefined;
      const isVisibleValue = objectCson["130"];
      const unpackedIsVisible = isVisibleValue != undefined ? Boolean(isVisibleValue) : undefined;
      const opacityValue = objectCson["131"];
      const unpackedOpacity = opacityValue != undefined ? Number(opacityValue) : undefined;
      const fillValue = objectCson["140"];
      const unpackedFill =
        fillValue != undefined ? (_Fill.unpack(2, fillValue, _session) as Fill) : undefined;
      const shadowValue = objectCson["141"];
      const unpackedShadow =
        shadowValue != undefined ? (_Shadow.unpack(2, shadowValue, _session) as Shadow) : undefined;
      const borderValue = objectCson["142"];
      const unpackedBorder =
        borderValue != undefined ? (_Border.unpack(2, borderValue, _session) as Border) : undefined;
      const radiusValue = objectCson["143"];
      const unpackedRadius =
        radiusValue != undefined
          ? (_Corner2.unpack(2, radiusValue, _session) as Corner2)
          : undefined;
      const positionValue = objectCson["110"];
      const unpackedPosition =
        positionValue != undefined
          ? (_Vector2.unpack(2, positionValue, _session) as Vector2)
          : undefined;
      const offsetValue = objectCson["111"];
      const unpackedOffset =
        offsetValue != undefined
          ? (_Offset2.unpack(2, offsetValue, _session) as Offset2)
          : undefined;
      const scaleValue = objectCson["112"];
      const unpackedScale =
        scaleValue != undefined ? (_Vector2.unpack(2, scaleValue, _session) as Vector2) : undefined;
      const rotationValue = objectCson["113"];
      const unpackedRotation =
        rotationValue != undefined
          ? (_Vector2.unpack(2, rotationValue, _session) as Vector2)
          : undefined;
      const skewValue = objectCson["114"];
      const unpackedSkew =
        skewValue != undefined ? (_Vector2.unpack(2, skewValue, _session) as Vector2) : undefined;
      const originValue = objectCson["115"];
      const unpackedOrigin =
        originValue != undefined
          ? (_Vector2.unpack(2, originValue, _session) as Vector2)
          : undefined;
      const anchorValue = objectCson["116"];
      const unpackedAnchor = anchorValue != undefined ? Number(anchorValue) : undefined;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[1805100] as typeof TextView)({
        text: unpackedText,
        font: unpackedFont,
        color: unpackedColor,
        width: unpackedWidth,
        height: unpackedHeight,
        minWidth: unpackedMinWidth,
        minHeight: unpackedMinHeight,
        maxWidth: unpackedMaxWidth,
        maxHeight: unpackedMaxHeight,
        isVisible: unpackedIsVisible,
        opacity: unpackedOpacity,
        fill: unpackedFill,
        shadow: unpackedShadow,
        border: unpackedBorder,
        radius: unpackedRadius,
        position: unpackedPosition,
        offset: unpackedOffset,
        scale: unpackedScale,
        rotation: unpackedRotation,
        skew: unpackedSkew,
        origin: unpackedOrigin,
        anchor: unpackedAnchor,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1805100)] = new TextViewCsonEncoder();

  class SceneCsonEncoder implements CsonObjectEncoder {
    packObject(object: Scene): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1700200;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      if (object._icon != null) {
        objectCson["102"] = object._icon.pack(2);
      }
      if (object._rootViewPtr != null) {
        objectCson["200"] = object._rootViewPtr.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Scene {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
      const rootViewPtrValue = objectCson["200"];
      const unpackedRootViewPtr =
        rootViewPtrValue != undefined
          ? (_NodeReference.unpack(2, rootViewPtrValue, _session) as NodeReference)
          : undefined;
      const iconValue = objectCson["102"];
      const unpackedIcon =
        iconValue != undefined ? (_Icon.unpack(2, iconValue, _session) as Icon) : undefined;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[1700200] as typeof Scene)({
        rootView: unpackedRootViewPtr,
        icon: unpackedIcon,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1700200)] = new SceneCsonEncoder();

  class StageCsonEncoder implements CsonObjectEncoder {
    packObject(object: Stage): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1700000;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Stage {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[1700000] as typeof Stage)({
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1700000)] = new StageCsonEncoder();

  class FollowCsonEncoder implements CsonObjectEncoder {
    packObject(object: Follow): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1400200;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Follow {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[1400200] as typeof Follow)({
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1400200)] = new FollowCsonEncoder();

  class FollowEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: FollowEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1400201;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      objectCson["101"] = object.nodePtr.pack(2);
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): FollowEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[1400201] as typeof FollowEvent)({
        node: _NodeReference.unpack(2, objectCson["101"], _session) as NodeReference,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1400201)] = new FollowEventCsonEncoder();

  class FollowAddedEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: FollowAddedEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1400202;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      objectCson["101"] = object.nodePtr.pack(2);
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): FollowAddedEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[1400202] as typeof FollowAddedEvent)({
        node: _NodeReference.unpack(2, objectCson["101"], _session) as NodeReference,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1400202)] = new FollowAddedEventCsonEncoder();

  class FollowRemovedEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: FollowRemovedEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1400203;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      objectCson["101"] = object.nodePtr.pack(2);
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): FollowRemovedEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[1400203] as typeof FollowRemovedEvent)({
        node: _NodeReference.unpack(2, objectCson["101"], _session) as NodeReference,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1400203)] = new FollowRemovedEventCsonEncoder();

  class NotificationSentEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: NotificationSentEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1400502;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      objectCson["101"] = object.nodePtr.pack(2);
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): NotificationSentEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[1400502] as typeof NotificationSentEvent)({
        node: _NodeReference.unpack(2, objectCson["101"], _session) as NodeReference,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1400502)] = new NotificationSentEventCsonEncoder();

  class NotificationRescindedEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: NotificationRescindedEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1400503;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      objectCson["101"] = object.nodePtr.pack(2);
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): NotificationRescindedEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[1400503] as typeof NotificationRescindedEvent)({
        node: _NodeReference.unpack(2, objectCson["101"], _session) as NodeReference,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1400503)] = new NotificationRescindedEventCsonEncoder();

  class NotificationReadEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: NotificationReadEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1400504;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      objectCson["101"] = object.nodePtr.pack(2);
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): NotificationReadEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[1400504] as typeof NotificationReadEvent)({
        node: _NodeReference.unpack(2, objectCson["101"], _session) as NodeReference,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1400504)] = new NotificationReadEventCsonEncoder();

  class NotificationDismissedEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: NotificationDismissedEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1400505;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      objectCson["101"] = object.nodePtr.pack(2);
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): NotificationDismissedEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[1400505] as typeof NotificationDismissedEvent)({
        node: _NodeReference.unpack(2, objectCson["101"], _session) as NodeReference,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1400505)] = new NotificationDismissedEventCsonEncoder();

  class NotificationExpiredEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: NotificationExpiredEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1400506;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      objectCson["101"] = object.nodePtr.pack(2);
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): NotificationExpiredEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[1400506] as typeof NotificationExpiredEvent)({
        node: _NodeReference.unpack(2, objectCson["101"], _session) as NodeReference,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1400506)] = new NotificationExpiredEventCsonEncoder();

  class NotificationCsonEncoder implements CsonObjectEncoder {
    packObject(object: Notification): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1400500;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      objectCson["101"] = object._title;
      objectCson["110"] = object._status;
      if (object._text != null) {
        objectCson["120"] = object._text.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Notification {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Text = STRUCT_CLASS_BY_TYPE[400020] as typeof Text;
      const textValue = objectCson["120"];
      const unpackedText =
        textValue != undefined ? (_Text.unpack(2, textValue, _session) as Text) : undefined;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[1400500] as typeof Notification)({
        title: objectCson["101"],
        status: Number(objectCson["110"]),
        text: unpackedText,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1400500)] = new NotificationCsonEncoder();

  class ReactionCsonEncoder implements CsonObjectEncoder {
    packObject(object: Reaction): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1400000;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      objectCson["101"] = object._content;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Reaction {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[1400000] as typeof Reaction)({
        content: objectCson["101"],
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1400000)] = new ReactionCsonEncoder();

  class ReactionEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: ReactionEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1400001;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      objectCson["101"] = object.nodePtr.pack(2);
      objectCson["102"] = object.content;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): ReactionEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[1400001] as typeof ReactionEvent)({
        node: _NodeReference.unpack(2, objectCson["101"], _session) as NodeReference,
        content: objectCson["102"],
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1400001)] = new ReactionEventCsonEncoder();

  class ReactionAddedEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: ReactionAddedEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1400002;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      objectCson["101"] = object.nodePtr.pack(2);
      objectCson["102"] = object.content;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): ReactionAddedEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[1400002] as typeof ReactionAddedEvent)({
        node: _NodeReference.unpack(2, objectCson["101"], _session) as NodeReference,
        content: objectCson["102"],
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1400002)] = new ReactionAddedEventCsonEncoder();

  class ReactionRemovedEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: ReactionRemovedEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1400003;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      objectCson["101"] = object.nodePtr.pack(2);
      objectCson["102"] = object.content;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): ReactionRemovedEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[1400003] as typeof ReactionRemovedEvent)({
        node: _NodeReference.unpack(2, objectCson["101"], _session) as NodeReference,
        content: objectCson["102"],
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1400003)] = new ReactionRemovedEventCsonEncoder();

  class StarCsonEncoder implements CsonObjectEncoder {
    packObject(object: Star): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1400100;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Star {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[1400100] as typeof Star)({
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1400100)] = new StarCsonEncoder();

  class StarEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: StarEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1400101;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      objectCson["101"] = object.nodePtr.pack(2);
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): StarEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[1400101] as typeof StarEvent)({
        node: _NodeReference.unpack(2, objectCson["101"], _session) as NodeReference,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1400101)] = new StarEventCsonEncoder();

  class StarAddedEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: StarAddedEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1400102;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      objectCson["101"] = object.nodePtr.pack(2);
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): StarAddedEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[1400102] as typeof StarAddedEvent)({
        node: _NodeReference.unpack(2, objectCson["101"], _session) as NodeReference,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1400102)] = new StarAddedEventCsonEncoder();

  class StarRemovedEventCsonEncoder implements CsonObjectEncoder {
    packObject(object: StarRemovedEvent): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1400103;
      objectCson["2"] = object.id;
      objectCson["5"] = object.spacePtr.pack(2);
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.causedByPtr != null) {
        objectCson["15"] = object.causedByPtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.clientPtr.pack(2);
      objectCson["24"] = object.clientNonce;
      objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
      objectCson["26"] = object.clientEpoch;
      objectCson["30"] = object.status;
      objectCson["101"] = object.nodePtr.pack(2);
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): StarRemovedEvent {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const causedByPtrValue = objectCson["15"];
      const unpackedCausedByPtr =
        causedByPtrValue != undefined
          ? (_NodeReference.unpack(2, causedByPtrValue, _session) as NodeReference)
          : undefined;
      return new (NODE_CLASS_BY_TYPE[1400103] as typeof StarRemovedEvent)({
        node: _NodeReference.unpack(2, objectCson["101"], _session) as NodeReference,
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        causedBy: unpackedCausedByPtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        client: _NodeReference.unpack(2, objectCson["23"], _session) as NodeReference,
        clientNonce: objectCson["24"],
        clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
        clientEpoch: Number(objectCson["26"]),
        status: Number(objectCson["30"]),
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 1400103)] = new StarRemovedEventCsonEncoder();

  class FolderCsonEncoder implements CsonObjectEncoder {
    packObject(object: Folder): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 240000;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      objectCson["100"] = object._type;
      if (object._icon != null) {
        objectCson["102"] = object._icon.pack(2);
      }
      if (object._slug != null) {
        objectCson["103"] = object._slug;
      }
      if (object._mainScenePtr != null) {
        objectCson["110"] = object._mainScenePtr.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Folder {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const iconValue = objectCson["102"];
      const unpackedIcon =
        iconValue != undefined ? (_Icon.unpack(2, iconValue, _session) as Icon) : undefined;
      const slugValue = objectCson["103"];
      const unpackedSlug = slugValue != undefined ? slugValue : undefined;
      const mainScenePtrValue = objectCson["110"];
      const unpackedMainScenePtr =
        mainScenePtrValue != undefined
          ? (_NodeReference.unpack(2, mainScenePtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[240000] as typeof Folder)({
        parent: unpackedParentPtr,
        type: Number(objectCson["100"]),
        icon: unpackedIcon,
        slug: unpackedSlug,
        mainScene: unpackedMainScenePtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 240000)] = new FolderCsonEncoder();

  class ClientCsonEncoder implements CsonObjectEncoder {
    packObject(object: Client): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 121300;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      if (object._browserVersion != null) {
        objectCson["44"] = object._browserVersion;
      }
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      objectCson["100"] = object._type;
      if (object._machinePtr != null) {
        objectCson["110"] = object._machinePtr.pack(2);
      }
      if (object._userPtr != null) {
        objectCson["111"] = object._userPtr.pack(2);
      }
      if (object._accessToken != null) {
        objectCson["120"] = object._accessToken;
      }
      if (object._seenAt != null) {
        objectCson["121"] = object._seenAt.toString({ timeZoneName: "never" });
      }
      if (object._loggedInAt != null) {
        objectCson["122"] = object._loggedInAt.toString({ timeZoneName: "never" });
      }
      if (object._deviceType != null) {
        objectCson["130"] = object._deviceType;
      }
      if (object._deviceName != null) {
        objectCson["131"] = object._deviceName;
      }
      if (object._operatingSystem != null) {
        objectCson["132"] = object._operatingSystem;
      }
      if (object._browserName != null) {
        objectCson["133"] = object._browserName;
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Client {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const machinePtrValue = objectCson["110"];
      const unpackedMachinePtr =
        machinePtrValue != undefined
          ? (_NodeReference.unpack(2, machinePtrValue, _session) as NodeReference)
          : undefined;
      const userPtrValue = objectCson["111"];
      const unpackedUserPtr =
        userPtrValue != undefined
          ? (_NodeReference.unpack(2, userPtrValue, _session) as NodeReference)
          : undefined;
      const accessTokenValue = objectCson["120"];
      const unpackedAccessToken = accessTokenValue != undefined ? accessTokenValue : undefined;
      const seenAtValue = objectCson["121"];
      const unpackedSeenAt =
        seenAtValue != undefined
          ? Temporal.Instant.from(seenAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const loggedInAtValue = objectCson["122"];
      const unpackedLoggedInAt =
        loggedInAtValue != undefined
          ? Temporal.Instant.from(loggedInAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const deviceTypeValue = objectCson["130"];
      const unpackedDeviceType = deviceTypeValue != undefined ? deviceTypeValue : undefined;
      const deviceNameValue = objectCson["131"];
      const unpackedDeviceName = deviceNameValue != undefined ? deviceNameValue : undefined;
      const operatingSystemValue = objectCson["132"];
      const unpackedOperatingSystem =
        operatingSystemValue != undefined ? operatingSystemValue : undefined;
      const browserNameValue = objectCson["133"];
      const unpackedBrowserName = browserNameValue != undefined ? browserNameValue : undefined;
      const browserVersionValue = objectCson["44"];
      const unpackedBrowserVersion =
        browserVersionValue != undefined ? browserVersionValue : undefined;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[121300] as typeof Client)({
        type: Number(objectCson["100"]),
        machine: unpackedMachinePtr,
        user: unpackedUserPtr,
        accessToken: unpackedAccessToken,
        seenAt: unpackedSeenAt,
        loggedInAt: unpackedLoggedInAt,
        deviceType: unpackedDeviceType,
        deviceName: unpackedDeviceName,
        operatingSystem: unpackedOperatingSystem,
        browserName: unpackedBrowserName,
        browserVersion: unpackedBrowserVersion,
        parent: unpackedParentPtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 121300)] = new ClientCsonEncoder();

  class HandleCsonEncoder implements CsonObjectEncoder {
    packObject(object: Handle): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 100200;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      objectCson["101"] = object._slug;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Handle {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[100200] as typeof Handle)({
        parent: unpackedParentPtr,
        slug: objectCson["101"],
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 100200)] = new HandleCsonEncoder();

  class OrganizationCsonEncoder implements CsonObjectEncoder {
    packObject(object: Organization): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 122000;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      objectCson["101"] = object._slug;
      objectCson["102"] = object._status;
      if (object._handlePtr != null) {
        objectCson["111"] = object._handlePtr.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Organization {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const handlePtrValue = objectCson["111"];
      const unpackedHandlePtr =
        handlePtrValue != undefined
          ? (_NodeReference.unpack(2, handlePtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[122000] as typeof Organization)({
        parent: unpackedParentPtr,
        slug: objectCson["101"],
        status: Number(objectCson["102"]),
        handle: unpackedHandlePtr,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 122000)] = new OrganizationCsonEncoder();

  class TeamCsonEncoder implements CsonObjectEncoder {
    packObject(object: Team): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 122100;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      objectCson["102"] = object._slug;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Team {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[122100] as typeof Team)({
        parent: unpackedParentPtr,
        slug: objectCson["102"],
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 122100)] = new TeamCsonEncoder();

  class UserCsonEncoder implements CsonObjectEncoder {
    packObject(object: User): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 121000;
      objectCson["2"] = object.id;
      if (object.parentPtr != null) {
        objectCson["3"] = object.parentPtr.pack(2);
      }
      objectCson["5"] = object.spacePtr.pack(2);
      objectCson["10"] = object.materialization;
      if (object.definitionPtr != null) {
        objectCson["11"] = object.definitionPtr.pack(2);
      }
      objectCson["12"] = object.branchPtr.pack(2);
      objectCson["13"] = object.snapshotPtr.pack(2);
      if (object.precededByPtr != null) {
        objectCson["14"] = object.precededByPtr.pack(2);
      }
      if (object.instancePtr != null) {
        objectCson["15"] = object.instancePtr.pack(2);
      }
      objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
      objectCson["21"] = object.createdEpoch;
      objectCson["22"] = object.createdByPtr.pack(2);
      objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
      objectCson["24"] = object.updatedEpoch;
      objectCson["25"] = object.updatedByPtr.pack(2);
      if (object.deletedAt != null) {
        objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
      }
      if (object._ownedByPtr != null) {
        objectCson["30"] = object._ownedByPtr.pack(2);
      }
      objectCson["40"] = object._name;
      objectCson["41"] = object.orderKey;
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(key)] = value.pack(2);
      }
      objectCson["45"] = packedCustomValues;
      if (object._scriptPtr != null) {
        objectCson["46"] = object._scriptPtr.pack(2);
      }
      if (object.isExtensible != null) {
        objectCson["50"] = object.isExtensible;
      }
      if (object.sourcePtr != null) {
        objectCson["80"] = object.sourcePtr.pack(2);
      }
      if (object._key != null) {
        objectCson["85"] = object._key;
      }
      objectCson["102"] = object._slug;
      objectCson["110"] = object._status;
      if (object._lastLoggedInAt != null) {
        objectCson["111"] = object._lastLoggedInAt.toString({ timeZoneName: "never" });
      }
      objectCson["112"] = object._isStaff;
      if (object._handlePtr != null) {
        objectCson["121"] = object._handlePtr.pack(2);
      }
      if (object._email != null) {
        objectCson["130"] = object._email;
      }
      if (object._passwordSalt != null) {
        objectCson["131"] = base64Encode(object._passwordSalt);
      }
      if (object._passwordHash != null) {
        objectCson["132"] = base64Encode(object._passwordHash);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): User {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const parentPtrValue = objectCson["3"];
      const unpackedParentPtr =
        parentPtrValue != undefined
          ? (_NodeReference.unpack(2, parentPtrValue, _session) as NodeReference)
          : undefined;
      const lastLoggedInAtValue = objectCson["111"];
      const unpackedLastLoggedInAt =
        lastLoggedInAtValue != undefined
          ? Temporal.Instant.from(lastLoggedInAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const handlePtrValue = objectCson["121"];
      const unpackedHandlePtr =
        handlePtrValue != undefined
          ? (_NodeReference.unpack(2, handlePtrValue, _session) as NodeReference)
          : undefined;
      const emailValue = objectCson["130"];
      const unpackedEmail = emailValue != undefined ? emailValue : undefined;
      const passwordSaltValue = objectCson["131"];
      const unpackedPasswordSalt =
        passwordSaltValue != undefined ? base64Decode(passwordSaltValue) : undefined;
      const passwordHashValue = objectCson["132"];
      const unpackedPasswordHash =
        passwordHashValue != undefined ? base64Decode(passwordHashValue) : undefined;
      const definitionPtrValue = objectCson["11"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      const precededByPtrValue = objectCson["14"];
      const unpackedPrecededByPtr =
        precededByPtrValue != undefined
          ? (_NodeReference.unpack(2, precededByPtrValue, _session) as NodeReference)
          : undefined;
      const instancePtrValue = objectCson["15"];
      const unpackedInstancePtr =
        instancePtrValue != undefined
          ? (_NodeReference.unpack(2, instancePtrValue, _session) as NodeReference)
          : undefined;
      const deletedAtValue = objectCson["26"];
      const unpackedDeletedAt =
        deletedAtValue != undefined
          ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
          : undefined;
      const ownedByPtrValue = objectCson["30"];
      const unpackedOwnedByPtr =
        ownedByPtrValue != undefined
          ? (_NodeReference.unpack(2, ownedByPtrValue, _session) as NodeReference)
          : undefined;
      const unpackedCustomValues = {} as any;
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[key] = _Value.unpack(2, value as any, _session) as Value;
      }
      const scriptPtrValue = objectCson["46"];
      const unpackedScriptPtr =
        scriptPtrValue != undefined
          ? (_NodeReference.unpack(2, scriptPtrValue, _session) as NodeReference)
          : undefined;
      const isExtensibleValue = objectCson["50"];
      const unpackedIsExtensible =
        isExtensibleValue != undefined ? Boolean(isExtensibleValue) : undefined;
      const sourcePtrValue = objectCson["80"];
      const unpackedSourcePtr =
        sourcePtrValue != undefined
          ? (_NodeReference.unpack(2, sourcePtrValue, _session) as NodeReference)
          : undefined;
      const keyValue = objectCson["85"];
      const unpackedKey = keyValue != undefined ? keyValue : undefined;
      return new (NODE_CLASS_BY_TYPE[121000] as typeof User)({
        parent: unpackedParentPtr,
        slug: objectCson["102"],
        status: Number(objectCson["110"]),
        lastLoggedInAt: unpackedLastLoggedInAt,
        isStaff: Boolean(objectCson["112"]),
        handle: unpackedHandlePtr,
        email: unpackedEmail,
        passwordSalt: unpackedPasswordSalt,
        passwordHash: unpackedPasswordHash,
        materialization: Number(objectCson["10"]),
        definition: unpackedDefinitionPtr,
        branch: _NodeReference.unpack(2, objectCson["12"], _session) as NodeReference,
        snapshot: _NodeReference.unpack(2, objectCson["13"], _session) as NodeReference,
        precededBy: unpackedPrecededByPtr,
        instance: unpackedInstancePtr,
        createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
        createdEpoch: Number(objectCson["21"]),
        createdBy: _NodeReference.unpack(2, objectCson["22"], _session) as NodeReference,
        updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
        updatedEpoch: Number(objectCson["24"]),
        updatedBy: _NodeReference.unpack(2, objectCson["25"], _session) as NodeReference,
        deletedAt: unpackedDeletedAt,
        ownedBy: unpackedOwnedByPtr,
        name: objectCson["40"],
        orderKey: objectCson["41"],
        customValues: unpackedCustomValues,
        script: unpackedScriptPtr,
        isExtensible: unpackedIsExtensible,
        source: unpackedSourcePtr,
        key: unpackedKey,
        id: objectCson["2"],
        space: _NodeReference.unpack(2, objectCson["5"], _session) as NodeReference,
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(1, 121000)] = new UserCsonEncoder();

  class NodeDefinitionReferenceCsonEncoder implements CsonObjectEncoder {
    packObject(object: NodeDefinitionReference): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 13;
      objectCson["100"] = object.type;
      objectCson["101"] = object.nodeType;
      if (object.definitionPtr != null) {
        objectCson["105"] = object.definitionPtr.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): NodeDefinitionReference {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const definitionPtrValue = objectCson["105"];
      const unpackedDefinitionPtr =
        definitionPtrValue != undefined
          ? (_NodeReference.unpack(2, definitionPtrValue, _session) as NodeReference)
          : undefined;
      return new (STRUCT_CLASS_BY_TYPE[13] as typeof NodeDefinitionReference)({
        type: Number(objectCson["100"]),
        nodeType: Number(objectCson["101"]),
        definition: unpackedDefinitionPtr,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 13)] = new NodeDefinitionReferenceCsonEncoder();

  class ObjectDefinitionReferenceCsonEncoder implements CsonObjectEncoder {
    packObject(object: ObjectDefinitionReference): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 11;
      objectCson["100"] = object.type;
      if (object.nodeType != null) {
        objectCson["101"] = object.nodeType;
      }
      if (object.traitType != null) {
        objectCson["102"] = object.traitType;
      }
      if (object.structType != null) {
        objectCson["103"] = object.structType;
      }
      if (object.customDefinitionPtr != null) {
        objectCson["105"] = object.customDefinitionPtr.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): ObjectDefinitionReference {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const nodeTypeValue = objectCson["101"];
      const unpackedNodeType = nodeTypeValue != undefined ? Number(nodeTypeValue) : undefined;
      const traitTypeValue = objectCson["102"];
      const unpackedTraitType = traitTypeValue != undefined ? Number(traitTypeValue) : undefined;
      const structTypeValue = objectCson["103"];
      const unpackedStructType = structTypeValue != undefined ? Number(structTypeValue) : undefined;
      const customDefinitionPtrValue = objectCson["105"];
      const unpackedCustomDefinitionPtr =
        customDefinitionPtrValue != undefined
          ? (_NodeReference.unpack(2, customDefinitionPtrValue, _session) as NodeReference)
          : undefined;
      return new (STRUCT_CLASS_BY_TYPE[11] as typeof ObjectDefinitionReference)({
        type: Number(objectCson["100"]),
        nodeType: unpackedNodeType,
        traitType: unpackedTraitType,
        structType: unpackedStructType,
        customDefinition: unpackedCustomDefinitionPtr,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 11)] = new ObjectDefinitionReferenceCsonEncoder();

  class StructDefinitionReferenceCsonEncoder implements CsonObjectEncoder {
    packObject(object: StructDefinitionReference): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 16;
      objectCson["100"] = object.type;
      if (object.structType != null) {
        objectCson["101"] = object.structType;
      }
      objectCson["105"] = object.definitionPtr.pack(2);
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): StructDefinitionReference {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const structTypeValue = objectCson["101"];
      const unpackedStructType = structTypeValue != undefined ? Number(structTypeValue) : undefined;
      return new (STRUCT_CLASS_BY_TYPE[16] as typeof StructDefinitionReference)({
        type: Number(objectCson["100"]),
        structType: unpackedStructType,
        definition: _NodeReference.unpack(2, objectCson["105"], _session) as NodeReference,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 16)] = new StructDefinitionReferenceCsonEncoder();

  class PropertyReferenceCsonEncoder implements CsonObjectEncoder {
    packObject(object: PropertyReference): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1001;
      objectCson["100"] = object.type;
      if (object.nodeType != null) {
        objectCson["101"] = object.nodeType;
      }
      if (object.traitType != null) {
        objectCson["102"] = object.traitType;
      }
      if (object.structType != null) {
        objectCson["103"] = object.structType;
      }
      if (object.id != null) {
        objectCson["105"] = object.id;
      }
      if (object.customPropertyPtr != null) {
        objectCson["106"] = object.customPropertyPtr.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): PropertyReference {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const nodeTypeValue = objectCson["101"];
      const unpackedNodeType = nodeTypeValue != undefined ? Number(nodeTypeValue) : undefined;
      const traitTypeValue = objectCson["102"];
      const unpackedTraitType = traitTypeValue != undefined ? Number(traitTypeValue) : undefined;
      const structTypeValue = objectCson["103"];
      const unpackedStructType = structTypeValue != undefined ? Number(structTypeValue) : undefined;
      const idValue = objectCson["105"];
      const unpackedId = idValue != undefined ? Number(idValue) : undefined;
      const customPropertyPtrValue = objectCson["106"];
      const unpackedCustomPropertyPtr =
        customPropertyPtrValue != undefined
          ? (_NodeReference.unpack(2, customPropertyPtrValue, _session) as NodeReference)
          : undefined;
      return new (STRUCT_CLASS_BY_TYPE[1001] as typeof PropertyReference)({
        type: Number(objectCson["100"]),
        nodeType: unpackedNodeType,
        traitType: unpackedTraitType,
        structType: unpackedStructType,
        id: unpackedId,
        customProperty: unpackedCustomPropertyPtr,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 1001)] = new PropertyReferenceCsonEncoder();

  class NodeReferenceCsonEncoder implements CsonObjectEncoder {
    packObject(object: NodeReference): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1000;
      objectCson["100"] = object.type;
      objectCson["101"] = object.id;
      objectCson["102"] = object.spaceId;
      if (object.definitionId != null) {
        objectCson["103"] = object.definitionId;
      }
      objectCson["104"] = object.branchId;
      objectCson["105"] = object.snapshotId;
      if (object.storeKey != null) {
        objectCson["110"] = object.storeKey;
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): NodeReference {
      const definitionIdValue = objectCson["103"];
      const unpackedDefinitionId = definitionIdValue != undefined ? definitionIdValue : undefined;
      const storeKeyValue = objectCson["110"];
      const unpackedStoreKey = storeKeyValue != undefined ? Number(storeKeyValue) : undefined;
      return new (STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference)({
        type: Number(objectCson["100"]),
        id: objectCson["101"],
        spaceId: objectCson["102"],
        definitionId: unpackedDefinitionId,
        branchId: objectCson["104"],
        snapshotId: objectCson["105"],
        storeKey: unpackedStoreKey,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 1000)] = new NodeReferenceCsonEncoder();

  class StringConstraintCsonEncoder implements CsonObjectEncoder {
    packObject(object: StringConstraint): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 111;
      if (object.format != null) {
        objectCson["40"] = object.format;
      }
      if (object.regex != null) {
        objectCson["41"] = object.regex;
      }
      if (object.startsWith != null) {
        objectCson["42"] = object.startsWith;
      }
      if (object.endsWith != null) {
        objectCson["43"] = object.endsWith;
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): StringConstraint {
      const formatValue = objectCson["40"];
      const unpackedFormat = formatValue != undefined ? Number(formatValue) : undefined;
      const regexValue = objectCson["41"];
      const unpackedRegex = regexValue != undefined ? regexValue : undefined;
      const startsWithValue = objectCson["42"];
      const unpackedStartsWith = startsWithValue != undefined ? startsWithValue : undefined;
      const endsWithValue = objectCson["43"];
      const unpackedEndsWith = endsWithValue != undefined ? endsWithValue : undefined;
      return new (STRUCT_CLASS_BY_TYPE[111] as typeof StringConstraint)({
        format: unpackedFormat,
        regex: unpackedRegex,
        startsWith: unpackedStartsWith,
        endsWith: unpackedEndsWith,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 111)] = new StringConstraintCsonEncoder();

  class NumberConstraintCsonEncoder implements CsonObjectEncoder {
    packObject(object: NumberConstraint): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 110;
      if (object.format != null) {
        objectCson["40"] = object.format;
      }
      if (object.minValue != null) {
        objectCson["41"] = object.minValue;
      }
      if (object.maxValue != null) {
        objectCson["42"] = object.maxValue;
      }
      if (object.stepValue != null) {
        objectCson["43"] = object.stepValue;
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): NumberConstraint {
      const formatValue = objectCson["40"];
      const unpackedFormat = formatValue != undefined ? Number(formatValue) : undefined;
      const minValueValue = objectCson["41"];
      const unpackedMinValue = minValueValue != undefined ? Number(minValueValue) : undefined;
      const maxValueValue = objectCson["42"];
      const unpackedMaxValue = maxValueValue != undefined ? Number(maxValueValue) : undefined;
      const stepValueValue = objectCson["43"];
      const unpackedStepValue = stepValueValue != undefined ? Number(stepValueValue) : undefined;
      return new (STRUCT_CLASS_BY_TYPE[110] as typeof NumberConstraint)({
        format: unpackedFormat,
        minValue: unpackedMinValue,
        maxValue: unpackedMaxValue,
        stepValue: unpackedStepValue,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 110)] = new NumberConstraintCsonEncoder();

  class CollectionConstraintCsonEncoder implements CsonObjectEncoder {
    packObject(object: CollectionConstraint): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 112;
      if (object.minLength != null) {
        objectCson["41"] = object.minLength;
      }
      if (object.maxLength != null) {
        objectCson["42"] = object.maxLength;
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): CollectionConstraint {
      const minLengthValue = objectCson["41"];
      const unpackedMinLength = minLengthValue != undefined ? Number(minLengthValue) : undefined;
      const maxLengthValue = objectCson["42"];
      const unpackedMaxLength = maxLengthValue != undefined ? Number(maxLengthValue) : undefined;
      return new (STRUCT_CLASS_BY_TYPE[112] as typeof CollectionConstraint)({
        minLength: unpackedMinLength,
        maxLength: unpackedMaxLength,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 112)] = new CollectionConstraintCsonEncoder();

  class TypeCsonEncoder implements CsonObjectEncoder {
    packObject(object: Type): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 101;
      objectCson["110"] = object.cardinality;
      if (object.keyType != null) {
        objectCson["111"] = object.keyType.pack(2);
      }
      if (object.valueType != null) {
        objectCson["112"] = object.valueType.pack(2);
      }
      if (object.elementTypes != null) {
        const packedElementTypes: any[] = [];
        for (const item of object.elementTypes) {
          packedElementTypes.push(item.pack(2));
        }
        objectCson["113"] = packedElementTypes;
      }
      if (object.scalarType != null) {
        objectCson["120"] = object.scalarType;
      }
      if (object.primitiveType != null) {
        objectCson["121"] = object.primitiveType;
      }
      if (object.enumType != null) {
        objectCson["122"] = object.enumType;
      }
      if (object.nodeTypes != null) {
        const packedNodeTypes: any[] = [];
        for (const item of object.nodeTypes) {
          packedNodeTypes.push(item);
        }
        objectCson["123"] = packedNodeTypes;
      }
      if (object.structType != null) {
        objectCson["124"] = object.structType;
      }
      if (object.literalValue != null) {
        objectCson["125"] = object.literalValue.pack(2);
      }
      if (object.unionTypes != null) {
        const packedUnionTypes: any[] = [];
        for (const item of object.unionTypes) {
          packedUnionTypes.push(item.pack(2));
        }
        objectCson["126"] = packedUnionTypes;
      }
      objectCson["130"] = object.isRequired;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Type {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _Type = STRUCT_CLASS_BY_TYPE[101] as typeof Type;
      const keyTypeValue = objectCson["111"];
      const unpackedKeyType =
        keyTypeValue != undefined ? (_Type.unpack(2, keyTypeValue, _session) as Type) : undefined;
      const valueTypeValue = objectCson["112"];
      const unpackedValueType =
        valueTypeValue != undefined
          ? (_Type.unpack(2, valueTypeValue, _session) as Type)
          : undefined;
      let unpackedElementTypes: any[] | undefined;
      if (objectCson["113"] != undefined) {
        unpackedElementTypes = [];
        for (const item of objectCson["113"]) {
          unpackedElementTypes.push(_Type.unpack(2, item, _session) as Type);
        }
      } else {
        unpackedElementTypes = undefined;
      }
      const scalarTypeValue = objectCson["120"];
      const unpackedScalarType = scalarTypeValue != undefined ? Number(scalarTypeValue) : undefined;
      const primitiveTypeValue = objectCson["121"];
      const unpackedPrimitiveType =
        primitiveTypeValue != undefined ? Number(primitiveTypeValue) : undefined;
      const enumTypeValue = objectCson["122"];
      const unpackedEnumType = enumTypeValue != undefined ? Number(enumTypeValue) : undefined;
      let unpackedNodeTypes: any[] | undefined;
      if (objectCson["123"] != undefined) {
        unpackedNodeTypes = [];
        for (const item of objectCson["123"]) {
          unpackedNodeTypes.push(Number(item));
        }
      } else {
        unpackedNodeTypes = undefined;
      }
      const structTypeValue = objectCson["124"];
      const unpackedStructType = structTypeValue != undefined ? Number(structTypeValue) : undefined;
      const literalValueValue = objectCson["125"];
      const unpackedLiteralValue =
        literalValueValue != undefined
          ? (_Value.unpack(2, literalValueValue, _session) as Value)
          : undefined;
      let unpackedUnionTypes: any[] | undefined;
      if (objectCson["126"] != undefined) {
        unpackedUnionTypes = [];
        for (const item of objectCson["126"]) {
          unpackedUnionTypes.push(_Type.unpack(2, item, _session) as Type);
        }
      } else {
        unpackedUnionTypes = undefined;
      }
      return new (STRUCT_CLASS_BY_TYPE[101] as typeof Type)({
        cardinality: Number(objectCson["110"]),
        keyType: unpackedKeyType,
        valueType: unpackedValueType,
        elementTypes: unpackedElementTypes,
        scalarType: unpackedScalarType,
        primitiveType: unpackedPrimitiveType,
        enumType: unpackedEnumType,
        nodeTypes: unpackedNodeTypes,
        structType: unpackedStructType,
        literalValue: unpackedLiteralValue,
        unionTypes: unpackedUnionTypes,
        isRequired: Boolean(objectCson["130"]),
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 101)] = new TypeCsonEncoder();

  class CheckedTypeCsonEncoder implements CsonObjectEncoder {
    packObject(object: CheckedType): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 102;
      objectCson["110"] = object.cardinality;
      if (object.keyType != null) {
        objectCson["111"] = object.keyType.pack(2);
      }
      if (object.valueType != null) {
        objectCson["112"] = object.valueType.pack(2);
      }
      if (object.elementTypes != null) {
        const packedElementTypes: any[] = [];
        for (const item of object.elementTypes) {
          packedElementTypes.push(item.pack(2));
        }
        objectCson["113"] = packedElementTypes;
      }
      if (object.scalarType != null) {
        objectCson["120"] = object.scalarType;
      }
      if (object.primitiveType != null) {
        objectCson["121"] = object.primitiveType;
      }
      if (object.enumType != null) {
        objectCson["122"] = object.enumType;
      }
      if (object.nodeTypes != null) {
        const packedNodeTypes: any[] = [];
        for (const item of object.nodeTypes) {
          packedNodeTypes.push(item);
        }
        objectCson["123"] = packedNodeTypes;
      }
      if (object.structType != null) {
        objectCson["124"] = object.structType;
      }
      if (object.literalValue != null) {
        objectCson["125"] = object.literalValue.pack(2);
      }
      if (object.unionTypes != null) {
        const packedUnionTypes: any[] = [];
        for (const item of object.unionTypes) {
          packedUnionTypes.push(item.pack(2));
        }
        objectCson["126"] = packedUnionTypes;
      }
      objectCson["130"] = object.isRequired;
      if (object.defaultValue != null) {
        objectCson["150"] = object.defaultValue.pack(2);
      }
      if (object.defaultFactory != null) {
        objectCson["151"] = object.defaultFactory;
      }
      if (object.collectionConstraint != null) {
        objectCson["160"] = object.collectionConstraint.pack(2);
      }
      if (object.stringConstraint != null) {
        objectCson["161"] = object.stringConstraint.pack(2);
      }
      if (object.numberConstraint != null) {
        objectCson["162"] = object.numberConstraint.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): CheckedType {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _Type = STRUCT_CLASS_BY_TYPE[101] as typeof Type;
      const _NumberConstraint = STRUCT_CLASS_BY_TYPE[110] as typeof NumberConstraint;
      const _StringConstraint = STRUCT_CLASS_BY_TYPE[111] as typeof StringConstraint;
      const _CollectionConstraint = STRUCT_CLASS_BY_TYPE[112] as typeof CollectionConstraint;
      const defaultValueValue = objectCson["150"];
      const unpackedDefaultValue =
        defaultValueValue != undefined
          ? (_Value.unpack(2, defaultValueValue, _session) as Value)
          : undefined;
      const defaultFactoryValue = objectCson["151"];
      const unpackedDefaultFactory =
        defaultFactoryValue != undefined ? Number(defaultFactoryValue) : undefined;
      const collectionConstraintValue = objectCson["160"];
      const unpackedCollectionConstraint =
        collectionConstraintValue != undefined
          ? (_CollectionConstraint.unpack(
              2,
              collectionConstraintValue,
              _session,
            ) as CollectionConstraint)
          : undefined;
      const stringConstraintValue = objectCson["161"];
      const unpackedStringConstraint =
        stringConstraintValue != undefined
          ? (_StringConstraint.unpack(2, stringConstraintValue, _session) as StringConstraint)
          : undefined;
      const numberConstraintValue = objectCson["162"];
      const unpackedNumberConstraint =
        numberConstraintValue != undefined
          ? (_NumberConstraint.unpack(2, numberConstraintValue, _session) as NumberConstraint)
          : undefined;
      const keyTypeValue = objectCson["111"];
      const unpackedKeyType =
        keyTypeValue != undefined ? (_Type.unpack(2, keyTypeValue, _session) as Type) : undefined;
      const valueTypeValue = objectCson["112"];
      const unpackedValueType =
        valueTypeValue != undefined
          ? (_Type.unpack(2, valueTypeValue, _session) as Type)
          : undefined;
      let unpackedElementTypes: any[] | undefined;
      if (objectCson["113"] != undefined) {
        unpackedElementTypes = [];
        for (const item of objectCson["113"]) {
          unpackedElementTypes.push(_Type.unpack(2, item, _session) as Type);
        }
      } else {
        unpackedElementTypes = undefined;
      }
      const scalarTypeValue = objectCson["120"];
      const unpackedScalarType = scalarTypeValue != undefined ? Number(scalarTypeValue) : undefined;
      const primitiveTypeValue = objectCson["121"];
      const unpackedPrimitiveType =
        primitiveTypeValue != undefined ? Number(primitiveTypeValue) : undefined;
      const enumTypeValue = objectCson["122"];
      const unpackedEnumType = enumTypeValue != undefined ? Number(enumTypeValue) : undefined;
      let unpackedNodeTypes: any[] | undefined;
      if (objectCson["123"] != undefined) {
        unpackedNodeTypes = [];
        for (const item of objectCson["123"]) {
          unpackedNodeTypes.push(Number(item));
        }
      } else {
        unpackedNodeTypes = undefined;
      }
      const structTypeValue = objectCson["124"];
      const unpackedStructType = structTypeValue != undefined ? Number(structTypeValue) : undefined;
      const literalValueValue = objectCson["125"];
      const unpackedLiteralValue =
        literalValueValue != undefined
          ? (_Value.unpack(2, literalValueValue, _session) as Value)
          : undefined;
      let unpackedUnionTypes: any[] | undefined;
      if (objectCson["126"] != undefined) {
        unpackedUnionTypes = [];
        for (const item of objectCson["126"]) {
          unpackedUnionTypes.push(_Type.unpack(2, item, _session) as Type);
        }
      } else {
        unpackedUnionTypes = undefined;
      }
      return new (STRUCT_CLASS_BY_TYPE[102] as typeof CheckedType)({
        defaultValue: unpackedDefaultValue,
        defaultFactory: unpackedDefaultFactory,
        collectionConstraint: unpackedCollectionConstraint,
        stringConstraint: unpackedStringConstraint,
        numberConstraint: unpackedNumberConstraint,
        cardinality: Number(objectCson["110"]),
        keyType: unpackedKeyType,
        valueType: unpackedValueType,
        elementTypes: unpackedElementTypes,
        scalarType: unpackedScalarType,
        primitiveType: unpackedPrimitiveType,
        enumType: unpackedEnumType,
        nodeTypes: unpackedNodeTypes,
        structType: unpackedStructType,
        literalValue: unpackedLiteralValue,
        unionTypes: unpackedUnionTypes,
        isRequired: Boolean(objectCson["130"]),
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 102)] = new CheckedTypeCsonEncoder();

  class ValueCsonEncoder implements CsonObjectEncoder {
    packObject(object: Value): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 100;
      objectCson["100"] = object.type.pack(2);
      if (object.value != null) {
        objectCson["110"] = object.value;
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Value {
      const _Type = STRUCT_CLASS_BY_TYPE[101] as typeof Type;
      const valueValue = objectCson["110"];
      const unpackedValue = valueValue != undefined ? valueValue : undefined;
      return new (STRUCT_CLASS_BY_TYPE[100] as typeof Value)({
        type: _Type.unpack(2, objectCson["100"], _session) as Type,
        value: unpackedValue,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 100)] = new ValueCsonEncoder();

  class NodeDefinitionCsonEncoder implements CsonObjectEncoder {
    packObject(object: NodeDefinition): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 12;
      objectCson["2"] = object.id;
      objectCson["100"] = object.type;
      objectCson["101"] = object.name;
      if (object.icon != null) {
        objectCson["102"] = object.icon.pack(2);
      }
      if (object.description != null) {
        objectCson["103"] = object.description;
      }
      const packedTaggings: any[] = [];
      for (const item of object.taggings) {
        packedTaggings.push(item);
      }
      objectCson["109"] = packedTaggings;
      objectCson["110"] = object.isAbstract;
      objectCson["111"] = object.isExtensible;
      objectCson["112"] = object.isFinal;
      objectCson["113"] = object.isFrozen;
      const packedProperties: any[] = [];
      for (const item of object.properties) {
        packedProperties.push(item.pack(2));
      }
      objectCson["120"] = packedProperties;
      const packedIndexes: any[] = [];
      for (const item of object.indexes) {
        packedIndexes.push(item.pack(2));
      }
      objectCson["121"] = packedIndexes;
      const packedConstraints: any[] = [];
      for (const item of object.constraints) {
        packedConstraints.push(item.pack(2));
      }
      objectCson["122"] = packedConstraints;
      const packedPermissions: any[] = [];
      for (const item of object.permissions) {
        packedPermissions.push(item.pack(2));
      }
      objectCson["123"] = packedPermissions;
      const packedMethods: any[] = [];
      for (const item of object.methods) {
        packedMethods.push(item.pack(2));
      }
      objectCson["125"] = packedMethods;
      const packedActions: any[] = [];
      for (const item of object.actions) {
        packedActions.push(item.pack(2));
      }
      objectCson["126"] = packedActions;
      const packedConstants: any[] = [];
      for (const item of object.constants) {
        packedConstants.push(item.pack(2));
      }
      objectCson["128"] = packedConstants;
      if (object.baseType != null) {
        objectCson["130"] = object.baseType;
      }
      const packedExtendedBy: any[] = [];
      for (const item of object.extendedBy) {
        packedExtendedBy.push(item);
      }
      objectCson["131"] = packedExtendedBy;
      const packedInherits: any[] = [];
      for (const item of object.inherits) {
        packedInherits.push(item);
      }
      objectCson["132"] = packedInherits;
      const packedInheritedBy: any[] = [];
      for (const item of object.inheritedBy) {
        packedInheritedBy.push(item);
      }
      objectCson["133"] = packedInheritedBy;
      const packedTraits: any[] = [];
      for (const item of object.traits) {
        packedTraits.push(item);
      }
      objectCson["134"] = packedTraits;
      const packedSelfTraits: any[] = [];
      for (const item of object.selfTraits) {
        packedSelfTraits.push(item);
      }
      objectCson["135"] = packedSelfTraits;
      const packedEventTypes: any[] = [];
      for (const item of object.eventTypes) {
        packedEventTypes.push(item);
      }
      objectCson["140"] = packedEventTypes;
      const packedSelfEventTypes: any[] = [];
      for (const item of object.selfEventTypes) {
        packedSelfEventTypes.push(item);
      }
      objectCson["141"] = packedSelfEventTypes;
      const packedEnumTypes: any[] = [];
      for (const item of object.enumTypes) {
        packedEnumTypes.push(item);
      }
      objectCson["150"] = packedEnumTypes;
      const packedSelfEnumTypes: any[] = [];
      for (const item of object.selfEnumTypes) {
        packedSelfEnumTypes.push(item);
      }
      objectCson["151"] = packedSelfEnumTypes;
      const packedParentTypes: any[] = [];
      for (const item of object.parentTypes) {
        packedParentTypes.push(item);
      }
      objectCson["160"] = packedParentTypes;
      const packedChildTypes: any[] = [];
      for (const item of object.childTypes) {
        packedChildTypes.push(item);
      }
      objectCson["161"] = packedChildTypes;
      const packedAncestorTypes: any[] = [];
      for (const item of object.ancestorTypes) {
        packedAncestorTypes.push(item);
      }
      objectCson["162"] = packedAncestorTypes;
      const packedDescendantTypes: any[] = [];
      for (const item of object.descendantTypes) {
        packedDescendantTypes.push(item);
      }
      objectCson["163"] = packedDescendantTypes;
      const packedExpectedParentTypes: any[] = [];
      for (const item of object.expectedParentTypes) {
        packedExpectedParentTypes.push(item);
      }
      objectCson["170"] = packedExpectedParentTypes;
      const packedExpectedChildTypes: any[] = [];
      for (const item of object.expectedChildTypes) {
        packedExpectedChildTypes.push(item);
      }
      objectCson["171"] = packedExpectedChildTypes;
      const packedExpectedAncestorTypes: any[] = [];
      for (const item of object.expectedAncestorTypes) {
        packedExpectedAncestorTypes.push(item);
      }
      objectCson["172"] = packedExpectedAncestorTypes;
      const packedExpectedDescendantTypes: any[] = [];
      for (const item of object.expectedDescendantTypes) {
        packedExpectedDescendantTypes.push(item);
      }
      objectCson["173"] = packedExpectedDescendantTypes;
      if (object.domain != null) {
        objectCson["200"] = object.domain;
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): NodeDefinition {
      const _PropertyDefinition = STRUCT_CLASS_BY_TYPE[18] as typeof PropertyDefinition;
      const _ConstantDefinition = STRUCT_CLASS_BY_TYPE[19] as typeof ConstantDefinition;
      const _IndexDefinition = STRUCT_CLASS_BY_TYPE[30100] as typeof IndexDefinition;
      const _ConstraintDefinition = STRUCT_CLASS_BY_TYPE[30200] as typeof ConstraintDefinition;
      const _MethodDefinition = STRUCT_CLASS_BY_TYPE[40000] as typeof MethodDefinition;
      const _ActionDefinition = STRUCT_CLASS_BY_TYPE[40100] as typeof ActionDefinition;
      const _PermissionDefinition = STRUCT_CLASS_BY_TYPE[50000] as typeof PermissionDefinition;
      const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
      const iconValue = objectCson["102"];
      const unpackedIcon =
        iconValue != undefined ? (_Icon.unpack(2, iconValue, _session) as Icon) : undefined;
      const descriptionValue = objectCson["103"];
      const unpackedDescription = descriptionValue != undefined ? descriptionValue : undefined;
      const unpackedTaggings: any[] = [];
      for (const item of objectCson["109"]) {
        unpackedTaggings.push(Number(item));
      }
      const unpackedProperties: any[] = [];
      for (const item of objectCson["120"]) {
        unpackedProperties.push(
          _PropertyDefinition.unpack(2, item, _session) as PropertyDefinition,
        );
      }
      const unpackedIndexes: any[] = [];
      for (const item of objectCson["121"]) {
        unpackedIndexes.push(_IndexDefinition.unpack(2, item, _session) as IndexDefinition);
      }
      const unpackedConstraints: any[] = [];
      for (const item of objectCson["122"]) {
        unpackedConstraints.push(
          _ConstraintDefinition.unpack(2, item, _session) as ConstraintDefinition,
        );
      }
      const unpackedPermissions: any[] = [];
      for (const item of objectCson["123"]) {
        unpackedPermissions.push(
          _PermissionDefinition.unpack(2, item, _session) as PermissionDefinition,
        );
      }
      const unpackedMethods: any[] = [];
      for (const item of objectCson["125"]) {
        unpackedMethods.push(_MethodDefinition.unpack(2, item, _session) as MethodDefinition);
      }
      const unpackedActions: any[] = [];
      for (const item of objectCson["126"]) {
        unpackedActions.push(_ActionDefinition.unpack(2, item, _session) as ActionDefinition);
      }
      const unpackedConstants: any[] = [];
      for (const item of objectCson["128"]) {
        unpackedConstants.push(_ConstantDefinition.unpack(2, item, _session) as ConstantDefinition);
      }
      const baseTypeValue = objectCson["130"];
      const unpackedBaseType = baseTypeValue != undefined ? Number(baseTypeValue) : undefined;
      const unpackedExtendedBy: any[] = [];
      for (const item of objectCson["131"]) {
        unpackedExtendedBy.push(Number(item));
      }
      const unpackedInherits: any[] = [];
      for (const item of objectCson["132"]) {
        unpackedInherits.push(Number(item));
      }
      const unpackedInheritedBy: any[] = [];
      for (const item of objectCson["133"]) {
        unpackedInheritedBy.push(Number(item));
      }
      const unpackedTraits: any[] = [];
      for (const item of objectCson["134"]) {
        unpackedTraits.push(Number(item));
      }
      const unpackedSelfTraits: any[] = [];
      for (const item of objectCson["135"]) {
        unpackedSelfTraits.push(Number(item));
      }
      const unpackedEventTypes: any[] = [];
      for (const item of objectCson["140"]) {
        unpackedEventTypes.push(Number(item));
      }
      const unpackedSelfEventTypes: any[] = [];
      for (const item of objectCson["141"]) {
        unpackedSelfEventTypes.push(Number(item));
      }
      const unpackedEnumTypes: any[] = [];
      for (const item of objectCson["150"]) {
        unpackedEnumTypes.push(Number(item));
      }
      const unpackedSelfEnumTypes: any[] = [];
      for (const item of objectCson["151"]) {
        unpackedSelfEnumTypes.push(Number(item));
      }
      const unpackedParentTypes: any[] = [];
      for (const item of objectCson["160"]) {
        unpackedParentTypes.push(Number(item));
      }
      const unpackedChildTypes: any[] = [];
      for (const item of objectCson["161"]) {
        unpackedChildTypes.push(Number(item));
      }
      const unpackedAncestorTypes: any[] = [];
      for (const item of objectCson["162"]) {
        unpackedAncestorTypes.push(Number(item));
      }
      const unpackedDescendantTypes: any[] = [];
      for (const item of objectCson["163"]) {
        unpackedDescendantTypes.push(Number(item));
      }
      const unpackedExpectedParentTypes: any[] = [];
      for (const item of objectCson["170"]) {
        unpackedExpectedParentTypes.push(Number(item));
      }
      const unpackedExpectedChildTypes: any[] = [];
      for (const item of objectCson["171"]) {
        unpackedExpectedChildTypes.push(Number(item));
      }
      const unpackedExpectedAncestorTypes: any[] = [];
      for (const item of objectCson["172"]) {
        unpackedExpectedAncestorTypes.push(Number(item));
      }
      const unpackedExpectedDescendantTypes: any[] = [];
      for (const item of objectCson["173"]) {
        unpackedExpectedDescendantTypes.push(Number(item));
      }
      const domainValue = objectCson["200"];
      const unpackedDomain = domainValue != undefined ? Number(domainValue) : undefined;
      return new (STRUCT_CLASS_BY_TYPE[12] as typeof NodeDefinition)({
        type: Number(objectCson["100"]),
        id: Number(objectCson["2"]),
        name: objectCson["101"],
        icon: unpackedIcon,
        description: unpackedDescription,
        taggings: unpackedTaggings,
        isAbstract: Boolean(objectCson["110"]),
        isExtensible: Boolean(objectCson["111"]),
        isFinal: Boolean(objectCson["112"]),
        isFrozen: Boolean(objectCson["113"]),
        properties: unpackedProperties,
        indexes: unpackedIndexes,
        constraints: unpackedConstraints,
        permissions: unpackedPermissions,
        methods: unpackedMethods,
        actions: unpackedActions,
        constants: unpackedConstants,
        baseType: unpackedBaseType,
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
        domain: unpackedDomain,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 12)] = new NodeDefinitionCsonEncoder();

  class TraitDefinitionCsonEncoder implements CsonObjectEncoder {
    packObject(object: TraitDefinition): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 14;
      objectCson["2"] = object.id;
      objectCson["100"] = object.type;
      objectCson["101"] = object.name;
      if (object.icon != null) {
        objectCson["102"] = object.icon.pack(2);
      }
      if (object.description != null) {
        objectCson["103"] = object.description;
      }
      const packedTaggings: any[] = [];
      for (const item of object.taggings) {
        packedTaggings.push(item);
      }
      objectCson["109"] = packedTaggings;
      objectCson["110"] = object.alias;
      objectCson["111"] = object.isExtensible;
      const packedPermissions: any[] = [];
      for (const item of object.permissions) {
        packedPermissions.push(item.pack(2));
      }
      objectCson["123"] = packedPermissions;
      const packedSelfTraits: any[] = [];
      for (const item of object.selfTraits) {
        packedSelfTraits.push(item);
      }
      objectCson["130"] = packedSelfTraits;
      const packedTraits: any[] = [];
      for (const item of object.traits) {
        packedTraits.push(item);
      }
      objectCson["131"] = packedTraits;
      const packedEventTypes: any[] = [];
      for (const item of object.eventTypes) {
        packedEventTypes.push(item);
      }
      objectCson["140"] = packedEventTypes;
      const packedSelfEventTypes: any[] = [];
      for (const item of object.selfEventTypes) {
        packedSelfEventTypes.push(item);
      }
      objectCson["141"] = packedSelfEventTypes;
      const packedEnumTypes: any[] = [];
      for (const item of object.enumTypes) {
        packedEnumTypes.push(item);
      }
      objectCson["150"] = packedEnumTypes;
      const packedSelfEnumTypes: any[] = [];
      for (const item of object.selfEnumTypes) {
        packedSelfEnumTypes.push(item);
      }
      objectCson["151"] = packedSelfEnumTypes;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): TraitDefinition {
      const _PermissionDefinition = STRUCT_CLASS_BY_TYPE[50000] as typeof PermissionDefinition;
      const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
      const iconValue = objectCson["102"];
      const unpackedIcon =
        iconValue != undefined ? (_Icon.unpack(2, iconValue, _session) as Icon) : undefined;
      const descriptionValue = objectCson["103"];
      const unpackedDescription = descriptionValue != undefined ? descriptionValue : undefined;
      const unpackedTaggings: any[] = [];
      for (const item of objectCson["109"]) {
        unpackedTaggings.push(Number(item));
      }
      const unpackedPermissions: any[] = [];
      for (const item of objectCson["123"]) {
        unpackedPermissions.push(
          _PermissionDefinition.unpack(2, item, _session) as PermissionDefinition,
        );
      }
      const unpackedSelfTraits: any[] = [];
      for (const item of objectCson["130"]) {
        unpackedSelfTraits.push(Number(item));
      }
      const unpackedTraits: any[] = [];
      for (const item of objectCson["131"]) {
        unpackedTraits.push(Number(item));
      }
      const unpackedEventTypes: any[] = [];
      for (const item of objectCson["140"]) {
        unpackedEventTypes.push(Number(item));
      }
      const unpackedSelfEventTypes: any[] = [];
      for (const item of objectCson["141"]) {
        unpackedSelfEventTypes.push(Number(item));
      }
      const unpackedEnumTypes: any[] = [];
      for (const item of objectCson["150"]) {
        unpackedEnumTypes.push(Number(item));
      }
      const unpackedSelfEnumTypes: any[] = [];
      for (const item of objectCson["151"]) {
        unpackedSelfEnumTypes.push(Number(item));
      }
      return new (STRUCT_CLASS_BY_TYPE[14] as typeof TraitDefinition)({
        type: Number(objectCson["100"]),
        id: Number(objectCson["2"]),
        name: objectCson["101"],
        icon: unpackedIcon,
        description: unpackedDescription,
        taggings: unpackedTaggings,
        alias: objectCson["110"],
        isExtensible: Boolean(objectCson["111"]),
        permissions: unpackedPermissions,
        selfTraits: unpackedSelfTraits,
        traits: unpackedTraits,
        eventTypes: unpackedEventTypes,
        selfEventTypes: unpackedSelfEventTypes,
        enumTypes: unpackedEnumTypes,
        selfEnumTypes: unpackedSelfEnumTypes,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 14)] = new TraitDefinitionCsonEncoder();

  class StructDefinitionCsonEncoder implements CsonObjectEncoder {
    packObject(object: StructDefinition): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 15;
      objectCson["2"] = object.id;
      objectCson["100"] = object.type;
      objectCson["101"] = object.name;
      if (object.icon != null) {
        objectCson["102"] = object.icon.pack(2);
      }
      if (object.description != null) {
        objectCson["103"] = object.description;
      }
      const packedTaggings: any[] = [];
      for (const item of object.taggings) {
        packedTaggings.push(item);
      }
      objectCson["109"] = packedTaggings;
      objectCson["110"] = object.isFrozen;
      objectCson["111"] = object.isAbstract;
      objectCson["112"] = object.isExtensible;
      const packedProperties: any[] = [];
      for (const item of object.properties) {
        packedProperties.push(item.pack(2));
      }
      objectCson["120"] = packedProperties;
      const packedMethods: any[] = [];
      for (const item of object.methods) {
        packedMethods.push(item.pack(2));
      }
      objectCson["125"] = packedMethods;
      const packedActions: any[] = [];
      for (const item of object.actions) {
        packedActions.push(item.pack(2));
      }
      objectCson["126"] = packedActions;
      const packedConstants: any[] = [];
      for (const item of object.constants) {
        packedConstants.push(item.pack(2));
      }
      objectCson["128"] = packedConstants;
      const packedTags: any[] = [];
      for (const item of object.tags) {
        packedTags.push(item.pack(2));
      }
      objectCson["129"] = packedTags;
      if (object.baseType != null) {
        objectCson["130"] = object.baseType;
      }
      const packedExtendedBy: any[] = [];
      for (const item of object.extendedBy) {
        packedExtendedBy.push(item);
      }
      objectCson["131"] = packedExtendedBy;
      const packedInherits: any[] = [];
      for (const item of object.inherits) {
        packedInherits.push(item);
      }
      objectCson["132"] = packedInherits;
      const packedInheritedBy: any[] = [];
      for (const item of object.inheritedBy) {
        packedInheritedBy.push(item);
      }
      objectCson["133"] = packedInheritedBy;
      const packedEnumTypes: any[] = [];
      for (const item of object.enumTypes) {
        packedEnumTypes.push(item);
      }
      objectCson["150"] = packedEnumTypes;
      const packedSelfEnumTypes: any[] = [];
      for (const item of object.selfEnumTypes) {
        packedSelfEnumTypes.push(item);
      }
      objectCson["151"] = packedSelfEnumTypes;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): StructDefinition {
      const _PropertyDefinition = STRUCT_CLASS_BY_TYPE[18] as typeof PropertyDefinition;
      const _ConstantDefinition = STRUCT_CLASS_BY_TYPE[19] as typeof ConstantDefinition;
      const _TagDefinition = STRUCT_CLASS_BY_TYPE[21] as typeof TagDefinition;
      const _MethodDefinition = STRUCT_CLASS_BY_TYPE[40000] as typeof MethodDefinition;
      const _ActionDefinition = STRUCT_CLASS_BY_TYPE[40100] as typeof ActionDefinition;
      const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
      const iconValue = objectCson["102"];
      const unpackedIcon =
        iconValue != undefined ? (_Icon.unpack(2, iconValue, _session) as Icon) : undefined;
      const descriptionValue = objectCson["103"];
      const unpackedDescription = descriptionValue != undefined ? descriptionValue : undefined;
      const unpackedTaggings: any[] = [];
      for (const item of objectCson["109"]) {
        unpackedTaggings.push(Number(item));
      }
      const unpackedProperties: any[] = [];
      for (const item of objectCson["120"]) {
        unpackedProperties.push(
          _PropertyDefinition.unpack(2, item, _session) as PropertyDefinition,
        );
      }
      const unpackedMethods: any[] = [];
      for (const item of objectCson["125"]) {
        unpackedMethods.push(_MethodDefinition.unpack(2, item, _session) as MethodDefinition);
      }
      const unpackedActions: any[] = [];
      for (const item of objectCson["126"]) {
        unpackedActions.push(_ActionDefinition.unpack(2, item, _session) as ActionDefinition);
      }
      const unpackedConstants: any[] = [];
      for (const item of objectCson["128"]) {
        unpackedConstants.push(_ConstantDefinition.unpack(2, item, _session) as ConstantDefinition);
      }
      const unpackedTags: any[] = [];
      for (const item of objectCson["129"]) {
        unpackedTags.push(_TagDefinition.unpack(2, item, _session) as TagDefinition);
      }
      const baseTypeValue = objectCson["130"];
      const unpackedBaseType = baseTypeValue != undefined ? Number(baseTypeValue) : undefined;
      const unpackedExtendedBy: any[] = [];
      for (const item of objectCson["131"]) {
        unpackedExtendedBy.push(Number(item));
      }
      const unpackedInherits: any[] = [];
      for (const item of objectCson["132"]) {
        unpackedInherits.push(Number(item));
      }
      const unpackedInheritedBy: any[] = [];
      for (const item of objectCson["133"]) {
        unpackedInheritedBy.push(Number(item));
      }
      const unpackedEnumTypes: any[] = [];
      for (const item of objectCson["150"]) {
        unpackedEnumTypes.push(Number(item));
      }
      const unpackedSelfEnumTypes: any[] = [];
      for (const item of objectCson["151"]) {
        unpackedSelfEnumTypes.push(Number(item));
      }
      return new (STRUCT_CLASS_BY_TYPE[15] as typeof StructDefinition)({
        type: Number(objectCson["100"]),
        id: Number(objectCson["2"]),
        name: objectCson["101"],
        icon: unpackedIcon,
        description: unpackedDescription,
        taggings: unpackedTaggings,
        isFrozen: Boolean(objectCson["110"]),
        isAbstract: Boolean(objectCson["111"]),
        isExtensible: Boolean(objectCson["112"]),
        properties: unpackedProperties,
        methods: unpackedMethods,
        actions: unpackedActions,
        constants: unpackedConstants,
        tags: unpackedTags,
        baseType: unpackedBaseType,
        extendedBy: unpackedExtendedBy,
        inherits: unpackedInherits,
        inheritedBy: unpackedInheritedBy,
        enumTypes: unpackedEnumTypes,
        selfEnumTypes: unpackedSelfEnumTypes,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 15)] = new StructDefinitionCsonEncoder();

  class EnumDefinitionCsonEncoder implements CsonObjectEncoder {
    packObject(object: EnumDefinition): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 17;
      objectCson["2"] = object.id;
      objectCson["100"] = object.type;
      objectCson["101"] = object.name;
      if (object.icon != null) {
        objectCson["102"] = object.icon.pack(2);
      }
      if (object.description != null) {
        objectCson["103"] = object.description;
      }
      const packedTaggings: any[] = [];
      for (const item of object.taggings) {
        packedTaggings.push(item);
      }
      objectCson["109"] = packedTaggings;
      const packedOptions: any[] = [];
      for (const item of object.options) {
        packedOptions.push(item.pack(2));
      }
      objectCson["120"] = packedOptions;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): EnumDefinition {
      const _OptionDefinition = STRUCT_CLASS_BY_TYPE[20] as typeof OptionDefinition;
      const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
      const iconValue = objectCson["102"];
      const unpackedIcon =
        iconValue != undefined ? (_Icon.unpack(2, iconValue, _session) as Icon) : undefined;
      const descriptionValue = objectCson["103"];
      const unpackedDescription = descriptionValue != undefined ? descriptionValue : undefined;
      const unpackedTaggings: any[] = [];
      for (const item of objectCson["109"]) {
        unpackedTaggings.push(Number(item));
      }
      const unpackedOptions: any[] = [];
      for (const item of objectCson["120"]) {
        unpackedOptions.push(_OptionDefinition.unpack(2, item, _session) as OptionDefinition);
      }
      return new (STRUCT_CLASS_BY_TYPE[17] as typeof EnumDefinition)({
        type: Number(objectCson["100"]),
        id: Number(objectCson["2"]),
        name: objectCson["101"],
        icon: unpackedIcon,
        description: unpackedDescription,
        taggings: unpackedTaggings,
        options: unpackedOptions,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 17)] = new EnumDefinitionCsonEncoder();

  class PropertyDefinitionCsonEncoder implements CsonObjectEncoder {
    packObject(object: PropertyDefinition): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 18;
      objectCson["2"] = object.id;
      objectCson["100"] = object.type;
      if (object.name != null) {
        objectCson["101"] = object.name;
      }
      if (object.description != null) {
        objectCson["103"] = object.description;
      }
      objectCson["104"] = object.object.pack(2);
      objectCson["105"] = object.originalObject.pack(2);
      const packedTaggings: any[] = [];
      for (const item of object.taggings) {
        packedTaggings.push(item);
      }
      objectCson["109"] = packedTaggings;
      objectCson["110"] = object.cardinality;
      if (object.keyType != null) {
        objectCson["111"] = object.keyType.pack(2);
      }
      if (object.valueType != null) {
        objectCson["112"] = object.valueType.pack(2);
      }
      if (object.elementTypes != null) {
        const packedElementTypes: any[] = [];
        for (const item of object.elementTypes) {
          packedElementTypes.push(item.pack(2));
        }
        objectCson["113"] = packedElementTypes;
      }
      if (object.scalarType != null) {
        objectCson["120"] = object.scalarType;
      }
      if (object.primitiveType != null) {
        objectCson["121"] = object.primitiveType;
      }
      if (object.enumType != null) {
        objectCson["122"] = object.enumType;
      }
      if (object.nodeTypes != null) {
        const packedNodeTypes: any[] = [];
        for (const item of object.nodeTypes) {
          packedNodeTypes.push(item);
        }
        objectCson["123"] = packedNodeTypes;
      }
      if (object.structType != null) {
        objectCson["124"] = object.structType;
      }
      if (object.literalValue != null) {
        objectCson["125"] = object.literalValue.pack(2);
      }
      if (object.unionTypes != null) {
        const packedUnionTypes: any[] = [];
        for (const item of object.unionTypes) {
          packedUnionTypes.push(item.pack(2));
        }
        objectCson["126"] = packedUnionTypes;
      }
      objectCson["130"] = object.isRequired;
      if (object.defaultValue != null) {
        objectCson["150"] = object.defaultValue.pack(2);
      }
      if (object.defaultFactory != null) {
        objectCson["151"] = object.defaultFactory;
      }
      if (object.collectionConstraint != null) {
        objectCson["160"] = object.collectionConstraint.pack(2);
      }
      if (object.stringConstraint != null) {
        objectCson["161"] = object.stringConstraint.pack(2);
      }
      if (object.numberConstraint != null) {
        objectCson["162"] = object.numberConstraint.pack(2);
      }
      if (object.edgeType != null) {
        objectCson["190"] = object.edgeType;
      }
      if (object.cascade != null) {
        objectCson["191"] = object.cascade;
      }
      objectCson["200"] = object.isIdentity;
      objectCson["201"] = object.isUnique;
      objectCson["202"] = object.isReadonly;
      objectCson["203"] = object.isMain;
      objectCson["210"] = object.isWired;
      objectCson["211"] = object.isStored;
      objectCson["212"] = object.isRepr;
      objectCson["213"] = object.isHash;
      objectCson["214"] = object.isEq;
      objectCson["215"] = object.isInternal;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): PropertyDefinition {
      const _ObjectDefinitionReference =
        STRUCT_CLASS_BY_TYPE[11] as typeof ObjectDefinitionReference;
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _Type = STRUCT_CLASS_BY_TYPE[101] as typeof Type;
      const _NumberConstraint = STRUCT_CLASS_BY_TYPE[110] as typeof NumberConstraint;
      const _StringConstraint = STRUCT_CLASS_BY_TYPE[111] as typeof StringConstraint;
      const _CollectionConstraint = STRUCT_CLASS_BY_TYPE[112] as typeof CollectionConstraint;
      const nameValue = objectCson["101"];
      const unpackedName = nameValue != undefined ? nameValue : undefined;
      const descriptionValue = objectCson["103"];
      const unpackedDescription = descriptionValue != undefined ? descriptionValue : undefined;
      const unpackedTaggings: any[] = [];
      for (const item of objectCson["109"]) {
        unpackedTaggings.push(Number(item));
      }
      const edgeTypeValue = objectCson["190"];
      const unpackedEdgeType = edgeTypeValue != undefined ? Number(edgeTypeValue) : undefined;
      const cascadeValue = objectCson["191"];
      const unpackedCascade = cascadeValue != undefined ? Number(cascadeValue) : undefined;
      const defaultValueValue = objectCson["150"];
      const unpackedDefaultValue =
        defaultValueValue != undefined
          ? (_Value.unpack(2, defaultValueValue, _session) as Value)
          : undefined;
      const defaultFactoryValue = objectCson["151"];
      const unpackedDefaultFactory =
        defaultFactoryValue != undefined ? Number(defaultFactoryValue) : undefined;
      const collectionConstraintValue = objectCson["160"];
      const unpackedCollectionConstraint =
        collectionConstraintValue != undefined
          ? (_CollectionConstraint.unpack(
              2,
              collectionConstraintValue,
              _session,
            ) as CollectionConstraint)
          : undefined;
      const stringConstraintValue = objectCson["161"];
      const unpackedStringConstraint =
        stringConstraintValue != undefined
          ? (_StringConstraint.unpack(2, stringConstraintValue, _session) as StringConstraint)
          : undefined;
      const numberConstraintValue = objectCson["162"];
      const unpackedNumberConstraint =
        numberConstraintValue != undefined
          ? (_NumberConstraint.unpack(2, numberConstraintValue, _session) as NumberConstraint)
          : undefined;
      const keyTypeValue = objectCson["111"];
      const unpackedKeyType =
        keyTypeValue != undefined ? (_Type.unpack(2, keyTypeValue, _session) as Type) : undefined;
      const valueTypeValue = objectCson["112"];
      const unpackedValueType =
        valueTypeValue != undefined
          ? (_Type.unpack(2, valueTypeValue, _session) as Type)
          : undefined;
      let unpackedElementTypes: any[] | undefined;
      if (objectCson["113"] != undefined) {
        unpackedElementTypes = [];
        for (const item of objectCson["113"]) {
          unpackedElementTypes.push(_Type.unpack(2, item, _session) as Type);
        }
      } else {
        unpackedElementTypes = undefined;
      }
      const scalarTypeValue = objectCson["120"];
      const unpackedScalarType = scalarTypeValue != undefined ? Number(scalarTypeValue) : undefined;
      const primitiveTypeValue = objectCson["121"];
      const unpackedPrimitiveType =
        primitiveTypeValue != undefined ? Number(primitiveTypeValue) : undefined;
      const enumTypeValue = objectCson["122"];
      const unpackedEnumType = enumTypeValue != undefined ? Number(enumTypeValue) : undefined;
      let unpackedNodeTypes: any[] | undefined;
      if (objectCson["123"] != undefined) {
        unpackedNodeTypes = [];
        for (const item of objectCson["123"]) {
          unpackedNodeTypes.push(Number(item));
        }
      } else {
        unpackedNodeTypes = undefined;
      }
      const structTypeValue = objectCson["124"];
      const unpackedStructType = structTypeValue != undefined ? Number(structTypeValue) : undefined;
      const literalValueValue = objectCson["125"];
      const unpackedLiteralValue =
        literalValueValue != undefined
          ? (_Value.unpack(2, literalValueValue, _session) as Value)
          : undefined;
      let unpackedUnionTypes: any[] | undefined;
      if (objectCson["126"] != undefined) {
        unpackedUnionTypes = [];
        for (const item of objectCson["126"]) {
          unpackedUnionTypes.push(_Type.unpack(2, item, _session) as Type);
        }
      } else {
        unpackedUnionTypes = undefined;
      }
      return new (STRUCT_CLASS_BY_TYPE[18] as typeof PropertyDefinition)({
        type: Number(objectCson["100"]),
        id: Number(objectCson["2"]),
        name: unpackedName,
        description: unpackedDescription,
        object: _ObjectDefinitionReference.unpack(
          2,
          objectCson["104"],
          _session,
        ) as ObjectDefinitionReference,
        originalObject: _ObjectDefinitionReference.unpack(
          2,
          objectCson["105"],
          _session,
        ) as ObjectDefinitionReference,
        taggings: unpackedTaggings,
        edgeType: unpackedEdgeType,
        cascade: unpackedCascade,
        isIdentity: Boolean(objectCson["200"]),
        isUnique: Boolean(objectCson["201"]),
        isReadonly: Boolean(objectCson["202"]),
        isMain: Boolean(objectCson["203"]),
        isWired: Boolean(objectCson["210"]),
        isStored: Boolean(objectCson["211"]),
        isRepr: Boolean(objectCson["212"]),
        isHash: Boolean(objectCson["213"]),
        isEq: Boolean(objectCson["214"]),
        isInternal: Boolean(objectCson["215"]),
        defaultValue: unpackedDefaultValue,
        defaultFactory: unpackedDefaultFactory,
        collectionConstraint: unpackedCollectionConstraint,
        stringConstraint: unpackedStringConstraint,
        numberConstraint: unpackedNumberConstraint,
        cardinality: Number(objectCson["110"]),
        keyType: unpackedKeyType,
        valueType: unpackedValueType,
        elementTypes: unpackedElementTypes,
        scalarType: unpackedScalarType,
        primitiveType: unpackedPrimitiveType,
        enumType: unpackedEnumType,
        nodeTypes: unpackedNodeTypes,
        structType: unpackedStructType,
        literalValue: unpackedLiteralValue,
        unionTypes: unpackedUnionTypes,
        isRequired: Boolean(objectCson["130"]),
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 18)] = new PropertyDefinitionCsonEncoder();

  class OptionDefinitionCsonEncoder implements CsonObjectEncoder {
    packObject(object: OptionDefinition): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 20;
      objectCson["2"] = object.id;
      objectCson["100"] = object.type;
      objectCson["101"] = object.name;
      if (object.icon != null) {
        objectCson["102"] = object.icon.pack(2);
      }
      if (object.description != null) {
        objectCson["103"] = object.description;
      }
      const packedTaggings: any[] = [];
      for (const item of object.taggings) {
        packedTaggings.push(item);
      }
      objectCson["109"] = packedTaggings;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): OptionDefinition {
      const _Icon = STRUCT_CLASS_BY_TYPE[400031] as typeof Icon;
      const iconValue = objectCson["102"];
      const unpackedIcon =
        iconValue != undefined ? (_Icon.unpack(2, iconValue, _session) as Icon) : undefined;
      const descriptionValue = objectCson["103"];
      const unpackedDescription = descriptionValue != undefined ? descriptionValue : undefined;
      const unpackedTaggings: any[] = [];
      for (const item of objectCson["109"]) {
        unpackedTaggings.push(Number(item));
      }
      return new (STRUCT_CLASS_BY_TYPE[20] as typeof OptionDefinition)({
        id: Number(objectCson["2"]),
        type: Number(objectCson["100"]),
        name: objectCson["101"],
        icon: unpackedIcon,
        description: unpackedDescription,
        taggings: unpackedTaggings,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 20)] = new OptionDefinitionCsonEncoder();

  class ConstantDefinitionCsonEncoder implements CsonObjectEncoder {
    packObject(object: ConstantDefinition): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 19;
      objectCson["2"] = object.id;
      objectCson["101"] = object.name;
      if (object.description != null) {
        objectCson["103"] = object.description;
      }
      const packedTaggings: any[] = [];
      for (const item of object.taggings) {
        packedTaggings.push(item);
      }
      objectCson["109"] = packedTaggings;
      objectCson["120"] = object.value.pack(2);
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): ConstantDefinition {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const descriptionValue = objectCson["103"];
      const unpackedDescription = descriptionValue != undefined ? descriptionValue : undefined;
      const unpackedTaggings: any[] = [];
      for (const item of objectCson["109"]) {
        unpackedTaggings.push(Number(item));
      }
      return new (STRUCT_CLASS_BY_TYPE[19] as typeof ConstantDefinition)({
        id: Number(objectCson["2"]),
        name: objectCson["101"],
        description: unpackedDescription,
        taggings: unpackedTaggings,
        value: _Value.unpack(2, objectCson["120"], _session) as Value,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 19)] = new ConstantDefinitionCsonEncoder();

  class TagDefinitionCsonEncoder implements CsonObjectEncoder {
    packObject(object: TagDefinition): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 21;
      objectCson["2"] = object.id;
      objectCson["101"] = object.name;
      if (object.description != null) {
        objectCson["103"] = object.description;
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): TagDefinition {
      const descriptionValue = objectCson["103"];
      const unpackedDescription = descriptionValue != undefined ? descriptionValue : undefined;
      return new (STRUCT_CLASS_BY_TYPE[21] as typeof TagDefinition)({
        id: Number(objectCson["2"]),
        name: objectCson["101"],
        description: unpackedDescription,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 21)] = new TagDefinitionCsonEncoder();

  class IndexDefinitionCsonEncoder implements CsonObjectEncoder {
    packObject(object: IndexDefinition): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 30100;
      objectCson["2"] = object.id;
      objectCson["100"] = object.type;
      objectCson["101"] = object.name;
      if (object.description != null) {
        objectCson["103"] = object.description;
      }
      const packedProperties: any[] = [];
      for (const item of object.properties) {
        packedProperties.push(item.pack(2));
      }
      objectCson["120"] = packedProperties;
      const packedCover: any[] = [];
      for (const item of object.cover) {
        packedCover.push(item.pack(2));
      }
      objectCson["121"] = packedCover;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): IndexDefinition {
      const _PropertyReference = STRUCT_CLASS_BY_TYPE[1001] as typeof PropertyReference;
      const descriptionValue = objectCson["103"];
      const unpackedDescription = descriptionValue != undefined ? descriptionValue : undefined;
      const unpackedProperties: any[] = [];
      for (const item of objectCson["120"]) {
        unpackedProperties.push(_PropertyReference.unpack(2, item, _session) as PropertyReference);
      }
      const unpackedCover: any[] = [];
      for (const item of objectCson["121"]) {
        unpackedCover.push(_PropertyReference.unpack(2, item, _session) as PropertyReference);
      }
      return new (STRUCT_CLASS_BY_TYPE[30100] as typeof IndexDefinition)({
        id: Number(objectCson["2"]),
        type: Number(objectCson["100"]),
        name: objectCson["101"],
        description: unpackedDescription,
        properties: unpackedProperties,
        cover: unpackedCover,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 30100)] = new IndexDefinitionCsonEncoder();

  class ConstraintDefinitionCsonEncoder implements CsonObjectEncoder {
    packObject(object: ConstraintDefinition): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 30200;
      objectCson["2"] = object.id;
      objectCson["100"] = object.type;
      objectCson["101"] = object.name;
      if (object.description != null) {
        objectCson["103"] = object.description;
      }
      const packedProperties: any[] = [];
      for (const item of object.properties) {
        packedProperties.push(item.pack(2));
      }
      objectCson["120"] = packedProperties;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): ConstraintDefinition {
      const _PropertyReference = STRUCT_CLASS_BY_TYPE[1001] as typeof PropertyReference;
      const descriptionValue = objectCson["103"];
      const unpackedDescription = descriptionValue != undefined ? descriptionValue : undefined;
      const unpackedProperties: any[] = [];
      for (const item of objectCson["120"]) {
        unpackedProperties.push(_PropertyReference.unpack(2, item, _session) as PropertyReference);
      }
      return new (STRUCT_CLASS_BY_TYPE[30200] as typeof ConstraintDefinition)({
        id: Number(objectCson["2"]),
        type: Number(objectCson["100"]),
        name: objectCson["101"],
        description: unpackedDescription,
        properties: unpackedProperties,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 30200)] = new ConstraintDefinitionCsonEncoder();

  class PermissionDefinitionCsonEncoder implements CsonObjectEncoder {
    packObject(object: PermissionDefinition): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 50000;
      objectCson["2"] = object.id;
      objectCson["101"] = object.name;
      if (object.description != null) {
        objectCson["103"] = object.description;
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): PermissionDefinition {
      const descriptionValue = objectCson["103"];
      const unpackedDescription = descriptionValue != undefined ? descriptionValue : undefined;
      return new (STRUCT_CLASS_BY_TYPE[50000] as typeof PermissionDefinition)({
        id: Number(objectCson["2"]),
        name: objectCson["101"],
        description: unpackedDescription,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 50000)] = new PermissionDefinitionCsonEncoder();

  class MethodDefinitionCsonEncoder implements CsonObjectEncoder {
    packObject(object: MethodDefinition): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 40000;
      objectCson["2"] = object.id;
      objectCson["100"] = object.type;
      objectCson["101"] = object.name;
      if (object.description != null) {
        objectCson["103"] = object.description;
      }
      const packedProperties: any[] = [];
      for (const item of object.properties) {
        packedProperties.push(item.pack(2));
      }
      objectCson["120"] = packedProperties;
      objectCson["121"] = object.cardinality;
      const packedPlatforms: any[] = [];
      for (const item of object.platforms) {
        packedPlatforms.push(item);
      }
      objectCson["130"] = packedPlatforms;
      const packedLanguages: any[] = [];
      for (const item of object.languages) {
        packedLanguages.push(item);
      }
      objectCson["131"] = packedLanguages;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): MethodDefinition {
      const _PropertyReference = STRUCT_CLASS_BY_TYPE[1001] as typeof PropertyReference;
      const descriptionValue = objectCson["103"];
      const unpackedDescription = descriptionValue != undefined ? descriptionValue : undefined;
      const unpackedProperties: any[] = [];
      for (const item of objectCson["120"]) {
        unpackedProperties.push(_PropertyReference.unpack(2, item, _session) as PropertyReference);
      }
      const unpackedPlatforms: any[] = [];
      for (const item of objectCson["130"]) {
        unpackedPlatforms.push(Number(item));
      }
      const unpackedLanguages: any[] = [];
      for (const item of objectCson["131"]) {
        unpackedLanguages.push(Number(item));
      }
      return new (STRUCT_CLASS_BY_TYPE[40000] as typeof MethodDefinition)({
        id: Number(objectCson["2"]),
        type: Number(objectCson["100"]),
        name: objectCson["101"],
        description: unpackedDescription,
        properties: unpackedProperties,
        cardinality: Number(objectCson["121"]),
        platforms: unpackedPlatforms,
        languages: unpackedLanguages,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 40000)] = new MethodDefinitionCsonEncoder();

  class ActionDefinitionCsonEncoder implements CsonObjectEncoder {
    packObject(object: ActionDefinition): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 40100;
      objectCson["2"] = object.id;
      objectCson["100"] = object.type;
      objectCson["101"] = object.name;
      if (object.description != null) {
        objectCson["103"] = object.description;
      }
      const packedProperties: any[] = [];
      for (const item of object.properties) {
        packedProperties.push(item.pack(2));
      }
      objectCson["120"] = packedProperties;
      objectCson["121"] = object.cardinality;
      const packedPlatforms: any[] = [];
      for (const item of object.platforms) {
        packedPlatforms.push(item);
      }
      objectCson["130"] = packedPlatforms;
      const packedLanguages: any[] = [];
      for (const item of object.languages) {
        packedLanguages.push(item);
      }
      objectCson["131"] = packedLanguages;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): ActionDefinition {
      const _PropertyReference = STRUCT_CLASS_BY_TYPE[1001] as typeof PropertyReference;
      const descriptionValue = objectCson["103"];
      const unpackedDescription = descriptionValue != undefined ? descriptionValue : undefined;
      const unpackedProperties: any[] = [];
      for (const item of objectCson["120"]) {
        unpackedProperties.push(_PropertyReference.unpack(2, item, _session) as PropertyReference);
      }
      const unpackedPlatforms: any[] = [];
      for (const item of objectCson["130"]) {
        unpackedPlatforms.push(Number(item));
      }
      const unpackedLanguages: any[] = [];
      for (const item of objectCson["131"]) {
        unpackedLanguages.push(Number(item));
      }
      return new (STRUCT_CLASS_BY_TYPE[40100] as typeof ActionDefinition)({
        id: Number(objectCson["2"]),
        type: Number(objectCson["100"]),
        name: objectCson["101"],
        description: unpackedDescription,
        properties: unpackedProperties,
        cardinality: Number(objectCson["121"]),
        platforms: unpackedPlatforms,
        languages: unpackedLanguages,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 40100)] = new ActionDefinitionCsonEncoder();

  class IconCsonEncoder implements CsonObjectEncoder {
    packObject(object: Icon): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 400031;
      objectCson["100"] = object.type;
      if (object.emoji != null) {
        objectCson["101"] = object.emoji;
      }
      if (object.faName != null) {
        objectCson["102"] = object.faName;
      }
      if (object.vscName != null) {
        objectCson["103"] = object.vscName;
      }
      if (object.filePtr != null) {
        objectCson["104"] = object.filePtr.pack(2);
      }
      if (object.fileUrl != null) {
        objectCson["105"] = object.fileUrl;
      }
      if (object.color != null) {
        objectCson["110"] = object.color.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Icon {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Color = STRUCT_CLASS_BY_TYPE[2100300] as typeof Color;
      const emojiValue = objectCson["101"];
      const unpackedEmoji = emojiValue != undefined ? emojiValue : undefined;
      const faNameValue = objectCson["102"];
      const unpackedFaName = faNameValue != undefined ? faNameValue : undefined;
      const vscNameValue = objectCson["103"];
      const unpackedVscName = vscNameValue != undefined ? vscNameValue : undefined;
      const filePtrValue = objectCson["104"];
      const unpackedFilePtr =
        filePtrValue != undefined
          ? (_NodeReference.unpack(2, filePtrValue, _session) as NodeReference)
          : undefined;
      const fileUrlValue = objectCson["105"];
      const unpackedFileUrl = fileUrlValue != undefined ? fileUrlValue : undefined;
      const colorValue = objectCson["110"];
      const unpackedColor =
        colorValue != undefined ? (_Color.unpack(2, colorValue, _session) as Color) : undefined;
      return new (STRUCT_CLASS_BY_TYPE[400031] as typeof Icon)({
        type: Number(objectCson["100"]),
        emoji: unpackedEmoji,
        faName: unpackedFaName,
        vscName: unpackedVscName,
        file: unpackedFilePtr,
        fileUrl: unpackedFileUrl,
        color: unpackedColor,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 400031)] = new IconCsonEncoder();

  class MigrationDefinitionCsonEncoder implements CsonObjectEncoder {
    packObject(object: MigrationDefinition): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 31000;
      objectCson["100"] = object.type;
      objectCson["101"] = object.name;
      if (object.description != null) {
        objectCson["103"] = object.description;
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): MigrationDefinition {
      const descriptionValue = objectCson["103"];
      const unpackedDescription = descriptionValue != undefined ? descriptionValue : undefined;
      return new (STRUCT_CLASS_BY_TYPE[31000] as typeof MigrationDefinition)({
        type: Number(objectCson["100"]),
        name: objectCson["101"],
        description: unpackedDescription,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 31000)] = new MigrationDefinitionCsonEncoder();

  class MigrationOperationDefinitionCsonEncoder implements CsonObjectEncoder {
    packObject(object: MigrationOperationDefinition): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 31100;
      objectCson["2"] = object.id;
      objectCson["100"] = object.type;
      objectCson["101"] = object.name;
      if (object.description != null) {
        objectCson["103"] = object.description;
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): MigrationOperationDefinition {
      const descriptionValue = objectCson["103"];
      const unpackedDescription = descriptionValue != undefined ? descriptionValue : undefined;
      return new (STRUCT_CLASS_BY_TYPE[31100] as typeof MigrationOperationDefinition)({
        id: Number(objectCson["2"]),
        name: objectCson["101"],
        description: unpackedDescription,
        type: Number(objectCson["100"]),
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 31100)] = new MigrationOperationDefinitionCsonEncoder();

  class FunctionCsonEncoder implements CsonObjectEncoder {
    packObject(object: Function): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 201;
      objectCson["100"] = object.type;
      objectCson["101"] = object.left.pack(2);
      if (object.right != null) {
        objectCson["102"] = object.right.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Function {
      const _Expression = STRUCT_CLASS_BY_TYPE[200] as typeof Expression;
      const rightValue = objectCson["102"];
      const unpackedRight =
        rightValue != undefined
          ? (_Expression.unpack(2, rightValue, _session) as Expression)
          : undefined;
      return new (STRUCT_CLASS_BY_TYPE[201] as typeof Function)({
        type: Number(objectCson["100"]),
        left: _Expression.unpack(2, objectCson["101"], _session) as Expression,
        right: unpackedRight,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 201)] = new FunctionCsonEncoder();

  class ConditionCsonEncoder implements CsonObjectEncoder {
    packObject(object: Condition): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 204;
      objectCson["100"] = object.type;
      objectCson["101"] = object.left.pack(2);
      if (object.right != null) {
        objectCson["102"] = object.right.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Condition {
      const _Expression = STRUCT_CLASS_BY_TYPE[200] as typeof Expression;
      const rightValue = objectCson["102"];
      const unpackedRight =
        rightValue != undefined
          ? (_Expression.unpack(2, rightValue, _session) as Expression)
          : undefined;
      return new (STRUCT_CLASS_BY_TYPE[204] as typeof Condition)({
        type: Number(objectCson["100"]),
        left: _Expression.unpack(2, objectCson["101"], _session) as Expression,
        right: unpackedRight,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 204)] = new ConditionCsonEncoder();

  class AggregationCsonEncoder implements CsonObjectEncoder {
    packObject(object: Aggregation): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 203;
      objectCson["100"] = object.type;
      if (object.expression != null) {
        objectCson["101"] = object.expression.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Aggregation {
      const _Expression = STRUCT_CLASS_BY_TYPE[200] as typeof Expression;
      const expressionValue = objectCson["101"];
      const unpackedExpression =
        expressionValue != undefined
          ? (_Expression.unpack(2, expressionValue, _session) as Expression)
          : undefined;
      return new (STRUCT_CLASS_BY_TYPE[203] as typeof Aggregation)({
        type: Number(objectCson["100"]),
        expression: unpackedExpression,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 203)] = new AggregationCsonEncoder();

  class ExpressionCsonEncoder implements CsonObjectEncoder {
    packObject(object: Expression): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 200;
      objectCson["100"] = object.type;
      if (object.literal != null) {
        objectCson["101"] = object.literal.pack(2);
      }
      if (object.attribute != null) {
        objectCson["102"] = object.attribute.pack(2);
      }
      if (object.condition != null) {
        objectCson["103"] = object.condition.pack(2);
      }
      if (object.function != null) {
        objectCson["104"] = object.function.pack(2);
      }
      if (object.aggregation != null) {
        objectCson["105"] = object.aggregation.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Expression {
      const _Value = STRUCT_CLASS_BY_TYPE[100] as typeof Value;
      const _Function = STRUCT_CLASS_BY_TYPE[201] as typeof Function;
      const _Aggregation = STRUCT_CLASS_BY_TYPE[203] as typeof Aggregation;
      const _Condition = STRUCT_CLASS_BY_TYPE[204] as typeof Condition;
      const _PropertyReference = STRUCT_CLASS_BY_TYPE[1001] as typeof PropertyReference;
      const literalValue = objectCson["101"];
      const unpackedLiteral =
        literalValue != undefined ? (_Value.unpack(2, literalValue, _session) as Value) : undefined;
      const attributeValue = objectCson["102"];
      const unpackedAttribute =
        attributeValue != undefined
          ? (_PropertyReference.unpack(2, attributeValue, _session) as PropertyReference)
          : undefined;
      const conditionValue = objectCson["103"];
      const unpackedCondition =
        conditionValue != undefined
          ? (_Condition.unpack(2, conditionValue, _session) as Condition)
          : undefined;
      const functionValue = objectCson["104"];
      const unpackedFunction =
        functionValue != undefined
          ? (_Function.unpack(2, functionValue, _session) as Function)
          : undefined;
      const aggregationValue = objectCson["105"];
      const unpackedAggregation =
        aggregationValue != undefined
          ? (_Aggregation.unpack(2, aggregationValue, _session) as Aggregation)
          : undefined;
      return new (STRUCT_CLASS_BY_TYPE[200] as typeof Expression)({
        type: Number(objectCson["100"]),
        literal: unpackedLiteral,
        attribute: unpackedAttribute,
        condition: unpackedCondition,
        function: unpackedFunction,
        aggregation: unpackedAggregation,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 200)] = new ExpressionCsonEncoder();

  class SortCsonEncoder implements CsonObjectEncoder {
    packObject(object: Sort): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 205;
      objectCson["100"] = object.type;
      objectCson["101"] = object.by.pack(2);
      if (object.mode != null) {
        objectCson["102"] = object.mode;
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Sort {
      const _Expression = STRUCT_CLASS_BY_TYPE[200] as typeof Expression;
      const modeValue = objectCson["102"];
      const unpackedMode = modeValue != undefined ? Number(modeValue) : undefined;
      return new (STRUCT_CLASS_BY_TYPE[205] as typeof Sort)({
        type: Number(objectCson["100"]),
        by: _Expression.unpack(2, objectCson["101"], _session) as Expression,
        mode: unpackedMode,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 205)] = new SortCsonEncoder();

  class SelectCsonEncoder implements CsonObjectEncoder {
    packObject(object: Select): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 206;
      const packedAttributes: any[] = [];
      for (const item of object.attributes) {
        packedAttributes.push(item.pack(2));
      }
      objectCson["101"] = packedAttributes;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Select {
      const _PropertyReference = STRUCT_CLASS_BY_TYPE[1001] as typeof PropertyReference;
      const unpackedAttributes: any[] = [];
      for (const item of objectCson["101"]) {
        unpackedAttributes.push(_PropertyReference.unpack(2, item, _session) as PropertyReference);
      }
      return new (STRUCT_CLASS_BY_TYPE[206] as typeof Select)({
        attributes: unpackedAttributes,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 206)] = new SelectCsonEncoder();

  class JoinCsonEncoder implements CsonObjectEncoder {
    packObject(object: Join): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 202;
      objectCson["100"] = object.type;
      objectCson["102"] = object.recursive;
      if (object.on != null) {
        objectCson["103"] = object.on.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Join {
      const _Condition = STRUCT_CLASS_BY_TYPE[204] as typeof Condition;
      const onValue = objectCson["103"];
      const unpackedOn =
        onValue != undefined ? (_Condition.unpack(2, onValue, _session) as Condition) : undefined;
      return new (STRUCT_CLASS_BY_TYPE[202] as typeof Join)({
        type: Number(objectCson["100"]),
        recursive: Boolean(objectCson["102"]),
        on: unpackedOn,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 202)] = new JoinCsonEncoder();

  class QueryCsonEncoder implements CsonObjectEncoder {
    packObject(object: Query): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 300;
      objectCson["2"] = object.id;
      objectCson["100"] = object.type;
      objectCson["101"] = object.domain;
      objectCson["105"] = object.name;
      objectCson["106"] = object.definition.pack(2);
      const packedSubqueries: any[] = [];
      for (const item of object.subqueries) {
        packedSubqueries.push(item.pack(2));
      }
      objectCson["109"] = packedSubqueries;
      if (object.join != null) {
        objectCson["110"] = object.join.pack(2);
      }
      if (object.select != null) {
        objectCson["111"] = object.select.pack(2);
      }
      if (object.where != null) {
        objectCson["112"] = object.where.pack(2);
      }
      if (object.having != null) {
        objectCson["113"] = object.having.pack(2);
      }
      const packedGroupBy: any[] = [];
      for (const item of object.groupBy) {
        packedGroupBy.push(item.pack(2));
      }
      objectCson["114"] = packedGroupBy;
      if (object.aggregation != null) {
        objectCson["115"] = object.aggregation.pack(2);
      }
      const packedSort: any[] = [];
      for (const item of object.sort) {
        packedSort.push(item.pack(2));
      }
      objectCson["116"] = packedSort;
      if (object.limit != null) {
        objectCson["120"] = object.limit;
      }
      if (object.offset != null) {
        objectCson["121"] = object.offset;
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Query {
      const _NodeDefinitionReference = STRUCT_CLASS_BY_TYPE[13] as typeof NodeDefinitionReference;
      const _Expression = STRUCT_CLASS_BY_TYPE[200] as typeof Expression;
      const _Join = STRUCT_CLASS_BY_TYPE[202] as typeof Join;
      const _Aggregation = STRUCT_CLASS_BY_TYPE[203] as typeof Aggregation;
      const _Condition = STRUCT_CLASS_BY_TYPE[204] as typeof Condition;
      const _Sort = STRUCT_CLASS_BY_TYPE[205] as typeof Sort;
      const _Select = STRUCT_CLASS_BY_TYPE[206] as typeof Select;
      const _Query = STRUCT_CLASS_BY_TYPE[300] as typeof Query;
      const unpackedSubqueries: any[] = [];
      for (const item of objectCson["109"]) {
        unpackedSubqueries.push(_Query.unpack(2, item, _session) as Query);
      }
      const joinValue = objectCson["110"];
      const unpackedJoin =
        joinValue != undefined ? (_Join.unpack(2, joinValue, _session) as Join) : undefined;
      const selectValue = objectCson["111"];
      const unpackedSelect =
        selectValue != undefined ? (_Select.unpack(2, selectValue, _session) as Select) : undefined;
      const whereValue = objectCson["112"];
      const unpackedWhere =
        whereValue != undefined
          ? (_Condition.unpack(2, whereValue, _session) as Condition)
          : undefined;
      const havingValue = objectCson["113"];
      const unpackedHaving =
        havingValue != undefined
          ? (_Condition.unpack(2, havingValue, _session) as Condition)
          : undefined;
      const unpackedGroupBy: any[] = [];
      for (const item of objectCson["114"]) {
        unpackedGroupBy.push(_Expression.unpack(2, item, _session) as Expression);
      }
      const aggregationValue = objectCson["115"];
      const unpackedAggregation =
        aggregationValue != undefined
          ? (_Aggregation.unpack(2, aggregationValue, _session) as Aggregation)
          : undefined;
      const unpackedSort: any[] = [];
      for (const item of objectCson["116"]) {
        unpackedSort.push(_Sort.unpack(2, item, _session) as Sort);
      }
      const limitValue = objectCson["120"];
      const unpackedLimit = limitValue != undefined ? Number(limitValue) : undefined;
      const offsetValue = objectCson["121"];
      const unpackedOffset = offsetValue != undefined ? Number(offsetValue) : undefined;
      return new (STRUCT_CLASS_BY_TYPE[300] as typeof Query)({
        id: objectCson["2"],
        type: Number(objectCson["100"]),
        domain: Number(objectCson["101"]),
        name: objectCson["105"],
        definition: _NodeDefinitionReference.unpack(
          2,
          objectCson["106"],
          _session,
        ) as NodeDefinitionReference,
        subqueries: unpackedSubqueries,
        join: unpackedJoin,
        select: unpackedSelect,
        where: unpackedWhere,
        having: unpackedHaving,
        groupBy: unpackedGroupBy,
        aggregation: unpackedAggregation,
        sort: unpackedSort,
        limit: unpackedLimit,
        offset: unpackedOffset,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 300)] = new QueryCsonEncoder();

  class TextSpanCsonEncoder implements CsonObjectEncoder {
    packObject(object: TextSpan): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 400021;
      objectCson["100"] = object.type;
      if (object.content != null) {
        objectCson["101"] = object.content;
      }
      if (object.nodePtr != null) {
        objectCson["102"] = object.nodePtr.pack(2);
      }
      if (object.url != null) {
        objectCson["105"] = object.url;
      }
      if (object.isBold != null) {
        objectCson["150"] = object.isBold;
      }
      if (object.isItalic != null) {
        objectCson["151"] = object.isItalic;
      }
      if (object.isStrikethrough != null) {
        objectCson["152"] = object.isStrikethrough;
      }
      if (object.isUnderline != null) {
        objectCson["153"] = object.isUnderline;
      }
      if (object.isCode != null) {
        objectCson["154"] = object.isCode;
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): TextSpan {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const contentValue = objectCson["101"];
      const unpackedContent = contentValue != undefined ? contentValue : undefined;
      const nodePtrValue = objectCson["102"];
      const unpackedNodePtr =
        nodePtrValue != undefined
          ? (_NodeReference.unpack(2, nodePtrValue, _session) as NodeReference)
          : undefined;
      const urlValue = objectCson["105"];
      const unpackedUrl = urlValue != undefined ? urlValue : undefined;
      const isBoldValue = objectCson["150"];
      const unpackedIsBold = isBoldValue != undefined ? Boolean(isBoldValue) : undefined;
      const isItalicValue = objectCson["151"];
      const unpackedIsItalic = isItalicValue != undefined ? Boolean(isItalicValue) : undefined;
      const isStrikethroughValue = objectCson["152"];
      const unpackedIsStrikethrough =
        isStrikethroughValue != undefined ? Boolean(isStrikethroughValue) : undefined;
      const isUnderlineValue = objectCson["153"];
      const unpackedIsUnderline =
        isUnderlineValue != undefined ? Boolean(isUnderlineValue) : undefined;
      const isCodeValue = objectCson["154"];
      const unpackedIsCode = isCodeValue != undefined ? Boolean(isCodeValue) : undefined;
      return new (STRUCT_CLASS_BY_TYPE[400021] as typeof TextSpan)({
        type: Number(objectCson["100"]),
        content: unpackedContent,
        node: unpackedNodePtr,
        url: unpackedUrl,
        isBold: unpackedIsBold,
        isItalic: unpackedIsItalic,
        isStrikethrough: unpackedIsStrikethrough,
        isUnderline: unpackedIsUnderline,
        isCode: unpackedIsCode,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 400021)] = new TextSpanCsonEncoder();

  class TextCsonEncoder implements CsonObjectEncoder {
    packObject(object: Text): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 400020;
      const packedSpans: any[] = [];
      for (const item of object.spans) {
        packedSpans.push(item.pack(2));
      }
      objectCson["103"] = packedSpans;
      if (object.isBold != null) {
        objectCson["150"] = object.isBold;
      }
      if (object.isItalic != null) {
        objectCson["151"] = object.isItalic;
      }
      if (object.isStrikethrough != null) {
        objectCson["152"] = object.isStrikethrough;
      }
      if (object.isUnderline != null) {
        objectCson["153"] = object.isUnderline;
      }
      if (object.isCode != null) {
        objectCson["154"] = object.isCode;
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Text {
      const _TextSpan = STRUCT_CLASS_BY_TYPE[400021] as typeof TextSpan;
      const unpackedSpans: any[] = [];
      for (const item of objectCson["103"]) {
        unpackedSpans.push(_TextSpan.unpack(2, item, _session) as TextSpan);
      }
      const isBoldValue = objectCson["150"];
      const unpackedIsBold = isBoldValue != undefined ? Boolean(isBoldValue) : undefined;
      const isItalicValue = objectCson["151"];
      const unpackedIsItalic = isItalicValue != undefined ? Boolean(isItalicValue) : undefined;
      const isStrikethroughValue = objectCson["152"];
      const unpackedIsStrikethrough =
        isStrikethroughValue != undefined ? Boolean(isStrikethroughValue) : undefined;
      const isUnderlineValue = objectCson["153"];
      const unpackedIsUnderline =
        isUnderlineValue != undefined ? Boolean(isUnderlineValue) : undefined;
      const isCodeValue = objectCson["154"];
      const unpackedIsCode = isCodeValue != undefined ? Boolean(isCodeValue) : undefined;
      return new (STRUCT_CLASS_BY_TYPE[400020] as typeof Text)({
        spans: unpackedSpans,
        isBold: unpackedIsBold,
        isItalic: unpackedIsItalic,
        isStrikethrough: unpackedIsStrikethrough,
        isUnderline: unpackedIsUnderline,
        isCode: unpackedIsCode,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 400020)] = new TextCsonEncoder();

  class ColorCsonEncoder implements CsonObjectEncoder {
    packObject(object: Color): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2100300;
      objectCson["100"] = object.type;
      if (object.stylePtr != null) {
        objectCson["101"] = object.stylePtr.pack(2);
      }
      if (object.x != null) {
        objectCson["105"] = object.x;
      }
      if (object.y != null) {
        objectCson["106"] = object.y;
      }
      if (object.z != null) {
        objectCson["107"] = object.z;
      }
      if (object.alpha != null) {
        objectCson["108"] = object.alpha;
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Color {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const stylePtrValue = objectCson["101"];
      const unpackedStylePtr =
        stylePtrValue != undefined
          ? (_NodeReference.unpack(2, stylePtrValue, _session) as NodeReference)
          : undefined;
      const xValue = objectCson["105"];
      const unpackedX = xValue != undefined ? Number(xValue) : undefined;
      const yValue = objectCson["106"];
      const unpackedY = yValue != undefined ? Number(yValue) : undefined;
      const zValue = objectCson["107"];
      const unpackedZ = zValue != undefined ? Number(zValue) : undefined;
      const alphaValue = objectCson["108"];
      const unpackedAlpha = alphaValue != undefined ? Number(alphaValue) : undefined;
      return new (STRUCT_CLASS_BY_TYPE[2100300] as typeof Color)({
        type: Number(objectCson["100"]),
        style: unpackedStylePtr,
        x: unpackedX,
        y: unpackedY,
        z: unpackedZ,
        alpha: unpackedAlpha,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 2100300)] = new ColorCsonEncoder();

  class BorderCsonEncoder implements CsonObjectEncoder {
    packObject(object: Border): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2100600;
      objectCson["100"] = object.type;
      if (object.color != null) {
        objectCson["101"] = object.color.pack(2);
      }
      if (object.width != null) {
        objectCson["102"] = object.width.pack(2);
      }
      if (object.stylePtr != null) {
        objectCson["103"] = object.stylePtr.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Border {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Color = STRUCT_CLASS_BY_TYPE[2100300] as typeof Color;
      const _Inset2 = STRUCT_CLASS_BY_TYPE[2400023] as typeof Inset2;
      const colorValue = objectCson["101"];
      const unpackedColor =
        colorValue != undefined ? (_Color.unpack(2, colorValue, _session) as Color) : undefined;
      const widthValue = objectCson["102"];
      const unpackedWidth =
        widthValue != undefined ? (_Inset2.unpack(2, widthValue, _session) as Inset2) : undefined;
      const stylePtrValue = objectCson["103"];
      const unpackedStylePtr =
        stylePtrValue != undefined
          ? (_NodeReference.unpack(2, stylePtrValue, _session) as NodeReference)
          : undefined;
      return new (STRUCT_CLASS_BY_TYPE[2100600] as typeof Border)({
        type: Number(objectCson["100"]),
        color: unpackedColor,
        width: unpackedWidth,
        style: unpackedStylePtr,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 2100600)] = new BorderCsonEncoder();

  class GradientStopCsonEncoder implements CsonObjectEncoder {
    packObject(object: GradientStop): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2100801;
      if (object.color != null) {
        objectCson["101"] = object.color.pack(2);
      }
      objectCson["102"] = object.position;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): GradientStop {
      const _Color = STRUCT_CLASS_BY_TYPE[2100300] as typeof Color;
      const colorValue = objectCson["101"];
      const unpackedColor =
        colorValue != undefined ? (_Color.unpack(2, colorValue, _session) as Color) : undefined;
      return new (STRUCT_CLASS_BY_TYPE[2100801] as typeof GradientStop)({
        color: unpackedColor,
        position: Number(objectCson["102"]),
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 2100801)] = new GradientStopCsonEncoder();

  class GradientCsonEncoder implements CsonObjectEncoder {
    packObject(object: Gradient): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2100800;
      objectCson["100"] = object.type;
      if (object.stylePtr != null) {
        objectCson["101"] = object.stylePtr.pack(2);
      }
      if (object.angle != null) {
        objectCson["102"] = object.angle;
      }
      const packedStops: any[] = [];
      for (const item of object.stops) {
        packedStops.push(item.pack(2));
      }
      objectCson["103"] = packedStops;
      if (object.centerAnchor != null) {
        objectCson["104"] = object.centerAnchor.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Gradient {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _GradientStop = STRUCT_CLASS_BY_TYPE[2100801] as typeof GradientStop;
      const _Axis2 = STRUCT_CLASS_BY_TYPE[2400025] as typeof Axis2;
      const stylePtrValue = objectCson["101"];
      const unpackedStylePtr =
        stylePtrValue != undefined
          ? (_NodeReference.unpack(2, stylePtrValue, _session) as NodeReference)
          : undefined;
      const angleValue = objectCson["102"];
      const unpackedAngle = angleValue != undefined ? Number(angleValue) : undefined;
      const unpackedStops: any[] = [];
      for (const item of objectCson["103"]) {
        unpackedStops.push(_GradientStop.unpack(2, item, _session) as GradientStop);
      }
      const centerAnchorValue = objectCson["104"];
      const unpackedCenterAnchor =
        centerAnchorValue != undefined
          ? (_Axis2.unpack(2, centerAnchorValue, _session) as Axis2)
          : undefined;
      return new (STRUCT_CLASS_BY_TYPE[2100800] as typeof Gradient)({
        type: Number(objectCson["100"]),
        style: unpackedStylePtr,
        angle: unpackedAngle,
        stops: unpackedStops,
        centerAnchor: unpackedCenterAnchor,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 2100800)] = new GradientCsonEncoder();

  class FillCsonEncoder implements CsonObjectEncoder {
    packObject(object: Fill): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2100400;
      objectCson["100"] = object.type;
      if (object.stylePtr != null) {
        objectCson["101"] = object.stylePtr.pack(2);
      }
      if (object.color != null) {
        objectCson["102"] = object.color.pack(2);
      }
      if (object.gradient != null) {
        objectCson["103"] = object.gradient.pack(2);
      }
      if (object.imagePtr != null) {
        objectCson["104"] = object.imagePtr.pack(2);
      }
      if (object.position != null) {
        objectCson["105"] = object.position;
      }
      if (object.size != null) {
        objectCson["106"] = object.size;
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Fill {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Color = STRUCT_CLASS_BY_TYPE[2100300] as typeof Color;
      const _Gradient = STRUCT_CLASS_BY_TYPE[2100800] as typeof Gradient;
      const stylePtrValue = objectCson["101"];
      const unpackedStylePtr =
        stylePtrValue != undefined
          ? (_NodeReference.unpack(2, stylePtrValue, _session) as NodeReference)
          : undefined;
      const colorValue = objectCson["102"];
      const unpackedColor =
        colorValue != undefined ? (_Color.unpack(2, colorValue, _session) as Color) : undefined;
      const gradientValue = objectCson["103"];
      const unpackedGradient =
        gradientValue != undefined
          ? (_Gradient.unpack(2, gradientValue, _session) as Gradient)
          : undefined;
      const imagePtrValue = objectCson["104"];
      const unpackedImagePtr =
        imagePtrValue != undefined
          ? (_NodeReference.unpack(2, imagePtrValue, _session) as NodeReference)
          : undefined;
      const positionValue = objectCson["105"];
      const unpackedPosition = positionValue != undefined ? Number(positionValue) : undefined;
      const sizeValue = objectCson["106"];
      const unpackedSize = sizeValue != undefined ? Number(sizeValue) : undefined;
      return new (STRUCT_CLASS_BY_TYPE[2100400] as typeof Fill)({
        type: Number(objectCson["100"]),
        style: unpackedStylePtr,
        color: unpackedColor,
        gradient: unpackedGradient,
        image: unpackedImagePtr,
        position: unpackedPosition,
        size: unpackedSize,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 2100400)] = new FillCsonEncoder();

  class FontCsonEncoder implements CsonObjectEncoder {
    packObject(object: Font): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2100500;
      objectCson["100"] = object.type;
      if (object.stylePtr != null) {
        objectCson["101"] = object.stylePtr.pack(2);
      }
      if (object.weight != null) {
        objectCson["102"] = object.weight;
      }
      if (object.color != null) {
        objectCson["103"] = object.color.pack(2);
      }
      if (object.size != null) {
        objectCson["104"] = object.size;
      }
      if (object.align != null) {
        objectCson["105"] = object.align;
      }
      if (object.lineHeight != null) {
        objectCson["106"] = object.lineHeight.pack(2);
      }
      if (object.letterSpacing != null) {
        objectCson["107"] = object.letterSpacing.pack(2);
      }
      if (object.decoration != null) {
        objectCson["108"] = object.decoration;
      }
      if (object.transform != null) {
        objectCson["109"] = object.transform;
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Font {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Length = STRUCT_CLASS_BY_TYPE[1800001] as typeof Length;
      const _Fill = STRUCT_CLASS_BY_TYPE[2100400] as typeof Fill;
      const stylePtrValue = objectCson["101"];
      const unpackedStylePtr =
        stylePtrValue != undefined
          ? (_NodeReference.unpack(2, stylePtrValue, _session) as NodeReference)
          : undefined;
      const weightValue = objectCson["102"];
      const unpackedWeight = weightValue != undefined ? Number(weightValue) : undefined;
      const colorValue = objectCson["103"];
      const unpackedColor =
        colorValue != undefined ? (_Fill.unpack(2, colorValue, _session) as Fill) : undefined;
      const sizeValue = objectCson["104"];
      const unpackedSize = sizeValue != undefined ? Number(sizeValue) : undefined;
      const alignValue = objectCson["105"];
      const unpackedAlign = alignValue != undefined ? Number(alignValue) : undefined;
      const lineHeightValue = objectCson["106"];
      const unpackedLineHeight =
        lineHeightValue != undefined
          ? (_Length.unpack(2, lineHeightValue, _session) as Length)
          : undefined;
      const letterSpacingValue = objectCson["107"];
      const unpackedLetterSpacing =
        letterSpacingValue != undefined
          ? (_Length.unpack(2, letterSpacingValue, _session) as Length)
          : undefined;
      const decorationValue = objectCson["108"];
      const unpackedDecoration = decorationValue != undefined ? Number(decorationValue) : undefined;
      const transformValue = objectCson["109"];
      const unpackedTransform = transformValue != undefined ? Number(transformValue) : undefined;
      return new (STRUCT_CLASS_BY_TYPE[2100500] as typeof Font)({
        type: Number(objectCson["100"]),
        style: unpackedStylePtr,
        weight: unpackedWeight,
        color: unpackedColor,
        size: unpackedSize,
        align: unpackedAlign,
        lineHeight: unpackedLineHeight,
        letterSpacing: unpackedLetterSpacing,
        decoration: unpackedDecoration,
        transform: unpackedTransform,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 2100500)] = new FontCsonEncoder();

  class ShadowCsonEncoder implements CsonObjectEncoder {
    packObject(object: Shadow): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2100700;
      objectCson["100"] = object.type;
      if (object.stylePtr != null) {
        objectCson["101"] = object.stylePtr.pack(2);
      }
      if (object.color != null) {
        objectCson["102"] = object.color.pack(2);
      }
      objectCson["103"] = object.position;
      if (object.offset != null) {
        objectCson["104"] = object.offset.pack(2);
      }
      if (object.blur != null) {
        objectCson["105"] = object.blur;
      }
      if (object.spread != null) {
        objectCson["106"] = object.spread;
      }
      if (object.diffusion != null) {
        objectCson["107"] = object.diffusion;
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Shadow {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Color = STRUCT_CLASS_BY_TYPE[2100300] as typeof Color;
      const _Axis2 = STRUCT_CLASS_BY_TYPE[2400025] as typeof Axis2;
      const stylePtrValue = objectCson["101"];
      const unpackedStylePtr =
        stylePtrValue != undefined
          ? (_NodeReference.unpack(2, stylePtrValue, _session) as NodeReference)
          : undefined;
      const colorValue = objectCson["102"];
      const unpackedColor =
        colorValue != undefined ? (_Color.unpack(2, colorValue, _session) as Color) : undefined;
      const offsetValue = objectCson["104"];
      const unpackedOffset =
        offsetValue != undefined ? (_Axis2.unpack(2, offsetValue, _session) as Axis2) : undefined;
      const blurValue = objectCson["105"];
      const unpackedBlur = blurValue != undefined ? Number(blurValue) : undefined;
      const spreadValue = objectCson["106"];
      const unpackedSpread = spreadValue != undefined ? Number(spreadValue) : undefined;
      const diffusionValue = objectCson["107"];
      const unpackedDiffusion = diffusionValue != undefined ? Number(diffusionValue) : undefined;
      return new (STRUCT_CLASS_BY_TYPE[2100700] as typeof Shadow)({
        type: Number(objectCson["100"]),
        style: unpackedStylePtr,
        color: unpackedColor,
        position: Number(objectCson["103"]),
        offset: unpackedOffset,
        blur: unpackedBlur,
        spread: unpackedSpread,
        diffusion: unpackedDiffusion,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 2100700)] = new ShadowCsonEncoder();

  class StrokeCsonEncoder implements CsonObjectEncoder {
    packObject(object: Stroke): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2101100;
      objectCson["100"] = object.type;
      objectCson["101"] = object.size;
      objectCson["102"] = object.thinning;
      objectCson["103"] = object.smoothing;
      objectCson["104"] = object.streamline;
      objectCson["105"] = object.easing;
      if (object.color != null) {
        objectCson["106"] = object.color.pack(2);
      }
      if (object.start != null) {
        objectCson["110"] = object.start.pack(2);
      }
      if (object.end != null) {
        objectCson["111"] = object.end.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Stroke {
      const _Color = STRUCT_CLASS_BY_TYPE[2100300] as typeof Color;
      const _StrokeCap = STRUCT_CLASS_BY_TYPE[2101101] as typeof StrokeCap;
      const colorValue = objectCson["106"];
      const unpackedColor =
        colorValue != undefined ? (_Color.unpack(2, colorValue, _session) as Color) : undefined;
      const startValue = objectCson["110"];
      const unpackedStart =
        startValue != undefined
          ? (_StrokeCap.unpack(2, startValue, _session) as StrokeCap)
          : undefined;
      const endValue = objectCson["111"];
      const unpackedEnd =
        endValue != undefined ? (_StrokeCap.unpack(2, endValue, _session) as StrokeCap) : undefined;
      return new (STRUCT_CLASS_BY_TYPE[2101100] as typeof Stroke)({
        type: Number(objectCson["100"]),
        size: Number(objectCson["101"]),
        thinning: Number(objectCson["102"]),
        smoothing: Number(objectCson["103"]),
        streamline: Number(objectCson["104"]),
        easing: Number(objectCson["105"]),
        color: unpackedColor,
        start: unpackedStart,
        end: unpackedEnd,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 2101100)] = new StrokeCsonEncoder();

  class StrokeCapCsonEncoder implements CsonObjectEncoder {
    packObject(object: StrokeCap): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2101101;
      objectCson["101"] = object.cap;
      objectCson["102"] = object.taper;
      objectCson["103"] = object.easing;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): StrokeCap {
      return new (STRUCT_CLASS_BY_TYPE[2101101] as typeof StrokeCap)({
        cap: Boolean(objectCson["101"]),
        taper: Boolean(objectCson["102"]),
        easing: Number(objectCson["103"]),
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 2101101)] = new StrokeCapCsonEncoder();

  class StrokePointCsonEncoder implements CsonObjectEncoder {
    packObject(object: StrokePoint): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2101103;
      objectCson["101"] = object.point.pack(2);
      objectCson["102"] = object.originalPoint.pack(2);
      objectCson["103"] = object.pressure;
      objectCson["104"] = object.direction.pack(2);
      objectCson["105"] = object.distance;
      objectCson["106"] = object.runningLength;
      objectCson["107"] = object.radius;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): StrokePoint {
      const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
      return new (STRUCT_CLASS_BY_TYPE[2101103] as typeof StrokePoint)({
        point: _Vector2.unpack(2, objectCson["101"], _session) as Vector2,
        originalPoint: _Vector2.unpack(2, objectCson["102"], _session) as Vector2,
        pressure: Number(objectCson["103"]),
        direction: _Vector2.unpack(2, objectCson["104"], _session) as Vector2,
        distance: Number(objectCson["105"]),
        runningLength: Number(objectCson["106"]),
        radius: Number(objectCson["107"]),
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 2101103)] = new StrokePointCsonEncoder();

  class StrokePathCsonEncoder implements CsonObjectEncoder {
    packObject(object: StrokePath): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2101102;
      const packedPoints: any[] = [];
      for (const item of object.points) {
        packedPoints.push(item.pack(2));
      }
      objectCson["101"] = packedPoints;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): StrokePath {
      const _StrokePoint = STRUCT_CLASS_BY_TYPE[2101103] as typeof StrokePoint;
      const unpackedPoints: any[] = [];
      for (const item of objectCson["101"]) {
        unpackedPoints.push(_StrokePoint.unpack(2, item, _session) as StrokePoint);
      }
      return new (STRUCT_CLASS_BY_TYPE[2101102] as typeof StrokePath)({
        points: unpackedPoints,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 2101102)] = new StrokePathCsonEncoder();

  class TransitionCsonEncoder implements CsonObjectEncoder {
    packObject(object: Transition): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2200000;
      objectCson["100"] = object.type;
      if (object.stylePtr != null) {
        objectCson["101"] = object.stylePtr.pack(2);
      }
      if (object.delay != null) {
        objectCson["102"] = object.delay;
      }
      if (object.duration != null) {
        objectCson["103"] = object.duration;
      }
      const packedEase: any[] = [];
      for (const item of object.ease) {
        packedEase.push(item);
      }
      objectCson["104"] = packedEase;
      if (object.stiffness != null) {
        objectCson["105"] = object.stiffness;
      }
      if (object.damping != null) {
        objectCson["106"] = object.damping;
      }
      if (object.mass != null) {
        objectCson["107"] = object.mass;
      }
      if (object.bounce != null) {
        objectCson["108"] = object.bounce;
      }
      if (object.springType != null) {
        objectCson["109"] = object.springType;
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Transition {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const stylePtrValue = objectCson["101"];
      const unpackedStylePtr =
        stylePtrValue != undefined
          ? (_NodeReference.unpack(2, stylePtrValue, _session) as NodeReference)
          : undefined;
      const delayValue = objectCson["102"];
      const unpackedDelay = delayValue != undefined ? Number(delayValue) : undefined;
      const durationValue = objectCson["103"];
      const unpackedDuration = durationValue != undefined ? Number(durationValue) : undefined;
      const unpackedEase: any[] = [];
      for (const item of objectCson["104"]) {
        unpackedEase.push(Number(item));
      }
      const stiffnessValue = objectCson["105"];
      const unpackedStiffness = stiffnessValue != undefined ? Number(stiffnessValue) : undefined;
      const dampingValue = objectCson["106"];
      const unpackedDamping = dampingValue != undefined ? Number(dampingValue) : undefined;
      const massValue = objectCson["107"];
      const unpackedMass = massValue != undefined ? Number(massValue) : undefined;
      const bounceValue = objectCson["108"];
      const unpackedBounce = bounceValue != undefined ? Number(bounceValue) : undefined;
      const springTypeValue = objectCson["109"];
      const unpackedSpringType = springTypeValue != undefined ? Number(springTypeValue) : undefined;
      return new (STRUCT_CLASS_BY_TYPE[2200000] as typeof Transition)({
        type: Number(objectCson["100"]),
        style: unpackedStylePtr,
        delay: unpackedDelay,
        duration: unpackedDuration,
        ease: unpackedEase,
        stiffness: unpackedStiffness,
        damping: unpackedDamping,
        mass: unpackedMass,
        bounce: unpackedBounce,
        springType: unpackedSpringType,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 2200000)] = new TransitionCsonEncoder();

  class EffectCsonEncoder implements CsonObjectEncoder {
    packObject(object: Effect): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2200100;
      objectCson["100"] = object.type;
      if (object.stylePtr != null) {
        objectCson["101"] = object.stylePtr.pack(2);
      }
      if (object.opacity != null) {
        objectCson["102"] = object.opacity;
      }
      if (object.offset != null) {
        objectCson["103"] = object.offset.pack(2);
      }
      if (object.scale != null) {
        objectCson["104"] = object.scale;
      }
      if (object.rotate != null) {
        objectCson["105"] = object.rotate.pack(2);
      }
      if (object.skew != null) {
        objectCson["106"] = object.skew.pack(2);
      }
      if (object.perspective != null) {
        objectCson["107"] = object.perspective;
      }
      if (object.delay != null) {
        objectCson["108"] = timedeltaToISOFormat(object.delay);
      }
      if (object.duration != null) {
        objectCson["109"] = object.duration;
      }
      if (object.threshold != null) {
        objectCson["110"] = object.threshold;
      }
      if (object.once != null) {
        objectCson["111"] = object.once;
      }
      if (object.repeat != null) {
        objectCson["112"] = object.repeat;
      }
      if (object.split != null) {
        objectCson["113"] = object.split;
      }
      if (object.offscreen != null) {
        objectCson["114"] = object.offscreen;
      }
      if (object.transition != null) {
        objectCson["115"] = object.transition.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Effect {
      const _NodeReference = STRUCT_CLASS_BY_TYPE[1000] as typeof NodeReference;
      const _Transition = STRUCT_CLASS_BY_TYPE[2200000] as typeof Transition;
      const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
      const _Axis3 = STRUCT_CLASS_BY_TYPE[2400026] as typeof Axis3;
      const stylePtrValue = objectCson["101"];
      const unpackedStylePtr =
        stylePtrValue != undefined
          ? (_NodeReference.unpack(2, stylePtrValue, _session) as NodeReference)
          : undefined;
      const opacityValue = objectCson["102"];
      const unpackedOpacity = opacityValue != undefined ? Number(opacityValue) : undefined;
      const offsetValue = objectCson["103"];
      const unpackedOffset =
        offsetValue != undefined
          ? (_Vector2.unpack(2, offsetValue, _session) as Vector2)
          : undefined;
      const scaleValue = objectCson["104"];
      const unpackedScale = scaleValue != undefined ? Number(scaleValue) : undefined;
      const rotateValue = objectCson["105"];
      const unpackedRotate =
        rotateValue != undefined ? (_Axis3.unpack(2, rotateValue, _session) as Axis3) : undefined;
      const skewValue = objectCson["106"];
      const unpackedSkew =
        skewValue != undefined ? (_Vector2.unpack(2, skewValue, _session) as Vector2) : undefined;
      const perspectiveValue = objectCson["107"];
      const unpackedPerspective =
        perspectiveValue != undefined ? Number(perspectiveValue) : undefined;
      const delayValue = objectCson["108"];
      const unpackedDelay =
        delayValue != undefined ? timedeltaFromISOFormat(delayValue) : undefined;
      const durationValue = objectCson["109"];
      const unpackedDuration = durationValue != undefined ? Number(durationValue) : undefined;
      const thresholdValue = objectCson["110"];
      const unpackedThreshold = thresholdValue != undefined ? Number(thresholdValue) : undefined;
      const onceValue = objectCson["111"];
      const unpackedOnce = onceValue != undefined ? Boolean(onceValue) : undefined;
      const repeatValue = objectCson["112"];
      const unpackedRepeat = repeatValue != undefined ? Number(repeatValue) : undefined;
      const splitValue = objectCson["113"];
      const unpackedSplit = splitValue != undefined ? Number(splitValue) : undefined;
      const offscreenValue = objectCson["114"];
      const unpackedOffscreen = offscreenValue != undefined ? Number(offscreenValue) : undefined;
      const transitionValue = objectCson["115"];
      const unpackedTransition =
        transitionValue != undefined
          ? (_Transition.unpack(2, transitionValue, _session) as Transition)
          : undefined;
      return new (STRUCT_CLASS_BY_TYPE[2200100] as typeof Effect)({
        type: Number(objectCson["100"]),
        style: unpackedStylePtr,
        opacity: unpackedOpacity,
        offset: unpackedOffset,
        scale: unpackedScale,
        rotate: unpackedRotate,
        skew: unpackedSkew,
        perspective: unpackedPerspective,
        delay: unpackedDelay,
        duration: unpackedDuration,
        threshold: unpackedThreshold,
        once: unpackedOnce,
        repeat: unpackedRepeat,
        split: unpackedSplit,
        offscreen: unpackedOffscreen,
        transition: unpackedTransition,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 2200100)] = new EffectCsonEncoder();

  class Arrow2DCsonEncoder implements CsonObjectEncoder {
    packObject(object: Arrow2D): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2411200;
      objectCson["200"] = object.startType;
      objectCson["201"] = object.start.pack(2);
      objectCson["210"] = object.endType;
      objectCson["211"] = object.end.pack(2);
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Arrow2D {
      const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
      return new (STRUCT_CLASS_BY_TYPE[2411200] as typeof Arrow2D)({
        startType: Number(objectCson["200"]),
        start: _Vector2.unpack(2, objectCson["201"], _session) as Vector2,
        endType: Number(objectCson["210"]),
        end: _Vector2.unpack(2, objectCson["211"], _session) as Vector2,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 2411200)] = new Arrow2DCsonEncoder();

  class Ellipse2DCsonEncoder implements CsonObjectEncoder {
    packObject(object: Ellipse2D): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2411400;
      if (object.stroke != null) {
        objectCson["200"] = object.stroke.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Ellipse2D {
      const _Stroke = STRUCT_CLASS_BY_TYPE[2101100] as typeof Stroke;
      const strokeValue = objectCson["200"];
      const unpackedStroke =
        strokeValue != undefined ? (_Stroke.unpack(2, strokeValue, _session) as Stroke) : undefined;
      return new (STRUCT_CLASS_BY_TYPE[2411400] as typeof Ellipse2D)({
        stroke: unpackedStroke,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 2411400)] = new Ellipse2DCsonEncoder();

  class Line2DCsonEncoder implements CsonObjectEncoder {
    packObject(object: Line2D): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2411100;
      if (object.stroke != null) {
        objectCson["200"] = object.stroke.pack(2);
      }
      objectCson["210"] = object.start.pack(2);
      objectCson["220"] = object.end.pack(2);
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Line2D {
      const _Stroke = STRUCT_CLASS_BY_TYPE[2101100] as typeof Stroke;
      const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
      const strokeValue = objectCson["200"];
      const unpackedStroke =
        strokeValue != undefined ? (_Stroke.unpack(2, strokeValue, _session) as Stroke) : undefined;
      return new (STRUCT_CLASS_BY_TYPE[2411100] as typeof Line2D)({
        stroke: unpackedStroke,
        start: _Vector2.unpack(2, objectCson["210"], _session) as Vector2,
        end: _Vector2.unpack(2, objectCson["220"], _session) as Vector2,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 2411100)] = new Line2DCsonEncoder();

  class Path2DCsonEncoder implements CsonObjectEncoder {
    packObject(object: Path2D): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2411600;
      if (object.stroke != null) {
        objectCson["200"] = object.stroke.pack(2);
      }
      const packedPoints: any[] = [];
      for (const item of object.points) {
        packedPoints.push(item.pack(2));
      }
      objectCson["210"] = packedPoints;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Path2D {
      const _Stroke = STRUCT_CLASS_BY_TYPE[2101100] as typeof Stroke;
      const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
      const strokeValue = objectCson["200"];
      const unpackedStroke =
        strokeValue != undefined ? (_Stroke.unpack(2, strokeValue, _session) as Stroke) : undefined;
      const unpackedPoints: any[] = [];
      for (const item of objectCson["210"]) {
        unpackedPoints.push(_Vector2.unpack(2, item, _session) as Vector2);
      }
      return new (STRUCT_CLASS_BY_TYPE[2411600] as typeof Path2D)({
        stroke: unpackedStroke,
        points: unpackedPoints,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 2411600)] = new Path2DCsonEncoder();

  class Polygon2DCsonEncoder implements CsonObjectEncoder {
    packObject(object: Polygon2D): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2411500;
      if (object.stroke != null) {
        objectCson["200"] = object.stroke.pack(2);
      }
      const packedPoints: any[] = [];
      for (const item of object.points) {
        packedPoints.push(item.pack(2));
      }
      objectCson["210"] = packedPoints;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Polygon2D {
      const _Stroke = STRUCT_CLASS_BY_TYPE[2101100] as typeof Stroke;
      const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
      const strokeValue = objectCson["200"];
      const unpackedStroke =
        strokeValue != undefined ? (_Stroke.unpack(2, strokeValue, _session) as Stroke) : undefined;
      const unpackedPoints: any[] = [];
      for (const item of objectCson["210"]) {
        unpackedPoints.push(_Vector2.unpack(2, item, _session) as Vector2);
      }
      return new (STRUCT_CLASS_BY_TYPE[2411500] as typeof Polygon2D)({
        stroke: unpackedStroke,
        points: unpackedPoints,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 2411500)] = new Polygon2DCsonEncoder();

  class Vector2CsonEncoder implements CsonObjectEncoder {
    packObject(object: Vector2): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2400000;
      objectCson["101"] = object.x;
      objectCson["102"] = object.y;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Vector2 {
      return new (STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2)({
        x: Number(objectCson["101"]),
        y: Number(objectCson["102"]),
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 2400000)] = new Vector2CsonEncoder();

  class Vector3CsonEncoder implements CsonObjectEncoder {
    packObject(object: Vector3): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2400002;
      objectCson["101"] = object.x;
      objectCson["102"] = object.y;
      objectCson["103"] = object.z;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Vector3 {
      return new (STRUCT_CLASS_BY_TYPE[2400002] as typeof Vector3)({
        x: Number(objectCson["101"]),
        y: Number(objectCson["102"]),
        z: Number(objectCson["103"]),
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 2400002)] = new Vector3CsonEncoder();

  class Vector4CsonEncoder implements CsonObjectEncoder {
    packObject(object: Vector4): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2400004;
      objectCson["101"] = object.x;
      objectCson["102"] = object.y;
      objectCson["103"] = object.z;
      objectCson["104"] = object.w;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Vector4 {
      return new (STRUCT_CLASS_BY_TYPE[2400004] as typeof Vector4)({
        x: Number(objectCson["101"]),
        y: Number(objectCson["102"]),
        z: Number(objectCson["103"]),
        w: Number(objectCson["104"]),
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 2400004)] = new Vector4CsonEncoder();

  class Vector2iCsonEncoder implements CsonObjectEncoder {
    packObject(object: Vector2i): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2400001;
      objectCson["101"] = object.x;
      objectCson["102"] = object.y;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Vector2i {
      return new (STRUCT_CLASS_BY_TYPE[2400001] as typeof Vector2i)({
        x: Number(objectCson["101"]),
        y: Number(objectCson["102"]),
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 2400001)] = new Vector2iCsonEncoder();

  class Vector3iCsonEncoder implements CsonObjectEncoder {
    packObject(object: Vector3i): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2400003;
      objectCson["101"] = object.x;
      objectCson["102"] = object.y;
      objectCson["103"] = object.z;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Vector3i {
      return new (STRUCT_CLASS_BY_TYPE[2400003] as typeof Vector3i)({
        x: Number(objectCson["101"]),
        y: Number(objectCson["102"]),
        z: Number(objectCson["103"]),
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 2400003)] = new Vector3iCsonEncoder();

  class Vector4iCsonEncoder implements CsonObjectEncoder {
    packObject(object: Vector4i): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2400005;
      objectCson["101"] = object.x;
      objectCson["102"] = object.y;
      objectCson["103"] = object.z;
      objectCson["104"] = object.w;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Vector4i {
      return new (STRUCT_CLASS_BY_TYPE[2400005] as typeof Vector4i)({
        x: Number(objectCson["101"]),
        y: Number(objectCson["102"]),
        z: Number(objectCson["103"]),
        w: Number(objectCson["104"]),
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 2400005)] = new Vector4iCsonEncoder();

  class QuaternionCsonEncoder implements CsonObjectEncoder {
    packObject(object: Quaternion): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2400010;
      objectCson["101"] = object.x;
      objectCson["102"] = object.y;
      objectCson["103"] = object.z;
      objectCson["104"] = object.w;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Quaternion {
      return new (STRUCT_CLASS_BY_TYPE[2400010] as typeof Quaternion)({
        x: Number(objectCson["101"]),
        y: Number(objectCson["102"]),
        z: Number(objectCson["103"]),
        w: Number(objectCson["104"]),
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 2400010)] = new QuaternionCsonEncoder();

  class Rectangle2DCsonEncoder implements CsonObjectEncoder {
    packObject(object: Rectangle2D): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2411300;
      if (object.stroke != null) {
        objectCson["200"] = object.stroke.pack(2);
      }
      if (object.width != null) {
        objectCson["210"] = object.width.pack(2);
      }
      if (object.height != null) {
        objectCson["220"] = object.height.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Rectangle2D {
      const _Stroke = STRUCT_CLASS_BY_TYPE[2101100] as typeof Stroke;
      const _Vector2 = STRUCT_CLASS_BY_TYPE[2400000] as typeof Vector2;
      const strokeValue = objectCson["200"];
      const unpackedStroke =
        strokeValue != undefined ? (_Stroke.unpack(2, strokeValue, _session) as Stroke) : undefined;
      const widthValue = objectCson["210"];
      const unpackedWidth =
        widthValue != undefined ? (_Vector2.unpack(2, widthValue, _session) as Vector2) : undefined;
      const heightValue = objectCson["220"];
      const unpackedHeight =
        heightValue != undefined
          ? (_Vector2.unpack(2, heightValue, _session) as Vector2)
          : undefined;
      return new (STRUCT_CLASS_BY_TYPE[2411300] as typeof Rectangle2D)({
        stroke: unpackedStroke,
        width: unpackedWidth,
        height: unpackedHeight,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 2411300)] = new Rectangle2DCsonEncoder();

  class LengthCsonEncoder implements CsonObjectEncoder {
    packObject(object: Length): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 1800001;
      objectCson["101"] = object.unit;
      objectCson["102"] = object.value;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Length {
      return new (STRUCT_CLASS_BY_TYPE[1800001] as typeof Length)({
        unit: Number(objectCson["101"]),
        value: Number(objectCson["102"]),
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 1800001)] = new LengthCsonEncoder();

  class Offset2CsonEncoder implements CsonObjectEncoder {
    packObject(object: Offset2): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2400020;
      objectCson["100"] = object.type;
      if (object.top != null) {
        objectCson["101"] = object.top.pack(2);
      }
      if (object.left != null) {
        objectCson["102"] = object.left.pack(2);
      }
      if (object.width != null) {
        objectCson["103"] = object.width.pack(2);
      }
      if (object.height != null) {
        objectCson["104"] = object.height.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Offset2 {
      const _Length = STRUCT_CLASS_BY_TYPE[1800001] as typeof Length;
      const topValue = objectCson["101"];
      const unpackedTop =
        topValue != undefined ? (_Length.unpack(2, topValue, _session) as Length) : undefined;
      const leftValue = objectCson["102"];
      const unpackedLeft =
        leftValue != undefined ? (_Length.unpack(2, leftValue, _session) as Length) : undefined;
      const widthValue = objectCson["103"];
      const unpackedWidth =
        widthValue != undefined ? (_Length.unpack(2, widthValue, _session) as Length) : undefined;
      const heightValue = objectCson["104"];
      const unpackedHeight =
        heightValue != undefined ? (_Length.unpack(2, heightValue, _session) as Length) : undefined;
      return new (STRUCT_CLASS_BY_TYPE[2400020] as typeof Offset2)({
        type: Number(objectCson["100"]),
        top: unpackedTop,
        left: unpackedLeft,
        width: unpackedWidth,
        height: unpackedHeight,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 2400020)] = new Offset2CsonEncoder();

  class Inset2CsonEncoder implements CsonObjectEncoder {
    packObject(object: Inset2): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2400023;
      objectCson["101"] = object.base;
      if (object.top != null) {
        objectCson["102"] = object.top;
      }
      if (object.left != null) {
        objectCson["103"] = object.left;
      }
      if (object.right != null) {
        objectCson["104"] = object.right;
      }
      if (object.bottom != null) {
        objectCson["105"] = object.bottom;
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Inset2 {
      const topValue = objectCson["102"];
      const unpackedTop = topValue != undefined ? Number(topValue) : undefined;
      const leftValue = objectCson["103"];
      const unpackedLeft = leftValue != undefined ? Number(leftValue) : undefined;
      const rightValue = objectCson["104"];
      const unpackedRight = rightValue != undefined ? Number(rightValue) : undefined;
      const bottomValue = objectCson["105"];
      const unpackedBottom = bottomValue != undefined ? Number(bottomValue) : undefined;
      return new (STRUCT_CLASS_BY_TYPE[2400023] as typeof Inset2)({
        base: Number(objectCson["101"]),
        top: unpackedTop,
        left: unpackedLeft,
        right: unpackedRight,
        bottom: unpackedBottom,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 2400023)] = new Inset2CsonEncoder();

  class Corner2CsonEncoder implements CsonObjectEncoder {
    packObject(object: Corner2): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2400024;
      objectCson["101"] = object.base;
      if (object.topLeft != null) {
        objectCson["102"] = object.topLeft;
      }
      if (object.topRight != null) {
        objectCson["103"] = object.topRight;
      }
      if (object.bottomLeft != null) {
        objectCson["104"] = object.bottomLeft;
      }
      if (object.bottomRight != null) {
        objectCson["105"] = object.bottomRight;
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Corner2 {
      const topLeftValue = objectCson["102"];
      const unpackedTopLeft = topLeftValue != undefined ? Number(topLeftValue) : undefined;
      const topRightValue = objectCson["103"];
      const unpackedTopRight = topRightValue != undefined ? Number(topRightValue) : undefined;
      const bottomLeftValue = objectCson["104"];
      const unpackedBottomLeft = bottomLeftValue != undefined ? Number(bottomLeftValue) : undefined;
      const bottomRightValue = objectCson["105"];
      const unpackedBottomRight =
        bottomRightValue != undefined ? Number(bottomRightValue) : undefined;
      return new (STRUCT_CLASS_BY_TYPE[2400024] as typeof Corner2)({
        base: Number(objectCson["101"]),
        topLeft: unpackedTopLeft,
        topRight: unpackedTopRight,
        bottomLeft: unpackedBottomLeft,
        bottomRight: unpackedBottomRight,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 2400024)] = new Corner2CsonEncoder();

  class Axis2CsonEncoder implements CsonObjectEncoder {
    packObject(object: Axis2): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2400025;
      objectCson["101"] = object.base;
      if (object.x != null) {
        objectCson["102"] = object.x;
      }
      if (object.y != null) {
        objectCson["103"] = object.y;
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Axis2 {
      const xValue = objectCson["102"];
      const unpackedX = xValue != undefined ? Number(xValue) : undefined;
      const yValue = objectCson["103"];
      const unpackedY = yValue != undefined ? Number(yValue) : undefined;
      return new (STRUCT_CLASS_BY_TYPE[2400025] as typeof Axis2)({
        base: Number(objectCson["101"]),
        x: unpackedX,
        y: unpackedY,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 2400025)] = new Axis2CsonEncoder();

  class Axis3CsonEncoder implements CsonObjectEncoder {
    packObject(object: Axis3): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2400026;
      objectCson["101"] = object.base;
      if (object.x != null) {
        objectCson["102"] = object.x;
      }
      if (object.y != null) {
        objectCson["103"] = object.y;
      }
      if (object.z != null) {
        objectCson["104"] = object.z;
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Axis3 {
      const xValue = objectCson["102"];
      const unpackedX = xValue != undefined ? Number(xValue) : undefined;
      const yValue = objectCson["103"];
      const unpackedY = yValue != undefined ? Number(yValue) : undefined;
      const zValue = objectCson["104"];
      const unpackedZ = zValue != undefined ? Number(zValue) : undefined;
      return new (STRUCT_CLASS_BY_TYPE[2400026] as typeof Axis3)({
        base: Number(objectCson["101"]),
        x: unpackedX,
        y: unpackedY,
        z: unpackedZ,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 2400026)] = new Axis3CsonEncoder();

  class Grid2CsonEncoder implements CsonObjectEncoder {
    packObject(object: Grid2): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2400021;
      objectCson["101"] = object.columns;
      objectCson["102"] = object.rows;
      if (object.columnWidth != null) {
        objectCson["103"] = object.columnWidth.pack(2);
      }
      if (object.columnMinWidth != null) {
        objectCson["104"] = object.columnMinWidth.pack(2);
      }
      if (object.rowHeight != null) {
        objectCson["105"] = object.rowHeight.pack(2);
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Grid2 {
      const _Length = STRUCT_CLASS_BY_TYPE[1800001] as typeof Length;
      const columnWidthValue = objectCson["103"];
      const unpackedColumnWidth =
        columnWidthValue != undefined
          ? (_Length.unpack(2, columnWidthValue, _session) as Length)
          : undefined;
      const columnMinWidthValue = objectCson["104"];
      const unpackedColumnMinWidth =
        columnMinWidthValue != undefined
          ? (_Length.unpack(2, columnMinWidthValue, _session) as Length)
          : undefined;
      const rowHeightValue = objectCson["105"];
      const unpackedRowHeight =
        rowHeightValue != undefined
          ? (_Length.unpack(2, rowHeightValue, _session) as Length)
          : undefined;
      return new (STRUCT_CLASS_BY_TYPE[2400021] as typeof Grid2)({
        columns: Number(objectCson["101"]),
        rows: Number(objectCson["102"]),
        columnWidth: unpackedColumnWidth,
        columnMinWidth: unpackedColumnMinWidth,
        rowHeight: unpackedRowHeight,
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 2400021)] = new Grid2CsonEncoder();

  class GridSpan2CsonEncoder implements CsonObjectEncoder {
    packObject(object: GridSpan2): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 2400022;
      objectCson["101"] = object.columns;
      objectCson["102"] = object.rows;
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): GridSpan2 {
      return new (STRUCT_CLASS_BY_TYPE[2400022] as typeof GridSpan2)({
        columns: Number(objectCson["101"]),
        rows: Number(objectCson["102"]),
        _packedCache: [{ encoding: 2, isBytes: false, packed: objectCson }],
        _session,
      });
    }
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 2400022)] = new GridSpan2CsonEncoder();

  class ScheduleCsonEncoder implements CsonObjectEncoder {
    packObject(object: Schedule): any {
      const objectCson: { [key: string]: any } = {};
      objectCson["1"] = 700001;
      objectCson["101"] = object.frequency;
      objectCson["102"] = object.interval;
      if (object.start != null) {
        objectCson["110"] = object.start.toString({ timeZoneName: "never" });
      }
      if (object.end != null) {
        objectCson["111"] = object.end.toString({ timeZoneName: "never" });
      }
      if (object.count != null) {
        objectCson["112"] = object.count;
      }
      if (object.weekStart != null) {
        objectCson["113"] = object.weekStart;
      }
      if (object.bySetPos != null) {
        const packedBySetPos: any[] = [];
        for (const item of object.bySetPos) {
          packedBySetPos.push(item);
        }
        objectCson["114"] = packedBySetPos;
      }
      if (object.byMonth != null) {
        const packedByMonth: any[] = [];
        for (const item of object.byMonth) {
          packedByMonth.push(item);
        }
        objectCson["115"] = packedByMonth;
      }
      if (object.byMonthDay != null) {
        const packedByMonthDay: any[] = [];
        for (const item of object.byMonthDay) {
          packedByMonthDay.push(item);
        }
        objectCson["116"] = packedByMonthDay;
      }
      if (object.byYearDay != null) {
        const packedByYearDay: any[] = [];
        for (const item of object.byYearDay) {
          packedByYearDay.push(item);
        }
        objectCson["117"] = packedByYearDay;
      }
      if (object.byEaster != null) {
        const packedByEaster: any[] = [];
        for (const item of object.byEaster) {
          packedByEaster.push(item);
        }
        objectCson["118"] = packedByEaster;
      }
      if (object.byWeekNo != null) {
        const packedByWeekNo: any[] = [];
        for (const item of object.byWeekNo) {
          packedByWeekNo.push(item);
        }
        objectCson["119"] = packedByWeekNo;
      }
      if (object.byWeekDay != null) {
        const packedByWeekDay: any[] = [];
        for (const item of object.byWeekDay) {
          packedByWeekDay.push(item);
        }
        objectCson["120"] = packedByWeekDay;
      }
      if (object.byHour != null) {
        const packedByHour: any[] = [];
        for (const item of object.byHour) {
          packedByHour.push(item);
        }
        objectCson["121"] = packedByHour;
      }
      if (object.byMinute != null) {
        const packedByMinute: any[] = [];
        for (const item of object.byMinute) {
          packedByMinute.push(item);
        }
        objectCson["122"] = packedByMinute;
      }
      if (object.bySecond != null) {
        const packedBySecond: any[] = [];
        for (const item of object.bySecond) {
          packedBySecond.push(item);
        }
        objectCson["123"] = packedBySecond;
      }
      return objectCson;
    }

    unpackObject(objectCson: any, _session: Session | null): Schedule {
      const startValue = objectCson["110"];
      const unpackedStart =
        startValue != undefined
          ? Temporal.Instant.from(startValue).toZonedDateTimeISO("UTC")
          : undefined;
      const endValue = objectCson["111"];
      const unpackedEnd =
        endValue != undefined
          ? Temporal.Instant.from(endValue).toZonedDateTimeISO("UTC")
          : undefined;
      const countValue = objectCson["112"];
      const unpackedCount = countValue != undefined ? Number(countValue) : undefined;
      const weekStartValue = objectCson["113"];
      const unpackedWeekStart = weekStartValue != undefined ? Number(weekStartValue) : undefined;
      let unpackedBySetPos: any[] | undefined;
      if (objectCson["114"] != undefined) {
        unpackedBySetPos = [];
        for (const item of objectCson["114"]) {
          unpackedBySetPos.push(Number(item));
        }
      } else {
        unpackedBySetPos = undefined;
      }
      let unpackedByMonth: any[] | undefined;
      if (objectCson["115"] != undefined) {
        unpackedByMonth = [];
        for (const item of objectCson["115"]) {
          unpackedByMonth.push(Number(item));
        }
      } else {
        unpackedByMonth = undefined;
      }
      let unpackedByMonthDay: any[] | undefined;
      if (objectCson["116"] != undefined) {
        unpackedByMonthDay = [];
        for (const item of objectCson["116"]) {
          unpackedByMonthDay.push(Number(item));
        }
      } else {
        unpackedByMonthDay = undefined;
      }
      let unpackedByYearDay: any[] | undefined;
      if (objectCson["117"] != undefined) {
        unpackedByYearDay = [];
        for (const item of objectCson["117"]) {
          unpackedByYearDay.push(Number(item));
        }
      } else {
        unpackedByYearDay = undefined;
      }
      let unpackedByEaster: any[] | undefined;
      if (objectCson["118"] != undefined) {
        unpackedByEaster = [];
        for (const item of objectCson["118"]) {
          unpackedByEaster.push(Number(item));
        }
      } else {
        unpackedByEaster = undefined;
      }
      let unpackedByWeekNo: any[] | undefined;
      if (objectCson["119"] != undefined) {
        unpackedByWeekNo = [];
        for (const item of objectCson["119"]) {
          unpackedByWeekNo.push(Number(item));
        }
      } else {
        unpackedByWeekNo = undefined;
      }
      let unpackedByWeekDay: any[] | undefined;
      if (objectCson["120"] != undefined) {
        unpackedByWeekDay = [];
        for (const item of objectCson["120"]) {
          unpackedByWeekDay.push(Number(item));
        }
      } else {
        unpackedByWeekDay = undefined;
      }
      let unpackedByHour: any[] | undefined;
      if (objectCson["121"] != undefined) {
        unpackedByHour = [];
        for (const item of objectCson["121"]) {
          unpackedByHour.push(Number(item));
        }
      } else {
        unpackedByHour = undefined;
      }
      let unpackedByMinute: any[] | undefined;
      if (objectCson["122"] != undefined) {
        unpackedByMinute = [];
        for (const item of objectCson["122"]) {
          unpackedByMinute.push(Number(item));
        }
      } else {
        unpackedByMinute = undefined;
      }
      let unpackedBySecond: any[] | undefined;
      if (objectCson["123"] != undefined) {
        unpackedBySecond = [];
        for (const item of objectCson["123"]) {
          unpackedBySecond.push(Number(item));
        }
      } else {
        unpackedBySecond = undefined;
      }
      return new (STRUCT_CLASS_BY_TYPE[700001] as typeof Schedule)({
        frequency: Number(objectCson["101"]),
        interval: Number(objectCson["102"]),
        start: unpackedStart,
        end: unpackedEnd,
        count: unpackedCount,
        weekStart: unpackedWeekStart,
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
  }

  CSON_OBJECT_ENCODERS[getObjectKey(2, 700001)] = new ScheduleCsonEncoder();
}

loadEncoders();
