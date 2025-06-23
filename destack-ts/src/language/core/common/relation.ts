import {
  CustomEntityDefinition,
  Field,
  Node,
  NodeType,
  Region,
  Session,
  StructFrozen,
  StructType,
  Supergraph,
  TraitType,
} from "@/language";

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
  readonly region: Region | null;
  readonly spaceId: string | null;

  constructor(options: {
    region?: Region | null;
    spaceId?: string | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // supergraph
      options._supergraph ?? null,
    );

    this.region = options.region ?? null;
    this.spaceId = options.spaceId ?? null;
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
  readonly type: RelationType;
  readonly nodeType: NodeType | null;
  get definition(): CustomEntityDefinition | null {
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
      // supergraph
      options._supergraph ?? null,
    );

    this.type = options.type;
    this.nodeType = options.nodeType ?? null;
    this.definitionPtr =
      options.definition != null
        ? options.definition.metatype == StructType.NODE_REFERENCE
          ? (options.definition as NodeReference)
          : (options.definition as Node).toRef()
        : null;
    this.traitType = options.traitType ?? null;
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
/* ==== DESTACK_GENERATED_END:STRUCT:50107 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50108 ==== */
export class AttributeReference extends StructFrozen {
  readonly type: AttributeType;
  readonly propPtr: PropertyReference | null;
  get field(): Field | null {
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
      // supergraph
      options._supergraph ?? null,
    );

    this.type = options.type;
    this.propPtr = options.propPtr ?? null;
    this.fieldPtr =
      options.field != null
        ? options.field.metatype == StructType.NODE_REFERENCE
          ? (options.field as NodeReference)
          : (options.field as Node).toRef()
        : null;
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
/* ==== DESTACK_GENERATED_END:STRUCT:50108 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50003 ==== */
export class PropertyReference extends StructFrozen {
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
      // supergraph
      options._supergraph ?? null,
    );

    this.type = options.type;
    this.nodeType = options.nodeType ?? null;
    this.traitType = options.traitType ?? null;
    this.structType = options.structType ?? null;
    this.id = options.id;
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
      // supergraph
      options._supergraph ?? null,
    );

    this.nodeType = options.nodeType;
    this.id = options.id;
    this.spaceId = options.spaceId ?? null;
    this.definitionId = options.definitionId ?? null;
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
