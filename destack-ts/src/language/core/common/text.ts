import { Folder, Session, Timer, Snapshot, Option, ScreenCursor, ShadowStyle, Interruption, NodeType, Action, Window, EffectStyle, Trigger, EventCursor, SliderInputView, HistogramMetric, CustomView, LabelView, Environment, ThreadView, Message, TransitionStyle, Space, CustomViewDefinition, Script, EnumType, Client, Layer, TextView, ColorStyle, FriendshipInviteEvent, Field, InviteEvent, Agent, Tagging, Follow, CustomEventDefinition, AnnotationShape, activeSession, StructType, RunEvent, CustomEvent, Role, Branch, HistogramMeasurement, NotificationEvent, WizardView, TriggerEvent, Variant, SceneEvent, EditEvent, Handle, Reaction, RoleEvent, CustomStructDefinition, Star, NumberInputView, Route, GaugeMetric, CustomEntityDefinition, File, ACTIVE_SESSION, LineShape, ThreadCursor, Machine, Tag, SanctionEvent, FrameView, Friendship, Supergraph, Database, Thread, FontStyle, Link, User, Team, MembershipEvent, ArrowShape, BorderStyle, EntitlementEvent, SplitView, Node, CounterMeasurement, PlaneShape, BuiltinObject, Organization, Scene, StructFrozen, GradientStyle, Entitlement, Graph, Service, Struct, Permission, Sanction, Canvas, Invite, Span, Palette, QueryConnection, GaugeMeasurement, NodeReference, CustomEnumDefinition, CounterMetric, TimerEvent, Membership, Run, Theme, CustomEntity, FriendshipInvite, Notification, Log, FillStyle } from '@/language';
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

  constructor(options: {
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
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(supergraph);
    this.type = options.type ?? TextSpanType.TEXT;
    this.content = options.content ?? null;
    this.nodePtr = options.node != null ? (options.node.metatype == StructType.NODE_REFERENCE ? (options.node as NodeReference) : (options.node as Node).toRef()) : null;
    this.url = options.url ?? null;
    this.isBold = options.isBold ?? null;
    this.isItalic = options.isItalic ?? null;
    this.isStrikethrough = options.isStrikethrough ?? null;
    this.isUnderline = options.isUnderline ?? null;
    this.isCode = options.isCode ?? null;
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

  constructor(options: {
    spans?: Array<TextSpan>,
    isBold?: boolean | null,
    isItalic?: boolean | null,
    isStrikethrough?: boolean | null,
    isUnderline?: boolean | null,
    isCode?: boolean | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(supergraph);
    this.spans = options.spans ?? [];
    this.isBold = options.isBold ?? null;
    this.isItalic = options.isItalic ?? null;
    this.isStrikethrough = options.isStrikethrough ?? null;
    this.isUnderline = options.isUnderline ?? null;
    this.isCode = options.isCode ?? null;
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