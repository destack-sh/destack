import { FriendshipInvite, HistogramMeasurement, CounterMeasurement, InviteEvent, Organization, Environment, BorderStyle, Invite, Field, FillStyle, BuiltinObject, Branch, Agent, FontStyle, PlaneShape, AnnotationShape, Scene, SliderInputView, Role, Trigger, Theme, NotificationEvent, Route, ThreadView, SplitView, RoleEvent, Script, ScreenCursor, Action, Session, ShadowStyle, CustomView, Layer, User, Handle, Timer, Star, CustomEntityDefinition, Graph, CustomStructDefinition, HistogramMetric, WizardView, FrameView, LabelView, ThreadCursor, Database, Thread, Variant, TextView, RunEvent, GradientStyle, Log, QueryConnection, Node, Client, Reaction, Notification, EntitlementEvent, Friendship, SceneEvent, Canvas, GaugeMeasurement, ArrowShape, Folder, TriggerEvent, Tagging, Tag, CustomEntity, Membership, Span, Follow, Interruption, SanctionEvent, Entitlement, Service, Palette, NumberInputView, Team, File, Permission, Link, TransitionStyle, ColorStyle, MembershipEvent, Sanction, CustomViewDefinition, CounterMetric, Snapshot, CustomEnumDefinition, NodeReference, Message, Space, Run, Window, Option, TimerEvent, CustomEventDefinition, LineShape, EffectStyle, Machine, Struct, Supergraph, CustomEvent, GaugeMetric, NodeType, EventCursor, EditEvent, FriendshipInviteEvent } from '@/language';
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
export class TextSpan extends BuiltinObject {
  readonly type: TextSpanType;
  readonly content: string | null;
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
    _supergraph: Supergraph
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


  static create(): TextSpan {

    return new TextSpan();
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
export class Text extends BuiltinObject {
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
    _supergraph: Supergraph
  ) {
    super(_supergraph);
    this.spans = spans;
    this.isBold = isBold;
    this.isItalic = isItalic;
    this.isStrikethrough = isStrikethrough;
    this.isUnderline = isUnderline;
    this.isCode = isCode;
  }


  static create(): Text {

    return new Text();
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