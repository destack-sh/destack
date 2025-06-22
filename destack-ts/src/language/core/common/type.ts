import { Folder, Session, Timer, Snapshot, Option, ScreenCursor, ShadowStyle, Interruption, NodeType, Action, Window, EffectStyle, Trigger, EventCursor, SliderInputView, HistogramMetric, Value, CustomView, LabelView, Environment, ThreadView, Message, TransitionStyle, Space, CustomViewDefinition, Script, EnumType, Client, Layer, TextView, ColorStyle, FriendshipInviteEvent, Field, InviteEvent, Agent, Tagging, Follow, CustomEventDefinition, AnnotationShape, activeSession, StructType, RunEvent, CustomEvent, Role, Branch, HistogramMeasurement, NotificationEvent, WizardView, TriggerEvent, Variant, SceneEvent, EditEvent, Handle, Reaction, RoleEvent, CustomStructDefinition, Star, NumberInputView, Route, GaugeMetric, CustomEntityDefinition, File, ACTIVE_SESSION, DefaultFactory, LineShape, ThreadCursor, PrimitiveType, Machine, ScalarType, SanctionEvent, FrameView, Friendship, Supergraph, Database, Thread, FontStyle, Link, User, TraitType, Team, MembershipEvent, ArrowShape, BorderStyle, EntitlementEvent, SplitView, TypeCardinality, Node, CounterMeasurement, PlaneShape, BuiltinObject, Organization, Scene, StructFrozen, GradientStyle, Entitlement, Graph, Service, Struct, Permission, Sanction, Log, Canvas, Invite, Span, Palette, QueryConnection, GaugeMeasurement, NodeReference, CustomEnumDefinition, CounterMetric, TimerEvent, Membership, Run, Theme, CustomEntity, FriendshipInvite, Notification, Tag, FillStyle } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:ENUM:2570 ==== */
export enum StringFormat {
  NAME = 1,
  SLUG = 2,
  EMAIL = 3,
  UUID = 10,
  URL = 11,
  EMOJI = 12,
  MIME = 13,
  BASE64 = 20,
}
/* ==== DESTACK_GENERATED_END:ENUM:2570 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:2571 ==== */
export enum NumberFormat {
  PERCENTAGE = 1,
  ANGLE = 2,
  CURRENCY = 3,
}
/* ==== DESTACK_GENERATED_END:ENUM:2571 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2503 ==== */
export class StringConstraint extends StructFrozen {
  readonly format: StringFormat | null;
  readonly regex: string | null;
  readonly startsWith: string | null;
  readonly endsWith: string | null;

  constructor(options: {
    format?: StringFormat | null,
    regex?: string | null,
    startsWith?: string | null,
    endsWith?: string | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(supergraph);
    this.format = options.format ?? null;
    this.regex = options.regex ?? null;
    this.startsWith = options.startsWith ?? null;
    this.endsWith = options.endsWith ?? null;
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
/* ==== DESTACK_GENERATED_END:STRUCT:2503 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2502 ==== */
export class NumberConstraint extends StructFrozen {
  readonly format: NumberFormat | null;
  readonly minValue: number | null;
  readonly maxValue: number | null;
  readonly stepValue: number | null;
  readonly precision: number | null;
  readonly scale: number | null;

  constructor(options: {
    format?: NumberFormat | null,
    minValue?: number | null,
    maxValue?: number | null,
    stepValue?: number | null,
    precision?: number | null,
    scale?: number | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(supergraph);
    this.format = options.format ?? null;
    this.minValue = options.minValue ?? null;
    this.maxValue = options.maxValue ?? null;
    this.stepValue = options.stepValue ?? null;
    this.precision = options.precision ?? null;
    this.scale = options.scale ?? null;
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
/* ==== DESTACK_GENERATED_END:STRUCT:2502 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2504 ==== */
export class CollectionConstraint extends StructFrozen {
  readonly minLength: number | null;
  readonly maxLength: number | null;

  constructor(options: {
    minLength?: number | null,
    maxLength?: number | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(supergraph);
    this.minLength = options.minLength ?? null;
    this.maxLength = options.maxLength ?? null;
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
/* ==== DESTACK_GENERATED_END:STRUCT:2504 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2505 ==== */
export class NodeConstraint extends StructFrozen {
  readonly nodeTypes: Array<NodeType>;
  readonly nodeTraits: Array<TraitType>;

  constructor(options: {
    nodeTypes?: Array<NodeType>,
    nodeTraits?: Array<TraitType>,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(supergraph);
    this.nodeTypes = options.nodeTypes ?? [];
    this.nodeTraits = options.nodeTraits ?? [];
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
/* ==== DESTACK_GENERATED_END:STRUCT:2505 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2501 ==== */
export class Type extends StructFrozen {
  readonly cardinality: TypeCardinality;
  readonly scalarType: ScalarType;
  readonly primitiveType: PrimitiveType | null;
  readonly enumType: EnumType | null;
  readonly nodeType: NodeType | null;
  get nodeDefinition(): CustomEntityDefinition | null {
      const nodePtr: NodeReference | null = this.nodeDefinitionPtr;
      if (nodePtr !== null) {
          if (this._supergraph === null) {
              return null;
          }
          return this._supergraph.get(nodePtr.id) as CustomEntityDefinition | null;
      }
      return null;
  }
  ;
  nodeDefinitionPtr: NodeReference | null
  readonly structType: StructType | null;
  get baseType(): Node | null {
      const nodePtr: NodeReference | null = this.baseTypePtr;
      if (nodePtr !== null) {
          if (this._supergraph === null) {
              return null;
          }
          return this._supergraph.get(nodePtr.id) as Node | null;
      }
      return null;
  }
  ;
  baseTypePtr: NodeReference | null
  readonly keyType: Type | null;
  readonly isRequired: boolean | null;
  readonly isVariable: boolean | null;
  readonly defaultValue: Value | null;
  readonly defaultFactory: DefaultFactory | null;
  readonly collectionConstraint: CollectionConstraint | null;
  readonly stringConstraint: StringConstraint | null;
  readonly numberConstraint: NumberConstraint | null;
  readonly nodeConstraint: NodeConstraint | null;

  constructor(options: {
    cardinality?: TypeCardinality,
    scalarType: ScalarType,
    primitiveType?: PrimitiveType | null,
    enumType?: EnumType | null,
    nodeType?: NodeType | null,
    nodeDefinition?: CustomEntityDefinition | NodeReference | null,
    structType?: StructType | null,
    baseType?: Node | NodeReference | null,
    keyType?: Type | null,
    isRequired?: boolean | null,
    isVariable?: boolean | null,
    defaultValue?: Value | null,
    defaultFactory?: DefaultFactory | null,
    collectionConstraint?: CollectionConstraint | null,
    stringConstraint?: StringConstraint | null,
    numberConstraint?: NumberConstraint | null,
    nodeConstraint?: NodeConstraint | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(supergraph);
    this.cardinality = options.cardinality ?? TypeCardinality.SCALAR;
    this.scalarType = options.scalarType;
    this.primitiveType = options.primitiveType ?? null;
    this.enumType = options.enumType ?? null;
    this.nodeType = options.nodeType ?? null;
    this.nodeDefinitionPtr = options.nodeDefinition != null ? (options.nodeDefinition.metatype == StructType.NODE_REFERENCE ? (options.nodeDefinition as NodeReference) : (options.nodeDefinition as Node).toRef()) : null;
    this.structType = options.structType ?? null;
    this.baseTypePtr = options.baseType != null ? (options.baseType.metatype == StructType.NODE_REFERENCE ? (options.baseType as NodeReference) : (options.baseType as Node).toRef()) : null;
    this.keyType = options.keyType ?? null;
    this.isRequired = options.isRequired ?? null;
    this.isVariable = options.isVariable ?? null;
    this.defaultValue = options.defaultValue ?? null;
    this.defaultFactory = options.defaultFactory ?? null;
    this.collectionConstraint = options.collectionConstraint ?? null;
    this.stringConstraint = options.stringConstraint ?? null;
    this.numberConstraint = options.numberConstraint ?? null;
    this.nodeConstraint = options.nodeConstraint ?? null;
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
/* ==== DESTACK_GENERATED_END:STRUCT:2501 ==== */