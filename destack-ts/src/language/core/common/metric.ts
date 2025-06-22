import { Agent, Measurement, BuiltinObject, ACTIVE_SESSION, Session, StructFrozen, IsOrdered, Graph, Struct, activeSession, IsCustomNodeDefinition, StructType, NodeType, Supergraph, QueryConnection, NodeReference, MaterializationType, Metric, User, Analytic, CustomView, Spatial, IsCustomNode, IsSourceable, Space, Entity, CustomEntity, EnumType, Script, Node, IsTracked } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:NODE:4110 ==== */
export class GaugeMetric extends Node implements Spatial, Entity, IsCustomNodeDefinition, IsTracked, IsOrdered, IsSourceable, Metric {
  readonly id: string;
  get parent(): Space | null | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Space | null | null;
      }
      return null;
  }
  ;
  parentPtr: NodeReference | null
  get space(): Space | null | null {
      const nodePtr: NodeReference | null = this.spacePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Space | null | null;
      }
      return null;
  }
  ;
  spacePtr: NodeReference | null
  get prototype(): CustomEntity | GaugeMeasurement | CounterMeasurement | HistogramMeasurement | CustomView | null | null {
      const nodePtr: NodeReference | null = this.prototypePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as CustomEntity | GaugeMeasurement | CounterMeasurement | HistogramMeasurement | CustomView | null | null;
      }
      return null;
  }

  set prototype(node: CustomEntity | GaugeMeasurement | CounterMeasurement | HistogramMeasurement | CustomView | null) {
      if (node === null) {
          this.prototypePtr = null;
      } else {
          this.prototypePtr = node.toRef();
      }
  }
  ;
  prototypePtr: NodeReference | null
  readonly materialization: MaterializationType;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): Agent | User | null | null {
      const nodePtr: NodeReference | null = this.createdByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
      }
      return null;
  }
  ;
  createdByPtr: NodeReference | null
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null | null {
      const nodePtr: NodeReference | null = this.updatedByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
      }
      return null;
  }
  ;
  updatedByPtr: NodeReference | null
  readonly orderKey: string;
  name: string;
  get source(): Script | null | null {
      const nodePtr: NodeReference | null = this.sourcePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Script | null | null;
      }
      return null;
  }
  ;
  sourcePtr: NodeReference | null

  constructor(options: {
    prototype?: CustomEntity | GaugeMeasurement | CounterMeasurement | HistogramMeasurement | CustomView | NodeReference | null,
    name: string,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(options.id, options.parent, session, supergraph, options._graph, options._connection);
    this.prototypePtr = options.prototype != null ? (options.prototype.metatype == StructType.NODE_REFERENCE ? (options.prototype as NodeReference) : (options.prototype as Node).toRef()) : null;
    this.name = options.name;
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

  __toRef__(): NodeReference {
    return new NodeReference(NodeType.GAUGE_METRIC, this.id, this.spacePtr?.id ?? null, null, this._supergraph);
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
}
/* ==== DESTACK_GENERATED_END:NODE:4110 ==== */

/* ==== DESTACK_GENERATED_START:NODE:4111 ==== */
export class GaugeMeasurement extends Node implements Spatial, Analytic, IsCustomNode, IsTracked, Measurement {
  readonly id: string;
  get parent(): Space | null | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Space | null | null;
      }
      return null;
  }
  ;
  parentPtr: NodeReference | null
  get space(): Space | null | null {
      const nodePtr: NodeReference | null = this.spacePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Space | null | null;
      }
      return null;
  }
  ;
  spacePtr: NodeReference | null
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
  ;
  definitionPtr: NodeReference
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): Agent | User | null | null {
      const nodePtr: NodeReference | null = this.createdByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
      }
      return null;
  }
  ;
  createdByPtr: NodeReference | null
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null | null {
      const nodePtr: NodeReference | null = this.updatedByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
      }
      return null;
  }
  ;
  updatedByPtr: NodeReference | null

  constructor(options: {
    definition: GaugeMetric | NodeReference,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(options.id, options.parent, session, supergraph, options._graph, options._connection);
    this.definitionPtr = options.definition != null ? (options.definition.metatype == StructType.NODE_REFERENCE ? (options.definition as NodeReference) : (options.definition as Node).toRef()) : null;
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

  __toRef__(): NodeReference {
    return new NodeReference(NodeType.GAUGE_MEASUREMENT, this.id, this.spacePtr?.id ?? null, this.definitionPtr?.id ?? null, this._supergraph);
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
}
/* ==== DESTACK_GENERATED_END:NODE:4111 ==== */

/* ==== DESTACK_GENERATED_START:NODE:4112 ==== */
export class CounterMetric extends Node implements Spatial, Entity, IsCustomNodeDefinition, IsTracked, IsOrdered, IsSourceable, Metric {
  readonly id: string;
  get parent(): Space | null | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Space | null | null;
      }
      return null;
  }
  ;
  parentPtr: NodeReference | null
  get space(): Space | null | null {
      const nodePtr: NodeReference | null = this.spacePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Space | null | null;
      }
      return null;
  }
  ;
  spacePtr: NodeReference | null
  get prototype(): CustomEntity | GaugeMeasurement | CounterMeasurement | HistogramMeasurement | CustomView | null | null {
      const nodePtr: NodeReference | null = this.prototypePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as CustomEntity | GaugeMeasurement | CounterMeasurement | HistogramMeasurement | CustomView | null | null;
      }
      return null;
  }

  set prototype(node: CustomEntity | GaugeMeasurement | CounterMeasurement | HistogramMeasurement | CustomView | null) {
      if (node === null) {
          this.prototypePtr = null;
      } else {
          this.prototypePtr = node.toRef();
      }
  }
  ;
  prototypePtr: NodeReference | null
  readonly materialization: MaterializationType;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): Agent | User | null | null {
      const nodePtr: NodeReference | null = this.createdByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
      }
      return null;
  }
  ;
  createdByPtr: NodeReference | null
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null | null {
      const nodePtr: NodeReference | null = this.updatedByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
      }
      return null;
  }
  ;
  updatedByPtr: NodeReference | null
  readonly orderKey: string;
  name: string;
  get source(): Script | null | null {
      const nodePtr: NodeReference | null = this.sourcePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Script | null | null;
      }
      return null;
  }
  ;
  sourcePtr: NodeReference | null

  constructor(options: {
    prototype?: CustomEntity | GaugeMeasurement | CounterMeasurement | HistogramMeasurement | CustomView | NodeReference | null,
    name: string,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(options.id, options.parent, session, supergraph, options._graph, options._connection);
    this.prototypePtr = options.prototype != null ? (options.prototype.metatype == StructType.NODE_REFERENCE ? (options.prototype as NodeReference) : (options.prototype as Node).toRef()) : null;
    this.name = options.name;
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

  __toRef__(): NodeReference {
    return new NodeReference(NodeType.COUNTER_METRIC, this.id, this.spacePtr?.id ?? null, null, this._supergraph);
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
}
/* ==== DESTACK_GENERATED_END:NODE:4112 ==== */

/* ==== DESTACK_GENERATED_START:NODE:4113 ==== */
export class CounterMeasurement extends Node implements Spatial, Analytic, IsCustomNode, IsTracked, Measurement {
  readonly id: string;
  get parent(): Space | null | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Space | null | null;
      }
      return null;
  }
  ;
  parentPtr: NodeReference | null
  get space(): Space | null | null {
      const nodePtr: NodeReference | null = this.spacePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Space | null | null;
      }
      return null;
  }
  ;
  spacePtr: NodeReference | null
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
  ;
  definitionPtr: NodeReference
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): Agent | User | null | null {
      const nodePtr: NodeReference | null = this.createdByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
      }
      return null;
  }
  ;
  createdByPtr: NodeReference | null
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null | null {
      const nodePtr: NodeReference | null = this.updatedByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
      }
      return null;
  }
  ;
  updatedByPtr: NodeReference | null

  constructor(options: {
    definition: CounterMetric | NodeReference,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(options.id, options.parent, session, supergraph, options._graph, options._connection);
    this.definitionPtr = options.definition != null ? (options.definition.metatype == StructType.NODE_REFERENCE ? (options.definition as NodeReference) : (options.definition as Node).toRef()) : null;
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

  __toRef__(): NodeReference {
    return new NodeReference(NodeType.COUNTER_MEASUREMENT, this.id, this.spacePtr?.id ?? null, this.definitionPtr?.id ?? null, this._supergraph);
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
}
/* ==== DESTACK_GENERATED_END:NODE:4113 ==== */

/* ==== DESTACK_GENERATED_START:NODE:4114 ==== */
export class HistogramMetric extends Node implements Spatial, Entity, IsCustomNodeDefinition, IsTracked, IsOrdered, IsSourceable, Metric {
  readonly id: string;
  get parent(): Space | null | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Space | null | null;
      }
      return null;
  }
  ;
  parentPtr: NodeReference | null
  get space(): Space | null | null {
      const nodePtr: NodeReference | null = this.spacePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Space | null | null;
      }
      return null;
  }
  ;
  spacePtr: NodeReference | null
  get prototype(): CustomEntity | GaugeMeasurement | CounterMeasurement | HistogramMeasurement | CustomView | null | null {
      const nodePtr: NodeReference | null = this.prototypePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as CustomEntity | GaugeMeasurement | CounterMeasurement | HistogramMeasurement | CustomView | null | null;
      }
      return null;
  }

  set prototype(node: CustomEntity | GaugeMeasurement | CounterMeasurement | HistogramMeasurement | CustomView | null) {
      if (node === null) {
          this.prototypePtr = null;
      } else {
          this.prototypePtr = node.toRef();
      }
  }
  ;
  prototypePtr: NodeReference | null
  readonly materialization: MaterializationType;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): Agent | User | null | null {
      const nodePtr: NodeReference | null = this.createdByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
      }
      return null;
  }
  ;
  createdByPtr: NodeReference | null
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null | null {
      const nodePtr: NodeReference | null = this.updatedByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
      }
      return null;
  }
  ;
  updatedByPtr: NodeReference | null
  readonly orderKey: string;
  name: string;
  get source(): Script | null | null {
      const nodePtr: NodeReference | null = this.sourcePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Script | null | null;
      }
      return null;
  }
  ;
  sourcePtr: NodeReference | null

  constructor(options: {
    prototype?: CustomEntity | GaugeMeasurement | CounterMeasurement | HistogramMeasurement | CustomView | NodeReference | null,
    name: string,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(options.id, options.parent, session, supergraph, options._graph, options._connection);
    this.prototypePtr = options.prototype != null ? (options.prototype.metatype == StructType.NODE_REFERENCE ? (options.prototype as NodeReference) : (options.prototype as Node).toRef()) : null;
    this.name = options.name;
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

  __toRef__(): NodeReference {
    return new NodeReference(NodeType.HISTOGRAM_METRIC, this.id, this.spacePtr?.id ?? null, null, this._supergraph);
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
}
/* ==== DESTACK_GENERATED_END:NODE:4114 ==== */

/* ==== DESTACK_GENERATED_START:NODE:4115 ==== */
export class HistogramMeasurement extends Node implements Spatial, Analytic, IsCustomNode, IsTracked, Measurement {
  readonly id: string;
  get parent(): Space | null | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Space | null | null;
      }
      return null;
  }
  ;
  parentPtr: NodeReference | null
  get space(): Space | null | null {
      const nodePtr: NodeReference | null = this.spacePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Space | null | null;
      }
      return null;
  }
  ;
  spacePtr: NodeReference | null
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
  ;
  definitionPtr: NodeReference
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): Agent | User | null | null {
      const nodePtr: NodeReference | null = this.createdByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
      }
      return null;
  }
  ;
  createdByPtr: NodeReference | null
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null | null {
      const nodePtr: NodeReference | null = this.updatedByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
      }
      return null;
  }
  ;
  updatedByPtr: NodeReference | null

  constructor(options: {
    definition: HistogramMetric | NodeReference,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(options.id, options.parent, session, supergraph, options._graph, options._connection);
    this.definitionPtr = options.definition != null ? (options.definition.metatype == StructType.NODE_REFERENCE ? (options.definition as NodeReference) : (options.definition as Node).toRef()) : null;
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

  __toRef__(): NodeReference {
    return new NodeReference(NodeType.HISTOGRAM_MEASUREMENT, this.id, this.spacePtr?.id ?? null, this.definitionPtr?.id ?? null, this._supergraph);
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
}
/* ==== DESTACK_GENERATED_END:NODE:4115 ==== */