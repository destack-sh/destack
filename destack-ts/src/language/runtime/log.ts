import {
  packProtoJson,
  packProtoTimestamp,
  unpackProtoJson,
  unpackProtoTimestamp,
} from "@destack/grpc";
import { Graph, NodeReference, QueryConnection, Session, Supergraph } from "@destack/language/core";
import {
  Analytic,
  EnumType,
  IsFrozen,
  IsSubject,
  Node,
  NodeType,
  Spatial,
  StructType,
  TraitType,
} from "@destack/language/core/builtin";
import { registerEnumClass, registerNodeClass } from "@destack/language/registry";
import { Space } from "@destack/language/space";
import { LogLevelProto, LogProto } from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:4100 ==== */
/**
 * LogLevel
 */
export enum LogLevel {
  TRACE = 1,
  DEBUG = 2,
  INFO = 3,
  WARNING = 4,
  ERROR = 5,
  PANIC = 6,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.LOG_LEVEL, LogLevel);
/* ==== DESTACK_GENERATED_END:ENUM:4100 ==== */

/* ==== DESTACK_GENERATED_START:NODE:4100 ==== */
/**
 * A Log message.
 */
export class Log extends Node implements Spatial, Analytic, IsFrozen {
  static metatype: NodeType = NodeType.LOG;
  static __traits__: TraitType[] = [
    TraitType.SPATIAL,
    TraitType.TRACKED,
    TraitType.ANALYTIC,
    TraitType.FROZEN,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.SPACE];
  static __childTypes__: NodeType[] = [];
  static __ancestorTypes__: NodeType[] = [NodeType.SPACE];
  static __descendantTypes__: NodeType[] = [];

  /**
   * Log.parent
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
   * IsTracked.createdAt
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * IsTracked.createdBy
   */
  get createdBy(): (Node & IsSubject) | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;

  /**
   * IsTracked.updatedAt
   */
  readonly updatedAt: Temporal.ZonedDateTime;

  /**
   * IsTracked.updatedBy
   */
  get updatedBy(): (Node & IsSubject) | null {
    const nodePtr: NodeReference | null = this.updatedByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null;
    }
    return null;
  }
  readonly updatedByPtr: NodeReference | null;

  /**
   * Log.content
   */
  content: string;

  /**
   * Log.attributes
   */
  attributes: Map<string, any>;

  /**
   * Log.level
   */
  level: LogLevel;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    content: string;
    attributes?: Map<string, any>;
    level: LogLevel;
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
    let _content = options.content;
    if (_content === null) {
      throw new Error(`Log.content is required`);
    }
    this.content = _content;
    let _attributes = options.attributes ?? null;
    if (_attributes === null) {
      _attributes = new Map();
    }
    this.attributes = _attributes;
    let _level = options.level;
    if (_level === null) {
      throw new Error(`Log.level is required`);
    }
    this.level = _level;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO();
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
    if (!(this.content === other.content)) {
      return false;
    }
    if (JSON.stringify(this.attributes) !== JSON.stringify(other.attributes)) {
      return false;
    }
    if (!(this.level === other.level)) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.LOG,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return "Log[id={this.id}]";
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

  toValue(): { [key: string]: any } {
    return Log.__packValue__(this);
  }

  static __packValue__(object: Log): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 4100;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    objectValue["15"] = object.createdAt.toString();
    if (object.createdByPtr != null) {
      objectValue["16"] = object.createdByPtr.toValue();
    }
    objectValue["17"] = object.updatedAt.toString();
    if (object.updatedByPtr != null) {
      objectValue["18"] = object.updatedByPtr.toValue();
    }
    objectValue["40"] = object.content;
    if (object.attributes.size > 0) {
      const packedAttributes: { [key: string]: any } = {};
      for (const [key, value] of object.attributes) {
        packedAttributes[String(key)] = value;
      }
      objectValue["41"] = packedAttributes;
    }
    objectValue["42"] = object.level;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Log {
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const unpackedAttributes = new Map();
    if (objectValue["41"] != undefined) {
      for (const [key, value] of Object.entries(objectValue["41"])) {
        unpackedAttributes.set(key, value as any);
      }
    }
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const createdByPtrValue = objectValue["16"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? NodeReference.fromValue(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByPtrValue = objectValue["18"];
    const unpackedUpdatedByPtr =
      updatedByPtrValue != undefined
        ? NodeReference.fromValue(updatedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new Log({
      parent: unpackedParentPtr,
      content: objectValue["40"],
      attributes: unpackedAttributes,
      level: Number(objectValue["42"]),
      space: unpackedSpacePtr,
      id: String(objectValue["2"]),
      createdAt: Temporal.ZonedDateTime.from(objectValue["15"]),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.ZonedDateTime.from(objectValue["17"]),
      updatedBy: unpackedUpdatedByPtr,
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
  ): Log {
    return Log.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): LogProto {
    return Log.__packProto__(this);
  }

  static __packProto__(object: Log): LogProto {
    const objectProto: Partial<LogProto> = { metatype: 4100 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    objectProto.createdAt = packProtoTimestamp(object.createdAt);
    if (object.createdByPtr != null) {
      objectProto.createdByPtr = object.createdByPtr.toProto();
    }
    objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
    if (object.updatedByPtr != null) {
      objectProto.updatedByPtr = object.updatedByPtr.toProto();
    }
    objectProto.content = object.content;
    if (object.attributes) {
      objectProto.attributes = {};
      for (const [key, value] of object.attributes) {
        objectProto.attributes![key] = packProtoJson(value);
      }
    }
    objectProto.level = Number(object.level) as LogLevelProto;
    return objectProto as LogProto;
  }

  static __unpackProto__(
    objectProto: LogProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Log {
    const unpackedAttributes = new Map();
    if (objectProto.attributes) {
      for (const [key, value] of Object.entries(objectProto.attributes)) {
        unpackedAttributes.set(key, unpackProtoJson(value as any));
      }
    }
    return new Log({
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
      content: objectProto.content,
      attributes: unpackedAttributes,
      level: Number(objectProto.level) as LogLevel,
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
      createdAt: unpackProtoTimestamp(objectProto.createdAt!),
      createdBy:
        objectProto.createdByPtr != undefined
          ? NodeReference.fromProto(
              objectProto.createdByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
      updatedBy:
        objectProto.updatedByPtr != undefined
          ? NodeReference.fromProto(
              objectProto.updatedByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: LogProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Log {
    return Log.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Log {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = LogProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.LOG, Log);
/* ==== DESTACK_GENERATED_END:NODE:4100 ==== */
