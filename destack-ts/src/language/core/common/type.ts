import { FriendshipInvite, HistogramMeasurement, CounterMeasurement, InviteEvent, Organization, Environment, BorderStyle, EnumType, Invite, Field, FillStyle, BuiltinObject, Branch, Agent, FontStyle, PlaneShape, AnnotationShape, Scene, SliderInputView, Role, Trigger, Theme, NotificationEvent, Route, ThreadView, SplitView, TraitType, RoleEvent, Script, ScreenCursor, Action, Session, ShadowStyle, CustomView, Layer, User, Handle, Timer, Star, CustomEntityDefinition, Graph, CustomStructDefinition, HistogramMetric, WizardView, FrameView, LabelView, ThreadCursor, Database, Thread, Variant, TextView, RunEvent, GradientStyle, Log, TypeCardinality, QueryConnection, Node, Client, ScalarType, PrimitiveType, Reaction, Notification, EntitlementEvent, Friendship, SceneEvent, Canvas, GaugeMeasurement, Value, ArrowShape, Folder, TriggerEvent, Tagging, Tag, CustomEntity, Membership, Span, Follow, Interruption, SanctionEvent, Entitlement, Service, Palette, NumberInputView, Team, File, StructType, Permission, Link, TransitionStyle, DefaultFactory, ColorStyle, MembershipEvent, Sanction, CustomViewDefinition, CounterMetric, Snapshot, CustomEnumDefinition, NodeReference, Message, Space, Run, Window, Option, TimerEvent, CustomEventDefinition, LineShape, EffectStyle, Machine, Struct, Supergraph, CustomEvent, GaugeMetric, NodeType, EventCursor, EditEvent, FriendshipInviteEvent } from '@/language';
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
export class StringConstraint extends BuiltinObject {
  readonly format: StringFormat | null;
  readonly regex: string | null;
  readonly startsWith: string | null;
  readonly endsWith: string | null;

  constructor(
    format: StringFormat | null,
    regex: string | null,
    startsWith: string | null,
    endsWith: string | null,
    _supergraph: Supergraph
  ) {
    super(_supergraph);
    this.format = format;
    this.regex = regex;
    this.startsWith = startsWith;
    this.endsWith = endsWith;
  }


  static create(): StringConstraint {

    return new StringConstraint();
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
export class NumberConstraint extends BuiltinObject {
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
    _supergraph: Supergraph
  ) {
    super(_supergraph);
    this.format = format;
    this.minValue = minValue;
    this.maxValue = maxValue;
    this.stepValue = stepValue;
    this.precision = precision;
    this.scale = scale;
  }


  static create(): NumberConstraint {

    return new NumberConstraint();
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
export class CollectionConstraint extends BuiltinObject {
  readonly minLength: number | null;
  readonly maxLength: number | null;

  constructor(
    minLength: number | null,
    maxLength: number | null,
    _supergraph: Supergraph
  ) {
    super(_supergraph);
    this.minLength = minLength;
    this.maxLength = maxLength;
  }


  static create(): CollectionConstraint {

    return new CollectionConstraint();
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
export class NodeConstraint extends BuiltinObject {
  readonly nodeTypes: Array<NodeType>;
  readonly nodeTraits: Array<TraitType>;

  constructor(
    nodeTypes: Array<NodeType>,
    nodeTraits: Array<TraitType>,
    _supergraph: Supergraph
  ) {
    super(_supergraph);
    this.nodeTypes = nodeTypes;
    this.nodeTraits = nodeTraits;
  }


  static create(): NodeConstraint {

    return new NodeConstraint();
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
export class Type extends BuiltinObject {
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
          return this._supergraph.get(nodePtr.id);
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
          return this._supergraph.get(nodePtr.id);
      }
      return null;
  }
  ;
  baseTypePtr: NodeReference | null
  readonly keyType: Type | null;
  readonly isRequired: boolean | null;
  readonly isVariable: boolean | null;
  readonly default: Value | null;
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
    default: Value | null,
    defaultFactory: DefaultFactory | null,
    collectionConstraint: CollectionConstraint | null,
    stringConstraint: StringConstraint | null,
    numberConstraint: NumberConstraint | null,
    nodeConstraint: NodeConstraint | null,
    _supergraph: Supergraph
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
    this.default = default;
    this.defaultFactory = defaultFactory;
    this.collectionConstraint = collectionConstraint;
    this.stringConstraint = stringConstraint;
    this.numberConstraint = numberConstraint;
    this.nodeConstraint = nodeConstraint;
  }


  static create(): Type {

    return new Type();
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