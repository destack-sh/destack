import { Graph, NodeReference, QueryConnection, Session, Supergraph } from "@destack/language/core";
import {
  EnumType,
  Event,
  Node,
  NodeType,
  StructType,
  TraitClass,
  TraitType,
} from "@destack/language/core/builtin";
import { Vector2 } from "@destack/language/core/common";
import {
  registerEnumClass,
  registerNodeClass,
  registerTraitClass,
} from "@destack/language/registry";
import { Space } from "@destack/language/space";
import {
  CopyEventProto,
  CutEventProto,
  DoubleClickEventProto,
  DragEndEventProto,
  DragEnterEventProto,
  DragLeaveEventProto,
  DragOverEventProto,
  DragStartEventProto,
  DropEventProto,
  FocusInEventProto,
  FocusOutEventProto,
  KeyDownEventProto,
  KeyPressEventProto,
  KeyUpEventProto,
  LeftClickEventProto,
  LongPressEventProto,
  MiddleClickEventProto,
  MouseButtonProto,
  PasteEventProto,
  PointerDownEventProto,
  PointerEnterEventProto,
  PointerLeaveEventProto,
  PointerMoveEventProto,
  PointerOverEventProto,
  PointerUpEventProto,
  RightClickEventProto,
  WheelEventProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashBool, hashFloat, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:9510 ==== */
/**
 * MouseButton
 */
export enum MouseButton {
  LEFT = 1,
  RIGHT = 2,
  MIDDLE = 3,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.MOUSE_BUTTON, MouseButton);
/* ==== DESTACK_GENERATED_END:ENUM:9510 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:9500 ==== */
/**
 * An InputEvent is an Event that corresponds to some direct user input.
 */
export interface InputEvent extends Event {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * An InputEvent is an Event that corresponds to some direct user input.
 */
class InputEvent$Type extends TraitClass<InputEvent, TraitType.INPUT_EVENT> {}

export const InputEvent = new InputEvent$Type(TraitType.INPUT_EVENT);
registerTraitClass(TraitType.INPUT_EVENT, InputEvent);
/* ==== DESTACK_GENERATED_END:TRAIT:9500 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:9501 ==== */
/**
 * A PointerEvent is an InputEvent that corresponds to some direct user input with a pointer.
 */
export interface PointerEvent extends InputEvent {
  /**
   * PointerEvent.position
   */
  position: Vector2;

  /**
   * PointerEvent.pressure
   */
  pressure: number;

  /**
   * PointerEvent.shiftKey
   */
  shiftKey: boolean;

  /**
   * PointerEvent.altKey
   */
  altKey: boolean;

  /**
   * PointerEvent.ctrlKey
   */
  ctrlKey: boolean;

  /**
   * PointerEvent.metaKey
   */
  metaKey: boolean;

  /**
   * PointerEvent.accelKey
   */
  accelKey: boolean;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A PointerEvent is an InputEvent that corresponds to some direct user input with a pointer.
 */
class PointerEvent$Type extends TraitClass<PointerEvent, TraitType.POINTER_EVENT> {}

export const PointerEvent = new PointerEvent$Type(TraitType.POINTER_EVENT);
registerTraitClass(TraitType.POINTER_EVENT, PointerEvent);
/* ==== DESTACK_GENERATED_END:TRAIT:9501 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:9510 ==== */
/**
 * A MouseEvent is a PointerEvent that corresponds to some direct user input with a mouse.
 */
export interface MouseEvent extends PointerEvent {
  /**
   * MouseEvent.button
   */
  button: MouseButton;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A MouseEvent is a PointerEvent that corresponds to some direct user input with a mouse.
 */
class MouseEvent$Type extends TraitClass<MouseEvent, TraitType.MOUSE_EVENT> {}

export const MouseEvent = new MouseEvent$Type(TraitType.MOUSE_EVENT);
registerTraitClass(TraitType.MOUSE_EVENT, MouseEvent);
/* ==== DESTACK_GENERATED_END:TRAIT:9510 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:9511 ==== */
/**
 * A ClickEvent is an InputEvent that corresponds to some direct user input with a click (left, right, middle).
 */
export interface ClickEvent extends MouseEvent {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A ClickEvent is an InputEvent that corresponds to some direct user input with a click (left, right, middle).
 */
class ClickEvent$Type extends TraitClass<ClickEvent, TraitType.CLICK_EVENT> {}

export const ClickEvent = new ClickEvent$Type(TraitType.CLICK_EVENT);
registerTraitClass(TraitType.CLICK_EVENT, ClickEvent);
/* ==== DESTACK_GENERATED_END:TRAIT:9511 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:9520 ==== */
/**
 * A KeyboardEvent is an InputEvent that corresponds to some direct user input with a keyboard.
 */
export interface KeyboardEvent extends InputEvent {
  /**
   * KeyboardEvent.key
   */
  key: string;

  /**
   * KeyboardEvent.code
   */
  code: string;

  /**
   * KeyboardEvent.repeat
   */
  repeat: boolean;

  /**
   * KeyboardEvent.shiftKey
   */
  shiftKey: boolean;

  /**
   * KeyboardEvent.altKey
   */
  altKey: boolean;

  /**
   * KeyboardEvent.ctrlKey
   */
  ctrlKey: boolean;

  /**
   * KeyboardEvent.metaKey
   */
  metaKey: boolean;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A KeyboardEvent is an InputEvent that corresponds to some direct user input with a keyboard.
 */
class KeyboardEvent$Type extends TraitClass<KeyboardEvent, TraitType.KEYBOARD_EVENT> {}

export const KeyboardEvent = new KeyboardEvent$Type(TraitType.KEYBOARD_EVENT);
registerTraitClass(TraitType.KEYBOARD_EVENT, KeyboardEvent);
/* ==== DESTACK_GENERATED_END:TRAIT:9520 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:9530 ==== */
/**
 * A DragEvent is an InputEvent that corresponds to some direct user input with a drag.
 */
export interface DragEvent extends InputEvent {
  /**
   * DragEvent.position
   */
  position: Vector2;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A DragEvent is an InputEvent that corresponds to some direct user input with a drag.
 */
class DragEvent$Type extends TraitClass<DragEvent, TraitType.DRAG_EVENT> {}

export const DragEvent = new DragEvent$Type(TraitType.DRAG_EVENT);
registerTraitClass(TraitType.DRAG_EVENT, DragEvent);
/* ==== DESTACK_GENERATED_END:TRAIT:9530 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:9540 ==== */
/**
 * A ClipboardEvent is an InputEvent that corresponds to some direct user input with a clipboard.
 */
export interface ClipboardEvent extends InputEvent {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A ClipboardEvent is an InputEvent that corresponds to some direct user input with a clipboard.
 */
class ClipboardEvent$Type extends TraitClass<ClipboardEvent, TraitType.CLIPBOARD_EVENT> {}

export const ClipboardEvent = new ClipboardEvent$Type(TraitType.CLIPBOARD_EVENT);
registerTraitClass(TraitType.CLIPBOARD_EVENT, ClipboardEvent);
/* ==== DESTACK_GENERATED_END:TRAIT:9540 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:9550 ==== */
/**
 * A FocusEvent is an InputEvent that corresponds to some direct user input with a focus.
 */
export interface FocusEvent extends InputEvent {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A FocusEvent is an InputEvent that corresponds to some direct user input with a focus.
 */
class FocusEvent$Type extends TraitClass<FocusEvent, TraitType.FOCUS_EVENT> {}

export const FocusEvent = new FocusEvent$Type(TraitType.FOCUS_EVENT);
registerTraitClass(TraitType.FOCUS_EVENT, FocusEvent);
/* ==== DESTACK_GENERATED_END:TRAIT:9550 ==== */

/* ==== DESTACK_GENERATED_START:NODE:9500 ==== */
/**
 * A PointerDownEvent is a PointerEvent when a pointer is pressed down.
 */
export class PointerDownEvent extends Node implements PointerEvent {
  static metatype: NodeType = NodeType.POINTER_DOWN_EVENT;
  static __traits__: TraitType[] = [
    TraitType.EVENT,
    TraitType.SPATIAL,
    TraitType.INPUT_EVENT,
    TraitType.POINTER_EVENT,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.SPACE];
  static __childTypes__: NodeType[] = [];
  static __ancestorTypes__: NodeType[] = [NodeType.SPACE];
  static __descendantTypes__: NodeType[] = [];

  /**
   * Spatial.parent
   */
  get parent(): Space | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference | null;

  /**
   * The Node this Event is about.
   */
  get node(): Node | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  set node(node: Node | null) {
    if (node === null) {
      this.nodePtr = null;
    } else {
      this.nodePtr = node.toRef();
    }
  }
  nodePtr: NodeReference | null;

  /**
   * PointerEvent.position
   */
  position: Vector2;

  /**
   * PointerEvent.pressure
   */
  pressure: number;

  /**
   * PointerEvent.shiftKey
   */
  shiftKey: boolean;

  /**
   * PointerEvent.altKey
   */
  altKey: boolean;

  /**
   * PointerEvent.ctrlKey
   */
  ctrlKey: boolean;

  /**
   * PointerEvent.metaKey
   */
  metaKey: boolean;

  /**
   * PointerEvent.accelKey
   */
  accelKey: boolean;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    node?: Node | NodeReference | null;
    position: Vector2;
    pressure: number;
    shiftKey: boolean;
    altKey: boolean;
    ctrlKey: boolean;
    metaKey: boolean;
    accelKey: boolean;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: QueryConnection | null;
  }) {
    super(
      // id
      options.id ?? null,
      // parent
      options.parent != null
        ? options.parent.metatype == StructType.NODE_REFERENCE
          ? (options.parent as NodeReference)
          : (options.parent as Node).toRef()
        : null,
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
      // graph
      options._graph ?? null,
      // connection
      options._connection ?? null,
      // is_new
      options.id == null,
      // is_attached
      options.id != null || options._graph != null,
    );

    // properties
    let _parent = options.parent ?? null;
    if (_parent != null && _parent instanceof Node) {
      _parent = _parent.toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space instanceof Node) {
      _space = _space.toRef();
    }
    this.spacePtr = _space;
    let _node = options.node ?? null;
    if (_node != null && _node instanceof Node) {
      _node = _node.toRef();
    }
    this.nodePtr = _node;
    let _position = options.position;
    if (_position === null) {
      throw new Error(`PointerDownEvent.position is required`);
    }
    this.position = _position;
    let _pressure = options.pressure;
    if (_pressure === null) {
      throw new Error(`PointerDownEvent.pressure is required`);
    }
    this.pressure = _pressure;
    let _shiftKey = options.shiftKey;
    if (_shiftKey === null) {
      throw new Error(`PointerDownEvent.shiftKey is required`);
    }
    this.shiftKey = _shiftKey;
    let _altKey = options.altKey;
    if (_altKey === null) {
      throw new Error(`PointerDownEvent.altKey is required`);
    }
    this.altKey = _altKey;
    let _ctrlKey = options.ctrlKey;
    if (_ctrlKey === null) {
      throw new Error(`PointerDownEvent.ctrlKey is required`);
    }
    this.ctrlKey = _ctrlKey;
    let _metaKey = options.metaKey;
    if (_metaKey === null) {
      throw new Error(`PointerDownEvent.metaKey is required`);
    }
    this.metaKey = _metaKey;
    let _accelKey = options.accelKey;
    if (_accelKey === null) {
      throw new Error(`PointerDownEvent.accelKey is required`);
    }
    this.accelKey = _accelKey;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(
          `{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`,
        );
      }
      this.createdAt = options.createdAt;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy instanceof Node
            ? options.createdBy.toRef()
            : options.createdBy
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedByPtr =
        options.updatedBy != null
          ? options.updatedBy instanceof Node
            ? options.updatedBy.toRef()
            : options.updatedBy
          : null;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!this.position.equals(other.position)) {
      return false;
    }
    if (!(this.pressure === other.pressure || Math.abs(this.pressure - other.pressure) < 1e-10)) {
      return false;
    }
    if (!(this.shiftKey === other.shiftKey)) {
      return false;
    }
    if (!(this.altKey === other.altKey)) {
      return false;
    }
    if (!(this.ctrlKey === other.ctrlKey)) {
      return false;
    }
    if (!(this.metaKey === other.metaKey)) {
      return false;
    }
    if (!(this.accelKey === other.accelKey)) {
      return false;
    }
    if (!(this.nodePtr?.id === other.nodePtr?.id)) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.position.hash()) & 0xffffffff;
    h = (h * 31 + hashFloat(this.pressure)) & 0xffffffff;
    h = (h * 31 + hashBool(this.shiftKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.altKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.ctrlKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.metaKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.accelKey)) & 0xffffffff;
    if (this.nodePtr !== null) {
      h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    }
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.POINTER_DOWN_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return "PointerDownEvent[id={this.id}]";
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Node | null = this;
    while (node !== null) {
      pathParts.push(node._pathKey);
      node = node.parent;
    }
    if (!this._isAttached) {
      pathParts.push("<detached>");
    }
    return pathParts.reverse().join("/");
  }

  repr(): string {
    return `<PointerDownEvent '${this.path}'>`;
  }

  toValue(): { [key: string]: any } {
    return PointerDownEvent.__packValue__(this);
  }

  static __packValue__(object: PointerDownEvent): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 9500;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    if (object.nodePtr != null) {
      objectValue["35"] = object.nodePtr.toValue();
    }
    objectValue["50"] = object.position.toValue();
    objectValue["51"] = object.pressure;
    objectValue["80"] = object.shiftKey;
    objectValue["81"] = object.altKey;
    objectValue["82"] = object.ctrlKey;
    objectValue["83"] = object.metaKey;
    objectValue["84"] = object.accelKey;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PointerDownEvent {
    const nodePtrValue = objectValue["35"];
    const unpackedNodePtr =
      nodePtrValue != undefined
        ? NodeReference.fromValue(nodePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new PointerDownEvent({
      position: Vector2.fromValue(objectValue["50"], _session, _supergraph, _graph, _connection),
      pressure: objectValue["51"],
      shiftKey: objectValue["80"],
      altKey: objectValue["81"],
      ctrlKey: objectValue["82"],
      metaKey: objectValue["83"],
      accelKey: objectValue["84"],
      node: unpackedNodePtr,
      parent: unpackedParentPtr,
      space: unpackedSpacePtr,
      id: String(objectValue["2"]),
      _session,
      _graph,
      _connection,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PointerDownEvent {
    return PointerDownEvent.__unpackValue__(
      objectValue,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  toProto(): PointerDownEventProto {
    return PointerDownEvent.__packProto__(this);
  }

  static __packProto__(object: PointerDownEvent): PointerDownEventProto {
    const objectProto: Partial<PointerDownEventProto> = { metatype: 9500 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    if (object.nodePtr != null) {
      objectProto.nodePtr = object.nodePtr.toProto();
    }
    objectProto.position = object.position.toProto();
    objectProto.pressure = object.pressure;
    objectProto.shiftKey = object.shiftKey;
    objectProto.altKey = object.altKey;
    objectProto.ctrlKey = object.ctrlKey;
    objectProto.metaKey = object.metaKey;
    objectProto.accelKey = object.accelKey;
    return objectProto as PointerDownEventProto;
  }

  static __unpackProto__(
    objectProto: PointerDownEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PointerDownEvent {
    return new PointerDownEvent({
      position: Vector2.fromProto(
        objectProto.position!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      pressure: objectProto.pressure,
      shiftKey: objectProto.shiftKey,
      altKey: objectProto.altKey,
      ctrlKey: objectProto.ctrlKey,
      metaKey: objectProto.metaKey,
      accelKey: objectProto.accelKey,
      node:
        objectProto.nodePtr != undefined
          ? NodeReference.fromProto(
              objectProto.nodePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(
              objectProto.spacePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      id: String(objectProto.id),
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: PointerDownEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PointerDownEvent {
    return PointerDownEvent.__unpackProto__(
      objectProto,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  static fromProtoString(packedProtoString: string): PointerDownEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = PointerDownEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.POINTER_DOWN_EVENT, PointerDownEvent);
/* ==== DESTACK_GENERATED_END:NODE:9500 ==== */

/* ==== DESTACK_GENERATED_START:NODE:9501 ==== */
/**
 * A PointerUpEvent is a PointerEvent when a pointer is released.
 */
export class PointerUpEvent extends Node implements PointerEvent {
  static metatype: NodeType = NodeType.POINTER_UP_EVENT;
  static __traits__: TraitType[] = [
    TraitType.EVENT,
    TraitType.SPATIAL,
    TraitType.INPUT_EVENT,
    TraitType.POINTER_EVENT,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.SPACE];
  static __childTypes__: NodeType[] = [];
  static __ancestorTypes__: NodeType[] = [NodeType.SPACE];
  static __descendantTypes__: NodeType[] = [];

  /**
   * Spatial.parent
   */
  get parent(): Space | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference | null;

  /**
   * The Node this Event is about.
   */
  get node(): Node | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  set node(node: Node | null) {
    if (node === null) {
      this.nodePtr = null;
    } else {
      this.nodePtr = node.toRef();
    }
  }
  nodePtr: NodeReference | null;

  /**
   * PointerEvent.position
   */
  position: Vector2;

  /**
   * PointerEvent.pressure
   */
  pressure: number;

  /**
   * PointerEvent.shiftKey
   */
  shiftKey: boolean;

  /**
   * PointerEvent.altKey
   */
  altKey: boolean;

  /**
   * PointerEvent.ctrlKey
   */
  ctrlKey: boolean;

  /**
   * PointerEvent.metaKey
   */
  metaKey: boolean;

  /**
   * PointerEvent.accelKey
   */
  accelKey: boolean;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    node?: Node | NodeReference | null;
    position: Vector2;
    pressure: number;
    shiftKey: boolean;
    altKey: boolean;
    ctrlKey: boolean;
    metaKey: boolean;
    accelKey: boolean;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: QueryConnection | null;
  }) {
    super(
      // id
      options.id ?? null,
      // parent
      options.parent != null
        ? options.parent.metatype == StructType.NODE_REFERENCE
          ? (options.parent as NodeReference)
          : (options.parent as Node).toRef()
        : null,
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
      // graph
      options._graph ?? null,
      // connection
      options._connection ?? null,
      // is_new
      options.id == null,
      // is_attached
      options.id != null || options._graph != null,
    );

    // properties
    let _parent = options.parent ?? null;
    if (_parent != null && _parent instanceof Node) {
      _parent = _parent.toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space instanceof Node) {
      _space = _space.toRef();
    }
    this.spacePtr = _space;
    let _node = options.node ?? null;
    if (_node != null && _node instanceof Node) {
      _node = _node.toRef();
    }
    this.nodePtr = _node;
    let _position = options.position;
    if (_position === null) {
      throw new Error(`PointerUpEvent.position is required`);
    }
    this.position = _position;
    let _pressure = options.pressure;
    if (_pressure === null) {
      throw new Error(`PointerUpEvent.pressure is required`);
    }
    this.pressure = _pressure;
    let _shiftKey = options.shiftKey;
    if (_shiftKey === null) {
      throw new Error(`PointerUpEvent.shiftKey is required`);
    }
    this.shiftKey = _shiftKey;
    let _altKey = options.altKey;
    if (_altKey === null) {
      throw new Error(`PointerUpEvent.altKey is required`);
    }
    this.altKey = _altKey;
    let _ctrlKey = options.ctrlKey;
    if (_ctrlKey === null) {
      throw new Error(`PointerUpEvent.ctrlKey is required`);
    }
    this.ctrlKey = _ctrlKey;
    let _metaKey = options.metaKey;
    if (_metaKey === null) {
      throw new Error(`PointerUpEvent.metaKey is required`);
    }
    this.metaKey = _metaKey;
    let _accelKey = options.accelKey;
    if (_accelKey === null) {
      throw new Error(`PointerUpEvent.accelKey is required`);
    }
    this.accelKey = _accelKey;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(
          `{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`,
        );
      }
      this.createdAt = options.createdAt;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy instanceof Node
            ? options.createdBy.toRef()
            : options.createdBy
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedByPtr =
        options.updatedBy != null
          ? options.updatedBy instanceof Node
            ? options.updatedBy.toRef()
            : options.updatedBy
          : null;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!this.position.equals(other.position)) {
      return false;
    }
    if (!(this.pressure === other.pressure || Math.abs(this.pressure - other.pressure) < 1e-10)) {
      return false;
    }
    if (!(this.shiftKey === other.shiftKey)) {
      return false;
    }
    if (!(this.altKey === other.altKey)) {
      return false;
    }
    if (!(this.ctrlKey === other.ctrlKey)) {
      return false;
    }
    if (!(this.metaKey === other.metaKey)) {
      return false;
    }
    if (!(this.accelKey === other.accelKey)) {
      return false;
    }
    if (!(this.nodePtr?.id === other.nodePtr?.id)) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.position.hash()) & 0xffffffff;
    h = (h * 31 + hashFloat(this.pressure)) & 0xffffffff;
    h = (h * 31 + hashBool(this.shiftKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.altKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.ctrlKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.metaKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.accelKey)) & 0xffffffff;
    if (this.nodePtr !== null) {
      h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    }
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.POINTER_UP_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return "PointerUpEvent[id={this.id}]";
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Node | null = this;
    while (node !== null) {
      pathParts.push(node._pathKey);
      node = node.parent;
    }
    if (!this._isAttached) {
      pathParts.push("<detached>");
    }
    return pathParts.reverse().join("/");
  }

  repr(): string {
    return `<PointerUpEvent '${this.path}'>`;
  }

  toValue(): { [key: string]: any } {
    return PointerUpEvent.__packValue__(this);
  }

  static __packValue__(object: PointerUpEvent): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 9501;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    if (object.nodePtr != null) {
      objectValue["35"] = object.nodePtr.toValue();
    }
    objectValue["50"] = object.position.toValue();
    objectValue["51"] = object.pressure;
    objectValue["80"] = object.shiftKey;
    objectValue["81"] = object.altKey;
    objectValue["82"] = object.ctrlKey;
    objectValue["83"] = object.metaKey;
    objectValue["84"] = object.accelKey;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PointerUpEvent {
    const nodePtrValue = objectValue["35"];
    const unpackedNodePtr =
      nodePtrValue != undefined
        ? NodeReference.fromValue(nodePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new PointerUpEvent({
      position: Vector2.fromValue(objectValue["50"], _session, _supergraph, _graph, _connection),
      pressure: objectValue["51"],
      shiftKey: objectValue["80"],
      altKey: objectValue["81"],
      ctrlKey: objectValue["82"],
      metaKey: objectValue["83"],
      accelKey: objectValue["84"],
      node: unpackedNodePtr,
      parent: unpackedParentPtr,
      space: unpackedSpacePtr,
      id: String(objectValue["2"]),
      _session,
      _graph,
      _connection,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PointerUpEvent {
    return PointerUpEvent.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): PointerUpEventProto {
    return PointerUpEvent.__packProto__(this);
  }

  static __packProto__(object: PointerUpEvent): PointerUpEventProto {
    const objectProto: Partial<PointerUpEventProto> = { metatype: 9501 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    if (object.nodePtr != null) {
      objectProto.nodePtr = object.nodePtr.toProto();
    }
    objectProto.position = object.position.toProto();
    objectProto.pressure = object.pressure;
    objectProto.shiftKey = object.shiftKey;
    objectProto.altKey = object.altKey;
    objectProto.ctrlKey = object.ctrlKey;
    objectProto.metaKey = object.metaKey;
    objectProto.accelKey = object.accelKey;
    return objectProto as PointerUpEventProto;
  }

  static __unpackProto__(
    objectProto: PointerUpEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PointerUpEvent {
    return new PointerUpEvent({
      position: Vector2.fromProto(
        objectProto.position!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      pressure: objectProto.pressure,
      shiftKey: objectProto.shiftKey,
      altKey: objectProto.altKey,
      ctrlKey: objectProto.ctrlKey,
      metaKey: objectProto.metaKey,
      accelKey: objectProto.accelKey,
      node:
        objectProto.nodePtr != undefined
          ? NodeReference.fromProto(
              objectProto.nodePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(
              objectProto.spacePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      id: String(objectProto.id),
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: PointerUpEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PointerUpEvent {
    return PointerUpEvent.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): PointerUpEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = PointerUpEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.POINTER_UP_EVENT, PointerUpEvent);
/* ==== DESTACK_GENERATED_END:NODE:9501 ==== */

/* ==== DESTACK_GENERATED_START:NODE:9502 ==== */
/**
 * A PointerMoveEvent is a PointerEvent when a pointer is moved.
 */
export class PointerMoveEvent extends Node implements PointerEvent {
  static metatype: NodeType = NodeType.POINTER_MOVE_EVENT;
  static __traits__: TraitType[] = [
    TraitType.EVENT,
    TraitType.SPATIAL,
    TraitType.INPUT_EVENT,
    TraitType.POINTER_EVENT,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.SPACE];
  static __childTypes__: NodeType[] = [];
  static __ancestorTypes__: NodeType[] = [NodeType.SPACE];
  static __descendantTypes__: NodeType[] = [];

  /**
   * Spatial.parent
   */
  get parent(): Space | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference | null;

  /**
   * The Node this Event is about.
   */
  get node(): Node | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  set node(node: Node | null) {
    if (node === null) {
      this.nodePtr = null;
    } else {
      this.nodePtr = node.toRef();
    }
  }
  nodePtr: NodeReference | null;

  /**
   * PointerEvent.position
   */
  position: Vector2;

  /**
   * PointerEvent.pressure
   */
  pressure: number;

  /**
   * PointerEvent.shiftKey
   */
  shiftKey: boolean;

  /**
   * PointerEvent.altKey
   */
  altKey: boolean;

  /**
   * PointerEvent.ctrlKey
   */
  ctrlKey: boolean;

  /**
   * PointerEvent.metaKey
   */
  metaKey: boolean;

  /**
   * PointerEvent.accelKey
   */
  accelKey: boolean;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    node?: Node | NodeReference | null;
    position: Vector2;
    pressure: number;
    shiftKey: boolean;
    altKey: boolean;
    ctrlKey: boolean;
    metaKey: boolean;
    accelKey: boolean;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: QueryConnection | null;
  }) {
    super(
      // id
      options.id ?? null,
      // parent
      options.parent != null
        ? options.parent.metatype == StructType.NODE_REFERENCE
          ? (options.parent as NodeReference)
          : (options.parent as Node).toRef()
        : null,
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
      // graph
      options._graph ?? null,
      // connection
      options._connection ?? null,
      // is_new
      options.id == null,
      // is_attached
      options.id != null || options._graph != null,
    );

    // properties
    let _parent = options.parent ?? null;
    if (_parent != null && _parent instanceof Node) {
      _parent = _parent.toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space instanceof Node) {
      _space = _space.toRef();
    }
    this.spacePtr = _space;
    let _node = options.node ?? null;
    if (_node != null && _node instanceof Node) {
      _node = _node.toRef();
    }
    this.nodePtr = _node;
    let _position = options.position;
    if (_position === null) {
      throw new Error(`PointerMoveEvent.position is required`);
    }
    this.position = _position;
    let _pressure = options.pressure;
    if (_pressure === null) {
      throw new Error(`PointerMoveEvent.pressure is required`);
    }
    this.pressure = _pressure;
    let _shiftKey = options.shiftKey;
    if (_shiftKey === null) {
      throw new Error(`PointerMoveEvent.shiftKey is required`);
    }
    this.shiftKey = _shiftKey;
    let _altKey = options.altKey;
    if (_altKey === null) {
      throw new Error(`PointerMoveEvent.altKey is required`);
    }
    this.altKey = _altKey;
    let _ctrlKey = options.ctrlKey;
    if (_ctrlKey === null) {
      throw new Error(`PointerMoveEvent.ctrlKey is required`);
    }
    this.ctrlKey = _ctrlKey;
    let _metaKey = options.metaKey;
    if (_metaKey === null) {
      throw new Error(`PointerMoveEvent.metaKey is required`);
    }
    this.metaKey = _metaKey;
    let _accelKey = options.accelKey;
    if (_accelKey === null) {
      throw new Error(`PointerMoveEvent.accelKey is required`);
    }
    this.accelKey = _accelKey;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(
          `{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`,
        );
      }
      this.createdAt = options.createdAt;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy instanceof Node
            ? options.createdBy.toRef()
            : options.createdBy
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedByPtr =
        options.updatedBy != null
          ? options.updatedBy instanceof Node
            ? options.updatedBy.toRef()
            : options.updatedBy
          : null;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!this.position.equals(other.position)) {
      return false;
    }
    if (!(this.pressure === other.pressure || Math.abs(this.pressure - other.pressure) < 1e-10)) {
      return false;
    }
    if (!(this.shiftKey === other.shiftKey)) {
      return false;
    }
    if (!(this.altKey === other.altKey)) {
      return false;
    }
    if (!(this.ctrlKey === other.ctrlKey)) {
      return false;
    }
    if (!(this.metaKey === other.metaKey)) {
      return false;
    }
    if (!(this.accelKey === other.accelKey)) {
      return false;
    }
    if (!(this.nodePtr?.id === other.nodePtr?.id)) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.position.hash()) & 0xffffffff;
    h = (h * 31 + hashFloat(this.pressure)) & 0xffffffff;
    h = (h * 31 + hashBool(this.shiftKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.altKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.ctrlKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.metaKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.accelKey)) & 0xffffffff;
    if (this.nodePtr !== null) {
      h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    }
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.POINTER_MOVE_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return "PointerMoveEvent[id={this.id}]";
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Node | null = this;
    while (node !== null) {
      pathParts.push(node._pathKey);
      node = node.parent;
    }
    if (!this._isAttached) {
      pathParts.push("<detached>");
    }
    return pathParts.reverse().join("/");
  }

  repr(): string {
    return `<PointerMoveEvent '${this.path}'>`;
  }

  toValue(): { [key: string]: any } {
    return PointerMoveEvent.__packValue__(this);
  }

  static __packValue__(object: PointerMoveEvent): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 9502;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    if (object.nodePtr != null) {
      objectValue["35"] = object.nodePtr.toValue();
    }
    objectValue["50"] = object.position.toValue();
    objectValue["51"] = object.pressure;
    objectValue["80"] = object.shiftKey;
    objectValue["81"] = object.altKey;
    objectValue["82"] = object.ctrlKey;
    objectValue["83"] = object.metaKey;
    objectValue["84"] = object.accelKey;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PointerMoveEvent {
    const nodePtrValue = objectValue["35"];
    const unpackedNodePtr =
      nodePtrValue != undefined
        ? NodeReference.fromValue(nodePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new PointerMoveEvent({
      position: Vector2.fromValue(objectValue["50"], _session, _supergraph, _graph, _connection),
      pressure: objectValue["51"],
      shiftKey: objectValue["80"],
      altKey: objectValue["81"],
      ctrlKey: objectValue["82"],
      metaKey: objectValue["83"],
      accelKey: objectValue["84"],
      node: unpackedNodePtr,
      parent: unpackedParentPtr,
      space: unpackedSpacePtr,
      id: String(objectValue["2"]),
      _session,
      _graph,
      _connection,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PointerMoveEvent {
    return PointerMoveEvent.__unpackValue__(
      objectValue,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  toProto(): PointerMoveEventProto {
    return PointerMoveEvent.__packProto__(this);
  }

  static __packProto__(object: PointerMoveEvent): PointerMoveEventProto {
    const objectProto: Partial<PointerMoveEventProto> = { metatype: 9502 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    if (object.nodePtr != null) {
      objectProto.nodePtr = object.nodePtr.toProto();
    }
    objectProto.position = object.position.toProto();
    objectProto.pressure = object.pressure;
    objectProto.shiftKey = object.shiftKey;
    objectProto.altKey = object.altKey;
    objectProto.ctrlKey = object.ctrlKey;
    objectProto.metaKey = object.metaKey;
    objectProto.accelKey = object.accelKey;
    return objectProto as PointerMoveEventProto;
  }

  static __unpackProto__(
    objectProto: PointerMoveEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PointerMoveEvent {
    return new PointerMoveEvent({
      position: Vector2.fromProto(
        objectProto.position!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      pressure: objectProto.pressure,
      shiftKey: objectProto.shiftKey,
      altKey: objectProto.altKey,
      ctrlKey: objectProto.ctrlKey,
      metaKey: objectProto.metaKey,
      accelKey: objectProto.accelKey,
      node:
        objectProto.nodePtr != undefined
          ? NodeReference.fromProto(
              objectProto.nodePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(
              objectProto.spacePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      id: String(objectProto.id),
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: PointerMoveEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PointerMoveEvent {
    return PointerMoveEvent.__unpackProto__(
      objectProto,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  static fromProtoString(packedProtoString: string): PointerMoveEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = PointerMoveEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.POINTER_MOVE_EVENT, PointerMoveEvent);
/* ==== DESTACK_GENERATED_END:NODE:9502 ==== */

/* ==== DESTACK_GENERATED_START:NODE:9503 ==== */
/**
 * A PointerEnterEvent is a PointerEvent when a pointer enters an element.
 */
export class PointerEnterEvent extends Node implements PointerEvent {
  static metatype: NodeType = NodeType.POINTER_ENTER_EVENT;
  static __traits__: TraitType[] = [
    TraitType.EVENT,
    TraitType.SPATIAL,
    TraitType.INPUT_EVENT,
    TraitType.POINTER_EVENT,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.SPACE];
  static __childTypes__: NodeType[] = [];
  static __ancestorTypes__: NodeType[] = [NodeType.SPACE];
  static __descendantTypes__: NodeType[] = [];

  /**
   * Spatial.parent
   */
  get parent(): Space | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference | null;

  /**
   * The Node this Event is about.
   */
  get node(): Node | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  set node(node: Node | null) {
    if (node === null) {
      this.nodePtr = null;
    } else {
      this.nodePtr = node.toRef();
    }
  }
  nodePtr: NodeReference | null;

  /**
   * PointerEvent.position
   */
  position: Vector2;

  /**
   * PointerEvent.pressure
   */
  pressure: number;

  /**
   * PointerEvent.shiftKey
   */
  shiftKey: boolean;

  /**
   * PointerEvent.altKey
   */
  altKey: boolean;

  /**
   * PointerEvent.ctrlKey
   */
  ctrlKey: boolean;

  /**
   * PointerEvent.metaKey
   */
  metaKey: boolean;

  /**
   * PointerEvent.accelKey
   */
  accelKey: boolean;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    node?: Node | NodeReference | null;
    position: Vector2;
    pressure: number;
    shiftKey: boolean;
    altKey: boolean;
    ctrlKey: boolean;
    metaKey: boolean;
    accelKey: boolean;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: QueryConnection | null;
  }) {
    super(
      // id
      options.id ?? null,
      // parent
      options.parent != null
        ? options.parent.metatype == StructType.NODE_REFERENCE
          ? (options.parent as NodeReference)
          : (options.parent as Node).toRef()
        : null,
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
      // graph
      options._graph ?? null,
      // connection
      options._connection ?? null,
      // is_new
      options.id == null,
      // is_attached
      options.id != null || options._graph != null,
    );

    // properties
    let _parent = options.parent ?? null;
    if (_parent != null && _parent instanceof Node) {
      _parent = _parent.toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space instanceof Node) {
      _space = _space.toRef();
    }
    this.spacePtr = _space;
    let _node = options.node ?? null;
    if (_node != null && _node instanceof Node) {
      _node = _node.toRef();
    }
    this.nodePtr = _node;
    let _position = options.position;
    if (_position === null) {
      throw new Error(`PointerEnterEvent.position is required`);
    }
    this.position = _position;
    let _pressure = options.pressure;
    if (_pressure === null) {
      throw new Error(`PointerEnterEvent.pressure is required`);
    }
    this.pressure = _pressure;
    let _shiftKey = options.shiftKey;
    if (_shiftKey === null) {
      throw new Error(`PointerEnterEvent.shiftKey is required`);
    }
    this.shiftKey = _shiftKey;
    let _altKey = options.altKey;
    if (_altKey === null) {
      throw new Error(`PointerEnterEvent.altKey is required`);
    }
    this.altKey = _altKey;
    let _ctrlKey = options.ctrlKey;
    if (_ctrlKey === null) {
      throw new Error(`PointerEnterEvent.ctrlKey is required`);
    }
    this.ctrlKey = _ctrlKey;
    let _metaKey = options.metaKey;
    if (_metaKey === null) {
      throw new Error(`PointerEnterEvent.metaKey is required`);
    }
    this.metaKey = _metaKey;
    let _accelKey = options.accelKey;
    if (_accelKey === null) {
      throw new Error(`PointerEnterEvent.accelKey is required`);
    }
    this.accelKey = _accelKey;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(
          `{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`,
        );
      }
      this.createdAt = options.createdAt;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy instanceof Node
            ? options.createdBy.toRef()
            : options.createdBy
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedByPtr =
        options.updatedBy != null
          ? options.updatedBy instanceof Node
            ? options.updatedBy.toRef()
            : options.updatedBy
          : null;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!this.position.equals(other.position)) {
      return false;
    }
    if (!(this.pressure === other.pressure || Math.abs(this.pressure - other.pressure) < 1e-10)) {
      return false;
    }
    if (!(this.shiftKey === other.shiftKey)) {
      return false;
    }
    if (!(this.altKey === other.altKey)) {
      return false;
    }
    if (!(this.ctrlKey === other.ctrlKey)) {
      return false;
    }
    if (!(this.metaKey === other.metaKey)) {
      return false;
    }
    if (!(this.accelKey === other.accelKey)) {
      return false;
    }
    if (!(this.nodePtr?.id === other.nodePtr?.id)) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.position.hash()) & 0xffffffff;
    h = (h * 31 + hashFloat(this.pressure)) & 0xffffffff;
    h = (h * 31 + hashBool(this.shiftKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.altKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.ctrlKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.metaKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.accelKey)) & 0xffffffff;
    if (this.nodePtr !== null) {
      h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    }
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.POINTER_ENTER_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return "PointerEnterEvent[id={this.id}]";
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Node | null = this;
    while (node !== null) {
      pathParts.push(node._pathKey);
      node = node.parent;
    }
    if (!this._isAttached) {
      pathParts.push("<detached>");
    }
    return pathParts.reverse().join("/");
  }

  repr(): string {
    return `<PointerEnterEvent '${this.path}'>`;
  }

  toValue(): { [key: string]: any } {
    return PointerEnterEvent.__packValue__(this);
  }

  static __packValue__(object: PointerEnterEvent): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 9503;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    if (object.nodePtr != null) {
      objectValue["35"] = object.nodePtr.toValue();
    }
    objectValue["50"] = object.position.toValue();
    objectValue["51"] = object.pressure;
    objectValue["80"] = object.shiftKey;
    objectValue["81"] = object.altKey;
    objectValue["82"] = object.ctrlKey;
    objectValue["83"] = object.metaKey;
    objectValue["84"] = object.accelKey;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PointerEnterEvent {
    const nodePtrValue = objectValue["35"];
    const unpackedNodePtr =
      nodePtrValue != undefined
        ? NodeReference.fromValue(nodePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new PointerEnterEvent({
      position: Vector2.fromValue(objectValue["50"], _session, _supergraph, _graph, _connection),
      pressure: objectValue["51"],
      shiftKey: objectValue["80"],
      altKey: objectValue["81"],
      ctrlKey: objectValue["82"],
      metaKey: objectValue["83"],
      accelKey: objectValue["84"],
      node: unpackedNodePtr,
      parent: unpackedParentPtr,
      space: unpackedSpacePtr,
      id: String(objectValue["2"]),
      _session,
      _graph,
      _connection,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PointerEnterEvent {
    return PointerEnterEvent.__unpackValue__(
      objectValue,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  toProto(): PointerEnterEventProto {
    return PointerEnterEvent.__packProto__(this);
  }

  static __packProto__(object: PointerEnterEvent): PointerEnterEventProto {
    const objectProto: Partial<PointerEnterEventProto> = { metatype: 9503 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    if (object.nodePtr != null) {
      objectProto.nodePtr = object.nodePtr.toProto();
    }
    objectProto.position = object.position.toProto();
    objectProto.pressure = object.pressure;
    objectProto.shiftKey = object.shiftKey;
    objectProto.altKey = object.altKey;
    objectProto.ctrlKey = object.ctrlKey;
    objectProto.metaKey = object.metaKey;
    objectProto.accelKey = object.accelKey;
    return objectProto as PointerEnterEventProto;
  }

  static __unpackProto__(
    objectProto: PointerEnterEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PointerEnterEvent {
    return new PointerEnterEvent({
      position: Vector2.fromProto(
        objectProto.position!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      pressure: objectProto.pressure,
      shiftKey: objectProto.shiftKey,
      altKey: objectProto.altKey,
      ctrlKey: objectProto.ctrlKey,
      metaKey: objectProto.metaKey,
      accelKey: objectProto.accelKey,
      node:
        objectProto.nodePtr != undefined
          ? NodeReference.fromProto(
              objectProto.nodePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(
              objectProto.spacePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      id: String(objectProto.id),
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: PointerEnterEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PointerEnterEvent {
    return PointerEnterEvent.__unpackProto__(
      objectProto,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  static fromProtoString(packedProtoString: string): PointerEnterEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = PointerEnterEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.POINTER_ENTER_EVENT, PointerEnterEvent);
/* ==== DESTACK_GENERATED_END:NODE:9503 ==== */

/* ==== DESTACK_GENERATED_START:NODE:9504 ==== */
/**
 * A PointerOverEvent is a PointerEvent when a pointer is over an element.
 */
export class PointerOverEvent extends Node implements PointerEvent {
  static metatype: NodeType = NodeType.POINTER_OVER_EVENT;
  static __traits__: TraitType[] = [
    TraitType.EVENT,
    TraitType.SPATIAL,
    TraitType.INPUT_EVENT,
    TraitType.POINTER_EVENT,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.SPACE];
  static __childTypes__: NodeType[] = [];
  static __ancestorTypes__: NodeType[] = [NodeType.SPACE];
  static __descendantTypes__: NodeType[] = [];

  /**
   * Spatial.parent
   */
  get parent(): Space | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference | null;

  /**
   * The Node this Event is about.
   */
  get node(): Node | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  set node(node: Node | null) {
    if (node === null) {
      this.nodePtr = null;
    } else {
      this.nodePtr = node.toRef();
    }
  }
  nodePtr: NodeReference | null;

  /**
   * PointerEvent.position
   */
  position: Vector2;

  /**
   * PointerEvent.pressure
   */
  pressure: number;

  /**
   * PointerEvent.shiftKey
   */
  shiftKey: boolean;

  /**
   * PointerEvent.altKey
   */
  altKey: boolean;

  /**
   * PointerEvent.ctrlKey
   */
  ctrlKey: boolean;

  /**
   * PointerEvent.metaKey
   */
  metaKey: boolean;

  /**
   * PointerEvent.accelKey
   */
  accelKey: boolean;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    node?: Node | NodeReference | null;
    position: Vector2;
    pressure: number;
    shiftKey: boolean;
    altKey: boolean;
    ctrlKey: boolean;
    metaKey: boolean;
    accelKey: boolean;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: QueryConnection | null;
  }) {
    super(
      // id
      options.id ?? null,
      // parent
      options.parent != null
        ? options.parent.metatype == StructType.NODE_REFERENCE
          ? (options.parent as NodeReference)
          : (options.parent as Node).toRef()
        : null,
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
      // graph
      options._graph ?? null,
      // connection
      options._connection ?? null,
      // is_new
      options.id == null,
      // is_attached
      options.id != null || options._graph != null,
    );

    // properties
    let _parent = options.parent ?? null;
    if (_parent != null && _parent instanceof Node) {
      _parent = _parent.toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space instanceof Node) {
      _space = _space.toRef();
    }
    this.spacePtr = _space;
    let _node = options.node ?? null;
    if (_node != null && _node instanceof Node) {
      _node = _node.toRef();
    }
    this.nodePtr = _node;
    let _position = options.position;
    if (_position === null) {
      throw new Error(`PointerOverEvent.position is required`);
    }
    this.position = _position;
    let _pressure = options.pressure;
    if (_pressure === null) {
      throw new Error(`PointerOverEvent.pressure is required`);
    }
    this.pressure = _pressure;
    let _shiftKey = options.shiftKey;
    if (_shiftKey === null) {
      throw new Error(`PointerOverEvent.shiftKey is required`);
    }
    this.shiftKey = _shiftKey;
    let _altKey = options.altKey;
    if (_altKey === null) {
      throw new Error(`PointerOverEvent.altKey is required`);
    }
    this.altKey = _altKey;
    let _ctrlKey = options.ctrlKey;
    if (_ctrlKey === null) {
      throw new Error(`PointerOverEvent.ctrlKey is required`);
    }
    this.ctrlKey = _ctrlKey;
    let _metaKey = options.metaKey;
    if (_metaKey === null) {
      throw new Error(`PointerOverEvent.metaKey is required`);
    }
    this.metaKey = _metaKey;
    let _accelKey = options.accelKey;
    if (_accelKey === null) {
      throw new Error(`PointerOverEvent.accelKey is required`);
    }
    this.accelKey = _accelKey;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(
          `{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`,
        );
      }
      this.createdAt = options.createdAt;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy instanceof Node
            ? options.createdBy.toRef()
            : options.createdBy
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedByPtr =
        options.updatedBy != null
          ? options.updatedBy instanceof Node
            ? options.updatedBy.toRef()
            : options.updatedBy
          : null;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!this.position.equals(other.position)) {
      return false;
    }
    if (!(this.pressure === other.pressure || Math.abs(this.pressure - other.pressure) < 1e-10)) {
      return false;
    }
    if (!(this.shiftKey === other.shiftKey)) {
      return false;
    }
    if (!(this.altKey === other.altKey)) {
      return false;
    }
    if (!(this.ctrlKey === other.ctrlKey)) {
      return false;
    }
    if (!(this.metaKey === other.metaKey)) {
      return false;
    }
    if (!(this.accelKey === other.accelKey)) {
      return false;
    }
    if (!(this.nodePtr?.id === other.nodePtr?.id)) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.position.hash()) & 0xffffffff;
    h = (h * 31 + hashFloat(this.pressure)) & 0xffffffff;
    h = (h * 31 + hashBool(this.shiftKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.altKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.ctrlKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.metaKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.accelKey)) & 0xffffffff;
    if (this.nodePtr !== null) {
      h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    }
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.POINTER_OVER_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return "PointerOverEvent[id={this.id}]";
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Node | null = this;
    while (node !== null) {
      pathParts.push(node._pathKey);
      node = node.parent;
    }
    if (!this._isAttached) {
      pathParts.push("<detached>");
    }
    return pathParts.reverse().join("/");
  }

  repr(): string {
    return `<PointerOverEvent '${this.path}'>`;
  }

  toValue(): { [key: string]: any } {
    return PointerOverEvent.__packValue__(this);
  }

  static __packValue__(object: PointerOverEvent): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 9504;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    if (object.nodePtr != null) {
      objectValue["35"] = object.nodePtr.toValue();
    }
    objectValue["50"] = object.position.toValue();
    objectValue["51"] = object.pressure;
    objectValue["80"] = object.shiftKey;
    objectValue["81"] = object.altKey;
    objectValue["82"] = object.ctrlKey;
    objectValue["83"] = object.metaKey;
    objectValue["84"] = object.accelKey;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PointerOverEvent {
    const nodePtrValue = objectValue["35"];
    const unpackedNodePtr =
      nodePtrValue != undefined
        ? NodeReference.fromValue(nodePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new PointerOverEvent({
      position: Vector2.fromValue(objectValue["50"], _session, _supergraph, _graph, _connection),
      pressure: objectValue["51"],
      shiftKey: objectValue["80"],
      altKey: objectValue["81"],
      ctrlKey: objectValue["82"],
      metaKey: objectValue["83"],
      accelKey: objectValue["84"],
      node: unpackedNodePtr,
      parent: unpackedParentPtr,
      space: unpackedSpacePtr,
      id: String(objectValue["2"]),
      _session,
      _graph,
      _connection,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PointerOverEvent {
    return PointerOverEvent.__unpackValue__(
      objectValue,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  toProto(): PointerOverEventProto {
    return PointerOverEvent.__packProto__(this);
  }

  static __packProto__(object: PointerOverEvent): PointerOverEventProto {
    const objectProto: Partial<PointerOverEventProto> = { metatype: 9504 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    if (object.nodePtr != null) {
      objectProto.nodePtr = object.nodePtr.toProto();
    }
    objectProto.position = object.position.toProto();
    objectProto.pressure = object.pressure;
    objectProto.shiftKey = object.shiftKey;
    objectProto.altKey = object.altKey;
    objectProto.ctrlKey = object.ctrlKey;
    objectProto.metaKey = object.metaKey;
    objectProto.accelKey = object.accelKey;
    return objectProto as PointerOverEventProto;
  }

  static __unpackProto__(
    objectProto: PointerOverEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PointerOverEvent {
    return new PointerOverEvent({
      position: Vector2.fromProto(
        objectProto.position!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      pressure: objectProto.pressure,
      shiftKey: objectProto.shiftKey,
      altKey: objectProto.altKey,
      ctrlKey: objectProto.ctrlKey,
      metaKey: objectProto.metaKey,
      accelKey: objectProto.accelKey,
      node:
        objectProto.nodePtr != undefined
          ? NodeReference.fromProto(
              objectProto.nodePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(
              objectProto.spacePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      id: String(objectProto.id),
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: PointerOverEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PointerOverEvent {
    return PointerOverEvent.__unpackProto__(
      objectProto,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  static fromProtoString(packedProtoString: string): PointerOverEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = PointerOverEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.POINTER_OVER_EVENT, PointerOverEvent);
/* ==== DESTACK_GENERATED_END:NODE:9504 ==== */

/* ==== DESTACK_GENERATED_START:NODE:9505 ==== */
/**
 * A PointerLeaveEvent is a PointerEvent when a pointer leaves an element.
 */
export class PointerLeaveEvent extends Node implements PointerEvent {
  static metatype: NodeType = NodeType.POINTER_LEAVE_EVENT;
  static __traits__: TraitType[] = [
    TraitType.EVENT,
    TraitType.SPATIAL,
    TraitType.INPUT_EVENT,
    TraitType.POINTER_EVENT,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.SPACE];
  static __childTypes__: NodeType[] = [];
  static __ancestorTypes__: NodeType[] = [NodeType.SPACE];
  static __descendantTypes__: NodeType[] = [];

  /**
   * Spatial.parent
   */
  get parent(): Space | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference | null;

  /**
   * The Node this Event is about.
   */
  get node(): Node | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  set node(node: Node | null) {
    if (node === null) {
      this.nodePtr = null;
    } else {
      this.nodePtr = node.toRef();
    }
  }
  nodePtr: NodeReference | null;

  /**
   * PointerEvent.position
   */
  position: Vector2;

  /**
   * PointerEvent.pressure
   */
  pressure: number;

  /**
   * PointerEvent.shiftKey
   */
  shiftKey: boolean;

  /**
   * PointerEvent.altKey
   */
  altKey: boolean;

  /**
   * PointerEvent.ctrlKey
   */
  ctrlKey: boolean;

  /**
   * PointerEvent.metaKey
   */
  metaKey: boolean;

  /**
   * PointerEvent.accelKey
   */
  accelKey: boolean;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    node?: Node | NodeReference | null;
    position: Vector2;
    pressure: number;
    shiftKey: boolean;
    altKey: boolean;
    ctrlKey: boolean;
    metaKey: boolean;
    accelKey: boolean;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: QueryConnection | null;
  }) {
    super(
      // id
      options.id ?? null,
      // parent
      options.parent != null
        ? options.parent.metatype == StructType.NODE_REFERENCE
          ? (options.parent as NodeReference)
          : (options.parent as Node).toRef()
        : null,
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
      // graph
      options._graph ?? null,
      // connection
      options._connection ?? null,
      // is_new
      options.id == null,
      // is_attached
      options.id != null || options._graph != null,
    );

    // properties
    let _parent = options.parent ?? null;
    if (_parent != null && _parent instanceof Node) {
      _parent = _parent.toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space instanceof Node) {
      _space = _space.toRef();
    }
    this.spacePtr = _space;
    let _node = options.node ?? null;
    if (_node != null && _node instanceof Node) {
      _node = _node.toRef();
    }
    this.nodePtr = _node;
    let _position = options.position;
    if (_position === null) {
      throw new Error(`PointerLeaveEvent.position is required`);
    }
    this.position = _position;
    let _pressure = options.pressure;
    if (_pressure === null) {
      throw new Error(`PointerLeaveEvent.pressure is required`);
    }
    this.pressure = _pressure;
    let _shiftKey = options.shiftKey;
    if (_shiftKey === null) {
      throw new Error(`PointerLeaveEvent.shiftKey is required`);
    }
    this.shiftKey = _shiftKey;
    let _altKey = options.altKey;
    if (_altKey === null) {
      throw new Error(`PointerLeaveEvent.altKey is required`);
    }
    this.altKey = _altKey;
    let _ctrlKey = options.ctrlKey;
    if (_ctrlKey === null) {
      throw new Error(`PointerLeaveEvent.ctrlKey is required`);
    }
    this.ctrlKey = _ctrlKey;
    let _metaKey = options.metaKey;
    if (_metaKey === null) {
      throw new Error(`PointerLeaveEvent.metaKey is required`);
    }
    this.metaKey = _metaKey;
    let _accelKey = options.accelKey;
    if (_accelKey === null) {
      throw new Error(`PointerLeaveEvent.accelKey is required`);
    }
    this.accelKey = _accelKey;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(
          `{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`,
        );
      }
      this.createdAt = options.createdAt;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy instanceof Node
            ? options.createdBy.toRef()
            : options.createdBy
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedByPtr =
        options.updatedBy != null
          ? options.updatedBy instanceof Node
            ? options.updatedBy.toRef()
            : options.updatedBy
          : null;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!this.position.equals(other.position)) {
      return false;
    }
    if (!(this.pressure === other.pressure || Math.abs(this.pressure - other.pressure) < 1e-10)) {
      return false;
    }
    if (!(this.shiftKey === other.shiftKey)) {
      return false;
    }
    if (!(this.altKey === other.altKey)) {
      return false;
    }
    if (!(this.ctrlKey === other.ctrlKey)) {
      return false;
    }
    if (!(this.metaKey === other.metaKey)) {
      return false;
    }
    if (!(this.accelKey === other.accelKey)) {
      return false;
    }
    if (!(this.nodePtr?.id === other.nodePtr?.id)) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.position.hash()) & 0xffffffff;
    h = (h * 31 + hashFloat(this.pressure)) & 0xffffffff;
    h = (h * 31 + hashBool(this.shiftKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.altKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.ctrlKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.metaKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.accelKey)) & 0xffffffff;
    if (this.nodePtr !== null) {
      h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    }
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.POINTER_LEAVE_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return "PointerLeaveEvent[id={this.id}]";
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Node | null = this;
    while (node !== null) {
      pathParts.push(node._pathKey);
      node = node.parent;
    }
    if (!this._isAttached) {
      pathParts.push("<detached>");
    }
    return pathParts.reverse().join("/");
  }

  repr(): string {
    return `<PointerLeaveEvent '${this.path}'>`;
  }

  toValue(): { [key: string]: any } {
    return PointerLeaveEvent.__packValue__(this);
  }

  static __packValue__(object: PointerLeaveEvent): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 9505;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    if (object.nodePtr != null) {
      objectValue["35"] = object.nodePtr.toValue();
    }
    objectValue["50"] = object.position.toValue();
    objectValue["51"] = object.pressure;
    objectValue["80"] = object.shiftKey;
    objectValue["81"] = object.altKey;
    objectValue["82"] = object.ctrlKey;
    objectValue["83"] = object.metaKey;
    objectValue["84"] = object.accelKey;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PointerLeaveEvent {
    const nodePtrValue = objectValue["35"];
    const unpackedNodePtr =
      nodePtrValue != undefined
        ? NodeReference.fromValue(nodePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new PointerLeaveEvent({
      position: Vector2.fromValue(objectValue["50"], _session, _supergraph, _graph, _connection),
      pressure: objectValue["51"],
      shiftKey: objectValue["80"],
      altKey: objectValue["81"],
      ctrlKey: objectValue["82"],
      metaKey: objectValue["83"],
      accelKey: objectValue["84"],
      node: unpackedNodePtr,
      parent: unpackedParentPtr,
      space: unpackedSpacePtr,
      id: String(objectValue["2"]),
      _session,
      _graph,
      _connection,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PointerLeaveEvent {
    return PointerLeaveEvent.__unpackValue__(
      objectValue,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  toProto(): PointerLeaveEventProto {
    return PointerLeaveEvent.__packProto__(this);
  }

  static __packProto__(object: PointerLeaveEvent): PointerLeaveEventProto {
    const objectProto: Partial<PointerLeaveEventProto> = { metatype: 9505 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    if (object.nodePtr != null) {
      objectProto.nodePtr = object.nodePtr.toProto();
    }
    objectProto.position = object.position.toProto();
    objectProto.pressure = object.pressure;
    objectProto.shiftKey = object.shiftKey;
    objectProto.altKey = object.altKey;
    objectProto.ctrlKey = object.ctrlKey;
    objectProto.metaKey = object.metaKey;
    objectProto.accelKey = object.accelKey;
    return objectProto as PointerLeaveEventProto;
  }

  static __unpackProto__(
    objectProto: PointerLeaveEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PointerLeaveEvent {
    return new PointerLeaveEvent({
      position: Vector2.fromProto(
        objectProto.position!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      pressure: objectProto.pressure,
      shiftKey: objectProto.shiftKey,
      altKey: objectProto.altKey,
      ctrlKey: objectProto.ctrlKey,
      metaKey: objectProto.metaKey,
      accelKey: objectProto.accelKey,
      node:
        objectProto.nodePtr != undefined
          ? NodeReference.fromProto(
              objectProto.nodePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(
              objectProto.spacePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      id: String(objectProto.id),
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: PointerLeaveEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PointerLeaveEvent {
    return PointerLeaveEvent.__unpackProto__(
      objectProto,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  static fromProtoString(packedProtoString: string): PointerLeaveEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = PointerLeaveEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.POINTER_LEAVE_EVENT, PointerLeaveEvent);
/* ==== DESTACK_GENERATED_END:NODE:9505 ==== */

/* ==== DESTACK_GENERATED_START:NODE:9506 ==== */
/**
 * A LongPressEvent is a PointerEvent when a pointer is pressed down and held for a long time.
 */
export class LongPressEvent extends Node implements PointerEvent {
  static metatype: NodeType = NodeType.LONG_PRESS_EVENT;
  static __traits__: TraitType[] = [
    TraitType.EVENT,
    TraitType.SPATIAL,
    TraitType.INPUT_EVENT,
    TraitType.POINTER_EVENT,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.SPACE];
  static __childTypes__: NodeType[] = [];
  static __ancestorTypes__: NodeType[] = [NodeType.SPACE];
  static __descendantTypes__: NodeType[] = [];

  /**
   * Spatial.parent
   */
  get parent(): Space | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference | null;

  /**
   * The Node this Event is about.
   */
  get node(): Node | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  set node(node: Node | null) {
    if (node === null) {
      this.nodePtr = null;
    } else {
      this.nodePtr = node.toRef();
    }
  }
  nodePtr: NodeReference | null;

  /**
   * PointerEvent.position
   */
  position: Vector2;

  /**
   * PointerEvent.pressure
   */
  pressure: number;

  /**
   * PointerEvent.shiftKey
   */
  shiftKey: boolean;

  /**
   * PointerEvent.altKey
   */
  altKey: boolean;

  /**
   * PointerEvent.ctrlKey
   */
  ctrlKey: boolean;

  /**
   * PointerEvent.metaKey
   */
  metaKey: boolean;

  /**
   * PointerEvent.accelKey
   */
  accelKey: boolean;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    node?: Node | NodeReference | null;
    position: Vector2;
    pressure: number;
    shiftKey: boolean;
    altKey: boolean;
    ctrlKey: boolean;
    metaKey: boolean;
    accelKey: boolean;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: QueryConnection | null;
  }) {
    super(
      // id
      options.id ?? null,
      // parent
      options.parent != null
        ? options.parent.metatype == StructType.NODE_REFERENCE
          ? (options.parent as NodeReference)
          : (options.parent as Node).toRef()
        : null,
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
      // graph
      options._graph ?? null,
      // connection
      options._connection ?? null,
      // is_new
      options.id == null,
      // is_attached
      options.id != null || options._graph != null,
    );

    // properties
    let _parent = options.parent ?? null;
    if (_parent != null && _parent instanceof Node) {
      _parent = _parent.toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space instanceof Node) {
      _space = _space.toRef();
    }
    this.spacePtr = _space;
    let _node = options.node ?? null;
    if (_node != null && _node instanceof Node) {
      _node = _node.toRef();
    }
    this.nodePtr = _node;
    let _position = options.position;
    if (_position === null) {
      throw new Error(`LongPressEvent.position is required`);
    }
    this.position = _position;
    let _pressure = options.pressure;
    if (_pressure === null) {
      throw new Error(`LongPressEvent.pressure is required`);
    }
    this.pressure = _pressure;
    let _shiftKey = options.shiftKey;
    if (_shiftKey === null) {
      throw new Error(`LongPressEvent.shiftKey is required`);
    }
    this.shiftKey = _shiftKey;
    let _altKey = options.altKey;
    if (_altKey === null) {
      throw new Error(`LongPressEvent.altKey is required`);
    }
    this.altKey = _altKey;
    let _ctrlKey = options.ctrlKey;
    if (_ctrlKey === null) {
      throw new Error(`LongPressEvent.ctrlKey is required`);
    }
    this.ctrlKey = _ctrlKey;
    let _metaKey = options.metaKey;
    if (_metaKey === null) {
      throw new Error(`LongPressEvent.metaKey is required`);
    }
    this.metaKey = _metaKey;
    let _accelKey = options.accelKey;
    if (_accelKey === null) {
      throw new Error(`LongPressEvent.accelKey is required`);
    }
    this.accelKey = _accelKey;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(
          `{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`,
        );
      }
      this.createdAt = options.createdAt;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy instanceof Node
            ? options.createdBy.toRef()
            : options.createdBy
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedByPtr =
        options.updatedBy != null
          ? options.updatedBy instanceof Node
            ? options.updatedBy.toRef()
            : options.updatedBy
          : null;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!this.position.equals(other.position)) {
      return false;
    }
    if (!(this.pressure === other.pressure || Math.abs(this.pressure - other.pressure) < 1e-10)) {
      return false;
    }
    if (!(this.shiftKey === other.shiftKey)) {
      return false;
    }
    if (!(this.altKey === other.altKey)) {
      return false;
    }
    if (!(this.ctrlKey === other.ctrlKey)) {
      return false;
    }
    if (!(this.metaKey === other.metaKey)) {
      return false;
    }
    if (!(this.accelKey === other.accelKey)) {
      return false;
    }
    if (!(this.nodePtr?.id === other.nodePtr?.id)) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.position.hash()) & 0xffffffff;
    h = (h * 31 + hashFloat(this.pressure)) & 0xffffffff;
    h = (h * 31 + hashBool(this.shiftKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.altKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.ctrlKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.metaKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.accelKey)) & 0xffffffff;
    if (this.nodePtr !== null) {
      h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    }
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.LONG_PRESS_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return "LongPressEvent[id={this.id}]";
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Node | null = this;
    while (node !== null) {
      pathParts.push(node._pathKey);
      node = node.parent;
    }
    if (!this._isAttached) {
      pathParts.push("<detached>");
    }
    return pathParts.reverse().join("/");
  }

  repr(): string {
    return `<LongPressEvent '${this.path}'>`;
  }

  toValue(): { [key: string]: any } {
    return LongPressEvent.__packValue__(this);
  }

  static __packValue__(object: LongPressEvent): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 9506;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    if (object.nodePtr != null) {
      objectValue["35"] = object.nodePtr.toValue();
    }
    objectValue["50"] = object.position.toValue();
    objectValue["51"] = object.pressure;
    objectValue["80"] = object.shiftKey;
    objectValue["81"] = object.altKey;
    objectValue["82"] = object.ctrlKey;
    objectValue["83"] = object.metaKey;
    objectValue["84"] = object.accelKey;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): LongPressEvent {
    const nodePtrValue = objectValue["35"];
    const unpackedNodePtr =
      nodePtrValue != undefined
        ? NodeReference.fromValue(nodePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new LongPressEvent({
      position: Vector2.fromValue(objectValue["50"], _session, _supergraph, _graph, _connection),
      pressure: objectValue["51"],
      shiftKey: objectValue["80"],
      altKey: objectValue["81"],
      ctrlKey: objectValue["82"],
      metaKey: objectValue["83"],
      accelKey: objectValue["84"],
      node: unpackedNodePtr,
      parent: unpackedParentPtr,
      space: unpackedSpacePtr,
      id: String(objectValue["2"]),
      _session,
      _graph,
      _connection,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): LongPressEvent {
    return LongPressEvent.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): LongPressEventProto {
    return LongPressEvent.__packProto__(this);
  }

  static __packProto__(object: LongPressEvent): LongPressEventProto {
    const objectProto: Partial<LongPressEventProto> = { metatype: 9506 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    if (object.nodePtr != null) {
      objectProto.nodePtr = object.nodePtr.toProto();
    }
    objectProto.position = object.position.toProto();
    objectProto.pressure = object.pressure;
    objectProto.shiftKey = object.shiftKey;
    objectProto.altKey = object.altKey;
    objectProto.ctrlKey = object.ctrlKey;
    objectProto.metaKey = object.metaKey;
    objectProto.accelKey = object.accelKey;
    return objectProto as LongPressEventProto;
  }

  static __unpackProto__(
    objectProto: LongPressEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): LongPressEvent {
    return new LongPressEvent({
      position: Vector2.fromProto(
        objectProto.position!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      pressure: objectProto.pressure,
      shiftKey: objectProto.shiftKey,
      altKey: objectProto.altKey,
      ctrlKey: objectProto.ctrlKey,
      metaKey: objectProto.metaKey,
      accelKey: objectProto.accelKey,
      node:
        objectProto.nodePtr != undefined
          ? NodeReference.fromProto(
              objectProto.nodePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(
              objectProto.spacePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      id: String(objectProto.id),
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: LongPressEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): LongPressEvent {
    return LongPressEvent.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): LongPressEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = LongPressEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.LONG_PRESS_EVENT, LongPressEvent);
/* ==== DESTACK_GENERATED_END:NODE:9506 ==== */

/* ==== DESTACK_GENERATED_START:NODE:9510 ==== */
/**
 * A LeftClickEvent is a ClickEvent when a pointer is clicked with the left button.
 */
export class LeftClickEvent extends Node implements ClickEvent {
  static metatype: NodeType = NodeType.LEFT_CLICK_EVENT;
  static __traits__: TraitType[] = [
    TraitType.SPATIAL,
    TraitType.MOUSE_EVENT,
    TraitType.CLICK_EVENT,
    TraitType.EVENT,
    TraitType.INPUT_EVENT,
    TraitType.POINTER_EVENT,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.SPACE];
  static __childTypes__: NodeType[] = [];
  static __ancestorTypes__: NodeType[] = [NodeType.SPACE];
  static __descendantTypes__: NodeType[] = [];

  /**
   * Spatial.parent
   */
  get parent(): Space | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference | null;

  /**
   * The Node this Event is about.
   */
  get node(): Node | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  set node(node: Node | null) {
    if (node === null) {
      this.nodePtr = null;
    } else {
      this.nodePtr = node.toRef();
    }
  }
  nodePtr: NodeReference | null;

  /**
   * PointerEvent.position
   */
  position: Vector2;

  /**
   * PointerEvent.pressure
   */
  pressure: number;

  /**
   * MouseEvent.button
   */
  button: MouseButton;

  /**
   * PointerEvent.shiftKey
   */
  shiftKey: boolean;

  /**
   * PointerEvent.altKey
   */
  altKey: boolean;

  /**
   * PointerEvent.ctrlKey
   */
  ctrlKey: boolean;

  /**
   * PointerEvent.metaKey
   */
  metaKey: boolean;

  /**
   * PointerEvent.accelKey
   */
  accelKey: boolean;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    node?: Node | NodeReference | null;
    position: Vector2;
    pressure: number;
    button: MouseButton;
    shiftKey: boolean;
    altKey: boolean;
    ctrlKey: boolean;
    metaKey: boolean;
    accelKey: boolean;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: QueryConnection | null;
  }) {
    super(
      // id
      options.id ?? null,
      // parent
      options.parent != null
        ? options.parent.metatype == StructType.NODE_REFERENCE
          ? (options.parent as NodeReference)
          : (options.parent as Node).toRef()
        : null,
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
      // graph
      options._graph ?? null,
      // connection
      options._connection ?? null,
      // is_new
      options.id == null,
      // is_attached
      options.id != null || options._graph != null,
    );

    // properties
    let _parent = options.parent ?? null;
    if (_parent != null && _parent instanceof Node) {
      _parent = _parent.toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space instanceof Node) {
      _space = _space.toRef();
    }
    this.spacePtr = _space;
    let _node = options.node ?? null;
    if (_node != null && _node instanceof Node) {
      _node = _node.toRef();
    }
    this.nodePtr = _node;
    let _position = options.position;
    if (_position === null) {
      throw new Error(`LeftClickEvent.position is required`);
    }
    this.position = _position;
    let _pressure = options.pressure;
    if (_pressure === null) {
      throw new Error(`LeftClickEvent.pressure is required`);
    }
    this.pressure = _pressure;
    let _button = options.button;
    if (_button === null) {
      throw new Error(`LeftClickEvent.button is required`);
    }
    this.button = _button;
    let _shiftKey = options.shiftKey;
    if (_shiftKey === null) {
      throw new Error(`LeftClickEvent.shiftKey is required`);
    }
    this.shiftKey = _shiftKey;
    let _altKey = options.altKey;
    if (_altKey === null) {
      throw new Error(`LeftClickEvent.altKey is required`);
    }
    this.altKey = _altKey;
    let _ctrlKey = options.ctrlKey;
    if (_ctrlKey === null) {
      throw new Error(`LeftClickEvent.ctrlKey is required`);
    }
    this.ctrlKey = _ctrlKey;
    let _metaKey = options.metaKey;
    if (_metaKey === null) {
      throw new Error(`LeftClickEvent.metaKey is required`);
    }
    this.metaKey = _metaKey;
    let _accelKey = options.accelKey;
    if (_accelKey === null) {
      throw new Error(`LeftClickEvent.accelKey is required`);
    }
    this.accelKey = _accelKey;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(
          `{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`,
        );
      }
      this.createdAt = options.createdAt;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy instanceof Node
            ? options.createdBy.toRef()
            : options.createdBy
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedByPtr =
        options.updatedBy != null
          ? options.updatedBy instanceof Node
            ? options.updatedBy.toRef()
            : options.updatedBy
          : null;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.button === other.button)) {
      return false;
    }
    if (!this.position.equals(other.position)) {
      return false;
    }
    if (!(this.pressure === other.pressure || Math.abs(this.pressure - other.pressure) < 1e-10)) {
      return false;
    }
    if (!(this.shiftKey === other.shiftKey)) {
      return false;
    }
    if (!(this.altKey === other.altKey)) {
      return false;
    }
    if (!(this.ctrlKey === other.ctrlKey)) {
      return false;
    }
    if (!(this.metaKey === other.metaKey)) {
      return false;
    }
    if (!(this.accelKey === other.accelKey)) {
      return false;
    }
    if (!(this.nodePtr?.id === other.nodePtr?.id)) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.button) & 0xffffffff;
    h = (h * 31 + this.position.hash()) & 0xffffffff;
    h = (h * 31 + hashFloat(this.pressure)) & 0xffffffff;
    h = (h * 31 + hashBool(this.shiftKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.altKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.ctrlKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.metaKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.accelKey)) & 0xffffffff;
    if (this.nodePtr !== null) {
      h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    }
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.LEFT_CLICK_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return "LeftClickEvent[id={this.id}]";
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Node | null = this;
    while (node !== null) {
      pathParts.push(node._pathKey);
      node = node.parent;
    }
    if (!this._isAttached) {
      pathParts.push("<detached>");
    }
    return pathParts.reverse().join("/");
  }

  repr(): string {
    return `<LeftClickEvent '${this.path}'>`;
  }

  toValue(): { [key: string]: any } {
    return LeftClickEvent.__packValue__(this);
  }

  static __packValue__(object: LeftClickEvent): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 9510;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    if (object.nodePtr != null) {
      objectValue["35"] = object.nodePtr.toValue();
    }
    objectValue["50"] = object.position.toValue();
    objectValue["51"] = object.pressure;
    objectValue["60"] = object.button;
    objectValue["80"] = object.shiftKey;
    objectValue["81"] = object.altKey;
    objectValue["82"] = object.ctrlKey;
    objectValue["83"] = object.metaKey;
    objectValue["84"] = object.accelKey;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): LeftClickEvent {
    const nodePtrValue = objectValue["35"];
    const unpackedNodePtr =
      nodePtrValue != undefined
        ? NodeReference.fromValue(nodePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new LeftClickEvent({
      button: Number(objectValue["60"]),
      position: Vector2.fromValue(objectValue["50"], _session, _supergraph, _graph, _connection),
      pressure: objectValue["51"],
      shiftKey: objectValue["80"],
      altKey: objectValue["81"],
      ctrlKey: objectValue["82"],
      metaKey: objectValue["83"],
      accelKey: objectValue["84"],
      node: unpackedNodePtr,
      parent: unpackedParentPtr,
      space: unpackedSpacePtr,
      id: String(objectValue["2"]),
      _session,
      _graph,
      _connection,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): LeftClickEvent {
    return LeftClickEvent.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): LeftClickEventProto {
    return LeftClickEvent.__packProto__(this);
  }

  static __packProto__(object: LeftClickEvent): LeftClickEventProto {
    const objectProto: Partial<LeftClickEventProto> = { metatype: 9510 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    if (object.nodePtr != null) {
      objectProto.nodePtr = object.nodePtr.toProto();
    }
    objectProto.position = object.position.toProto();
    objectProto.pressure = object.pressure;
    objectProto.button = Number(object.button) as MouseButtonProto;
    objectProto.shiftKey = object.shiftKey;
    objectProto.altKey = object.altKey;
    objectProto.ctrlKey = object.ctrlKey;
    objectProto.metaKey = object.metaKey;
    objectProto.accelKey = object.accelKey;
    return objectProto as LeftClickEventProto;
  }

  static __unpackProto__(
    objectProto: LeftClickEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): LeftClickEvent {
    return new LeftClickEvent({
      button: Number(objectProto.button) as MouseButton,
      position: Vector2.fromProto(
        objectProto.position!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      pressure: objectProto.pressure,
      shiftKey: objectProto.shiftKey,
      altKey: objectProto.altKey,
      ctrlKey: objectProto.ctrlKey,
      metaKey: objectProto.metaKey,
      accelKey: objectProto.accelKey,
      node:
        objectProto.nodePtr != undefined
          ? NodeReference.fromProto(
              objectProto.nodePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(
              objectProto.spacePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      id: String(objectProto.id),
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: LeftClickEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): LeftClickEvent {
    return LeftClickEvent.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): LeftClickEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = LeftClickEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.LEFT_CLICK_EVENT, LeftClickEvent);
/* ==== DESTACK_GENERATED_END:NODE:9510 ==== */

/* ==== DESTACK_GENERATED_START:NODE:9511 ==== */
/**
 * A RightClickEvent is a ClickEvent when a pointer is clicked with the right button.
 */
export class RightClickEvent extends Node implements ClickEvent {
  static metatype: NodeType = NodeType.RIGHT_CLICK_EVENT;
  static __traits__: TraitType[] = [
    TraitType.SPATIAL,
    TraitType.MOUSE_EVENT,
    TraitType.CLICK_EVENT,
    TraitType.EVENT,
    TraitType.INPUT_EVENT,
    TraitType.POINTER_EVENT,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.SPACE];
  static __childTypes__: NodeType[] = [];
  static __ancestorTypes__: NodeType[] = [NodeType.SPACE];
  static __descendantTypes__: NodeType[] = [];

  /**
   * Spatial.parent
   */
  get parent(): Space | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference | null;

  /**
   * The Node this Event is about.
   */
  get node(): Node | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  set node(node: Node | null) {
    if (node === null) {
      this.nodePtr = null;
    } else {
      this.nodePtr = node.toRef();
    }
  }
  nodePtr: NodeReference | null;

  /**
   * PointerEvent.position
   */
  position: Vector2;

  /**
   * PointerEvent.pressure
   */
  pressure: number;

  /**
   * MouseEvent.button
   */
  button: MouseButton;

  /**
   * PointerEvent.shiftKey
   */
  shiftKey: boolean;

  /**
   * PointerEvent.altKey
   */
  altKey: boolean;

  /**
   * PointerEvent.ctrlKey
   */
  ctrlKey: boolean;

  /**
   * PointerEvent.metaKey
   */
  metaKey: boolean;

  /**
   * PointerEvent.accelKey
   */
  accelKey: boolean;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    node?: Node | NodeReference | null;
    position: Vector2;
    pressure: number;
    button: MouseButton;
    shiftKey: boolean;
    altKey: boolean;
    ctrlKey: boolean;
    metaKey: boolean;
    accelKey: boolean;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: QueryConnection | null;
  }) {
    super(
      // id
      options.id ?? null,
      // parent
      options.parent != null
        ? options.parent.metatype == StructType.NODE_REFERENCE
          ? (options.parent as NodeReference)
          : (options.parent as Node).toRef()
        : null,
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
      // graph
      options._graph ?? null,
      // connection
      options._connection ?? null,
      // is_new
      options.id == null,
      // is_attached
      options.id != null || options._graph != null,
    );

    // properties
    let _parent = options.parent ?? null;
    if (_parent != null && _parent instanceof Node) {
      _parent = _parent.toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space instanceof Node) {
      _space = _space.toRef();
    }
    this.spacePtr = _space;
    let _node = options.node ?? null;
    if (_node != null && _node instanceof Node) {
      _node = _node.toRef();
    }
    this.nodePtr = _node;
    let _position = options.position;
    if (_position === null) {
      throw new Error(`RightClickEvent.position is required`);
    }
    this.position = _position;
    let _pressure = options.pressure;
    if (_pressure === null) {
      throw new Error(`RightClickEvent.pressure is required`);
    }
    this.pressure = _pressure;
    let _button = options.button;
    if (_button === null) {
      throw new Error(`RightClickEvent.button is required`);
    }
    this.button = _button;
    let _shiftKey = options.shiftKey;
    if (_shiftKey === null) {
      throw new Error(`RightClickEvent.shiftKey is required`);
    }
    this.shiftKey = _shiftKey;
    let _altKey = options.altKey;
    if (_altKey === null) {
      throw new Error(`RightClickEvent.altKey is required`);
    }
    this.altKey = _altKey;
    let _ctrlKey = options.ctrlKey;
    if (_ctrlKey === null) {
      throw new Error(`RightClickEvent.ctrlKey is required`);
    }
    this.ctrlKey = _ctrlKey;
    let _metaKey = options.metaKey;
    if (_metaKey === null) {
      throw new Error(`RightClickEvent.metaKey is required`);
    }
    this.metaKey = _metaKey;
    let _accelKey = options.accelKey;
    if (_accelKey === null) {
      throw new Error(`RightClickEvent.accelKey is required`);
    }
    this.accelKey = _accelKey;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(
          `{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`,
        );
      }
      this.createdAt = options.createdAt;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy instanceof Node
            ? options.createdBy.toRef()
            : options.createdBy
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedByPtr =
        options.updatedBy != null
          ? options.updatedBy instanceof Node
            ? options.updatedBy.toRef()
            : options.updatedBy
          : null;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.button === other.button)) {
      return false;
    }
    if (!this.position.equals(other.position)) {
      return false;
    }
    if (!(this.pressure === other.pressure || Math.abs(this.pressure - other.pressure) < 1e-10)) {
      return false;
    }
    if (!(this.shiftKey === other.shiftKey)) {
      return false;
    }
    if (!(this.altKey === other.altKey)) {
      return false;
    }
    if (!(this.ctrlKey === other.ctrlKey)) {
      return false;
    }
    if (!(this.metaKey === other.metaKey)) {
      return false;
    }
    if (!(this.accelKey === other.accelKey)) {
      return false;
    }
    if (!(this.nodePtr?.id === other.nodePtr?.id)) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.button) & 0xffffffff;
    h = (h * 31 + this.position.hash()) & 0xffffffff;
    h = (h * 31 + hashFloat(this.pressure)) & 0xffffffff;
    h = (h * 31 + hashBool(this.shiftKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.altKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.ctrlKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.metaKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.accelKey)) & 0xffffffff;
    if (this.nodePtr !== null) {
      h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    }
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.RIGHT_CLICK_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return "RightClickEvent[id={this.id}]";
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Node | null = this;
    while (node !== null) {
      pathParts.push(node._pathKey);
      node = node.parent;
    }
    if (!this._isAttached) {
      pathParts.push("<detached>");
    }
    return pathParts.reverse().join("/");
  }

  repr(): string {
    return `<RightClickEvent '${this.path}'>`;
  }

  toValue(): { [key: string]: any } {
    return RightClickEvent.__packValue__(this);
  }

  static __packValue__(object: RightClickEvent): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 9511;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    if (object.nodePtr != null) {
      objectValue["35"] = object.nodePtr.toValue();
    }
    objectValue["50"] = object.position.toValue();
    objectValue["51"] = object.pressure;
    objectValue["60"] = object.button;
    objectValue["80"] = object.shiftKey;
    objectValue["81"] = object.altKey;
    objectValue["82"] = object.ctrlKey;
    objectValue["83"] = object.metaKey;
    objectValue["84"] = object.accelKey;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): RightClickEvent {
    const nodePtrValue = objectValue["35"];
    const unpackedNodePtr =
      nodePtrValue != undefined
        ? NodeReference.fromValue(nodePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new RightClickEvent({
      button: Number(objectValue["60"]),
      position: Vector2.fromValue(objectValue["50"], _session, _supergraph, _graph, _connection),
      pressure: objectValue["51"],
      shiftKey: objectValue["80"],
      altKey: objectValue["81"],
      ctrlKey: objectValue["82"],
      metaKey: objectValue["83"],
      accelKey: objectValue["84"],
      node: unpackedNodePtr,
      parent: unpackedParentPtr,
      space: unpackedSpacePtr,
      id: String(objectValue["2"]),
      _session,
      _graph,
      _connection,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): RightClickEvent {
    return RightClickEvent.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): RightClickEventProto {
    return RightClickEvent.__packProto__(this);
  }

  static __packProto__(object: RightClickEvent): RightClickEventProto {
    const objectProto: Partial<RightClickEventProto> = { metatype: 9511 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    if (object.nodePtr != null) {
      objectProto.nodePtr = object.nodePtr.toProto();
    }
    objectProto.position = object.position.toProto();
    objectProto.pressure = object.pressure;
    objectProto.button = Number(object.button) as MouseButtonProto;
    objectProto.shiftKey = object.shiftKey;
    objectProto.altKey = object.altKey;
    objectProto.ctrlKey = object.ctrlKey;
    objectProto.metaKey = object.metaKey;
    objectProto.accelKey = object.accelKey;
    return objectProto as RightClickEventProto;
  }

  static __unpackProto__(
    objectProto: RightClickEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): RightClickEvent {
    return new RightClickEvent({
      button: Number(objectProto.button) as MouseButton,
      position: Vector2.fromProto(
        objectProto.position!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      pressure: objectProto.pressure,
      shiftKey: objectProto.shiftKey,
      altKey: objectProto.altKey,
      ctrlKey: objectProto.ctrlKey,
      metaKey: objectProto.metaKey,
      accelKey: objectProto.accelKey,
      node:
        objectProto.nodePtr != undefined
          ? NodeReference.fromProto(
              objectProto.nodePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(
              objectProto.spacePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      id: String(objectProto.id),
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: RightClickEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): RightClickEvent {
    return RightClickEvent.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): RightClickEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = RightClickEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.RIGHT_CLICK_EVENT, RightClickEvent);
/* ==== DESTACK_GENERATED_END:NODE:9511 ==== */

/* ==== DESTACK_GENERATED_START:NODE:9512 ==== */
/**
 * A MiddleClickEvent is a ClickEvent when a pointer is clicked with the middle button.
 */
export class MiddleClickEvent extends Node implements ClickEvent {
  static metatype: NodeType = NodeType.MIDDLE_CLICK_EVENT;
  static __traits__: TraitType[] = [
    TraitType.SPATIAL,
    TraitType.MOUSE_EVENT,
    TraitType.CLICK_EVENT,
    TraitType.EVENT,
    TraitType.INPUT_EVENT,
    TraitType.POINTER_EVENT,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.SPACE];
  static __childTypes__: NodeType[] = [];
  static __ancestorTypes__: NodeType[] = [NodeType.SPACE];
  static __descendantTypes__: NodeType[] = [];

  /**
   * Spatial.parent
   */
  get parent(): Space | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference | null;

  /**
   * The Node this Event is about.
   */
  get node(): Node | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  set node(node: Node | null) {
    if (node === null) {
      this.nodePtr = null;
    } else {
      this.nodePtr = node.toRef();
    }
  }
  nodePtr: NodeReference | null;

  /**
   * PointerEvent.position
   */
  position: Vector2;

  /**
   * PointerEvent.pressure
   */
  pressure: number;

  /**
   * MouseEvent.button
   */
  button: MouseButton;

  /**
   * PointerEvent.shiftKey
   */
  shiftKey: boolean;

  /**
   * PointerEvent.altKey
   */
  altKey: boolean;

  /**
   * PointerEvent.ctrlKey
   */
  ctrlKey: boolean;

  /**
   * PointerEvent.metaKey
   */
  metaKey: boolean;

  /**
   * PointerEvent.accelKey
   */
  accelKey: boolean;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    node?: Node | NodeReference | null;
    position: Vector2;
    pressure: number;
    button: MouseButton;
    shiftKey: boolean;
    altKey: boolean;
    ctrlKey: boolean;
    metaKey: boolean;
    accelKey: boolean;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: QueryConnection | null;
  }) {
    super(
      // id
      options.id ?? null,
      // parent
      options.parent != null
        ? options.parent.metatype == StructType.NODE_REFERENCE
          ? (options.parent as NodeReference)
          : (options.parent as Node).toRef()
        : null,
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
      // graph
      options._graph ?? null,
      // connection
      options._connection ?? null,
      // is_new
      options.id == null,
      // is_attached
      options.id != null || options._graph != null,
    );

    // properties
    let _parent = options.parent ?? null;
    if (_parent != null && _parent instanceof Node) {
      _parent = _parent.toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space instanceof Node) {
      _space = _space.toRef();
    }
    this.spacePtr = _space;
    let _node = options.node ?? null;
    if (_node != null && _node instanceof Node) {
      _node = _node.toRef();
    }
    this.nodePtr = _node;
    let _position = options.position;
    if (_position === null) {
      throw new Error(`MiddleClickEvent.position is required`);
    }
    this.position = _position;
    let _pressure = options.pressure;
    if (_pressure === null) {
      throw new Error(`MiddleClickEvent.pressure is required`);
    }
    this.pressure = _pressure;
    let _button = options.button;
    if (_button === null) {
      throw new Error(`MiddleClickEvent.button is required`);
    }
    this.button = _button;
    let _shiftKey = options.shiftKey;
    if (_shiftKey === null) {
      throw new Error(`MiddleClickEvent.shiftKey is required`);
    }
    this.shiftKey = _shiftKey;
    let _altKey = options.altKey;
    if (_altKey === null) {
      throw new Error(`MiddleClickEvent.altKey is required`);
    }
    this.altKey = _altKey;
    let _ctrlKey = options.ctrlKey;
    if (_ctrlKey === null) {
      throw new Error(`MiddleClickEvent.ctrlKey is required`);
    }
    this.ctrlKey = _ctrlKey;
    let _metaKey = options.metaKey;
    if (_metaKey === null) {
      throw new Error(`MiddleClickEvent.metaKey is required`);
    }
    this.metaKey = _metaKey;
    let _accelKey = options.accelKey;
    if (_accelKey === null) {
      throw new Error(`MiddleClickEvent.accelKey is required`);
    }
    this.accelKey = _accelKey;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(
          `{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`,
        );
      }
      this.createdAt = options.createdAt;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy instanceof Node
            ? options.createdBy.toRef()
            : options.createdBy
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedByPtr =
        options.updatedBy != null
          ? options.updatedBy instanceof Node
            ? options.updatedBy.toRef()
            : options.updatedBy
          : null;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.button === other.button)) {
      return false;
    }
    if (!this.position.equals(other.position)) {
      return false;
    }
    if (!(this.pressure === other.pressure || Math.abs(this.pressure - other.pressure) < 1e-10)) {
      return false;
    }
    if (!(this.shiftKey === other.shiftKey)) {
      return false;
    }
    if (!(this.altKey === other.altKey)) {
      return false;
    }
    if (!(this.ctrlKey === other.ctrlKey)) {
      return false;
    }
    if (!(this.metaKey === other.metaKey)) {
      return false;
    }
    if (!(this.accelKey === other.accelKey)) {
      return false;
    }
    if (!(this.nodePtr?.id === other.nodePtr?.id)) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.button) & 0xffffffff;
    h = (h * 31 + this.position.hash()) & 0xffffffff;
    h = (h * 31 + hashFloat(this.pressure)) & 0xffffffff;
    h = (h * 31 + hashBool(this.shiftKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.altKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.ctrlKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.metaKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.accelKey)) & 0xffffffff;
    if (this.nodePtr !== null) {
      h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    }
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.MIDDLE_CLICK_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return "MiddleClickEvent[id={this.id}]";
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Node | null = this;
    while (node !== null) {
      pathParts.push(node._pathKey);
      node = node.parent;
    }
    if (!this._isAttached) {
      pathParts.push("<detached>");
    }
    return pathParts.reverse().join("/");
  }

  repr(): string {
    return `<MiddleClickEvent '${this.path}'>`;
  }

  toValue(): { [key: string]: any } {
    return MiddleClickEvent.__packValue__(this);
  }

  static __packValue__(object: MiddleClickEvent): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 9512;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    if (object.nodePtr != null) {
      objectValue["35"] = object.nodePtr.toValue();
    }
    objectValue["50"] = object.position.toValue();
    objectValue["51"] = object.pressure;
    objectValue["60"] = object.button;
    objectValue["80"] = object.shiftKey;
    objectValue["81"] = object.altKey;
    objectValue["82"] = object.ctrlKey;
    objectValue["83"] = object.metaKey;
    objectValue["84"] = object.accelKey;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): MiddleClickEvent {
    const nodePtrValue = objectValue["35"];
    const unpackedNodePtr =
      nodePtrValue != undefined
        ? NodeReference.fromValue(nodePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new MiddleClickEvent({
      button: Number(objectValue["60"]),
      position: Vector2.fromValue(objectValue["50"], _session, _supergraph, _graph, _connection),
      pressure: objectValue["51"],
      shiftKey: objectValue["80"],
      altKey: objectValue["81"],
      ctrlKey: objectValue["82"],
      metaKey: objectValue["83"],
      accelKey: objectValue["84"],
      node: unpackedNodePtr,
      parent: unpackedParentPtr,
      space: unpackedSpacePtr,
      id: String(objectValue["2"]),
      _session,
      _graph,
      _connection,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): MiddleClickEvent {
    return MiddleClickEvent.__unpackValue__(
      objectValue,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  toProto(): MiddleClickEventProto {
    return MiddleClickEvent.__packProto__(this);
  }

  static __packProto__(object: MiddleClickEvent): MiddleClickEventProto {
    const objectProto: Partial<MiddleClickEventProto> = { metatype: 9512 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    if (object.nodePtr != null) {
      objectProto.nodePtr = object.nodePtr.toProto();
    }
    objectProto.position = object.position.toProto();
    objectProto.pressure = object.pressure;
    objectProto.button = Number(object.button) as MouseButtonProto;
    objectProto.shiftKey = object.shiftKey;
    objectProto.altKey = object.altKey;
    objectProto.ctrlKey = object.ctrlKey;
    objectProto.metaKey = object.metaKey;
    objectProto.accelKey = object.accelKey;
    return objectProto as MiddleClickEventProto;
  }

  static __unpackProto__(
    objectProto: MiddleClickEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): MiddleClickEvent {
    return new MiddleClickEvent({
      button: Number(objectProto.button) as MouseButton,
      position: Vector2.fromProto(
        objectProto.position!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      pressure: objectProto.pressure,
      shiftKey: objectProto.shiftKey,
      altKey: objectProto.altKey,
      ctrlKey: objectProto.ctrlKey,
      metaKey: objectProto.metaKey,
      accelKey: objectProto.accelKey,
      node:
        objectProto.nodePtr != undefined
          ? NodeReference.fromProto(
              objectProto.nodePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(
              objectProto.spacePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      id: String(objectProto.id),
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: MiddleClickEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): MiddleClickEvent {
    return MiddleClickEvent.__unpackProto__(
      objectProto,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  static fromProtoString(packedProtoString: string): MiddleClickEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = MiddleClickEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.MIDDLE_CLICK_EVENT, MiddleClickEvent);
/* ==== DESTACK_GENERATED_END:NODE:9512 ==== */

/* ==== DESTACK_GENERATED_START:NODE:9513 ==== */
/**
 * A DoubleClickEvent is a ClickEvent when a pointer is clicked twice in a short time.
 */
export class DoubleClickEvent extends Node implements ClickEvent {
  static metatype: NodeType = NodeType.DOUBLE_CLICK_EVENT;
  static __traits__: TraitType[] = [
    TraitType.SPATIAL,
    TraitType.MOUSE_EVENT,
    TraitType.CLICK_EVENT,
    TraitType.EVENT,
    TraitType.INPUT_EVENT,
    TraitType.POINTER_EVENT,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.SPACE];
  static __childTypes__: NodeType[] = [];
  static __ancestorTypes__: NodeType[] = [NodeType.SPACE];
  static __descendantTypes__: NodeType[] = [];

  /**
   * Spatial.parent
   */
  get parent(): Space | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference | null;

  /**
   * The Node this Event is about.
   */
  get node(): Node | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  set node(node: Node | null) {
    if (node === null) {
      this.nodePtr = null;
    } else {
      this.nodePtr = node.toRef();
    }
  }
  nodePtr: NodeReference | null;

  /**
   * PointerEvent.position
   */
  position: Vector2;

  /**
   * PointerEvent.pressure
   */
  pressure: number;

  /**
   * MouseEvent.button
   */
  button: MouseButton;

  /**
   * PointerEvent.shiftKey
   */
  shiftKey: boolean;

  /**
   * PointerEvent.altKey
   */
  altKey: boolean;

  /**
   * PointerEvent.ctrlKey
   */
  ctrlKey: boolean;

  /**
   * PointerEvent.metaKey
   */
  metaKey: boolean;

  /**
   * PointerEvent.accelKey
   */
  accelKey: boolean;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    node?: Node | NodeReference | null;
    position: Vector2;
    pressure: number;
    button: MouseButton;
    shiftKey: boolean;
    altKey: boolean;
    ctrlKey: boolean;
    metaKey: boolean;
    accelKey: boolean;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: QueryConnection | null;
  }) {
    super(
      // id
      options.id ?? null,
      // parent
      options.parent != null
        ? options.parent.metatype == StructType.NODE_REFERENCE
          ? (options.parent as NodeReference)
          : (options.parent as Node).toRef()
        : null,
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
      // graph
      options._graph ?? null,
      // connection
      options._connection ?? null,
      // is_new
      options.id == null,
      // is_attached
      options.id != null || options._graph != null,
    );

    // properties
    let _parent = options.parent ?? null;
    if (_parent != null && _parent instanceof Node) {
      _parent = _parent.toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space instanceof Node) {
      _space = _space.toRef();
    }
    this.spacePtr = _space;
    let _node = options.node ?? null;
    if (_node != null && _node instanceof Node) {
      _node = _node.toRef();
    }
    this.nodePtr = _node;
    let _position = options.position;
    if (_position === null) {
      throw new Error(`DoubleClickEvent.position is required`);
    }
    this.position = _position;
    let _pressure = options.pressure;
    if (_pressure === null) {
      throw new Error(`DoubleClickEvent.pressure is required`);
    }
    this.pressure = _pressure;
    let _button = options.button;
    if (_button === null) {
      throw new Error(`DoubleClickEvent.button is required`);
    }
    this.button = _button;
    let _shiftKey = options.shiftKey;
    if (_shiftKey === null) {
      throw new Error(`DoubleClickEvent.shiftKey is required`);
    }
    this.shiftKey = _shiftKey;
    let _altKey = options.altKey;
    if (_altKey === null) {
      throw new Error(`DoubleClickEvent.altKey is required`);
    }
    this.altKey = _altKey;
    let _ctrlKey = options.ctrlKey;
    if (_ctrlKey === null) {
      throw new Error(`DoubleClickEvent.ctrlKey is required`);
    }
    this.ctrlKey = _ctrlKey;
    let _metaKey = options.metaKey;
    if (_metaKey === null) {
      throw new Error(`DoubleClickEvent.metaKey is required`);
    }
    this.metaKey = _metaKey;
    let _accelKey = options.accelKey;
    if (_accelKey === null) {
      throw new Error(`DoubleClickEvent.accelKey is required`);
    }
    this.accelKey = _accelKey;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(
          `{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`,
        );
      }
      this.createdAt = options.createdAt;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy instanceof Node
            ? options.createdBy.toRef()
            : options.createdBy
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedByPtr =
        options.updatedBy != null
          ? options.updatedBy instanceof Node
            ? options.updatedBy.toRef()
            : options.updatedBy
          : null;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.button === other.button)) {
      return false;
    }
    if (!this.position.equals(other.position)) {
      return false;
    }
    if (!(this.pressure === other.pressure || Math.abs(this.pressure - other.pressure) < 1e-10)) {
      return false;
    }
    if (!(this.shiftKey === other.shiftKey)) {
      return false;
    }
    if (!(this.altKey === other.altKey)) {
      return false;
    }
    if (!(this.ctrlKey === other.ctrlKey)) {
      return false;
    }
    if (!(this.metaKey === other.metaKey)) {
      return false;
    }
    if (!(this.accelKey === other.accelKey)) {
      return false;
    }
    if (!(this.nodePtr?.id === other.nodePtr?.id)) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.button) & 0xffffffff;
    h = (h * 31 + this.position.hash()) & 0xffffffff;
    h = (h * 31 + hashFloat(this.pressure)) & 0xffffffff;
    h = (h * 31 + hashBool(this.shiftKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.altKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.ctrlKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.metaKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.accelKey)) & 0xffffffff;
    if (this.nodePtr !== null) {
      h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    }
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.DOUBLE_CLICK_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return "DoubleClickEvent[id={this.id}]";
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Node | null = this;
    while (node !== null) {
      pathParts.push(node._pathKey);
      node = node.parent;
    }
    if (!this._isAttached) {
      pathParts.push("<detached>");
    }
    return pathParts.reverse().join("/");
  }

  repr(): string {
    return `<DoubleClickEvent '${this.path}'>`;
  }

  toValue(): { [key: string]: any } {
    return DoubleClickEvent.__packValue__(this);
  }

  static __packValue__(object: DoubleClickEvent): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 9513;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    if (object.nodePtr != null) {
      objectValue["35"] = object.nodePtr.toValue();
    }
    objectValue["50"] = object.position.toValue();
    objectValue["51"] = object.pressure;
    objectValue["60"] = object.button;
    objectValue["80"] = object.shiftKey;
    objectValue["81"] = object.altKey;
    objectValue["82"] = object.ctrlKey;
    objectValue["83"] = object.metaKey;
    objectValue["84"] = object.accelKey;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): DoubleClickEvent {
    const nodePtrValue = objectValue["35"];
    const unpackedNodePtr =
      nodePtrValue != undefined
        ? NodeReference.fromValue(nodePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new DoubleClickEvent({
      button: Number(objectValue["60"]),
      position: Vector2.fromValue(objectValue["50"], _session, _supergraph, _graph, _connection),
      pressure: objectValue["51"],
      shiftKey: objectValue["80"],
      altKey: objectValue["81"],
      ctrlKey: objectValue["82"],
      metaKey: objectValue["83"],
      accelKey: objectValue["84"],
      node: unpackedNodePtr,
      parent: unpackedParentPtr,
      space: unpackedSpacePtr,
      id: String(objectValue["2"]),
      _session,
      _graph,
      _connection,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): DoubleClickEvent {
    return DoubleClickEvent.__unpackValue__(
      objectValue,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  toProto(): DoubleClickEventProto {
    return DoubleClickEvent.__packProto__(this);
  }

  static __packProto__(object: DoubleClickEvent): DoubleClickEventProto {
    const objectProto: Partial<DoubleClickEventProto> = { metatype: 9513 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    if (object.nodePtr != null) {
      objectProto.nodePtr = object.nodePtr.toProto();
    }
    objectProto.position = object.position.toProto();
    objectProto.pressure = object.pressure;
    objectProto.button = Number(object.button) as MouseButtonProto;
    objectProto.shiftKey = object.shiftKey;
    objectProto.altKey = object.altKey;
    objectProto.ctrlKey = object.ctrlKey;
    objectProto.metaKey = object.metaKey;
    objectProto.accelKey = object.accelKey;
    return objectProto as DoubleClickEventProto;
  }

  static __unpackProto__(
    objectProto: DoubleClickEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): DoubleClickEvent {
    return new DoubleClickEvent({
      button: Number(objectProto.button) as MouseButton,
      position: Vector2.fromProto(
        objectProto.position!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      pressure: objectProto.pressure,
      shiftKey: objectProto.shiftKey,
      altKey: objectProto.altKey,
      ctrlKey: objectProto.ctrlKey,
      metaKey: objectProto.metaKey,
      accelKey: objectProto.accelKey,
      node:
        objectProto.nodePtr != undefined
          ? NodeReference.fromProto(
              objectProto.nodePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(
              objectProto.spacePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      id: String(objectProto.id),
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: DoubleClickEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): DoubleClickEvent {
    return DoubleClickEvent.__unpackProto__(
      objectProto,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  static fromProtoString(packedProtoString: string): DoubleClickEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = DoubleClickEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.DOUBLE_CLICK_EVENT, DoubleClickEvent);
/* ==== DESTACK_GENERATED_END:NODE:9513 ==== */

/* ==== DESTACK_GENERATED_START:NODE:9514 ==== */
/**
 * A WheelEvent is a MouseEvent when a wheel is scrolled.
 */
export class WheelEvent extends Node implements MouseEvent {
  static metatype: NodeType = NodeType.WHEEL_EVENT;
  static __traits__: TraitType[] = [
    TraitType.SPATIAL,
    TraitType.MOUSE_EVENT,
    TraitType.EVENT,
    TraitType.INPUT_EVENT,
    TraitType.POINTER_EVENT,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.SPACE];
  static __childTypes__: NodeType[] = [];
  static __ancestorTypes__: NodeType[] = [NodeType.SPACE];
  static __descendantTypes__: NodeType[] = [];

  /**
   * Spatial.parent
   */
  get parent(): Space | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference | null;

  /**
   * The Node this Event is about.
   */
  get node(): Node | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  set node(node: Node | null) {
    if (node === null) {
      this.nodePtr = null;
    } else {
      this.nodePtr = node.toRef();
    }
  }
  nodePtr: NodeReference | null;

  /**
   * PointerEvent.position
   */
  position: Vector2;

  /**
   * PointerEvent.pressure
   */
  pressure: number;

  /**
   * MouseEvent.button
   */
  button: MouseButton;

  /**
   * WheelEvent.delta
   */
  delta: Vector2;

  /**
   * PointerEvent.shiftKey
   */
  shiftKey: boolean;

  /**
   * PointerEvent.altKey
   */
  altKey: boolean;

  /**
   * PointerEvent.ctrlKey
   */
  ctrlKey: boolean;

  /**
   * PointerEvent.metaKey
   */
  metaKey: boolean;

  /**
   * PointerEvent.accelKey
   */
  accelKey: boolean;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    node?: Node | NodeReference | null;
    position: Vector2;
    pressure: number;
    button: MouseButton;
    delta: Vector2;
    shiftKey: boolean;
    altKey: boolean;
    ctrlKey: boolean;
    metaKey: boolean;
    accelKey: boolean;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: QueryConnection | null;
  }) {
    super(
      // id
      options.id ?? null,
      // parent
      options.parent != null
        ? options.parent.metatype == StructType.NODE_REFERENCE
          ? (options.parent as NodeReference)
          : (options.parent as Node).toRef()
        : null,
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
      // graph
      options._graph ?? null,
      // connection
      options._connection ?? null,
      // is_new
      options.id == null,
      // is_attached
      options.id != null || options._graph != null,
    );

    // properties
    let _parent = options.parent ?? null;
    if (_parent != null && _parent instanceof Node) {
      _parent = _parent.toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space instanceof Node) {
      _space = _space.toRef();
    }
    this.spacePtr = _space;
    let _node = options.node ?? null;
    if (_node != null && _node instanceof Node) {
      _node = _node.toRef();
    }
    this.nodePtr = _node;
    let _position = options.position;
    if (_position === null) {
      throw new Error(`WheelEvent.position is required`);
    }
    this.position = _position;
    let _pressure = options.pressure;
    if (_pressure === null) {
      throw new Error(`WheelEvent.pressure is required`);
    }
    this.pressure = _pressure;
    let _button = options.button;
    if (_button === null) {
      throw new Error(`WheelEvent.button is required`);
    }
    this.button = _button;
    let _delta = options.delta;
    if (_delta === null) {
      throw new Error(`WheelEvent.delta is required`);
    }
    this.delta = _delta;
    let _shiftKey = options.shiftKey;
    if (_shiftKey === null) {
      throw new Error(`WheelEvent.shiftKey is required`);
    }
    this.shiftKey = _shiftKey;
    let _altKey = options.altKey;
    if (_altKey === null) {
      throw new Error(`WheelEvent.altKey is required`);
    }
    this.altKey = _altKey;
    let _ctrlKey = options.ctrlKey;
    if (_ctrlKey === null) {
      throw new Error(`WheelEvent.ctrlKey is required`);
    }
    this.ctrlKey = _ctrlKey;
    let _metaKey = options.metaKey;
    if (_metaKey === null) {
      throw new Error(`WheelEvent.metaKey is required`);
    }
    this.metaKey = _metaKey;
    let _accelKey = options.accelKey;
    if (_accelKey === null) {
      throw new Error(`WheelEvent.accelKey is required`);
    }
    this.accelKey = _accelKey;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(
          `{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`,
        );
      }
      this.createdAt = options.createdAt;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy instanceof Node
            ? options.createdBy.toRef()
            : options.createdBy
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedByPtr =
        options.updatedBy != null
          ? options.updatedBy instanceof Node
            ? options.updatedBy.toRef()
            : options.updatedBy
          : null;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!this.delta.equals(other.delta)) {
      return false;
    }
    if (!(this.button === other.button)) {
      return false;
    }
    if (!this.position.equals(other.position)) {
      return false;
    }
    if (!(this.pressure === other.pressure || Math.abs(this.pressure - other.pressure) < 1e-10)) {
      return false;
    }
    if (!(this.shiftKey === other.shiftKey)) {
      return false;
    }
    if (!(this.altKey === other.altKey)) {
      return false;
    }
    if (!(this.ctrlKey === other.ctrlKey)) {
      return false;
    }
    if (!(this.metaKey === other.metaKey)) {
      return false;
    }
    if (!(this.accelKey === other.accelKey)) {
      return false;
    }
    if (!(this.nodePtr?.id === other.nodePtr?.id)) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.delta.hash()) & 0xffffffff;
    h = (h * 31 + this.button) & 0xffffffff;
    h = (h * 31 + this.position.hash()) & 0xffffffff;
    h = (h * 31 + hashFloat(this.pressure)) & 0xffffffff;
    h = (h * 31 + hashBool(this.shiftKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.altKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.ctrlKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.metaKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.accelKey)) & 0xffffffff;
    if (this.nodePtr !== null) {
      h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    }
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.WHEEL_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return "WheelEvent[id={this.id}]";
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Node | null = this;
    while (node !== null) {
      pathParts.push(node._pathKey);
      node = node.parent;
    }
    if (!this._isAttached) {
      pathParts.push("<detached>");
    }
    return pathParts.reverse().join("/");
  }

  repr(): string {
    return `<WheelEvent '${this.path}'>`;
  }

  toValue(): { [key: string]: any } {
    return WheelEvent.__packValue__(this);
  }

  static __packValue__(object: WheelEvent): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 9514;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    if (object.nodePtr != null) {
      objectValue["35"] = object.nodePtr.toValue();
    }
    objectValue["50"] = object.position.toValue();
    objectValue["51"] = object.pressure;
    objectValue["60"] = object.button;
    objectValue["70"] = object.delta.toValue();
    objectValue["80"] = object.shiftKey;
    objectValue["81"] = object.altKey;
    objectValue["82"] = object.ctrlKey;
    objectValue["83"] = object.metaKey;
    objectValue["84"] = object.accelKey;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): WheelEvent {
    const nodePtrValue = objectValue["35"];
    const unpackedNodePtr =
      nodePtrValue != undefined
        ? NodeReference.fromValue(nodePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new WheelEvent({
      delta: Vector2.fromValue(objectValue["70"], _session, _supergraph, _graph, _connection),
      button: Number(objectValue["60"]),
      position: Vector2.fromValue(objectValue["50"], _session, _supergraph, _graph, _connection),
      pressure: objectValue["51"],
      shiftKey: objectValue["80"],
      altKey: objectValue["81"],
      ctrlKey: objectValue["82"],
      metaKey: objectValue["83"],
      accelKey: objectValue["84"],
      node: unpackedNodePtr,
      parent: unpackedParentPtr,
      space: unpackedSpacePtr,
      id: String(objectValue["2"]),
      _session,
      _graph,
      _connection,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): WheelEvent {
    return WheelEvent.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): WheelEventProto {
    return WheelEvent.__packProto__(this);
  }

  static __packProto__(object: WheelEvent): WheelEventProto {
    const objectProto: Partial<WheelEventProto> = { metatype: 9514 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    if (object.nodePtr != null) {
      objectProto.nodePtr = object.nodePtr.toProto();
    }
    objectProto.position = object.position.toProto();
    objectProto.pressure = object.pressure;
    objectProto.button = Number(object.button) as MouseButtonProto;
    objectProto.delta = object.delta.toProto();
    objectProto.shiftKey = object.shiftKey;
    objectProto.altKey = object.altKey;
    objectProto.ctrlKey = object.ctrlKey;
    objectProto.metaKey = object.metaKey;
    objectProto.accelKey = object.accelKey;
    return objectProto as WheelEventProto;
  }

  static __unpackProto__(
    objectProto: WheelEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): WheelEvent {
    return new WheelEvent({
      delta: Vector2.fromProto(objectProto.delta!, _session, _supergraph, _graph, _connection),
      button: Number(objectProto.button) as MouseButton,
      position: Vector2.fromProto(
        objectProto.position!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      pressure: objectProto.pressure,
      shiftKey: objectProto.shiftKey,
      altKey: objectProto.altKey,
      ctrlKey: objectProto.ctrlKey,
      metaKey: objectProto.metaKey,
      accelKey: objectProto.accelKey,
      node:
        objectProto.nodePtr != undefined
          ? NodeReference.fromProto(
              objectProto.nodePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(
              objectProto.spacePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      id: String(objectProto.id),
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: WheelEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): WheelEvent {
    return WheelEvent.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): WheelEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = WheelEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.WHEEL_EVENT, WheelEvent);
/* ==== DESTACK_GENERATED_END:NODE:9514 ==== */

/* ==== DESTACK_GENERATED_START:NODE:9520 ==== */
/**
 * A KeyDownEvent is a KeyboardEvent when a key is pressed down.
 */
export class KeyDownEvent extends Node implements KeyboardEvent {
  static metatype: NodeType = NodeType.KEY_DOWN_EVENT;
  static __traits__: TraitType[] = [
    TraitType.KEYBOARD_EVENT,
    TraitType.SPATIAL,
    TraitType.INPUT_EVENT,
    TraitType.EVENT,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.SPACE];
  static __childTypes__: NodeType[] = [];
  static __ancestorTypes__: NodeType[] = [NodeType.SPACE];
  static __descendantTypes__: NodeType[] = [];

  /**
   * Spatial.parent
   */
  get parent(): Space | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference | null;

  /**
   * The Node this Event is about.
   */
  get node(): Node | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  set node(node: Node | null) {
    if (node === null) {
      this.nodePtr = null;
    } else {
      this.nodePtr = node.toRef();
    }
  }
  nodePtr: NodeReference | null;

  /**
   * KeyboardEvent.key
   */
  key: string;

  /**
   * KeyboardEvent.code
   */
  code: string;

  /**
   * KeyboardEvent.repeat
   */
  repeat: boolean;

  /**
   * KeyboardEvent.shiftKey
   */
  shiftKey: boolean;

  /**
   * KeyboardEvent.altKey
   */
  altKey: boolean;

  /**
   * KeyboardEvent.ctrlKey
   */
  ctrlKey: boolean;

  /**
   * KeyboardEvent.metaKey
   */
  metaKey: boolean;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    node?: Node | NodeReference | null;
    key: string;
    code: string;
    repeat: boolean;
    shiftKey: boolean;
    altKey: boolean;
    ctrlKey: boolean;
    metaKey: boolean;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: QueryConnection | null;
  }) {
    super(
      // id
      options.id ?? null,
      // parent
      options.parent != null
        ? options.parent.metatype == StructType.NODE_REFERENCE
          ? (options.parent as NodeReference)
          : (options.parent as Node).toRef()
        : null,
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
      // graph
      options._graph ?? null,
      // connection
      options._connection ?? null,
      // is_new
      options.id == null,
      // is_attached
      options.id != null || options._graph != null,
    );

    // properties
    let _parent = options.parent ?? null;
    if (_parent != null && _parent instanceof Node) {
      _parent = _parent.toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space instanceof Node) {
      _space = _space.toRef();
    }
    this.spacePtr = _space;
    let _node = options.node ?? null;
    if (_node != null && _node instanceof Node) {
      _node = _node.toRef();
    }
    this.nodePtr = _node;
    let _key = options.key;
    if (_key === null) {
      throw new Error(`KeyDownEvent.key is required`);
    }
    this.key = _key;
    let _code = options.code;
    if (_code === null) {
      throw new Error(`KeyDownEvent.code is required`);
    }
    this.code = _code;
    let _repeat = options.repeat;
    if (_repeat === null) {
      throw new Error(`KeyDownEvent.repeat is required`);
    }
    this.repeat = _repeat;
    let _shiftKey = options.shiftKey;
    if (_shiftKey === null) {
      throw new Error(`KeyDownEvent.shiftKey is required`);
    }
    this.shiftKey = _shiftKey;
    let _altKey = options.altKey;
    if (_altKey === null) {
      throw new Error(`KeyDownEvent.altKey is required`);
    }
    this.altKey = _altKey;
    let _ctrlKey = options.ctrlKey;
    if (_ctrlKey === null) {
      throw new Error(`KeyDownEvent.ctrlKey is required`);
    }
    this.ctrlKey = _ctrlKey;
    let _metaKey = options.metaKey;
    if (_metaKey === null) {
      throw new Error(`KeyDownEvent.metaKey is required`);
    }
    this.metaKey = _metaKey;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(
          `{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`,
        );
      }
      this.createdAt = options.createdAt;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy instanceof Node
            ? options.createdBy.toRef()
            : options.createdBy
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedByPtr =
        options.updatedBy != null
          ? options.updatedBy instanceof Node
            ? options.updatedBy.toRef()
            : options.updatedBy
          : null;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.key === other.key)) {
      return false;
    }
    if (!(this.code === other.code)) {
      return false;
    }
    if (!(this.repeat === other.repeat)) {
      return false;
    }
    if (!(this.shiftKey === other.shiftKey)) {
      return false;
    }
    if (!(this.altKey === other.altKey)) {
      return false;
    }
    if (!(this.ctrlKey === other.ctrlKey)) {
      return false;
    }
    if (!(this.metaKey === other.metaKey)) {
      return false;
    }
    if (!(this.nodePtr?.id === other.nodePtr?.id)) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashString(this.key)) & 0xffffffff;
    h = (h * 31 + hashString(this.code)) & 0xffffffff;
    h = (h * 31 + hashBool(this.repeat)) & 0xffffffff;
    h = (h * 31 + hashBool(this.shiftKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.altKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.ctrlKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.metaKey)) & 0xffffffff;
    if (this.nodePtr !== null) {
      h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    }
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.KEY_DOWN_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return "KeyDownEvent[id={this.id}]";
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Node | null = this;
    while (node !== null) {
      pathParts.push(node._pathKey);
      node = node.parent;
    }
    if (!this._isAttached) {
      pathParts.push("<detached>");
    }
    return pathParts.reverse().join("/");
  }

  repr(): string {
    return `<KeyDownEvent '${this.path}'>`;
  }

  toValue(): { [key: string]: any } {
    return KeyDownEvent.__packValue__(this);
  }

  static __packValue__(object: KeyDownEvent): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 9520;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    if (object.nodePtr != null) {
      objectValue["35"] = object.nodePtr.toValue();
    }
    objectValue["50"] = object.key;
    objectValue["51"] = object.code;
    objectValue["52"] = object.repeat;
    objectValue["80"] = object.shiftKey;
    objectValue["81"] = object.altKey;
    objectValue["82"] = object.ctrlKey;
    objectValue["83"] = object.metaKey;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): KeyDownEvent {
    const nodePtrValue = objectValue["35"];
    const unpackedNodePtr =
      nodePtrValue != undefined
        ? NodeReference.fromValue(nodePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new KeyDownEvent({
      key: objectValue["50"],
      code: objectValue["51"],
      repeat: objectValue["52"],
      shiftKey: objectValue["80"],
      altKey: objectValue["81"],
      ctrlKey: objectValue["82"],
      metaKey: objectValue["83"],
      node: unpackedNodePtr,
      parent: unpackedParentPtr,
      space: unpackedSpacePtr,
      id: String(objectValue["2"]),
      _session,
      _graph,
      _connection,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): KeyDownEvent {
    return KeyDownEvent.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): KeyDownEventProto {
    return KeyDownEvent.__packProto__(this);
  }

  static __packProto__(object: KeyDownEvent): KeyDownEventProto {
    const objectProto: Partial<KeyDownEventProto> = { metatype: 9520 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    if (object.nodePtr != null) {
      objectProto.nodePtr = object.nodePtr.toProto();
    }
    objectProto.key = object.key;
    objectProto.code = object.code;
    objectProto.repeat = object.repeat;
    objectProto.shiftKey = object.shiftKey;
    objectProto.altKey = object.altKey;
    objectProto.ctrlKey = object.ctrlKey;
    objectProto.metaKey = object.metaKey;
    return objectProto as KeyDownEventProto;
  }

  static __unpackProto__(
    objectProto: KeyDownEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): KeyDownEvent {
    return new KeyDownEvent({
      key: objectProto.key,
      code: objectProto.code,
      repeat: objectProto.repeat,
      shiftKey: objectProto.shiftKey,
      altKey: objectProto.altKey,
      ctrlKey: objectProto.ctrlKey,
      metaKey: objectProto.metaKey,
      node:
        objectProto.nodePtr != undefined
          ? NodeReference.fromProto(
              objectProto.nodePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(
              objectProto.spacePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      id: String(objectProto.id),
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: KeyDownEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): KeyDownEvent {
    return KeyDownEvent.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): KeyDownEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = KeyDownEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.KEY_DOWN_EVENT, KeyDownEvent);
/* ==== DESTACK_GENERATED_END:NODE:9520 ==== */

/* ==== DESTACK_GENERATED_START:NODE:9521 ==== */
/**
 * A KeyUpEvent is a KeyboardEvent when a key is released.
 */
export class KeyUpEvent extends Node implements KeyboardEvent {
  static metatype: NodeType = NodeType.KEY_UP_EVENT;
  static __traits__: TraitType[] = [
    TraitType.KEYBOARD_EVENT,
    TraitType.SPATIAL,
    TraitType.INPUT_EVENT,
    TraitType.EVENT,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.SPACE];
  static __childTypes__: NodeType[] = [];
  static __ancestorTypes__: NodeType[] = [NodeType.SPACE];
  static __descendantTypes__: NodeType[] = [];

  /**
   * Spatial.parent
   */
  get parent(): Space | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference | null;

  /**
   * The Node this Event is about.
   */
  get node(): Node | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  set node(node: Node | null) {
    if (node === null) {
      this.nodePtr = null;
    } else {
      this.nodePtr = node.toRef();
    }
  }
  nodePtr: NodeReference | null;

  /**
   * KeyboardEvent.key
   */
  key: string;

  /**
   * KeyboardEvent.code
   */
  code: string;

  /**
   * KeyboardEvent.repeat
   */
  repeat: boolean;

  /**
   * KeyboardEvent.shiftKey
   */
  shiftKey: boolean;

  /**
   * KeyboardEvent.altKey
   */
  altKey: boolean;

  /**
   * KeyboardEvent.ctrlKey
   */
  ctrlKey: boolean;

  /**
   * KeyboardEvent.metaKey
   */
  metaKey: boolean;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    node?: Node | NodeReference | null;
    key: string;
    code: string;
    repeat: boolean;
    shiftKey: boolean;
    altKey: boolean;
    ctrlKey: boolean;
    metaKey: boolean;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: QueryConnection | null;
  }) {
    super(
      // id
      options.id ?? null,
      // parent
      options.parent != null
        ? options.parent.metatype == StructType.NODE_REFERENCE
          ? (options.parent as NodeReference)
          : (options.parent as Node).toRef()
        : null,
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
      // graph
      options._graph ?? null,
      // connection
      options._connection ?? null,
      // is_new
      options.id == null,
      // is_attached
      options.id != null || options._graph != null,
    );

    // properties
    let _parent = options.parent ?? null;
    if (_parent != null && _parent instanceof Node) {
      _parent = _parent.toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space instanceof Node) {
      _space = _space.toRef();
    }
    this.spacePtr = _space;
    let _node = options.node ?? null;
    if (_node != null && _node instanceof Node) {
      _node = _node.toRef();
    }
    this.nodePtr = _node;
    let _key = options.key;
    if (_key === null) {
      throw new Error(`KeyUpEvent.key is required`);
    }
    this.key = _key;
    let _code = options.code;
    if (_code === null) {
      throw new Error(`KeyUpEvent.code is required`);
    }
    this.code = _code;
    let _repeat = options.repeat;
    if (_repeat === null) {
      throw new Error(`KeyUpEvent.repeat is required`);
    }
    this.repeat = _repeat;
    let _shiftKey = options.shiftKey;
    if (_shiftKey === null) {
      throw new Error(`KeyUpEvent.shiftKey is required`);
    }
    this.shiftKey = _shiftKey;
    let _altKey = options.altKey;
    if (_altKey === null) {
      throw new Error(`KeyUpEvent.altKey is required`);
    }
    this.altKey = _altKey;
    let _ctrlKey = options.ctrlKey;
    if (_ctrlKey === null) {
      throw new Error(`KeyUpEvent.ctrlKey is required`);
    }
    this.ctrlKey = _ctrlKey;
    let _metaKey = options.metaKey;
    if (_metaKey === null) {
      throw new Error(`KeyUpEvent.metaKey is required`);
    }
    this.metaKey = _metaKey;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(
          `{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`,
        );
      }
      this.createdAt = options.createdAt;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy instanceof Node
            ? options.createdBy.toRef()
            : options.createdBy
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedByPtr =
        options.updatedBy != null
          ? options.updatedBy instanceof Node
            ? options.updatedBy.toRef()
            : options.updatedBy
          : null;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.key === other.key)) {
      return false;
    }
    if (!(this.code === other.code)) {
      return false;
    }
    if (!(this.repeat === other.repeat)) {
      return false;
    }
    if (!(this.shiftKey === other.shiftKey)) {
      return false;
    }
    if (!(this.altKey === other.altKey)) {
      return false;
    }
    if (!(this.ctrlKey === other.ctrlKey)) {
      return false;
    }
    if (!(this.metaKey === other.metaKey)) {
      return false;
    }
    if (!(this.nodePtr?.id === other.nodePtr?.id)) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashString(this.key)) & 0xffffffff;
    h = (h * 31 + hashString(this.code)) & 0xffffffff;
    h = (h * 31 + hashBool(this.repeat)) & 0xffffffff;
    h = (h * 31 + hashBool(this.shiftKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.altKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.ctrlKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.metaKey)) & 0xffffffff;
    if (this.nodePtr !== null) {
      h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    }
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.KEY_UP_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return "KeyUpEvent[id={this.id}]";
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Node | null = this;
    while (node !== null) {
      pathParts.push(node._pathKey);
      node = node.parent;
    }
    if (!this._isAttached) {
      pathParts.push("<detached>");
    }
    return pathParts.reverse().join("/");
  }

  repr(): string {
    return `<KeyUpEvent '${this.path}'>`;
  }

  toValue(): { [key: string]: any } {
    return KeyUpEvent.__packValue__(this);
  }

  static __packValue__(object: KeyUpEvent): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 9521;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    if (object.nodePtr != null) {
      objectValue["35"] = object.nodePtr.toValue();
    }
    objectValue["50"] = object.key;
    objectValue["51"] = object.code;
    objectValue["52"] = object.repeat;
    objectValue["80"] = object.shiftKey;
    objectValue["81"] = object.altKey;
    objectValue["82"] = object.ctrlKey;
    objectValue["83"] = object.metaKey;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): KeyUpEvent {
    const nodePtrValue = objectValue["35"];
    const unpackedNodePtr =
      nodePtrValue != undefined
        ? NodeReference.fromValue(nodePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new KeyUpEvent({
      key: objectValue["50"],
      code: objectValue["51"],
      repeat: objectValue["52"],
      shiftKey: objectValue["80"],
      altKey: objectValue["81"],
      ctrlKey: objectValue["82"],
      metaKey: objectValue["83"],
      node: unpackedNodePtr,
      parent: unpackedParentPtr,
      space: unpackedSpacePtr,
      id: String(objectValue["2"]),
      _session,
      _graph,
      _connection,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): KeyUpEvent {
    return KeyUpEvent.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): KeyUpEventProto {
    return KeyUpEvent.__packProto__(this);
  }

  static __packProto__(object: KeyUpEvent): KeyUpEventProto {
    const objectProto: Partial<KeyUpEventProto> = { metatype: 9521 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    if (object.nodePtr != null) {
      objectProto.nodePtr = object.nodePtr.toProto();
    }
    objectProto.key = object.key;
    objectProto.code = object.code;
    objectProto.repeat = object.repeat;
    objectProto.shiftKey = object.shiftKey;
    objectProto.altKey = object.altKey;
    objectProto.ctrlKey = object.ctrlKey;
    objectProto.metaKey = object.metaKey;
    return objectProto as KeyUpEventProto;
  }

  static __unpackProto__(
    objectProto: KeyUpEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): KeyUpEvent {
    return new KeyUpEvent({
      key: objectProto.key,
      code: objectProto.code,
      repeat: objectProto.repeat,
      shiftKey: objectProto.shiftKey,
      altKey: objectProto.altKey,
      ctrlKey: objectProto.ctrlKey,
      metaKey: objectProto.metaKey,
      node:
        objectProto.nodePtr != undefined
          ? NodeReference.fromProto(
              objectProto.nodePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(
              objectProto.spacePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      id: String(objectProto.id),
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: KeyUpEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): KeyUpEvent {
    return KeyUpEvent.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): KeyUpEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = KeyUpEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.KEY_UP_EVENT, KeyUpEvent);
/* ==== DESTACK_GENERATED_END:NODE:9521 ==== */

/* ==== DESTACK_GENERATED_START:NODE:9522 ==== */
/**
 * A KeyPressEvent is a KeyboardEvent when a key is pressed.
 */
export class KeyPressEvent extends Node implements KeyboardEvent {
  static metatype: NodeType = NodeType.KEY_PRESS_EVENT;
  static __traits__: TraitType[] = [
    TraitType.KEYBOARD_EVENT,
    TraitType.SPATIAL,
    TraitType.INPUT_EVENT,
    TraitType.EVENT,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.SPACE];
  static __childTypes__: NodeType[] = [];
  static __ancestorTypes__: NodeType[] = [NodeType.SPACE];
  static __descendantTypes__: NodeType[] = [];

  /**
   * Spatial.parent
   */
  get parent(): Space | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference | null;

  /**
   * The Node this Event is about.
   */
  get node(): Node | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  set node(node: Node | null) {
    if (node === null) {
      this.nodePtr = null;
    } else {
      this.nodePtr = node.toRef();
    }
  }
  nodePtr: NodeReference | null;

  /**
   * KeyboardEvent.key
   */
  key: string;

  /**
   * KeyboardEvent.code
   */
  code: string;

  /**
   * KeyboardEvent.repeat
   */
  repeat: boolean;

  /**
   * KeyboardEvent.shiftKey
   */
  shiftKey: boolean;

  /**
   * KeyboardEvent.altKey
   */
  altKey: boolean;

  /**
   * KeyboardEvent.ctrlKey
   */
  ctrlKey: boolean;

  /**
   * KeyboardEvent.metaKey
   */
  metaKey: boolean;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    node?: Node | NodeReference | null;
    key: string;
    code: string;
    repeat: boolean;
    shiftKey: boolean;
    altKey: boolean;
    ctrlKey: boolean;
    metaKey: boolean;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: QueryConnection | null;
  }) {
    super(
      // id
      options.id ?? null,
      // parent
      options.parent != null
        ? options.parent.metatype == StructType.NODE_REFERENCE
          ? (options.parent as NodeReference)
          : (options.parent as Node).toRef()
        : null,
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
      // graph
      options._graph ?? null,
      // connection
      options._connection ?? null,
      // is_new
      options.id == null,
      // is_attached
      options.id != null || options._graph != null,
    );

    // properties
    let _parent = options.parent ?? null;
    if (_parent != null && _parent instanceof Node) {
      _parent = _parent.toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space instanceof Node) {
      _space = _space.toRef();
    }
    this.spacePtr = _space;
    let _node = options.node ?? null;
    if (_node != null && _node instanceof Node) {
      _node = _node.toRef();
    }
    this.nodePtr = _node;
    let _key = options.key;
    if (_key === null) {
      throw new Error(`KeyPressEvent.key is required`);
    }
    this.key = _key;
    let _code = options.code;
    if (_code === null) {
      throw new Error(`KeyPressEvent.code is required`);
    }
    this.code = _code;
    let _repeat = options.repeat;
    if (_repeat === null) {
      throw new Error(`KeyPressEvent.repeat is required`);
    }
    this.repeat = _repeat;
    let _shiftKey = options.shiftKey;
    if (_shiftKey === null) {
      throw new Error(`KeyPressEvent.shiftKey is required`);
    }
    this.shiftKey = _shiftKey;
    let _altKey = options.altKey;
    if (_altKey === null) {
      throw new Error(`KeyPressEvent.altKey is required`);
    }
    this.altKey = _altKey;
    let _ctrlKey = options.ctrlKey;
    if (_ctrlKey === null) {
      throw new Error(`KeyPressEvent.ctrlKey is required`);
    }
    this.ctrlKey = _ctrlKey;
    let _metaKey = options.metaKey;
    if (_metaKey === null) {
      throw new Error(`KeyPressEvent.metaKey is required`);
    }
    this.metaKey = _metaKey;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(
          `{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`,
        );
      }
      this.createdAt = options.createdAt;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy instanceof Node
            ? options.createdBy.toRef()
            : options.createdBy
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedByPtr =
        options.updatedBy != null
          ? options.updatedBy instanceof Node
            ? options.updatedBy.toRef()
            : options.updatedBy
          : null;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.key === other.key)) {
      return false;
    }
    if (!(this.code === other.code)) {
      return false;
    }
    if (!(this.repeat === other.repeat)) {
      return false;
    }
    if (!(this.shiftKey === other.shiftKey)) {
      return false;
    }
    if (!(this.altKey === other.altKey)) {
      return false;
    }
    if (!(this.ctrlKey === other.ctrlKey)) {
      return false;
    }
    if (!(this.metaKey === other.metaKey)) {
      return false;
    }
    if (!(this.nodePtr?.id === other.nodePtr?.id)) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashString(this.key)) & 0xffffffff;
    h = (h * 31 + hashString(this.code)) & 0xffffffff;
    h = (h * 31 + hashBool(this.repeat)) & 0xffffffff;
    h = (h * 31 + hashBool(this.shiftKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.altKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.ctrlKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.metaKey)) & 0xffffffff;
    if (this.nodePtr !== null) {
      h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    }
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.KEY_PRESS_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return "KeyPressEvent[id={this.id}]";
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Node | null = this;
    while (node !== null) {
      pathParts.push(node._pathKey);
      node = node.parent;
    }
    if (!this._isAttached) {
      pathParts.push("<detached>");
    }
    return pathParts.reverse().join("/");
  }

  repr(): string {
    return `<KeyPressEvent '${this.path}'>`;
  }

  toValue(): { [key: string]: any } {
    return KeyPressEvent.__packValue__(this);
  }

  static __packValue__(object: KeyPressEvent): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 9522;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    if (object.nodePtr != null) {
      objectValue["35"] = object.nodePtr.toValue();
    }
    objectValue["50"] = object.key;
    objectValue["51"] = object.code;
    objectValue["52"] = object.repeat;
    objectValue["80"] = object.shiftKey;
    objectValue["81"] = object.altKey;
    objectValue["82"] = object.ctrlKey;
    objectValue["83"] = object.metaKey;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): KeyPressEvent {
    const nodePtrValue = objectValue["35"];
    const unpackedNodePtr =
      nodePtrValue != undefined
        ? NodeReference.fromValue(nodePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new KeyPressEvent({
      key: objectValue["50"],
      code: objectValue["51"],
      repeat: objectValue["52"],
      shiftKey: objectValue["80"],
      altKey: objectValue["81"],
      ctrlKey: objectValue["82"],
      metaKey: objectValue["83"],
      node: unpackedNodePtr,
      parent: unpackedParentPtr,
      space: unpackedSpacePtr,
      id: String(objectValue["2"]),
      _session,
      _graph,
      _connection,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): KeyPressEvent {
    return KeyPressEvent.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): KeyPressEventProto {
    return KeyPressEvent.__packProto__(this);
  }

  static __packProto__(object: KeyPressEvent): KeyPressEventProto {
    const objectProto: Partial<KeyPressEventProto> = { metatype: 9522 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    if (object.nodePtr != null) {
      objectProto.nodePtr = object.nodePtr.toProto();
    }
    objectProto.key = object.key;
    objectProto.code = object.code;
    objectProto.repeat = object.repeat;
    objectProto.shiftKey = object.shiftKey;
    objectProto.altKey = object.altKey;
    objectProto.ctrlKey = object.ctrlKey;
    objectProto.metaKey = object.metaKey;
    return objectProto as KeyPressEventProto;
  }

  static __unpackProto__(
    objectProto: KeyPressEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): KeyPressEvent {
    return new KeyPressEvent({
      key: objectProto.key,
      code: objectProto.code,
      repeat: objectProto.repeat,
      shiftKey: objectProto.shiftKey,
      altKey: objectProto.altKey,
      ctrlKey: objectProto.ctrlKey,
      metaKey: objectProto.metaKey,
      node:
        objectProto.nodePtr != undefined
          ? NodeReference.fromProto(
              objectProto.nodePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(
              objectProto.spacePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      id: String(objectProto.id),
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: KeyPressEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): KeyPressEvent {
    return KeyPressEvent.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): KeyPressEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = KeyPressEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.KEY_PRESS_EVENT, KeyPressEvent);
/* ==== DESTACK_GENERATED_END:NODE:9522 ==== */

/* ==== DESTACK_GENERATED_START:NODE:9530 ==== */
/**
 * A DragStartEvent is a DragEvent when a drag starts.
 */
export class DragStartEvent extends Node implements DragEvent {
  static metatype: NodeType = NodeType.DRAG_START_EVENT;
  static __traits__: TraitType[] = [
    TraitType.DRAG_EVENT,
    TraitType.SPATIAL,
    TraitType.INPUT_EVENT,
    TraitType.EVENT,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.SPACE];
  static __childTypes__: NodeType[] = [];
  static __ancestorTypes__: NodeType[] = [NodeType.SPACE];
  static __descendantTypes__: NodeType[] = [];

  /**
   * Spatial.parent
   */
  get parent(): Space | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference | null;

  /**
   * The Node this Event is about.
   */
  get node(): Node | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  set node(node: Node | null) {
    if (node === null) {
      this.nodePtr = null;
    } else {
      this.nodePtr = node.toRef();
    }
  }
  nodePtr: NodeReference | null;

  /**
   * DragEvent.position
   */
  position: Vector2;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    node?: Node | NodeReference | null;
    position: Vector2;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: QueryConnection | null;
  }) {
    super(
      // id
      options.id ?? null,
      // parent
      options.parent != null
        ? options.parent.metatype == StructType.NODE_REFERENCE
          ? (options.parent as NodeReference)
          : (options.parent as Node).toRef()
        : null,
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
      // graph
      options._graph ?? null,
      // connection
      options._connection ?? null,
      // is_new
      options.id == null,
      // is_attached
      options.id != null || options._graph != null,
    );

    // properties
    let _parent = options.parent ?? null;
    if (_parent != null && _parent instanceof Node) {
      _parent = _parent.toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space instanceof Node) {
      _space = _space.toRef();
    }
    this.spacePtr = _space;
    let _node = options.node ?? null;
    if (_node != null && _node instanceof Node) {
      _node = _node.toRef();
    }
    this.nodePtr = _node;
    let _position = options.position;
    if (_position === null) {
      throw new Error(`DragStartEvent.position is required`);
    }
    this.position = _position;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(
          `{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`,
        );
      }
      this.createdAt = options.createdAt;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy instanceof Node
            ? options.createdBy.toRef()
            : options.createdBy
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedByPtr =
        options.updatedBy != null
          ? options.updatedBy instanceof Node
            ? options.updatedBy.toRef()
            : options.updatedBy
          : null;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!this.position.equals(other.position)) {
      return false;
    }
    if (!(this.nodePtr?.id === other.nodePtr?.id)) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.position.hash()) & 0xffffffff;
    if (this.nodePtr !== null) {
      h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    }
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.DRAG_START_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return "DragStartEvent[id={this.id}]";
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Node | null = this;
    while (node !== null) {
      pathParts.push(node._pathKey);
      node = node.parent;
    }
    if (!this._isAttached) {
      pathParts.push("<detached>");
    }
    return pathParts.reverse().join("/");
  }

  repr(): string {
    return `<DragStartEvent '${this.path}'>`;
  }

  toValue(): { [key: string]: any } {
    return DragStartEvent.__packValue__(this);
  }

  static __packValue__(object: DragStartEvent): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 9530;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    if (object.nodePtr != null) {
      objectValue["35"] = object.nodePtr.toValue();
    }
    objectValue["50"] = object.position.toValue();
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): DragStartEvent {
    const nodePtrValue = objectValue["35"];
    const unpackedNodePtr =
      nodePtrValue != undefined
        ? NodeReference.fromValue(nodePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new DragStartEvent({
      position: Vector2.fromValue(objectValue["50"], _session, _supergraph, _graph, _connection),
      node: unpackedNodePtr,
      parent: unpackedParentPtr,
      space: unpackedSpacePtr,
      id: String(objectValue["2"]),
      _session,
      _graph,
      _connection,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): DragStartEvent {
    return DragStartEvent.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): DragStartEventProto {
    return DragStartEvent.__packProto__(this);
  }

  static __packProto__(object: DragStartEvent): DragStartEventProto {
    const objectProto: Partial<DragStartEventProto> = { metatype: 9530 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    if (object.nodePtr != null) {
      objectProto.nodePtr = object.nodePtr.toProto();
    }
    objectProto.position = object.position.toProto();
    return objectProto as DragStartEventProto;
  }

  static __unpackProto__(
    objectProto: DragStartEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): DragStartEvent {
    return new DragStartEvent({
      position: Vector2.fromProto(
        objectProto.position!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      node:
        objectProto.nodePtr != undefined
          ? NodeReference.fromProto(
              objectProto.nodePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(
              objectProto.spacePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      id: String(objectProto.id),
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: DragStartEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): DragStartEvent {
    return DragStartEvent.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): DragStartEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = DragStartEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.DRAG_START_EVENT, DragStartEvent);
/* ==== DESTACK_GENERATED_END:NODE:9530 ==== */

/* ==== DESTACK_GENERATED_START:NODE:9531 ==== */
/**
 * A DragEndEvent is a DragEvent when a drag ends.
 */
export class DragEndEvent extends Node implements DragEvent {
  static metatype: NodeType = NodeType.DRAG_END_EVENT;
  static __traits__: TraitType[] = [
    TraitType.DRAG_EVENT,
    TraitType.SPATIAL,
    TraitType.INPUT_EVENT,
    TraitType.EVENT,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.SPACE];
  static __childTypes__: NodeType[] = [];
  static __ancestorTypes__: NodeType[] = [NodeType.SPACE];
  static __descendantTypes__: NodeType[] = [];

  /**
   * Spatial.parent
   */
  get parent(): Space | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference | null;

  /**
   * The Node this Event is about.
   */
  get node(): Node | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  set node(node: Node | null) {
    if (node === null) {
      this.nodePtr = null;
    } else {
      this.nodePtr = node.toRef();
    }
  }
  nodePtr: NodeReference | null;

  /**
   * DragEvent.position
   */
  position: Vector2;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    node?: Node | NodeReference | null;
    position: Vector2;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: QueryConnection | null;
  }) {
    super(
      // id
      options.id ?? null,
      // parent
      options.parent != null
        ? options.parent.metatype == StructType.NODE_REFERENCE
          ? (options.parent as NodeReference)
          : (options.parent as Node).toRef()
        : null,
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
      // graph
      options._graph ?? null,
      // connection
      options._connection ?? null,
      // is_new
      options.id == null,
      // is_attached
      options.id != null || options._graph != null,
    );

    // properties
    let _parent = options.parent ?? null;
    if (_parent != null && _parent instanceof Node) {
      _parent = _parent.toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space instanceof Node) {
      _space = _space.toRef();
    }
    this.spacePtr = _space;
    let _node = options.node ?? null;
    if (_node != null && _node instanceof Node) {
      _node = _node.toRef();
    }
    this.nodePtr = _node;
    let _position = options.position;
    if (_position === null) {
      throw new Error(`DragEndEvent.position is required`);
    }
    this.position = _position;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(
          `{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`,
        );
      }
      this.createdAt = options.createdAt;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy instanceof Node
            ? options.createdBy.toRef()
            : options.createdBy
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedByPtr =
        options.updatedBy != null
          ? options.updatedBy instanceof Node
            ? options.updatedBy.toRef()
            : options.updatedBy
          : null;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!this.position.equals(other.position)) {
      return false;
    }
    if (!(this.nodePtr?.id === other.nodePtr?.id)) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.position.hash()) & 0xffffffff;
    if (this.nodePtr !== null) {
      h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    }
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.DRAG_END_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return "DragEndEvent[id={this.id}]";
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Node | null = this;
    while (node !== null) {
      pathParts.push(node._pathKey);
      node = node.parent;
    }
    if (!this._isAttached) {
      pathParts.push("<detached>");
    }
    return pathParts.reverse().join("/");
  }

  repr(): string {
    return `<DragEndEvent '${this.path}'>`;
  }

  toValue(): { [key: string]: any } {
    return DragEndEvent.__packValue__(this);
  }

  static __packValue__(object: DragEndEvent): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 9531;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    if (object.nodePtr != null) {
      objectValue["35"] = object.nodePtr.toValue();
    }
    objectValue["50"] = object.position.toValue();
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): DragEndEvent {
    const nodePtrValue = objectValue["35"];
    const unpackedNodePtr =
      nodePtrValue != undefined
        ? NodeReference.fromValue(nodePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new DragEndEvent({
      position: Vector2.fromValue(objectValue["50"], _session, _supergraph, _graph, _connection),
      node: unpackedNodePtr,
      parent: unpackedParentPtr,
      space: unpackedSpacePtr,
      id: String(objectValue["2"]),
      _session,
      _graph,
      _connection,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): DragEndEvent {
    return DragEndEvent.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): DragEndEventProto {
    return DragEndEvent.__packProto__(this);
  }

  static __packProto__(object: DragEndEvent): DragEndEventProto {
    const objectProto: Partial<DragEndEventProto> = { metatype: 9531 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    if (object.nodePtr != null) {
      objectProto.nodePtr = object.nodePtr.toProto();
    }
    objectProto.position = object.position.toProto();
    return objectProto as DragEndEventProto;
  }

  static __unpackProto__(
    objectProto: DragEndEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): DragEndEvent {
    return new DragEndEvent({
      position: Vector2.fromProto(
        objectProto.position!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      node:
        objectProto.nodePtr != undefined
          ? NodeReference.fromProto(
              objectProto.nodePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(
              objectProto.spacePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      id: String(objectProto.id),
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: DragEndEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): DragEndEvent {
    return DragEndEvent.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): DragEndEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = DragEndEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.DRAG_END_EVENT, DragEndEvent);
/* ==== DESTACK_GENERATED_END:NODE:9531 ==== */

/* ==== DESTACK_GENERATED_START:NODE:9532 ==== */
/**
 * A DragOverEvent is a DragEvent when a drag is over an element.
 */
export class DragOverEvent extends Node implements DragEvent {
  static metatype: NodeType = NodeType.DRAG_OVER_EVENT;
  static __traits__: TraitType[] = [
    TraitType.DRAG_EVENT,
    TraitType.SPATIAL,
    TraitType.INPUT_EVENT,
    TraitType.EVENT,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.SPACE];
  static __childTypes__: NodeType[] = [];
  static __ancestorTypes__: NodeType[] = [NodeType.SPACE];
  static __descendantTypes__: NodeType[] = [];

  /**
   * Spatial.parent
   */
  get parent(): Space | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference | null;

  /**
   * The Node this Event is about.
   */
  get node(): Node | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  set node(node: Node | null) {
    if (node === null) {
      this.nodePtr = null;
    } else {
      this.nodePtr = node.toRef();
    }
  }
  nodePtr: NodeReference | null;

  /**
   * DragEvent.position
   */
  position: Vector2;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    node?: Node | NodeReference | null;
    position: Vector2;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: QueryConnection | null;
  }) {
    super(
      // id
      options.id ?? null,
      // parent
      options.parent != null
        ? options.parent.metatype == StructType.NODE_REFERENCE
          ? (options.parent as NodeReference)
          : (options.parent as Node).toRef()
        : null,
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
      // graph
      options._graph ?? null,
      // connection
      options._connection ?? null,
      // is_new
      options.id == null,
      // is_attached
      options.id != null || options._graph != null,
    );

    // properties
    let _parent = options.parent ?? null;
    if (_parent != null && _parent instanceof Node) {
      _parent = _parent.toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space instanceof Node) {
      _space = _space.toRef();
    }
    this.spacePtr = _space;
    let _node = options.node ?? null;
    if (_node != null && _node instanceof Node) {
      _node = _node.toRef();
    }
    this.nodePtr = _node;
    let _position = options.position;
    if (_position === null) {
      throw new Error(`DragOverEvent.position is required`);
    }
    this.position = _position;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(
          `{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`,
        );
      }
      this.createdAt = options.createdAt;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy instanceof Node
            ? options.createdBy.toRef()
            : options.createdBy
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedByPtr =
        options.updatedBy != null
          ? options.updatedBy instanceof Node
            ? options.updatedBy.toRef()
            : options.updatedBy
          : null;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!this.position.equals(other.position)) {
      return false;
    }
    if (!(this.nodePtr?.id === other.nodePtr?.id)) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.position.hash()) & 0xffffffff;
    if (this.nodePtr !== null) {
      h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    }
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.DRAG_OVER_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return "DragOverEvent[id={this.id}]";
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Node | null = this;
    while (node !== null) {
      pathParts.push(node._pathKey);
      node = node.parent;
    }
    if (!this._isAttached) {
      pathParts.push("<detached>");
    }
    return pathParts.reverse().join("/");
  }

  repr(): string {
    return `<DragOverEvent '${this.path}'>`;
  }

  toValue(): { [key: string]: any } {
    return DragOverEvent.__packValue__(this);
  }

  static __packValue__(object: DragOverEvent): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 9532;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    if (object.nodePtr != null) {
      objectValue["35"] = object.nodePtr.toValue();
    }
    objectValue["50"] = object.position.toValue();
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): DragOverEvent {
    const nodePtrValue = objectValue["35"];
    const unpackedNodePtr =
      nodePtrValue != undefined
        ? NodeReference.fromValue(nodePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new DragOverEvent({
      position: Vector2.fromValue(objectValue["50"], _session, _supergraph, _graph, _connection),
      node: unpackedNodePtr,
      parent: unpackedParentPtr,
      space: unpackedSpacePtr,
      id: String(objectValue["2"]),
      _session,
      _graph,
      _connection,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): DragOverEvent {
    return DragOverEvent.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): DragOverEventProto {
    return DragOverEvent.__packProto__(this);
  }

  static __packProto__(object: DragOverEvent): DragOverEventProto {
    const objectProto: Partial<DragOverEventProto> = { metatype: 9532 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    if (object.nodePtr != null) {
      objectProto.nodePtr = object.nodePtr.toProto();
    }
    objectProto.position = object.position.toProto();
    return objectProto as DragOverEventProto;
  }

  static __unpackProto__(
    objectProto: DragOverEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): DragOverEvent {
    return new DragOverEvent({
      position: Vector2.fromProto(
        objectProto.position!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      node:
        objectProto.nodePtr != undefined
          ? NodeReference.fromProto(
              objectProto.nodePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(
              objectProto.spacePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      id: String(objectProto.id),
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: DragOverEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): DragOverEvent {
    return DragOverEvent.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): DragOverEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = DragOverEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.DRAG_OVER_EVENT, DragOverEvent);
/* ==== DESTACK_GENERATED_END:NODE:9532 ==== */

/* ==== DESTACK_GENERATED_START:NODE:9533 ==== */
/**
 * A DragEnterEvent is a DragEvent when a drag enters an element.
 */
export class DragEnterEvent extends Node implements DragEvent {
  static metatype: NodeType = NodeType.DRAG_ENTER_EVENT;
  static __traits__: TraitType[] = [
    TraitType.DRAG_EVENT,
    TraitType.SPATIAL,
    TraitType.INPUT_EVENT,
    TraitType.EVENT,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.SPACE];
  static __childTypes__: NodeType[] = [];
  static __ancestorTypes__: NodeType[] = [NodeType.SPACE];
  static __descendantTypes__: NodeType[] = [];

  /**
   * Spatial.parent
   */
  get parent(): Space | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference | null;

  /**
   * The Node this Event is about.
   */
  get node(): Node | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  set node(node: Node | null) {
    if (node === null) {
      this.nodePtr = null;
    } else {
      this.nodePtr = node.toRef();
    }
  }
  nodePtr: NodeReference | null;

  /**
   * DragEvent.position
   */
  position: Vector2;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    node?: Node | NodeReference | null;
    position: Vector2;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: QueryConnection | null;
  }) {
    super(
      // id
      options.id ?? null,
      // parent
      options.parent != null
        ? options.parent.metatype == StructType.NODE_REFERENCE
          ? (options.parent as NodeReference)
          : (options.parent as Node).toRef()
        : null,
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
      // graph
      options._graph ?? null,
      // connection
      options._connection ?? null,
      // is_new
      options.id == null,
      // is_attached
      options.id != null || options._graph != null,
    );

    // properties
    let _parent = options.parent ?? null;
    if (_parent != null && _parent instanceof Node) {
      _parent = _parent.toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space instanceof Node) {
      _space = _space.toRef();
    }
    this.spacePtr = _space;
    let _node = options.node ?? null;
    if (_node != null && _node instanceof Node) {
      _node = _node.toRef();
    }
    this.nodePtr = _node;
    let _position = options.position;
    if (_position === null) {
      throw new Error(`DragEnterEvent.position is required`);
    }
    this.position = _position;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(
          `{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`,
        );
      }
      this.createdAt = options.createdAt;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy instanceof Node
            ? options.createdBy.toRef()
            : options.createdBy
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedByPtr =
        options.updatedBy != null
          ? options.updatedBy instanceof Node
            ? options.updatedBy.toRef()
            : options.updatedBy
          : null;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!this.position.equals(other.position)) {
      return false;
    }
    if (!(this.nodePtr?.id === other.nodePtr?.id)) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.position.hash()) & 0xffffffff;
    if (this.nodePtr !== null) {
      h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    }
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.DRAG_ENTER_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return "DragEnterEvent[id={this.id}]";
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Node | null = this;
    while (node !== null) {
      pathParts.push(node._pathKey);
      node = node.parent;
    }
    if (!this._isAttached) {
      pathParts.push("<detached>");
    }
    return pathParts.reverse().join("/");
  }

  repr(): string {
    return `<DragEnterEvent '${this.path}'>`;
  }

  toValue(): { [key: string]: any } {
    return DragEnterEvent.__packValue__(this);
  }

  static __packValue__(object: DragEnterEvent): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 9533;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    if (object.nodePtr != null) {
      objectValue["35"] = object.nodePtr.toValue();
    }
    objectValue["50"] = object.position.toValue();
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): DragEnterEvent {
    const nodePtrValue = objectValue["35"];
    const unpackedNodePtr =
      nodePtrValue != undefined
        ? NodeReference.fromValue(nodePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new DragEnterEvent({
      position: Vector2.fromValue(objectValue["50"], _session, _supergraph, _graph, _connection),
      node: unpackedNodePtr,
      parent: unpackedParentPtr,
      space: unpackedSpacePtr,
      id: String(objectValue["2"]),
      _session,
      _graph,
      _connection,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): DragEnterEvent {
    return DragEnterEvent.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): DragEnterEventProto {
    return DragEnterEvent.__packProto__(this);
  }

  static __packProto__(object: DragEnterEvent): DragEnterEventProto {
    const objectProto: Partial<DragEnterEventProto> = { metatype: 9533 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    if (object.nodePtr != null) {
      objectProto.nodePtr = object.nodePtr.toProto();
    }
    objectProto.position = object.position.toProto();
    return objectProto as DragEnterEventProto;
  }

  static __unpackProto__(
    objectProto: DragEnterEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): DragEnterEvent {
    return new DragEnterEvent({
      position: Vector2.fromProto(
        objectProto.position!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      node:
        objectProto.nodePtr != undefined
          ? NodeReference.fromProto(
              objectProto.nodePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(
              objectProto.spacePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      id: String(objectProto.id),
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: DragEnterEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): DragEnterEvent {
    return DragEnterEvent.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): DragEnterEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = DragEnterEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.DRAG_ENTER_EVENT, DragEnterEvent);
/* ==== DESTACK_GENERATED_END:NODE:9533 ==== */

/* ==== DESTACK_GENERATED_START:NODE:9534 ==== */
/**
 * A DragLeaveEvent is a DragEvent when a drag leaves an element.
 */
export class DragLeaveEvent extends Node implements DragEvent {
  static metatype: NodeType = NodeType.DRAG_LEAVE_EVENT;
  static __traits__: TraitType[] = [
    TraitType.DRAG_EVENT,
    TraitType.SPATIAL,
    TraitType.INPUT_EVENT,
    TraitType.EVENT,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.SPACE];
  static __childTypes__: NodeType[] = [];
  static __ancestorTypes__: NodeType[] = [NodeType.SPACE];
  static __descendantTypes__: NodeType[] = [];

  /**
   * Spatial.parent
   */
  get parent(): Space | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference | null;

  /**
   * The Node this Event is about.
   */
  get node(): Node | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  set node(node: Node | null) {
    if (node === null) {
      this.nodePtr = null;
    } else {
      this.nodePtr = node.toRef();
    }
  }
  nodePtr: NodeReference | null;

  /**
   * DragEvent.position
   */
  position: Vector2;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    node?: Node | NodeReference | null;
    position: Vector2;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: QueryConnection | null;
  }) {
    super(
      // id
      options.id ?? null,
      // parent
      options.parent != null
        ? options.parent.metatype == StructType.NODE_REFERENCE
          ? (options.parent as NodeReference)
          : (options.parent as Node).toRef()
        : null,
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
      // graph
      options._graph ?? null,
      // connection
      options._connection ?? null,
      // is_new
      options.id == null,
      // is_attached
      options.id != null || options._graph != null,
    );

    // properties
    let _parent = options.parent ?? null;
    if (_parent != null && _parent instanceof Node) {
      _parent = _parent.toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space instanceof Node) {
      _space = _space.toRef();
    }
    this.spacePtr = _space;
    let _node = options.node ?? null;
    if (_node != null && _node instanceof Node) {
      _node = _node.toRef();
    }
    this.nodePtr = _node;
    let _position = options.position;
    if (_position === null) {
      throw new Error(`DragLeaveEvent.position is required`);
    }
    this.position = _position;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(
          `{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`,
        );
      }
      this.createdAt = options.createdAt;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy instanceof Node
            ? options.createdBy.toRef()
            : options.createdBy
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedByPtr =
        options.updatedBy != null
          ? options.updatedBy instanceof Node
            ? options.updatedBy.toRef()
            : options.updatedBy
          : null;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!this.position.equals(other.position)) {
      return false;
    }
    if (!(this.nodePtr?.id === other.nodePtr?.id)) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.position.hash()) & 0xffffffff;
    if (this.nodePtr !== null) {
      h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    }
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.DRAG_LEAVE_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return "DragLeaveEvent[id={this.id}]";
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Node | null = this;
    while (node !== null) {
      pathParts.push(node._pathKey);
      node = node.parent;
    }
    if (!this._isAttached) {
      pathParts.push("<detached>");
    }
    return pathParts.reverse().join("/");
  }

  repr(): string {
    return `<DragLeaveEvent '${this.path}'>`;
  }

  toValue(): { [key: string]: any } {
    return DragLeaveEvent.__packValue__(this);
  }

  static __packValue__(object: DragLeaveEvent): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 9534;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    if (object.nodePtr != null) {
      objectValue["35"] = object.nodePtr.toValue();
    }
    objectValue["50"] = object.position.toValue();
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): DragLeaveEvent {
    const nodePtrValue = objectValue["35"];
    const unpackedNodePtr =
      nodePtrValue != undefined
        ? NodeReference.fromValue(nodePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new DragLeaveEvent({
      position: Vector2.fromValue(objectValue["50"], _session, _supergraph, _graph, _connection),
      node: unpackedNodePtr,
      parent: unpackedParentPtr,
      space: unpackedSpacePtr,
      id: String(objectValue["2"]),
      _session,
      _graph,
      _connection,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): DragLeaveEvent {
    return DragLeaveEvent.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): DragLeaveEventProto {
    return DragLeaveEvent.__packProto__(this);
  }

  static __packProto__(object: DragLeaveEvent): DragLeaveEventProto {
    const objectProto: Partial<DragLeaveEventProto> = { metatype: 9534 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    if (object.nodePtr != null) {
      objectProto.nodePtr = object.nodePtr.toProto();
    }
    objectProto.position = object.position.toProto();
    return objectProto as DragLeaveEventProto;
  }

  static __unpackProto__(
    objectProto: DragLeaveEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): DragLeaveEvent {
    return new DragLeaveEvent({
      position: Vector2.fromProto(
        objectProto.position!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      node:
        objectProto.nodePtr != undefined
          ? NodeReference.fromProto(
              objectProto.nodePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(
              objectProto.spacePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      id: String(objectProto.id),
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: DragLeaveEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): DragLeaveEvent {
    return DragLeaveEvent.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): DragLeaveEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = DragLeaveEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.DRAG_LEAVE_EVENT, DragLeaveEvent);
/* ==== DESTACK_GENERATED_END:NODE:9534 ==== */

/* ==== DESTACK_GENERATED_START:NODE:9535 ==== */
/**
 * A DropEvent is a DragEvent when a drag is dropped on an element.
 */
export class DropEvent extends Node implements DragEvent {
  static metatype: NodeType = NodeType.DROP_EVENT;
  static __traits__: TraitType[] = [
    TraitType.DRAG_EVENT,
    TraitType.SPATIAL,
    TraitType.INPUT_EVENT,
    TraitType.EVENT,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.SPACE];
  static __childTypes__: NodeType[] = [];
  static __ancestorTypes__: NodeType[] = [NodeType.SPACE];
  static __descendantTypes__: NodeType[] = [];

  /**
   * Spatial.parent
   */
  get parent(): Space | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference | null;

  /**
   * The Node this Event is about.
   */
  get node(): Node | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  set node(node: Node | null) {
    if (node === null) {
      this.nodePtr = null;
    } else {
      this.nodePtr = node.toRef();
    }
  }
  nodePtr: NodeReference | null;

  /**
   * DragEvent.position
   */
  position: Vector2;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    node?: Node | NodeReference | null;
    position: Vector2;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: QueryConnection | null;
  }) {
    super(
      // id
      options.id ?? null,
      // parent
      options.parent != null
        ? options.parent.metatype == StructType.NODE_REFERENCE
          ? (options.parent as NodeReference)
          : (options.parent as Node).toRef()
        : null,
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
      // graph
      options._graph ?? null,
      // connection
      options._connection ?? null,
      // is_new
      options.id == null,
      // is_attached
      options.id != null || options._graph != null,
    );

    // properties
    let _parent = options.parent ?? null;
    if (_parent != null && _parent instanceof Node) {
      _parent = _parent.toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space instanceof Node) {
      _space = _space.toRef();
    }
    this.spacePtr = _space;
    let _node = options.node ?? null;
    if (_node != null && _node instanceof Node) {
      _node = _node.toRef();
    }
    this.nodePtr = _node;
    let _position = options.position;
    if (_position === null) {
      throw new Error(`DropEvent.position is required`);
    }
    this.position = _position;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(
          `{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`,
        );
      }
      this.createdAt = options.createdAt;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy instanceof Node
            ? options.createdBy.toRef()
            : options.createdBy
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedByPtr =
        options.updatedBy != null
          ? options.updatedBy instanceof Node
            ? options.updatedBy.toRef()
            : options.updatedBy
          : null;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!this.position.equals(other.position)) {
      return false;
    }
    if (!(this.nodePtr?.id === other.nodePtr?.id)) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.position.hash()) & 0xffffffff;
    if (this.nodePtr !== null) {
      h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    }
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.DROP_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return "DropEvent[id={this.id}]";
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Node | null = this;
    while (node !== null) {
      pathParts.push(node._pathKey);
      node = node.parent;
    }
    if (!this._isAttached) {
      pathParts.push("<detached>");
    }
    return pathParts.reverse().join("/");
  }

  repr(): string {
    return `<DropEvent '${this.path}'>`;
  }

  toValue(): { [key: string]: any } {
    return DropEvent.__packValue__(this);
  }

  static __packValue__(object: DropEvent): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 9535;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    if (object.nodePtr != null) {
      objectValue["35"] = object.nodePtr.toValue();
    }
    objectValue["50"] = object.position.toValue();
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): DropEvent {
    const nodePtrValue = objectValue["35"];
    const unpackedNodePtr =
      nodePtrValue != undefined
        ? NodeReference.fromValue(nodePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new DropEvent({
      position: Vector2.fromValue(objectValue["50"], _session, _supergraph, _graph, _connection),
      node: unpackedNodePtr,
      parent: unpackedParentPtr,
      space: unpackedSpacePtr,
      id: String(objectValue["2"]),
      _session,
      _graph,
      _connection,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): DropEvent {
    return DropEvent.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): DropEventProto {
    return DropEvent.__packProto__(this);
  }

  static __packProto__(object: DropEvent): DropEventProto {
    const objectProto: Partial<DropEventProto> = { metatype: 9535 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    if (object.nodePtr != null) {
      objectProto.nodePtr = object.nodePtr.toProto();
    }
    objectProto.position = object.position.toProto();
    return objectProto as DropEventProto;
  }

  static __unpackProto__(
    objectProto: DropEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): DropEvent {
    return new DropEvent({
      position: Vector2.fromProto(
        objectProto.position!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      node:
        objectProto.nodePtr != undefined
          ? NodeReference.fromProto(
              objectProto.nodePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(
              objectProto.spacePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      id: String(objectProto.id),
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: DropEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): DropEvent {
    return DropEvent.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): DropEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = DropEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.DROP_EVENT, DropEvent);
/* ==== DESTACK_GENERATED_END:NODE:9535 ==== */

/* ==== DESTACK_GENERATED_START:NODE:9540 ==== */
/**
 * A CopyEvent is a ClipboardEvent when a copy is performed.
 */
export class CopyEvent extends Node implements ClipboardEvent {
  static metatype: NodeType = NodeType.COPY_EVENT;
  static __traits__: TraitType[] = [
    TraitType.EVENT,
    TraitType.SPATIAL,
    TraitType.CLIPBOARD_EVENT,
    TraitType.INPUT_EVENT,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.SPACE];
  static __childTypes__: NodeType[] = [];
  static __ancestorTypes__: NodeType[] = [NodeType.SPACE];
  static __descendantTypes__: NodeType[] = [];

  /**
   * Spatial.parent
   */
  get parent(): Space | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference | null;

  /**
   * The Node this Event is about.
   */
  get node(): Node | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  set node(node: Node | null) {
    if (node === null) {
      this.nodePtr = null;
    } else {
      this.nodePtr = node.toRef();
    }
  }
  nodePtr: NodeReference | null;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    node?: Node | NodeReference | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: QueryConnection | null;
  }) {
    super(
      // id
      options.id ?? null,
      // parent
      options.parent != null
        ? options.parent.metatype == StructType.NODE_REFERENCE
          ? (options.parent as NodeReference)
          : (options.parent as Node).toRef()
        : null,
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
      // graph
      options._graph ?? null,
      // connection
      options._connection ?? null,
      // is_new
      options.id == null,
      // is_attached
      options.id != null || options._graph != null,
    );

    // properties
    let _parent = options.parent ?? null;
    if (_parent != null && _parent instanceof Node) {
      _parent = _parent.toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space instanceof Node) {
      _space = _space.toRef();
    }
    this.spacePtr = _space;
    let _node = options.node ?? null;
    if (_node != null && _node instanceof Node) {
      _node = _node.toRef();
    }
    this.nodePtr = _node;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(
          `{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`,
        );
      }
      this.createdAt = options.createdAt;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy instanceof Node
            ? options.createdBy.toRef()
            : options.createdBy
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedByPtr =
        options.updatedBy != null
          ? options.updatedBy instanceof Node
            ? options.updatedBy.toRef()
            : options.updatedBy
          : null;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.nodePtr?.id === other.nodePtr?.id)) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    if (this.nodePtr !== null) {
      h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    }
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.COPY_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return "CopyEvent[id={this.id}]";
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Node | null = this;
    while (node !== null) {
      pathParts.push(node._pathKey);
      node = node.parent;
    }
    if (!this._isAttached) {
      pathParts.push("<detached>");
    }
    return pathParts.reverse().join("/");
  }

  repr(): string {
    return `<CopyEvent '${this.path}'>`;
  }

  toValue(): { [key: string]: any } {
    return CopyEvent.__packValue__(this);
  }

  static __packValue__(object: CopyEvent): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 9540;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    if (object.nodePtr != null) {
      objectValue["35"] = object.nodePtr.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): CopyEvent {
    const nodePtrValue = objectValue["35"];
    const unpackedNodePtr =
      nodePtrValue != undefined
        ? NodeReference.fromValue(nodePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new CopyEvent({
      node: unpackedNodePtr,
      parent: unpackedParentPtr,
      space: unpackedSpacePtr,
      id: String(objectValue["2"]),
      _session,
      _graph,
      _connection,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): CopyEvent {
    return CopyEvent.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): CopyEventProto {
    return CopyEvent.__packProto__(this);
  }

  static __packProto__(object: CopyEvent): CopyEventProto {
    const objectProto: Partial<CopyEventProto> = { metatype: 9540 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    if (object.nodePtr != null) {
      objectProto.nodePtr = object.nodePtr.toProto();
    }
    return objectProto as CopyEventProto;
  }

  static __unpackProto__(
    objectProto: CopyEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): CopyEvent {
    return new CopyEvent({
      node:
        objectProto.nodePtr != undefined
          ? NodeReference.fromProto(
              objectProto.nodePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(
              objectProto.spacePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      id: String(objectProto.id),
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: CopyEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): CopyEvent {
    return CopyEvent.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): CopyEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = CopyEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.COPY_EVENT, CopyEvent);
/* ==== DESTACK_GENERATED_END:NODE:9540 ==== */

/* ==== DESTACK_GENERATED_START:NODE:9541 ==== */
/**
 * A CutEvent is a ClipboardEvent when a cut is performed.
 */
export class CutEvent extends Node implements ClipboardEvent {
  static metatype: NodeType = NodeType.CUT_EVENT;
  static __traits__: TraitType[] = [
    TraitType.EVENT,
    TraitType.SPATIAL,
    TraitType.CLIPBOARD_EVENT,
    TraitType.INPUT_EVENT,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.SPACE];
  static __childTypes__: NodeType[] = [];
  static __ancestorTypes__: NodeType[] = [NodeType.SPACE];
  static __descendantTypes__: NodeType[] = [];

  /**
   * Spatial.parent
   */
  get parent(): Space | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference | null;

  /**
   * The Node this Event is about.
   */
  get node(): Node | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  set node(node: Node | null) {
    if (node === null) {
      this.nodePtr = null;
    } else {
      this.nodePtr = node.toRef();
    }
  }
  nodePtr: NodeReference | null;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    node?: Node | NodeReference | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: QueryConnection | null;
  }) {
    super(
      // id
      options.id ?? null,
      // parent
      options.parent != null
        ? options.parent.metatype == StructType.NODE_REFERENCE
          ? (options.parent as NodeReference)
          : (options.parent as Node).toRef()
        : null,
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
      // graph
      options._graph ?? null,
      // connection
      options._connection ?? null,
      // is_new
      options.id == null,
      // is_attached
      options.id != null || options._graph != null,
    );

    // properties
    let _parent = options.parent ?? null;
    if (_parent != null && _parent instanceof Node) {
      _parent = _parent.toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space instanceof Node) {
      _space = _space.toRef();
    }
    this.spacePtr = _space;
    let _node = options.node ?? null;
    if (_node != null && _node instanceof Node) {
      _node = _node.toRef();
    }
    this.nodePtr = _node;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(
          `{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`,
        );
      }
      this.createdAt = options.createdAt;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy instanceof Node
            ? options.createdBy.toRef()
            : options.createdBy
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedByPtr =
        options.updatedBy != null
          ? options.updatedBy instanceof Node
            ? options.updatedBy.toRef()
            : options.updatedBy
          : null;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.nodePtr?.id === other.nodePtr?.id)) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    if (this.nodePtr !== null) {
      h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    }
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.CUT_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return "CutEvent[id={this.id}]";
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Node | null = this;
    while (node !== null) {
      pathParts.push(node._pathKey);
      node = node.parent;
    }
    if (!this._isAttached) {
      pathParts.push("<detached>");
    }
    return pathParts.reverse().join("/");
  }

  repr(): string {
    return `<CutEvent '${this.path}'>`;
  }

  toValue(): { [key: string]: any } {
    return CutEvent.__packValue__(this);
  }

  static __packValue__(object: CutEvent): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 9541;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    if (object.nodePtr != null) {
      objectValue["35"] = object.nodePtr.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): CutEvent {
    const nodePtrValue = objectValue["35"];
    const unpackedNodePtr =
      nodePtrValue != undefined
        ? NodeReference.fromValue(nodePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new CutEvent({
      node: unpackedNodePtr,
      parent: unpackedParentPtr,
      space: unpackedSpacePtr,
      id: String(objectValue["2"]),
      _session,
      _graph,
      _connection,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): CutEvent {
    return CutEvent.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): CutEventProto {
    return CutEvent.__packProto__(this);
  }

  static __packProto__(object: CutEvent): CutEventProto {
    const objectProto: Partial<CutEventProto> = { metatype: 9541 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    if (object.nodePtr != null) {
      objectProto.nodePtr = object.nodePtr.toProto();
    }
    return objectProto as CutEventProto;
  }

  static __unpackProto__(
    objectProto: CutEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): CutEvent {
    return new CutEvent({
      node:
        objectProto.nodePtr != undefined
          ? NodeReference.fromProto(
              objectProto.nodePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(
              objectProto.spacePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      id: String(objectProto.id),
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: CutEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): CutEvent {
    return CutEvent.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): CutEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = CutEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.CUT_EVENT, CutEvent);
/* ==== DESTACK_GENERATED_END:NODE:9541 ==== */

/* ==== DESTACK_GENERATED_START:NODE:9542 ==== */
/**
 * A PasteEvent is a ClipboardEvent when a paste is performed.
 */
export class PasteEvent extends Node implements ClipboardEvent {
  static metatype: NodeType = NodeType.PASTE_EVENT;
  static __traits__: TraitType[] = [
    TraitType.EVENT,
    TraitType.SPATIAL,
    TraitType.CLIPBOARD_EVENT,
    TraitType.INPUT_EVENT,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.SPACE];
  static __childTypes__: NodeType[] = [];
  static __ancestorTypes__: NodeType[] = [NodeType.SPACE];
  static __descendantTypes__: NodeType[] = [];

  /**
   * Spatial.parent
   */
  get parent(): Space | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference | null;

  /**
   * The Node this Event is about.
   */
  get node(): Node | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  set node(node: Node | null) {
    if (node === null) {
      this.nodePtr = null;
    } else {
      this.nodePtr = node.toRef();
    }
  }
  nodePtr: NodeReference | null;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    node?: Node | NodeReference | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: QueryConnection | null;
  }) {
    super(
      // id
      options.id ?? null,
      // parent
      options.parent != null
        ? options.parent.metatype == StructType.NODE_REFERENCE
          ? (options.parent as NodeReference)
          : (options.parent as Node).toRef()
        : null,
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
      // graph
      options._graph ?? null,
      // connection
      options._connection ?? null,
      // is_new
      options.id == null,
      // is_attached
      options.id != null || options._graph != null,
    );

    // properties
    let _parent = options.parent ?? null;
    if (_parent != null && _parent instanceof Node) {
      _parent = _parent.toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space instanceof Node) {
      _space = _space.toRef();
    }
    this.spacePtr = _space;
    let _node = options.node ?? null;
    if (_node != null && _node instanceof Node) {
      _node = _node.toRef();
    }
    this.nodePtr = _node;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(
          `{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`,
        );
      }
      this.createdAt = options.createdAt;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy instanceof Node
            ? options.createdBy.toRef()
            : options.createdBy
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedByPtr =
        options.updatedBy != null
          ? options.updatedBy instanceof Node
            ? options.updatedBy.toRef()
            : options.updatedBy
          : null;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.nodePtr?.id === other.nodePtr?.id)) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    if (this.nodePtr !== null) {
      h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    }
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.PASTE_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return "PasteEvent[id={this.id}]";
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Node | null = this;
    while (node !== null) {
      pathParts.push(node._pathKey);
      node = node.parent;
    }
    if (!this._isAttached) {
      pathParts.push("<detached>");
    }
    return pathParts.reverse().join("/");
  }

  repr(): string {
    return `<PasteEvent '${this.path}'>`;
  }

  toValue(): { [key: string]: any } {
    return PasteEvent.__packValue__(this);
  }

  static __packValue__(object: PasteEvent): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 9542;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    if (object.nodePtr != null) {
      objectValue["35"] = object.nodePtr.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PasteEvent {
    const nodePtrValue = objectValue["35"];
    const unpackedNodePtr =
      nodePtrValue != undefined
        ? NodeReference.fromValue(nodePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new PasteEvent({
      node: unpackedNodePtr,
      parent: unpackedParentPtr,
      space: unpackedSpacePtr,
      id: String(objectValue["2"]),
      _session,
      _graph,
      _connection,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PasteEvent {
    return PasteEvent.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): PasteEventProto {
    return PasteEvent.__packProto__(this);
  }

  static __packProto__(object: PasteEvent): PasteEventProto {
    const objectProto: Partial<PasteEventProto> = { metatype: 9542 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    if (object.nodePtr != null) {
      objectProto.nodePtr = object.nodePtr.toProto();
    }
    return objectProto as PasteEventProto;
  }

  static __unpackProto__(
    objectProto: PasteEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PasteEvent {
    return new PasteEvent({
      node:
        objectProto.nodePtr != undefined
          ? NodeReference.fromProto(
              objectProto.nodePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(
              objectProto.spacePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      id: String(objectProto.id),
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: PasteEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PasteEvent {
    return PasteEvent.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): PasteEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = PasteEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.PASTE_EVENT, PasteEvent);
/* ==== DESTACK_GENERATED_END:NODE:9542 ==== */

/* ==== DESTACK_GENERATED_START:NODE:9552 ==== */
/**
 * A FocusInEvent is a FocusEvent when a focus is gained.
 */
export class FocusInEvent extends Node implements FocusEvent {
  static metatype: NodeType = NodeType.FOCUS_IN_EVENT;
  static __traits__: TraitType[] = [
    TraitType.SPATIAL,
    TraitType.INPUT_EVENT,
    TraitType.EVENT,
    TraitType.FOCUS_EVENT,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.SPACE];
  static __childTypes__: NodeType[] = [];
  static __ancestorTypes__: NodeType[] = [NodeType.SPACE];
  static __descendantTypes__: NodeType[] = [];

  /**
   * Spatial.parent
   */
  get parent(): Space | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference | null;

  /**
   * The Node this Event is about.
   */
  get node(): Node | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  set node(node: Node | null) {
    if (node === null) {
      this.nodePtr = null;
    } else {
      this.nodePtr = node.toRef();
    }
  }
  nodePtr: NodeReference | null;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    node?: Node | NodeReference | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: QueryConnection | null;
  }) {
    super(
      // id
      options.id ?? null,
      // parent
      options.parent != null
        ? options.parent.metatype == StructType.NODE_REFERENCE
          ? (options.parent as NodeReference)
          : (options.parent as Node).toRef()
        : null,
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
      // graph
      options._graph ?? null,
      // connection
      options._connection ?? null,
      // is_new
      options.id == null,
      // is_attached
      options.id != null || options._graph != null,
    );

    // properties
    let _parent = options.parent ?? null;
    if (_parent != null && _parent instanceof Node) {
      _parent = _parent.toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space instanceof Node) {
      _space = _space.toRef();
    }
    this.spacePtr = _space;
    let _node = options.node ?? null;
    if (_node != null && _node instanceof Node) {
      _node = _node.toRef();
    }
    this.nodePtr = _node;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(
          `{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`,
        );
      }
      this.createdAt = options.createdAt;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy instanceof Node
            ? options.createdBy.toRef()
            : options.createdBy
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedByPtr =
        options.updatedBy != null
          ? options.updatedBy instanceof Node
            ? options.updatedBy.toRef()
            : options.updatedBy
          : null;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.nodePtr?.id === other.nodePtr?.id)) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    if (this.nodePtr !== null) {
      h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    }
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.FOCUS_IN_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return "FocusInEvent[id={this.id}]";
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Node | null = this;
    while (node !== null) {
      pathParts.push(node._pathKey);
      node = node.parent;
    }
    if (!this._isAttached) {
      pathParts.push("<detached>");
    }
    return pathParts.reverse().join("/");
  }

  repr(): string {
    return `<FocusInEvent '${this.path}'>`;
  }

  toValue(): { [key: string]: any } {
    return FocusInEvent.__packValue__(this);
  }

  static __packValue__(object: FocusInEvent): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 9552;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    if (object.nodePtr != null) {
      objectValue["35"] = object.nodePtr.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): FocusInEvent {
    const nodePtrValue = objectValue["35"];
    const unpackedNodePtr =
      nodePtrValue != undefined
        ? NodeReference.fromValue(nodePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new FocusInEvent({
      node: unpackedNodePtr,
      parent: unpackedParentPtr,
      space: unpackedSpacePtr,
      id: String(objectValue["2"]),
      _session,
      _graph,
      _connection,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): FocusInEvent {
    return FocusInEvent.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): FocusInEventProto {
    return FocusInEvent.__packProto__(this);
  }

  static __packProto__(object: FocusInEvent): FocusInEventProto {
    const objectProto: Partial<FocusInEventProto> = { metatype: 9552 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    if (object.nodePtr != null) {
      objectProto.nodePtr = object.nodePtr.toProto();
    }
    return objectProto as FocusInEventProto;
  }

  static __unpackProto__(
    objectProto: FocusInEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): FocusInEvent {
    return new FocusInEvent({
      node:
        objectProto.nodePtr != undefined
          ? NodeReference.fromProto(
              objectProto.nodePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(
              objectProto.spacePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      id: String(objectProto.id),
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: FocusInEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): FocusInEvent {
    return FocusInEvent.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): FocusInEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = FocusInEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.FOCUS_IN_EVENT, FocusInEvent);
/* ==== DESTACK_GENERATED_END:NODE:9552 ==== */

/* ==== DESTACK_GENERATED_START:NODE:9553 ==== */
/**
 * A FocusOutEvent is a FocusEvent when a focus is lost.
 */
export class FocusOutEvent extends Node implements FocusEvent {
  static metatype: NodeType = NodeType.FOCUS_OUT_EVENT;
  static __traits__: TraitType[] = [
    TraitType.SPATIAL,
    TraitType.INPUT_EVENT,
    TraitType.EVENT,
    TraitType.FOCUS_EVENT,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.SPACE];
  static __childTypes__: NodeType[] = [];
  static __ancestorTypes__: NodeType[] = [NodeType.SPACE];
  static __descendantTypes__: NodeType[] = [];

  /**
   * Spatial.parent
   */
  get parent(): Space | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference | null;

  /**
   * The Node this Event is about.
   */
  get node(): Node | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  set node(node: Node | null) {
    if (node === null) {
      this.nodePtr = null;
    } else {
      this.nodePtr = node.toRef();
    }
  }
  nodePtr: NodeReference | null;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    node?: Node | NodeReference | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: QueryConnection | null;
  }) {
    super(
      // id
      options.id ?? null,
      // parent
      options.parent != null
        ? options.parent.metatype == StructType.NODE_REFERENCE
          ? (options.parent as NodeReference)
          : (options.parent as Node).toRef()
        : null,
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
      // graph
      options._graph ?? null,
      // connection
      options._connection ?? null,
      // is_new
      options.id == null,
      // is_attached
      options.id != null || options._graph != null,
    );

    // properties
    let _parent = options.parent ?? null;
    if (_parent != null && _parent instanceof Node) {
      _parent = _parent.toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space instanceof Node) {
      _space = _space.toRef();
    }
    this.spacePtr = _space;
    let _node = options.node ?? null;
    if (_node != null && _node instanceof Node) {
      _node = _node.toRef();
    }
    this.nodePtr = _node;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(
          `{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`,
        );
      }
      this.createdAt = options.createdAt;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy instanceof Node
            ? options.createdBy.toRef()
            : options.createdBy
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedByPtr =
        options.updatedBy != null
          ? options.updatedBy instanceof Node
            ? options.updatedBy.toRef()
            : options.updatedBy
          : null;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.nodePtr?.id === other.nodePtr?.id)) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    if (this.nodePtr !== null) {
      h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    }
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.FOCUS_OUT_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return "FocusOutEvent[id={this.id}]";
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Node | null = this;
    while (node !== null) {
      pathParts.push(node._pathKey);
      node = node.parent;
    }
    if (!this._isAttached) {
      pathParts.push("<detached>");
    }
    return pathParts.reverse().join("/");
  }

  repr(): string {
    return `<FocusOutEvent '${this.path}'>`;
  }

  toValue(): { [key: string]: any } {
    return FocusOutEvent.__packValue__(this);
  }

  static __packValue__(object: FocusOutEvent): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 9553;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    if (object.nodePtr != null) {
      objectValue["35"] = object.nodePtr.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): FocusOutEvent {
    const nodePtrValue = objectValue["35"];
    const unpackedNodePtr =
      nodePtrValue != undefined
        ? NodeReference.fromValue(nodePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new FocusOutEvent({
      node: unpackedNodePtr,
      parent: unpackedParentPtr,
      space: unpackedSpacePtr,
      id: String(objectValue["2"]),
      _session,
      _graph,
      _connection,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): FocusOutEvent {
    return FocusOutEvent.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): FocusOutEventProto {
    return FocusOutEvent.__packProto__(this);
  }

  static __packProto__(object: FocusOutEvent): FocusOutEventProto {
    const objectProto: Partial<FocusOutEventProto> = { metatype: 9553 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    if (object.nodePtr != null) {
      objectProto.nodePtr = object.nodePtr.toProto();
    }
    return objectProto as FocusOutEventProto;
  }

  static __unpackProto__(
    objectProto: FocusOutEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): FocusOutEvent {
    return new FocusOutEvent({
      node:
        objectProto.nodePtr != undefined
          ? NodeReference.fromProto(
              objectProto.nodePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(
              objectProto.spacePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      id: String(objectProto.id),
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: FocusOutEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): FocusOutEvent {
    return FocusOutEvent.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): FocusOutEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = FocusOutEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.FOCUS_OUT_EVENT, FocusOutEvent);
/* ==== DESTACK_GENERATED_END:NODE:9553 ==== */
