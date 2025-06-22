import { Folder, Session, Timer, Snapshot, Option, ScreenCursor, ShadowStyle, Interruption, NodeType, Action, Window, EffectStyle, Trigger, EventCursor, SliderInputView, HistogramMetric, Value, CustomView, LabelView, Environment, ThreadView, Message, TransitionStyle, Space, CustomViewDefinition, Script, EnumType, Client, Layer, TextView, ColorStyle, FriendshipInviteEvent, Field, InviteEvent, Agent, Tagging, Follow, CustomEventDefinition, AnnotationShape, activeSession, StructType, RunEvent, CustomEvent, Role, Branch, HistogramMeasurement, NotificationEvent, WizardView, TriggerEvent, Variant, SceneEvent, Origin, EditEvent, Handle, Reaction, RoleEvent, CustomStructDefinition, Star, NumberInputView, Route, GaugeMetric, CustomEntityDefinition, File, ACTIVE_SESSION, LineShape, ThreadCursor, Machine, Tag, SanctionEvent, FrameView, Friendship, Supergraph, Database, Thread, FontStyle, Link, User, Team, MembershipEvent, ArrowShape, BorderStyle, EntitlementEvent, SplitView, Node, CounterMeasurement, PlaneShape, BuiltinObject, Organization, Scene, StructFrozen, GradientStyle, Entitlement, Graph, Service, Struct, Permission, Sanction, Canvas, Invite, Span, Palette, QueryConnection, GaugeMeasurement, NodeReference, CustomEnumDefinition, CounterMetric, TimerEvent, Membership, PropertyReference, Run, Theme, CustomEntity, FriendshipInvite, Notification, Log, FillStyle } from '@/language';
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

  constructor(options: {
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
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(supergraph);
    this.type = options.type;
    this.operation = options.operation ?? null;
    this.nodePtr = options.node != null ? (options.node.metatype == StructType.NODE_REFERENCE ? (options.node as NodeReference) : (options.node as Node).toRef()) : null;
    this.propPtr = options.propPtr ?? null;
    this.fieldPtr = options.field != null ? (options.field.metatype == StructType.NODE_REFERENCE ? (options.field as NodeReference) : (options.field as Node).toRef()) : null;
    this.key = options.key ?? null;
    this.value = options.value ?? null;
    this.undo = options.undo ?? null;
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

  constructor(options: {
    name?: string | null,
    edits?: Array<Edit>,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(supergraph);
    this.name = options.name ?? null;
    this.edits = options.edits ?? [];
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

  constructor(options: {
    status: ChangeStatus,
    edits?: Array<Edit>,
    cascadedEdits?: Array<Edit>,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(supergraph);
    this.status = options.status;
    this.edits = options.edits ?? [];
    this.cascadedEdits = options.cascadedEdits ?? [];
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