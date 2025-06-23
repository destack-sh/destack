import { EventCursor, Link, ACTIVE_SESSION, Field, FriendshipInviteEvent, CustomEvent, NotificationEvent, FriendshipInvite, Message, CounterMeasurement, ArrowShape, BorderStyle, StructType, CustomStructDefinition, SplitView, RunEvent, Struct, Organization, Palette, Run, InviteEvent, EntitlementEvent, CounterMetric, Agent, Team, MembershipEvent, Supergraph, Folder, SliderInputView, GaugeMeasurement, WizardView, Tag, GradientStyle, EnumType, RoleEvent, Tagging, HistogramMeasurement, AnnotationShape, CustomView, Reaction, ColorStyle, Timer, Thread, ThreadCursor, ShadowStyle, Branch, CustomEventDefinition, NodeType, Graph, User, Service, EditEvent, Script, Star, Canvas, Handle, Span, BuiltinObject, Session, FillStyle, Sanction, GaugeMetric, Window, Permission, Log, TriggerEvent, CustomEnumDefinition, HistogramMetric, Variant, Invite, Friendship, File, Theme, FontStyle, Trigger, PlaneShape, Snapshot, ThreadView, Scene, NodeReference, SanctionEvent, EffectStyle, Action, Node, ScreenCursor, CustomViewDefinition, StructFrozen, CustomEntity, FrameView, Database, Environment, Route, Entitlement, Follow, SceneEvent, TransitionStyle, Space, Machine, NumberInputView, CustomEntityDefinition, Membership, Client, LabelView, TimerEvent, LineShape, Option, Role, Layer, Interruption, QueryConnection, TextView, Notification, activeSession } from '@/language';
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
    super(
        // supergraph
        supergraph,
    );

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
    throw new Error("not implemented");
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
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
    super(
        // supergraph
        supergraph,
    );

    this.spans = options.spans ?? [];
    this.isBold = options.isBold ?? null;
    this.isItalic = options.isItalic ?? null;
    this.isStrikethrough = options.isStrikethrough ?? null;
    this.isUnderline = options.isUnderline ?? null;
    this.isCode = options.isCode ?? null;
  }

  equals(other: any): boolean {
    throw new Error("not implemented");
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:2520 ==== */