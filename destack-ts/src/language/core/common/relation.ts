import { Graph, Supergraph, CustomEntityDefinition, Struct, QueryConnection, activeSession, StructType, BuiltinObject, EnumType, ACTIVE_SESSION, NodeType, Session, TraitType, StructFrozen, Node, Region, Field } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

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
    region?: Region | null,
    spaceId?: string | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(supergraph);
    this.region = options.region ?? null;
    this.spaceId = options.spaceId ?? null;
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
  ;
  definitionPtr: NodeReference | null
  readonly traitType: TraitType | null;

  constructor(options: {
    type: RelationType,
    nodeType?: NodeType | null,
    definition?: CustomEntityDefinition | NodeReference | null,
    traitType?: TraitType | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(supergraph);
    this.type = options.type;
    this.nodeType = options.nodeType ?? null;
    this.definitionPtr = options.definition != null ? (options.definition.metatype == StructType.NODE_REFERENCE ? (options.definition as NodeReference) : (options.definition as Node).toRef()) : null;
    this.traitType = options.traitType ?? null;
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
  ;
  fieldPtr: NodeReference | null

  constructor(options: {
    type: AttributeType,
    propPtr?: PropertyReference | null,
    field?: Field | NodeReference | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(supergraph);
    this.type = options.type;
    this.propPtr = options.propPtr ?? null;
    this.fieldPtr = options.field != null ? (options.field.metatype == StructType.NODE_REFERENCE ? (options.field as NodeReference) : (options.field as Node).toRef()) : null;
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
/* ==== DESTACK_GENERATED_END:STRUCT:50108 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50003 ==== */
export class PropertyReference extends StructFrozen {
  readonly type: PropertyReferenceType;
  readonly nodeType: NodeType | null;
  readonly traitType: TraitType | null;
  readonly structType: StructType | null;
  readonly id: number;

  constructor(options: {
    type: PropertyReferenceType,
    nodeType?: NodeType | null,
    traitType?: TraitType | null,
    structType?: StructType | null,
    id: number,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(supergraph);
    this.type = options.type;
    this.nodeType = options.nodeType ?? null;
    this.traitType = options.traitType ?? null;
    this.structType = options.structType ?? null;
    this.id = options.id;
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
/* ==== DESTACK_GENERATED_END:STRUCT:50003 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50002 ==== */
export class NodeReference extends StructFrozen {
  readonly nodeType: NodeType;
  readonly id: string;
  readonly spaceId: string | null;
  readonly definitionId: string | null;

  constructor(options: {
    nodeType: NodeType,
    id: string,
    spaceId?: string | null,
    definitionId?: string | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(supergraph);
    this.nodeType = options.nodeType;
    this.id = options.id;
    this.spaceId = options.spaceId ?? null;
    this.definitionId = options.definitionId ?? null;
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
/* ==== DESTACK_GENERATED_END:STRUCT:50002 ==== */