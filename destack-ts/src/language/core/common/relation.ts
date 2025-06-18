import { Graph, Field, TraitType, Supergraph, Region, CustomEntityDefinition, StructFrozen, QueryConnection, StructType, NodeType, Session, Node, Struct, EnumType, BuiltinObject } from '@/language';
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

  constructor(
    region: Region | null,
    spaceId: string | null,
    _supergraph: Supergraph
  ) {
    super(_supergraph);
    this.region = region;
    this.spaceId = spaceId;
  }


  static create(options: {
    region?: Region | null,
    spaceId?: string | null
  }): Scope {

    return new Scope(

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
          return this._supergraph.get(nodePtr.id);
      }
      return null;
  }
  ;
  definitionPtr: NodeReference | null
  readonly traitType: TraitType | null;

  constructor(
    type: RelationType,
    nodeType: NodeType | null,
    definitionPtr: NodeReference | null,
    traitType: TraitType | null,
    _supergraph: Supergraph
  ) {
    super(_supergraph);
    this.type = type;
    this.nodeType = nodeType;
    this.definitionPtr = definitionPtr;
    this.traitType = traitType;
  }


  static create(options: {
    type: RelationType,
    nodeType?: NodeType | null,
    definition?: CustomEntityDefinition | NodeReference | null,
    traitType?: TraitType | null
  }): RelationReference {

    return new RelationReference(

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
          return this._supergraph.get(nodePtr.id);
      }
      return null;
  }
  ;
  fieldPtr: NodeReference | null

  constructor(
    type: AttributeType,
    propPtr: PropertyReference | null,
    fieldPtr: NodeReference | null,
    _supergraph: Supergraph
  ) {
    super(_supergraph);
    this.type = type;
    this.propPtr = propPtr;
    this.fieldPtr = fieldPtr;
  }


  static create(options: {
    type: AttributeType,
    propPtr?: PropertyReference | null,
    field?: Field | NodeReference | null
  }): AttributeReference {

    return new AttributeReference(

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
/* ==== DESTACK_GENERATED_END:STRUCT:50108 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50003 ==== */
export class PropertyReference extends StructFrozen {
  readonly type: PropertyReferenceType;
  readonly nodeType: NodeType | null;
  readonly traitType: TraitType | null;
  readonly structType: StructType | null;
  readonly id: number;

  constructor(
    type: PropertyReferenceType,
    nodeType: NodeType | null,
    traitType: TraitType | null,
    structType: StructType | null,
    id: number,
    _supergraph: Supergraph
  ) {
    super(_supergraph);
    this.type = type;
    this.nodeType = nodeType;
    this.traitType = traitType;
    this.structType = structType;
    this.id = id;
  }


  static create(options: {
    type: PropertyReferenceType,
    nodeType?: NodeType | null,
    traitType?: TraitType | null,
    structType?: StructType | null,
    id: number
  }): PropertyReference {

    return new PropertyReference(

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
/* ==== DESTACK_GENERATED_END:STRUCT:50003 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50002 ==== */
export class NodeReference extends StructFrozen {
  readonly nodeType: NodeType;
  readonly id: string;
  readonly spaceId: string | null;
  readonly definitionId: string | null;

  constructor(
    nodeType: NodeType,
    id: string,
    spaceId: string | null,
    definitionId: string | null,
    _supergraph: Supergraph
  ) {
    super(_supergraph);
    this.nodeType = nodeType;
    this.id = id;
    this.spaceId = spaceId;
    this.definitionId = definitionId;
  }


  static create(options: {
    nodeType: NodeType,
    id: string,
    spaceId?: string | null,
    definitionId?: string | null
  }): NodeReference {

    return new NodeReference(

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
/* ==== DESTACK_GENERATED_END:STRUCT:50002 ==== */