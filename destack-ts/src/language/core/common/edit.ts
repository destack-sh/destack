import { FriendshipInvite, HistogramMeasurement, CounterMeasurement, InviteEvent, Organization, Environment, BorderStyle, Invite, Field, FillStyle, BuiltinObject, Branch, Agent, FontStyle, PlaneShape, AnnotationShape, Scene, SliderInputView, Role, Trigger, Theme, NotificationEvent, Route, ThreadView, SplitView, RoleEvent, Script, ScreenCursor, Action, Session, ShadowStyle, CustomView, Layer, User, Handle, Timer, Star, CustomEntityDefinition, Graph, CustomStructDefinition, HistogramMetric, WizardView, FrameView, LabelView, ThreadCursor, Origin, Database, Thread, Variant, TextView, RunEvent, GradientStyle, Log, QueryConnection, Node, Client, Reaction, Notification, EntitlementEvent, Friendship, SceneEvent, Canvas, GaugeMeasurement, Value, ArrowShape, Folder, TriggerEvent, Tagging, Tag, CustomEntity, Membership, Span, Follow, Interruption, SanctionEvent, Entitlement, Service, Palette, NumberInputView, Team, File, Permission, Link, TransitionStyle, ColorStyle, MembershipEvent, Sanction, PropertyReference, CustomViewDefinition, CounterMetric, Snapshot, CustomEnumDefinition, NodeReference, Message, Space, Run, Window, Option, TimerEvent, CustomEventDefinition, LineShape, EffectStyle, Machine, Struct, Supergraph, CustomEvent, GaugeMetric, NodeType, EventCursor, EditEvent, FriendshipInviteEvent } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:ENUM:50050 ==== */
export enum EditType {
  CREATE = 1,
  UPSERT = 2,
  UPDATE = 3,
  MOVE = 4,
  ARCHIVE = 5,
  UNARCHIVE = 6,
  DELETE = 7,
  RESTORE = 8,
  ERASE = 9,
}
/* ==== DESTACK_GENERATED_END:ENUM:50050 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:50051 ==== */
export enum EditOperation {
  SET = 1,
  CLEAR = 2,
}
/* ==== DESTACK_GENERATED_END:ENUM:50051 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:50052 ==== */
export enum ChangeStatus {
  COMPLETED = 10,
  FAILED = 12,
  REJECTED = 13,
}
/* ==== DESTACK_GENERATED_END:ENUM:50052 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:50053 ==== */
export enum ChangeDebounce {
  LAZY = 10,
}
/* ==== DESTACK_GENERATED_END:ENUM:50053 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50020 ==== */
export class Edit extends BuiltinObject {
  readonly id: string;
  readonly type: EditType;
  readonly operation: EditOperation | null;
  get node(): Node | null {
      const nodePtr: NodeReference | null = this.nodePtr;
      if (nodePtr !== null) {
          if (this._supergraph === null) {
              return null;
          }
          return this._supergraph.get(nodePtr.id);
      }
      return null;
  }
  ;
  nodePtr: NodeReference
  readonly propPtr: PropertyReference | null;
  get field(): Field | null {
      const nodePtr: NodeReference | null = this.fieldPtr;
      if (nodePtr !== null) {
          if (this._supergraph === null) {
              return null;
          }
          return this._supergraph.get(nodePtr.id);
      }
      return null;
  }
  ;
  fieldPtr: NodeReference | null
  readonly key: Value | null;
  readonly value: Value | null;
  readonly undo: Edit | null;

  constructor(
    id: string,
    type: EditType,
    operation: EditOperation | null,
    nodePtr: NodeReference,
    propPtr: PropertyReference | null,
    fieldPtr: NodeReference | null,
    key: Value | null,
    value: Value | null,
    undo: Edit | null,
    _supergraph: Supergraph
  ) {
    super(_supergraph);
    this.id = id;
    this.type = type;
    this.operation = operation;
    this.nodePtr = nodePtr;
    this.propPtr = propPtr;
    this.fieldPtr = fieldPtr;
    this.key = key;
    this.value = value;
    this.undo = undo;
  }


  static create(): Edit {

    return new Edit();
  }

  equals(other: any): boolean {
    throw new Error("Not implemented");
  }

  hash(): number {
    throw new Error("Not implemented");
  }

  validate(): void {
    throw new Error("Not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:50020 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50021 ==== */
export class Change extends BuiltinObject {
  readonly id: string;
  readonly name: string | null;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): Agent | User | null {
      const nodePtr: NodeReference | null = this.createdByPtr;
      if (nodePtr !== null) {
          if (this._supergraph === null) {
              return null;
          }
          return this._supergraph.get(nodePtr.id);
      }
      return null;
  }
  ;
  createdByPtr: NodeReference | null
  readonly origin: Origin | null;
  readonly debounce: ChangeDebounce | null;
  readonly edits: Array<Edit>;

  constructor(
    id: string,
    name: string | null,
    createdAt: Temporal.ZonedDateTime,
    createdByPtr: NodeReference | null,
    origin: Origin | null,
    debounce: ChangeDebounce | null,
    edits: Array<Edit>,
    _supergraph: Supergraph
  ) {
    super(_supergraph);
    this.id = id;
    this.name = name;
    this.createdAt = createdAt;
    this.createdByPtr = createdByPtr;
    this.origin = origin;
    this.debounce = debounce;
    this.edits = edits;
  }


  static create(): Change {

    return new Change();
  }

  equals(other: any): boolean {
    throw new Error("Not implemented");
  }

  hash(): number {
    throw new Error("Not implemented");
  }

  validate(): void {
    throw new Error("Not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:50021 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50022 ==== */
export class ChangeResult extends BuiltinObject {
  readonly id: string;
  readonly createdAt: Temporal.ZonedDateTime;
  readonly debounce: ChangeDebounce | null;
  readonly status: ChangeStatus;
  readonly edits: Array<Edit>;
  readonly cascadedEdits: Array<Edit>;

  constructor(
    id: string,
    createdAt: Temporal.ZonedDateTime,
    debounce: ChangeDebounce | null,
    status: ChangeStatus,
    edits: Array<Edit>,
    cascadedEdits: Array<Edit>,
    _supergraph: Supergraph
  ) {
    super(_supergraph);
    this.id = id;
    this.createdAt = createdAt;
    this.debounce = debounce;
    this.status = status;
    this.edits = edits;
    this.cascadedEdits = cascadedEdits;
  }


  static create(): ChangeResult {

    return new ChangeResult();
  }

  equals(other: any): boolean {
    throw new Error("Not implemented");
  }

  hash(): number {
    throw new Error("Not implemented");
  }

  validate(): void {
    throw new Error("Not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:50022 ==== */