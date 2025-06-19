import { CustomEnumDefinition, Interruption, Team, Follow, EntitlementEvent, LineShape, CustomEventDefinition, ThreadView, Action, HistogramMeasurement, CounterMetric, FillStyle, Struct, SliderInputView, SplitView, Membership, FriendshipInvite, ArrowShape, Scene, Database, FontStyle, Option, GaugeMetric, Value, Service, Canvas, PlaneShape, NotificationEvent, Client, Agent, Folder, CustomStructDefinition, NodeType, File, Sanction, GradientStyle, TriggerEvent, HistogramMetric, Supergraph, Notification, Link, WizardView, Graph, User, Friendship, Thread, Snapshot, ThreadCursor, PropertyReference, Permission, Layer, Origin, CustomEntityDefinition, CustomEvent, EventCursor, CustomViewDefinition, Trigger, FrameView, StructType, Palette, TextView, EditEvent, CounterMeasurement, EffectStyle, Machine, ColorStyle, Message, Session, Variant, Log, Role, CustomEntity, SceneEvent, BorderStyle, ShadowStyle, Reaction, Star, TransitionStyle, Handle, CustomView, SanctionEvent, Field, Route, BuiltinObject, Tagging, Timer, Script, EnumType, Invite, StructFrozen, NumberInputView, InviteEvent, QueryConnection, Entitlement, FriendshipInviteEvent, Branch, AnnotationShape, RunEvent, Node, LabelView, Organization, TimerEvent, Tag, ScreenCursor, Span, GaugeMeasurement, RoleEvent, Theme, Environment, Window, MembershipEvent, Run, NodeReference, Space } from '@/language';
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
export class Edit extends StructFrozen {
  readonly id: string;
  readonly type: EditType;
  readonly operation: EditOperation | null;
  get node(): Node {
      const nodePtr: NodeReference | null = this.nodePtr;
      if (nodePtr !== null) {
          if (this._supergraph === null) {
              return null;
          }
          return this._supergraph.get(nodePtr.id) as Node;
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
          return this._supergraph.get(nodePtr.id) as Field | null;
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
    _supergraph: Supergraph | null
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


  static create(options: {
    type: EditType,
    operation?: EditOperation | null,
    node: Node | NodeReference,
    propPtr?: PropertyReference | null,
    field?: Field | NodeReference | null,
    key?: Value | null,
    value?: Value | null,
    undo?: Edit | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }): Edit {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new Edit(
      options.type,
      options.operation ?? null,
      options.node != null ? (options.node.metatype == StructType.NODE_REFERENCE ? options.node : options.node.toRef()) : null,
      options.propPtr ?? null,
      options.field != null ? (options.field.metatype == StructType.NODE_REFERENCE ? options.field : options.field.toRef()) : null,
      options.key ?? null,
      options.value ?? null,
      options.undo ?? null,
      supergraph
    );
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
export class Change extends StructFrozen {
  readonly id: string;
  readonly name: string | null;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): Agent | User | null {
      const nodePtr: NodeReference | null = this.createdByPtr;
      if (nodePtr !== null) {
          if (this._supergraph === null) {
              return null;
          }
          return this._supergraph.get(nodePtr.id) as Agent | User | null;
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
    _supergraph: Supergraph | null
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


  static create(options: {
    name?: string | null,
    edits?: Array<Edit>,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }): Change {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new Change(
      options.name ?? null,
      options.edits ?? [],
      supergraph
    );
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
export class ChangeResult extends StructFrozen {
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
    _supergraph: Supergraph | null
  ) {
    super(_supergraph);
    this.id = id;
    this.createdAt = createdAt;
    this.debounce = debounce;
    this.status = status;
    this.edits = edits;
    this.cascadedEdits = cascadedEdits;
  }


  static create(options: {
    status: ChangeStatus,
    edits?: Array<Edit>,
    cascadedEdits?: Array<Edit>,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }): ChangeResult {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new ChangeResult(
      options.status,
      options.edits ?? [],
      options.cascadedEdits ?? [],
      supergraph
    );
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