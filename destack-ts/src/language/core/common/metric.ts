import {
  Graph,
  HasName,
  IsCustomNode,
  IsSubject,
  MaterializationType,
  Measurement,
  Metric,
  Node,
  NodeReference,
  NodeType,
  QueryConnection,
  Session,
  Spatial,
  StructType,
  Supergraph,
  TraitType,
} from "@destack/language/core";
import { Script } from "@destack/language/logic";
import { Space } from "@destack/language/space";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:4110 ==== */
/**
 * A Gauge Metric.
 */
export class GaugeMetric extends Node implements Spatial, Metric, HasName {
  static metatype: NodeType = NodeType.GAUGE_METRIC;
  static __traits__: TraitType[] = [
    TraitType.SPATIAL,
    TraitType.METRIC,
    TraitType.ENTITY,
    TraitType.TRACKED,
    TraitType.CUSTOM_NODE_DEFINITION,
    TraitType.ORDERED,
    TraitType.SOURCEABLE,
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
   * A custom Node Definition's prototype is the default template new CustomNode instances are based on.
   */
  get prototype(): (Node & IsCustomNode) | null {
    const nodePtr: NodeReference | null = this.prototypePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsCustomNode) | null;
    }
    return null;
  }
  set prototype(node: (Node & IsCustomNode) | null) {
    if (node === null) {
      this.prototypePtr = null;
    } else {
      this.prototypePtr = node.toRef();
    }
  }
  prototypePtr: NodeReference | null;

  /**
   * Entity.materialization
   */
  readonly materialization: MaterializationType;

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
   * IsOrdered.orderKey
   */
  readonly orderKey: string;

  /**
   * HasName.name
   */
  name: string;

  /**
   * IsSourceable.source
   */
  get source(): Script | null {
    const nodePtr: NodeReference | null = this.sourcePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Script | null;
    }
    return null;
  }
  readonly sourcePtr: NodeReference | null;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    prototype?: (Node & IsCustomNode) | NodeReference | null;
    materialization?: MaterializationType;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    orderKey?: string;
    name: string;
    source?: Script | NodeReference | null;
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
    let _prototype = options.prototype ?? null;
    if (_prototype != null && _prototype instanceof Node) {
      _prototype = _prototype.toRef();
    }
    this.prototypePtr = _prototype;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = MaterializationType.FULL_GRAPH;
    }
    if (_materialization === null) {
      throw new Error(`GaugeMetric.materialization is required`);
    }
    this.materialization = _materialization;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey === null) {
      _orderKey = "a0";
    }
    if (_orderKey === null) {
      throw new Error(`GaugeMetric.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`GaugeMetric.name is required`);
    }
    this.name = _name;
    let _source = options.source ?? null;
    if (_source != null && _source instanceof Node) {
      _source = _source.toRef();
    }
    this.sourcePtr = _source;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO();
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(`{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`);
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
    throw new Error("not implemented");
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.GAUGE_METRIC,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return this.name;
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
    return GaugeMetric.__packValue__(this);
  }

  static __packValue__(object: GaugeMetric): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 4110;
    objectValue["2"] = String(object.id);
    if (object.parentPtr !== null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr !== null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    if (object.prototypePtr !== null) {
      objectValue["6"] = object.prototypePtr.toValue();
    }
    objectValue["7"] = object.materialization;
    objectValue["15"] = object.createdAt.toString();
    if (object.createdByPtr !== null) {
      objectValue["16"] = object.createdByPtr.toValue();
    }
    objectValue["17"] = object.updatedAt.toString();
    if (object.updatedByPtr !== null) {
      objectValue["18"] = object.updatedByPtr.toValue();
    }
    objectValue["22"] = object.orderKey;
    objectValue["31"] = object.name;
    if (object.sourcePtr !== null) {
      objectValue["210"] = object.sourcePtr.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): GaugeMetric {
    const parentValue = objectValue["3"];
    const unpackedParent =
      parentValue !== undefined
        ? NodeReference.fromValue(parentValue, _session, _supergraph, _graph, _connection)
        : null;
    const spaceValue = objectValue["5"];
    const unpackedSpace =
      spaceValue !== undefined ? NodeReference.fromValue(spaceValue, _session, _supergraph, _graph, _connection) : null;
    const createdByValue = objectValue["16"];
    const unpackedCreatedBy =
      createdByValue !== undefined
        ? NodeReference.fromValue(createdByValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByValue = objectValue["18"];
    const unpackedUpdatedBy =
      updatedByValue !== undefined
        ? NodeReference.fromValue(updatedByValue, _session, _supergraph, _graph, _connection)
        : null;
    const sourceValue = objectValue["210"];
    const unpackedSource =
      sourceValue !== undefined
        ? NodeReference.fromValue(sourceValue, _session, _supergraph, _graph, _connection)
        : null;
    const prototypeValue = objectValue["6"];
    const unpackedPrototype =
      prototypeValue !== undefined
        ? NodeReference.fromValue(prototypeValue, _session, _supergraph, _graph, _connection)
        : null;
    return new GaugeMetric({
      id: String(objectValue["2"]),
      materialization: Number(objectValue["7"]),
      createdAt: Temporal.ZonedDateTime.from(objectValue["15"]),
      updatedAt: Temporal.ZonedDateTime.from(objectValue["17"]),
      orderKey: objectValue["22"],
      name: objectValue["31"],
      parent: unpackedParent,
      space: unpackedSpace,
      createdBy: unpackedCreatedBy,
      updatedBy: unpackedUpdatedBy,
      source: unpackedSource,
      prototype: unpackedPrototype,
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
  ): GaugeMetric {
    return GaugeMetric.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }
}
/* ==== DESTACK_GENERATED_END:NODE:4110 ==== */

/* ==== DESTACK_GENERATED_START:NODE:4111 ==== */
/**
 * A Gauge Measurement.
 */
export class GaugeMeasurement extends Node implements Spatial, Measurement {
  static metatype: NodeType = NodeType.GAUGE_MEASUREMENT;
  static __traits__: TraitType[] = [
    TraitType.SPATIAL,
    TraitType.MEASUREMENT,
    TraitType.ANALYTIC,
    TraitType.TRACKED,
    TraitType.CUSTOM_NODE,
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
   * GaugeMeasurement.definition
   */
  get definition(): GaugeMetric | null {
    const nodePtr: NodeReference | null = this.definitionPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as GaugeMetric | null;
    }
    return null;
  }
  set definition(node: GaugeMetric) {
    this.definitionPtr = node.toRef();
  }
  definitionPtr: NodeReference;

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

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    definition: GaugeMetric | NodeReference;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
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
    let _definition = options.definition;
    if (_definition != null && _definition instanceof Node) {
      _definition = _definition.toRef();
    }
    if (_definition === null) {
      throw new Error(`GaugeMeasurement.definition is required`);
    }
    this.definitionPtr = _definition;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO();
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(`{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`);
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
    throw new Error("not implemented");
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.GAUGE_MEASUREMENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      definitionId: this.definitionPtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return "GaugeMeasurement[id={this.id}]";
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
    return GaugeMeasurement.__packValue__(this);
  }

  static __packValue__(object: GaugeMeasurement): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 4111;
    objectValue["2"] = String(object.id);
    if (object.parentPtr !== null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr !== null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    objectValue["6"] = object.definitionPtr.toValue();
    objectValue["15"] = object.createdAt.toString();
    if (object.createdByPtr !== null) {
      objectValue["16"] = object.createdByPtr.toValue();
    }
    objectValue["17"] = object.updatedAt.toString();
    if (object.updatedByPtr !== null) {
      objectValue["18"] = object.updatedByPtr.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): GaugeMeasurement {
    const parentValue = objectValue["3"];
    const unpackedParent =
      parentValue !== undefined
        ? NodeReference.fromValue(parentValue, _session, _supergraph, _graph, _connection)
        : null;
    const spaceValue = objectValue["5"];
    const unpackedSpace =
      spaceValue !== undefined ? NodeReference.fromValue(spaceValue, _session, _supergraph, _graph, _connection) : null;
    const createdByValue = objectValue["16"];
    const unpackedCreatedBy =
      createdByValue !== undefined
        ? NodeReference.fromValue(createdByValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByValue = objectValue["18"];
    const unpackedUpdatedBy =
      updatedByValue !== undefined
        ? NodeReference.fromValue(updatedByValue, _session, _supergraph, _graph, _connection)
        : null;
    return new GaugeMeasurement({
      id: String(objectValue["2"]),
      createdAt: Temporal.ZonedDateTime.from(objectValue["15"]),
      updatedAt: Temporal.ZonedDateTime.from(objectValue["17"]),
      definition: NodeReference.fromValue(objectValue["6"], _session, _supergraph, _graph, _connection),
      parent: unpackedParent,
      space: unpackedSpace,
      createdBy: unpackedCreatedBy,
      updatedBy: unpackedUpdatedBy,
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
  ): GaugeMeasurement {
    return GaugeMeasurement.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }
}
/* ==== DESTACK_GENERATED_END:NODE:4111 ==== */

/* ==== DESTACK_GENERATED_START:NODE:4112 ==== */
/**
 * A Counter Metric.
 */
export class CounterMetric extends Node implements Spatial, Metric, HasName {
  static metatype: NodeType = NodeType.COUNTER_METRIC;
  static __traits__: TraitType[] = [
    TraitType.SPATIAL,
    TraitType.METRIC,
    TraitType.ENTITY,
    TraitType.TRACKED,
    TraitType.CUSTOM_NODE_DEFINITION,
    TraitType.ORDERED,
    TraitType.SOURCEABLE,
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
   * A custom Node Definition's prototype is the default template new CustomNode instances are based on.
   */
  get prototype(): (Node & IsCustomNode) | null {
    const nodePtr: NodeReference | null = this.prototypePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsCustomNode) | null;
    }
    return null;
  }
  set prototype(node: (Node & IsCustomNode) | null) {
    if (node === null) {
      this.prototypePtr = null;
    } else {
      this.prototypePtr = node.toRef();
    }
  }
  prototypePtr: NodeReference | null;

  /**
   * Entity.materialization
   */
  readonly materialization: MaterializationType;

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
   * IsOrdered.orderKey
   */
  readonly orderKey: string;

  /**
   * HasName.name
   */
  name: string;

  /**
   * IsSourceable.source
   */
  get source(): Script | null {
    const nodePtr: NodeReference | null = this.sourcePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Script | null;
    }
    return null;
  }
  readonly sourcePtr: NodeReference | null;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    prototype?: (Node & IsCustomNode) | NodeReference | null;
    materialization?: MaterializationType;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    orderKey?: string;
    name: string;
    source?: Script | NodeReference | null;
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
    let _prototype = options.prototype ?? null;
    if (_prototype != null && _prototype instanceof Node) {
      _prototype = _prototype.toRef();
    }
    this.prototypePtr = _prototype;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = MaterializationType.FULL_GRAPH;
    }
    if (_materialization === null) {
      throw new Error(`CounterMetric.materialization is required`);
    }
    this.materialization = _materialization;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey === null) {
      _orderKey = "a0";
    }
    if (_orderKey === null) {
      throw new Error(`CounterMetric.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`CounterMetric.name is required`);
    }
    this.name = _name;
    let _source = options.source ?? null;
    if (_source != null && _source instanceof Node) {
      _source = _source.toRef();
    }
    this.sourcePtr = _source;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO();
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(`{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`);
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
    throw new Error("not implemented");
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.COUNTER_METRIC,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return this.name;
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
    return CounterMetric.__packValue__(this);
  }

  static __packValue__(object: CounterMetric): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 4112;
    objectValue["2"] = String(object.id);
    if (object.parentPtr !== null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr !== null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    if (object.prototypePtr !== null) {
      objectValue["6"] = object.prototypePtr.toValue();
    }
    objectValue["7"] = object.materialization;
    objectValue["15"] = object.createdAt.toString();
    if (object.createdByPtr !== null) {
      objectValue["16"] = object.createdByPtr.toValue();
    }
    objectValue["17"] = object.updatedAt.toString();
    if (object.updatedByPtr !== null) {
      objectValue["18"] = object.updatedByPtr.toValue();
    }
    objectValue["22"] = object.orderKey;
    objectValue["31"] = object.name;
    if (object.sourcePtr !== null) {
      objectValue["210"] = object.sourcePtr.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): CounterMetric {
    const parentValue = objectValue["3"];
    const unpackedParent =
      parentValue !== undefined
        ? NodeReference.fromValue(parentValue, _session, _supergraph, _graph, _connection)
        : null;
    const spaceValue = objectValue["5"];
    const unpackedSpace =
      spaceValue !== undefined ? NodeReference.fromValue(spaceValue, _session, _supergraph, _graph, _connection) : null;
    const createdByValue = objectValue["16"];
    const unpackedCreatedBy =
      createdByValue !== undefined
        ? NodeReference.fromValue(createdByValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByValue = objectValue["18"];
    const unpackedUpdatedBy =
      updatedByValue !== undefined
        ? NodeReference.fromValue(updatedByValue, _session, _supergraph, _graph, _connection)
        : null;
    const sourceValue = objectValue["210"];
    const unpackedSource =
      sourceValue !== undefined
        ? NodeReference.fromValue(sourceValue, _session, _supergraph, _graph, _connection)
        : null;
    const prototypeValue = objectValue["6"];
    const unpackedPrototype =
      prototypeValue !== undefined
        ? NodeReference.fromValue(prototypeValue, _session, _supergraph, _graph, _connection)
        : null;
    return new CounterMetric({
      id: String(objectValue["2"]),
      materialization: Number(objectValue["7"]),
      createdAt: Temporal.ZonedDateTime.from(objectValue["15"]),
      updatedAt: Temporal.ZonedDateTime.from(objectValue["17"]),
      orderKey: objectValue["22"],
      name: objectValue["31"],
      parent: unpackedParent,
      space: unpackedSpace,
      createdBy: unpackedCreatedBy,
      updatedBy: unpackedUpdatedBy,
      source: unpackedSource,
      prototype: unpackedPrototype,
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
  ): CounterMetric {
    return CounterMetric.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }
}
/* ==== DESTACK_GENERATED_END:NODE:4112 ==== */

/* ==== DESTACK_GENERATED_START:NODE:4113 ==== */
/**
 * A Counter Measurement.
 */
export class CounterMeasurement extends Node implements Spatial, Measurement {
  static metatype: NodeType = NodeType.COUNTER_MEASUREMENT;
  static __traits__: TraitType[] = [
    TraitType.SPATIAL,
    TraitType.MEASUREMENT,
    TraitType.ANALYTIC,
    TraitType.TRACKED,
    TraitType.CUSTOM_NODE,
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
   * CounterMeasurement.definition
   */
  get definition(): CounterMetric | null {
    const nodePtr: NodeReference | null = this.definitionPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as CounterMetric | null;
    }
    return null;
  }
  set definition(node: CounterMetric) {
    this.definitionPtr = node.toRef();
  }
  definitionPtr: NodeReference;

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

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    definition: CounterMetric | NodeReference;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
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
    let _definition = options.definition;
    if (_definition != null && _definition instanceof Node) {
      _definition = _definition.toRef();
    }
    if (_definition === null) {
      throw new Error(`CounterMeasurement.definition is required`);
    }
    this.definitionPtr = _definition;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO();
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(`{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`);
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
    throw new Error("not implemented");
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.COUNTER_MEASUREMENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      definitionId: this.definitionPtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return "CounterMeasurement[id={this.id}]";
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
    return CounterMeasurement.__packValue__(this);
  }

  static __packValue__(object: CounterMeasurement): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 4113;
    objectValue["2"] = String(object.id);
    if (object.parentPtr !== null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr !== null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    objectValue["6"] = object.definitionPtr.toValue();
    objectValue["15"] = object.createdAt.toString();
    if (object.createdByPtr !== null) {
      objectValue["16"] = object.createdByPtr.toValue();
    }
    objectValue["17"] = object.updatedAt.toString();
    if (object.updatedByPtr !== null) {
      objectValue["18"] = object.updatedByPtr.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): CounterMeasurement {
    const parentValue = objectValue["3"];
    const unpackedParent =
      parentValue !== undefined
        ? NodeReference.fromValue(parentValue, _session, _supergraph, _graph, _connection)
        : null;
    const spaceValue = objectValue["5"];
    const unpackedSpace =
      spaceValue !== undefined ? NodeReference.fromValue(spaceValue, _session, _supergraph, _graph, _connection) : null;
    const createdByValue = objectValue["16"];
    const unpackedCreatedBy =
      createdByValue !== undefined
        ? NodeReference.fromValue(createdByValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByValue = objectValue["18"];
    const unpackedUpdatedBy =
      updatedByValue !== undefined
        ? NodeReference.fromValue(updatedByValue, _session, _supergraph, _graph, _connection)
        : null;
    return new CounterMeasurement({
      id: String(objectValue["2"]),
      createdAt: Temporal.ZonedDateTime.from(objectValue["15"]),
      updatedAt: Temporal.ZonedDateTime.from(objectValue["17"]),
      definition: NodeReference.fromValue(objectValue["6"], _session, _supergraph, _graph, _connection),
      parent: unpackedParent,
      space: unpackedSpace,
      createdBy: unpackedCreatedBy,
      updatedBy: unpackedUpdatedBy,
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
  ): CounterMeasurement {
    return CounterMeasurement.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }
}
/* ==== DESTACK_GENERATED_END:NODE:4113 ==== */

/* ==== DESTACK_GENERATED_START:NODE:4114 ==== */
/**
 * A Histogram Metric.
 */
export class HistogramMetric extends Node implements Spatial, Metric, HasName {
  static metatype: NodeType = NodeType.HISTOGRAM_METRIC;
  static __traits__: TraitType[] = [
    TraitType.SPATIAL,
    TraitType.METRIC,
    TraitType.ENTITY,
    TraitType.TRACKED,
    TraitType.CUSTOM_NODE_DEFINITION,
    TraitType.ORDERED,
    TraitType.SOURCEABLE,
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
   * A custom Node Definition's prototype is the default template new CustomNode instances are based on.
   */
  get prototype(): (Node & IsCustomNode) | null {
    const nodePtr: NodeReference | null = this.prototypePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsCustomNode) | null;
    }
    return null;
  }
  set prototype(node: (Node & IsCustomNode) | null) {
    if (node === null) {
      this.prototypePtr = null;
    } else {
      this.prototypePtr = node.toRef();
    }
  }
  prototypePtr: NodeReference | null;

  /**
   * Entity.materialization
   */
  readonly materialization: MaterializationType;

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
   * IsOrdered.orderKey
   */
  readonly orderKey: string;

  /**
   * HasName.name
   */
  name: string;

  /**
   * IsSourceable.source
   */
  get source(): Script | null {
    const nodePtr: NodeReference | null = this.sourcePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Script | null;
    }
    return null;
  }
  readonly sourcePtr: NodeReference | null;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    prototype?: (Node & IsCustomNode) | NodeReference | null;
    materialization?: MaterializationType;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    orderKey?: string;
    name: string;
    source?: Script | NodeReference | null;
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
    let _prototype = options.prototype ?? null;
    if (_prototype != null && _prototype instanceof Node) {
      _prototype = _prototype.toRef();
    }
    this.prototypePtr = _prototype;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = MaterializationType.FULL_GRAPH;
    }
    if (_materialization === null) {
      throw new Error(`HistogramMetric.materialization is required`);
    }
    this.materialization = _materialization;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey === null) {
      _orderKey = "a0";
    }
    if (_orderKey === null) {
      throw new Error(`HistogramMetric.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`HistogramMetric.name is required`);
    }
    this.name = _name;
    let _source = options.source ?? null;
    if (_source != null && _source instanceof Node) {
      _source = _source.toRef();
    }
    this.sourcePtr = _source;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO();
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(`{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`);
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
    throw new Error("not implemented");
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.HISTOGRAM_METRIC,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return this.name;
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
    return HistogramMetric.__packValue__(this);
  }

  static __packValue__(object: HistogramMetric): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 4114;
    objectValue["2"] = String(object.id);
    if (object.parentPtr !== null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr !== null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    if (object.prototypePtr !== null) {
      objectValue["6"] = object.prototypePtr.toValue();
    }
    objectValue["7"] = object.materialization;
    objectValue["15"] = object.createdAt.toString();
    if (object.createdByPtr !== null) {
      objectValue["16"] = object.createdByPtr.toValue();
    }
    objectValue["17"] = object.updatedAt.toString();
    if (object.updatedByPtr !== null) {
      objectValue["18"] = object.updatedByPtr.toValue();
    }
    objectValue["22"] = object.orderKey;
    objectValue["31"] = object.name;
    if (object.sourcePtr !== null) {
      objectValue["210"] = object.sourcePtr.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): HistogramMetric {
    const parentValue = objectValue["3"];
    const unpackedParent =
      parentValue !== undefined
        ? NodeReference.fromValue(parentValue, _session, _supergraph, _graph, _connection)
        : null;
    const spaceValue = objectValue["5"];
    const unpackedSpace =
      spaceValue !== undefined ? NodeReference.fromValue(spaceValue, _session, _supergraph, _graph, _connection) : null;
    const createdByValue = objectValue["16"];
    const unpackedCreatedBy =
      createdByValue !== undefined
        ? NodeReference.fromValue(createdByValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByValue = objectValue["18"];
    const unpackedUpdatedBy =
      updatedByValue !== undefined
        ? NodeReference.fromValue(updatedByValue, _session, _supergraph, _graph, _connection)
        : null;
    const sourceValue = objectValue["210"];
    const unpackedSource =
      sourceValue !== undefined
        ? NodeReference.fromValue(sourceValue, _session, _supergraph, _graph, _connection)
        : null;
    const prototypeValue = objectValue["6"];
    const unpackedPrototype =
      prototypeValue !== undefined
        ? NodeReference.fromValue(prototypeValue, _session, _supergraph, _graph, _connection)
        : null;
    return new HistogramMetric({
      id: String(objectValue["2"]),
      materialization: Number(objectValue["7"]),
      createdAt: Temporal.ZonedDateTime.from(objectValue["15"]),
      updatedAt: Temporal.ZonedDateTime.from(objectValue["17"]),
      orderKey: objectValue["22"],
      name: objectValue["31"],
      parent: unpackedParent,
      space: unpackedSpace,
      createdBy: unpackedCreatedBy,
      updatedBy: unpackedUpdatedBy,
      source: unpackedSource,
      prototype: unpackedPrototype,
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
  ): HistogramMetric {
    return HistogramMetric.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }
}
/* ==== DESTACK_GENERATED_END:NODE:4114 ==== */

/* ==== DESTACK_GENERATED_START:NODE:4115 ==== */
/**
 * A Histogram Measurement.
 */
export class HistogramMeasurement extends Node implements Spatial, Measurement {
  static metatype: NodeType = NodeType.HISTOGRAM_MEASUREMENT;
  static __traits__: TraitType[] = [
    TraitType.SPATIAL,
    TraitType.MEASUREMENT,
    TraitType.ANALYTIC,
    TraitType.TRACKED,
    TraitType.CUSTOM_NODE,
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
   * HistogramMeasurement.definition
   */
  get definition(): HistogramMetric | null {
    const nodePtr: NodeReference | null = this.definitionPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as HistogramMetric | null;
    }
    return null;
  }
  set definition(node: HistogramMetric) {
    this.definitionPtr = node.toRef();
  }
  definitionPtr: NodeReference;

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

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    definition: HistogramMetric | NodeReference;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
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
    let _definition = options.definition;
    if (_definition != null && _definition instanceof Node) {
      _definition = _definition.toRef();
    }
    if (_definition === null) {
      throw new Error(`HistogramMeasurement.definition is required`);
    }
    this.definitionPtr = _definition;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO();
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(`{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`);
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
    throw new Error("not implemented");
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.HISTOGRAM_MEASUREMENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      definitionId: this.definitionPtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return "HistogramMeasurement[id={this.id}]";
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
    return HistogramMeasurement.__packValue__(this);
  }

  static __packValue__(object: HistogramMeasurement): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 4115;
    objectValue["2"] = String(object.id);
    if (object.parentPtr !== null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr !== null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    objectValue["6"] = object.definitionPtr.toValue();
    objectValue["15"] = object.createdAt.toString();
    if (object.createdByPtr !== null) {
      objectValue["16"] = object.createdByPtr.toValue();
    }
    objectValue["17"] = object.updatedAt.toString();
    if (object.updatedByPtr !== null) {
      objectValue["18"] = object.updatedByPtr.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): HistogramMeasurement {
    const parentValue = objectValue["3"];
    const unpackedParent =
      parentValue !== undefined
        ? NodeReference.fromValue(parentValue, _session, _supergraph, _graph, _connection)
        : null;
    const spaceValue = objectValue["5"];
    const unpackedSpace =
      spaceValue !== undefined ? NodeReference.fromValue(spaceValue, _session, _supergraph, _graph, _connection) : null;
    const createdByValue = objectValue["16"];
    const unpackedCreatedBy =
      createdByValue !== undefined
        ? NodeReference.fromValue(createdByValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByValue = objectValue["18"];
    const unpackedUpdatedBy =
      updatedByValue !== undefined
        ? NodeReference.fromValue(updatedByValue, _session, _supergraph, _graph, _connection)
        : null;
    return new HistogramMeasurement({
      id: String(objectValue["2"]),
      createdAt: Temporal.ZonedDateTime.from(objectValue["15"]),
      updatedAt: Temporal.ZonedDateTime.from(objectValue["17"]),
      definition: NodeReference.fromValue(objectValue["6"], _session, _supergraph, _graph, _connection),
      parent: unpackedParent,
      space: unpackedSpace,
      createdBy: unpackedCreatedBy,
      updatedBy: unpackedUpdatedBy,
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
  ): HistogramMeasurement {
    return HistogramMeasurement.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }
}
/* ==== DESTACK_GENERATED_END:NODE:4115 ==== */
