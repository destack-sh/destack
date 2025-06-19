import { CustomEnumDefinition, TypeCardinality, Interruption, Team, Follow, EntitlementEvent, ScalarType, LineShape, CustomEventDefinition, ThreadView, DefaultFactory, Action, HistogramMeasurement, CounterMetric, FillStyle, Struct, SliderInputView, SplitView, Membership, FriendshipInvite, ArrowShape, Scene, Database, FontStyle, Option, GaugeMetric, Value, Service, Canvas, PlaneShape, NotificationEvent, Client, Agent, Folder, CustomStructDefinition, NodeType, File, Sanction, GradientStyle, TriggerEvent, HistogramMetric, Supergraph, Notification, TraitType, Link, WizardView, Graph, User, Friendship, Thread, Snapshot, ThreadCursor, Permission, Layer, CustomEntityDefinition, CustomEvent, EventCursor, CustomViewDefinition, Trigger, FrameView, StructType, Palette, TextView, EditEvent, CounterMeasurement, EffectStyle, Machine, ColorStyle, Message, Session, Variant, Log, Role, CustomEntity, SceneEvent, BorderStyle, ShadowStyle, Reaction, Star, TransitionStyle, Handle, CustomView, SanctionEvent, Field, Route, BuiltinObject, Tagging, Timer, Script, EnumType, Invite, StructFrozen, NumberInputView, InviteEvent, QueryConnection, Entitlement, FriendshipInviteEvent, Branch, AnnotationShape, RunEvent, Node, LabelView, Organization, TimerEvent, Tag, ScreenCursor, Span, GaugeMeasurement, PrimitiveType, RoleEvent, Theme, Environment, Window, MembershipEvent, Run, NodeReference, Space } from '@/language';
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

  constructor(
    format: StringFormat | null,
    regex: string | null,
    startsWith: string | null,
    endsWith: string | null,
    _supergraph: Supergraph | null
  ) {
    super(_supergraph);
    this.format = format;
    this.regex = regex;
    this.startsWith = startsWith;
    this.endsWith = endsWith;
  }


  static create(options: {
    format?: StringFormat | null,
    regex?: string | null,
    startsWith?: string | null,
    endsWith?: string | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }): StringConstraint {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new StringConstraint(
      options.format ?? null,
      options.regex ?? null,
      options.startsWith ?? null,
      options.endsWith ?? null,
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
/* ==== DESTACK_GENERATED_END:STRUCT:2503 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2502 ==== */
export class NumberConstraint extends StructFrozen {
  readonly format: NumberFormat | null;
  readonly minValue: number | null;
  readonly maxValue: number | null;
  readonly stepValue: number | null;
  readonly precision: number | null;
  readonly scale: number | null;

  constructor(
    format: NumberFormat | null,
    minValue: number | null,
    maxValue: number | null,
    stepValue: number | null,
    precision: number | null,
    scale: number | null,
    _supergraph: Supergraph | null
  ) {
    super(_supergraph);
    this.format = format;
    this.minValue = minValue;
    this.maxValue = maxValue;
    this.stepValue = stepValue;
    this.precision = precision;
    this.scale = scale;
  }


  static create(options: {
    format?: NumberFormat | null,
    minValue?: number | null,
    maxValue?: number | null,
    stepValue?: number | null,
    precision?: number | null,
    scale?: number | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }): NumberConstraint {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new NumberConstraint(
      options.format ?? null,
      options.minValue ?? null,
      options.maxValue ?? null,
      options.stepValue ?? null,
      options.precision ?? null,
      options.scale ?? null,
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
/* ==== DESTACK_GENERATED_END:STRUCT:2502 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2504 ==== */
export class CollectionConstraint extends StructFrozen {
  readonly minLength: number | null;
  readonly maxLength: number | null;

  constructor(
    minLength: number | null,
    maxLength: number | null,
    _supergraph: Supergraph | null
  ) {
    super(_supergraph);
    this.minLength = minLength;
    this.maxLength = maxLength;
  }


  static create(options: {
    minLength?: number | null,
    maxLength?: number | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }): CollectionConstraint {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new CollectionConstraint(
      options.minLength ?? null,
      options.maxLength ?? null,
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
/* ==== DESTACK_GENERATED_END:STRUCT:2504 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2505 ==== */
export class NodeConstraint extends StructFrozen {
  readonly nodeTypes: Array<NodeType>;
  readonly nodeTraits: Array<TraitType>;

  constructor(
    nodeTypes: Array<NodeType>,
    nodeTraits: Array<TraitType>,
    _supergraph: Supergraph | null
  ) {
    super(_supergraph);
    this.nodeTypes = nodeTypes;
    this.nodeTraits = nodeTraits;
  }


  static create(options: {
    nodeTypes?: Array<NodeType>,
    nodeTraits?: Array<TraitType>,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }): NodeConstraint {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new NodeConstraint(
      options.nodeTypes ?? [],
      options.nodeTraits ?? [],
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

  constructor(
    cardinality: TypeCardinality,
    scalarType: ScalarType,
    primitiveType: PrimitiveType | null,
    enumType: EnumType | null,
    nodeType: NodeType | null,
    nodeDefinitionPtr: NodeReference | null,
    structType: StructType | null,
    baseTypePtr: NodeReference | null,
    keyType: Type | null,
    isRequired: boolean | null,
    isVariable: boolean | null,
    defaultValue: Value | null,
    defaultFactory: DefaultFactory | null,
    collectionConstraint: CollectionConstraint | null,
    stringConstraint: StringConstraint | null,
    numberConstraint: NumberConstraint | null,
    nodeConstraint: NodeConstraint | null,
    _supergraph: Supergraph | null
  ) {
    super(_supergraph);
    this.cardinality = cardinality;
    this.scalarType = scalarType;
    this.primitiveType = primitiveType;
    this.enumType = enumType;
    this.nodeType = nodeType;
    this.nodeDefinitionPtr = nodeDefinitionPtr;
    this.structType = structType;
    this.baseTypePtr = baseTypePtr;
    this.keyType = keyType;
    this.isRequired = isRequired;
    this.isVariable = isVariable;
    this.defaultValue = defaultValue;
    this.defaultFactory = defaultFactory;
    this.collectionConstraint = collectionConstraint;
    this.stringConstraint = stringConstraint;
    this.numberConstraint = numberConstraint;
    this.nodeConstraint = nodeConstraint;
  }


  static create(options: {
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
  }): Type {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new Type(
      options.cardinality ?? TypeCardinality.SCALAR,
      options.scalarType,
      options.primitiveType ?? null,
      options.enumType ?? null,
      options.nodeType ?? null,
      options.nodeDefinition != null ? (options.nodeDefinition.metatype == StructType.NODE_REFERENCE ? options.nodeDefinition : options.nodeDefinition.toRef()) : null,
      options.structType ?? null,
      options.baseType != null ? (options.baseType.metatype == StructType.NODE_REFERENCE ? options.baseType : options.baseType.toRef()) : null,
      options.keyType ?? null,
      options.isRequired ?? null,
      options.isVariable ?? null,
      options.defaultValue ?? null,
      options.defaultFactory ?? null,
      options.collectionConstraint ?? null,
      options.stringConstraint ?? null,
      options.numberConstraint ?? null,
      options.nodeConstraint ?? null,
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
/* ==== DESTACK_GENERATED_END:STRUCT:2501 ==== */