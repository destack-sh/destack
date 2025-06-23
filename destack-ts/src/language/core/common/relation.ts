import {
  CustomEntityDefinition,
  Field,
  Node,
  NodeClass,
  NodeType,
  Region,
  Session,
  StructFrozen,
  StructType,
  Supergraph,
  TraitType,
} from "@/language";
import { assertNever } from "@/utils/functools";

/* ==== DESTACK_GENERATED_START:ENUM:50010 ==== */
export enum RelationType {
  BUILTIN_NODE = 1,
  CUSTOM_NODE = 2,
  TRAIT = 3,
}
/* ==== DESTACK_GENERATED_END:ENUM:50010 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:50011 ==== */
export enum AttributeType {
  PROPERTY = 1,
  FIELD = 2,
}
/* ==== DESTACK_GENERATED_END:ENUM:50011 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:50012 ==== */
export enum PropertyReferenceType {
  NODE = 1,
  TRAIT = 2,
  STRUCT = 3,
}
/* ==== DESTACK_GENERATED_END:ENUM:50012 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50000 ==== */
export class Scope extends StructFrozen {
  static metatype: StructType = StructType.SCOPE;
  static __isFrozen__: boolean = true;

  readonly region: Region | null;
  readonly spaceId: string | null;

  constructor(options: {
    region?: Region | null;
    spaceId?: string | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties
    let _region = options.region ?? null;
    this.region = _region;
    let _spaceId = options.spaceId ?? null;
    this.spaceId = _spaceId;
    // identity
    // ...
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
/* ==== DESTACK_GENERATED_END:STRUCT:50000 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50107 ==== */
export class RelationReference extends StructFrozen {
  static metatype: StructType = StructType.RELATION_REFERENCE;
  static __isFrozen__: boolean = true;

  readonly type: RelationType;
  readonly nodeType: NodeType | null;
  get definition(): CustomEntityDefinition | null | null {
    const nodePtr: NodeReference | null = this.definitionPtr;
    if (nodePtr !== null) {
      if (this._supergraph === null) {
        return null;
      }
      return this._supergraph.get(nodePtr.id) as CustomEntityDefinition | null;
    }
    return null;
  }
  readonly definitionPtr: NodeReference | null;
  readonly traitType: TraitType | null;

  constructor(options: {
    type: RelationType;
    nodeType?: NodeType | null;
    definition?: CustomEntityDefinition | NodeReference | null;
    traitType?: TraitType | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties
    let _type = options.type;
    if (_type === null) {
      throw new Error(`RelationReference.type is required`);
    }
    this.type = _type;
    let _nodeType = options.nodeType ?? null;
    this.nodeType = _nodeType;
    let _definition = options.definition ?? null;
    if (_definition != null && _definition instanceof Node) {
      _definition = _definition.toRef();
    }
    this.definitionPtr = _definition;
    let _traitType = options.traitType ?? null;
    this.traitType = _traitType;
    // identity
    // ...
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

  /* ==== DESTACK_CUSTOM_START ==== */

  static of(base: NodeType | NodeClass | CustomEntityDefinition) {
    if (typeof base == "number") {
      return new RelationReference({ type: RelationType.BUILTIN_NODE, nodeType: base });
    } else if (base instanceof CustomEntityDefinition) {
      return new RelationReference({ type: RelationType.CUSTOM_NODE, definition: base });
    } else {
      return new RelationReference({ type: RelationType.BUILTIN_NODE, nodeType: base.metatype });
    }
  }
}

/* ==== DESTACK_GENERATED_END:STRUCT:50107 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50108 ==== */
export class AttributeReference extends StructFrozen {
  static metatype: StructType = StructType.ATTRIBUTE_REFERENCE;
  static __isFrozen__: boolean = true;

  readonly type: AttributeType;
  readonly propPtr: PropertyReference | null;
  get field(): Field | null | null {
    const nodePtr: NodeReference | null = this.fieldPtr;
    if (nodePtr !== null) {
      if (this._supergraph === null) {
        return null;
      }
      return this._supergraph.get(nodePtr.id) as Field | null;
    }
    return null;
  }
  readonly fieldPtr: NodeReference | null;

  constructor(options: {
    type: AttributeType;
    propPtr?: PropertyReference | null;
    field?: Field | NodeReference | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties
    let _type = options.type;
    if (_type === null) {
      throw new Error(`AttributeReference.type is required`);
    }
    this.type = _type;
    let _propPtr = options.propPtr ?? null;
    this.propPtr = _propPtr;
    let _field = options.field ?? null;
    if (_field != null && _field instanceof Node) {
      _field = _field.toRef();
    }
    this.fieldPtr = _field;
    // identity
    // ...
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

  /* ==== DESTACK_CUSTOM_START ==== */

  static of(attribute: Field | PropertyReference | AttributeReference) {
    if (attribute instanceof PropertyReference) {
      return new AttributeReference({ type: AttributeType.PROPERTY, propPtr: attribute });
    } else if (attribute instanceof Field) {
      return new AttributeReference({ type: AttributeType.FIELD, field: attribute });
    } else if (attribute instanceof AttributeReference) {
      return attribute;
    } else {
      assertNever(attribute);
    }
  }
}

/* ==== DESTACK_GENERATED_END:STRUCT:50108 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50003 ==== */
export class PropertyReference extends StructFrozen {
  static metatype: StructType = StructType.PROPERTY_REFERENCE;
  static __isFrozen__: boolean = true;

  readonly type: PropertyReferenceType;
  readonly nodeType: NodeType | null;
  readonly traitType: TraitType | null;
  readonly structType: StructType | null;
  readonly id: number;

  constructor(options: {
    type: PropertyReferenceType;
    nodeType?: NodeType | null;
    traitType?: TraitType | null;
    structType?: StructType | null;
    id: number;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties
    let _type = options.type;
    if (_type === null) {
      throw new Error(`PropertyReference.type is required`);
    }
    this.type = _type;
    let _nodeType = options.nodeType ?? null;
    this.nodeType = _nodeType;
    let _traitType = options.traitType ?? null;
    this.traitType = _traitType;
    let _structType = options.structType ?? null;
    this.structType = _structType;
    let _id = options.id;
    if (_id === null) {
      throw new Error(`PropertyReference.id is required`);
    }
    this.id = _id;
    // identity
    // ...
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
/* ==== DESTACK_GENERATED_END:STRUCT:50003 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50002 ==== */
export class NodeReference extends StructFrozen {
  static metatype: StructType = StructType.NODE_REFERENCE;
  static __isFrozen__: boolean = true;

  readonly nodeType: NodeType;
  readonly id: string;
  readonly spaceId: string | null;
  readonly definitionId: string | null;

  constructor(options: {
    nodeType: NodeType;
    id: string;
    spaceId?: string | null;
    definitionId?: string | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties
    let _nodeType = options.nodeType;
    if (_nodeType === null) {
      throw new Error(`NodeReference.nodeType is required`);
    }
    this.nodeType = _nodeType;
    let _id = options.id;
    if (_id === null) {
      throw new Error(`NodeReference.id is required`);
    }
    this.id = _id;
    let _spaceId = options.spaceId ?? null;
    this.spaceId = _spaceId;
    let _definitionId = options.definitionId ?? null;
    this.definitionId = _definitionId;
    // identity
    // ...
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
/* ==== DESTACK_GENERATED_END:STRUCT:50002 ==== */
