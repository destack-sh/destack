import { PlaneShape, GaugeMetric, EffectStyle, FontStyle, Machine, Interruption, HistogramMetric, Log, Tag, Database, FriendshipInvite, NodeReference, Entitlement, Layer, Branch, BorderStyle, CounterMetric, Action, EditEvent, AnnotationShape, Session, FriendshipInviteEvent, NumberInputView, CustomEvent, Trigger, ShadowStyle, Tagging, User, CustomEntityDefinition, SliderInputView, Snapshot, TransitionStyle, CustomViewDefinition, Struct, Handle, Friendship, ThreadCursor, Invite, ColorStyle, Role, Message, QueryConnection, ScreenCursor, Link, CustomView, Span, WizardView, NodeType, TriggerEvent, CustomStructDefinition, TimerEvent, CustomEntity, Organization, SanctionEvent, Thread, LineShape, GaugeMeasurement, Palette, RunEvent, BuiltinObject, InviteEvent, Team, Membership, Permission, Option, EnumType, Timer, Field, HistogramMeasurement, CustomEnumDefinition, Service, Node, Scene, CustomEventDefinition, Notification, SceneEvent, Folder, MembershipEvent, GradientStyle, Run, Window, Canvas, Variant, LabelView, Graph, ThreadView, StructFrozen, CounterMeasurement, Sanction, Script, Environment, Star, Reaction, SplitView, RoleEvent, FrameView, Theme, EntitlementEvent, NotificationEvent, Follow, File, Client, Agent, Space, ArrowShape, TextView, StructType, Route, Supergraph, EventCursor, FillStyle } from '@/language';
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


  static create(options: {
    type?: TextSpanType,
    content?: string | null,
    node?: Node | NodeReference | null,
    url?: string | null,
    isBold?: boolean | null,
    isItalic?: boolean | null,
    isStrikethrough?: boolean | null,
    isUnderline?: boolean | null,
    isCode?: boolean | null
  }): TextSpan {

    return new TextSpan(

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


  static create(options: {
    spans?: Array<TextSpan>,
    isBold?: boolean | null,
    isItalic?: boolean | null,
    isStrikethrough?: boolean | null,
    isUnderline?: boolean | null,
    isCode?: boolean | null
  }): Text {

    return new Text(

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