import { CustomEnumDefinition, Interruption, Team, Follow, EntitlementEvent, LineShape, CustomEventDefinition, ThreadView, Action, HistogramMeasurement, CounterMetric, FillStyle, Struct, SliderInputView, SplitView, Membership, FriendshipInvite, ArrowShape, Scene, Database, FontStyle, Option, GaugeMetric, Service, Canvas, PlaneShape, NotificationEvent, Client, Agent, Folder, CustomStructDefinition, NodeType, File, Sanction, GradientStyle, TriggerEvent, HistogramMetric, Supergraph, Notification, Link, WizardView, Graph, User, Friendship, Thread, Snapshot, ThreadCursor, Permission, Layer, CustomEntityDefinition, CustomEvent, EventCursor, CustomViewDefinition, Trigger, FrameView, StructType, Palette, TextView, EditEvent, CounterMeasurement, EffectStyle, Machine, ColorStyle, Message, Session, Variant, Log, Role, CustomEntity, SceneEvent, BorderStyle, ShadowStyle, Reaction, Star, TransitionStyle, Handle, CustomView, SanctionEvent, Field, Route, BuiltinObject, Tagging, Timer, Script, EnumType, Invite, StructFrozen, NumberInputView, InviteEvent, QueryConnection, Entitlement, FriendshipInviteEvent, Branch, AnnotationShape, RunEvent, Node, LabelView, Organization, TimerEvent, Tag, ScreenCursor, Span, GaugeMeasurement, RoleEvent, Theme, Environment, Window, MembershipEvent, Run, NodeReference, Space } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:ENUM:2521 ==== */
export enum TextSpanType {
  TEXT = 1,
  HARD_BREAK = 2,
  MENTION = 10,
  LINK = 11,
  CITATION = 12,
  EQUATION = 20,
}
/* ==== DESTACK_GENERATED_END:ENUM:2521 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2521 ==== */
export class TextSpan extends StructFrozen {
  readonly type: TextSpanType;
  readonly content: string | null;
  get node(): Node | null {
      const nodePtr: NodeReference | null = this.nodePtr;
      if (nodePtr !== null) {
          if (this._supergraph === null) {
              return null;
          }
          return this._supergraph.get(nodePtr.id) as Node | null;
      }
      return null;
  }
  ;
  nodePtr: NodeReference | null
  readonly url: string | null;
  readonly isBold: boolean | null;
  readonly isItalic: boolean | null;
  readonly isStrikethrough: boolean | null;
  readonly isUnderline: boolean | null;
  readonly isCode: boolean | null;

  constructor(
    type: TextSpanType,
    content: string | null,
    nodePtr: NodeReference | null,
    url: string | null,
    isBold: boolean | null,
    isItalic: boolean | null,
    isStrikethrough: boolean | null,
    isUnderline: boolean | null,
    isCode: boolean | null,
    _supergraph: Supergraph | null
  ) {
    super(_supergraph);
    this.type = type;
    this.content = content;
    this.nodePtr = nodePtr;
    this.url = url;
    this.isBold = isBold;
    this.isItalic = isItalic;
    this.isStrikethrough = isStrikethrough;
    this.isUnderline = isUnderline;
    this.isCode = isCode;
  }


  static create(options: {
    type?: TextSpanType,
    content?: string | null,
    node?: Node | NodeReference | null,
    url?: string | null,
    isBold?: boolean | null,
    isItalic?: boolean | null,
    isStrikethrough?: boolean | null,
    isUnderline?: boolean | null,
    isCode?: boolean | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }): TextSpan {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new TextSpan(
      options.type ?? TextSpanType.TEXT,
      options.content ?? null,
      options.node != null ? (options.node.metatype == StructType.NODE_REFERENCE ? options.node : options.node.toRef()) : null,
      options.url ?? null,
      options.isBold ?? null,
      options.isItalic ?? null,
      options.isStrikethrough ?? null,
      options.isUnderline ?? null,
      options.isCode ?? null,
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
/* ==== DESTACK_GENERATED_END:STRUCT:2521 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2520 ==== */
export class Text extends StructFrozen {
  readonly spans: Array<TextSpan>;
  readonly isBold: boolean | null;
  readonly isItalic: boolean | null;
  readonly isStrikethrough: boolean | null;
  readonly isUnderline: boolean | null;
  readonly isCode: boolean | null;

  constructor(
    spans: Array<TextSpan>,
    isBold: boolean | null,
    isItalic: boolean | null,
    isStrikethrough: boolean | null,
    isUnderline: boolean | null,
    isCode: boolean | null,
    _supergraph: Supergraph | null
  ) {
    super(_supergraph);
    this.spans = spans;
    this.isBold = isBold;
    this.isItalic = isItalic;
    this.isStrikethrough = isStrikethrough;
    this.isUnderline = isUnderline;
    this.isCode = isCode;
  }


  static create(options: {
    spans?: Array<TextSpan>,
    isBold?: boolean | null,
    isItalic?: boolean | null,
    isStrikethrough?: boolean | null,
    isUnderline?: boolean | null,
    isCode?: boolean | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }): Text {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new Text(
      options.spans ?? [],
      options.isBold ?? null,
      options.isItalic ?? null,
      options.isStrikethrough ?? null,
      options.isUnderline ?? null,
      options.isCode ?? null,
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
/* ==== DESTACK_GENERATED_END:STRUCT:2520 ==== */