import {
  CustomEntityDefinition,
  DefaultFactory,
  EnumType,
  Node,
  NodeReference,
  NodeType,
  PrimitiveType,
  ScalarType,
  Session,
  StructFrozen,
  StructType,
  Supergraph,
  TraitType,
  TypeCardinality,
  Value,
} from "@destack/language/core";

/* ==== DESTACK_GENERATED_START:ENUM:2570 ==== */
/**
 * StringFormat
 */
export enum StringFormat {
  NAME = 1,
  SLUG = 2,
  EMAIL = 3,
  UUID = 10,
  URL = 11,
  EMOJI = 12,
  MIME = 13,
  BASE64 = 20,
}
/* ==== DESTACK_GENERATED_END:ENUM:2570 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:2571 ==== */
/**
 * NumberFormat
 */
export enum NumberFormat {
  PERCENTAGE = 1,
  ANGLE = 2,
  CURRENCY = 3,
}
/* ==== DESTACK_GENERATED_END:ENUM:2571 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2503 ==== */
/**
 * The constraint of a string.
 */
export class StringConstraint extends StructFrozen {
  static metatype: StructType = StructType.STRING_CONSTRAINT;
  static __isFrozen__: boolean = true;

  /**
   * StringConstraint.format
   */
  readonly format: StringFormat | null;

  /**
   * StringConstraint.regex
   */
  readonly regex: string | null;

  /**
   * StringConstraint.startsWith
   */
  readonly startsWith: string | null;

  /**
   * StringConstraint.endsWith
   */
  readonly endsWith: string | null;

  constructor(options: {
    format?: StringFormat | null;
    regex?: string | null;
    startsWith?: string | null;
    endsWith?: string | null;
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
    let _format = options.format ?? null;
    this.format = _format;
    let _regex = options.regex ?? null;
    this.regex = _regex;
    let _startsWith = options.startsWith ?? null;
    this.startsWith = _startsWith;
    let _endsWith = options.endsWith ?? null;
    this.endsWith = _endsWith;

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
/* ==== DESTACK_GENERATED_END:STRUCT:2503 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2502 ==== */
/**
 * The constraint of a number.
 */
export class NumberConstraint extends StructFrozen {
  static metatype: StructType = StructType.NUMBER_CONSTRAINT;
  static __isFrozen__: boolean = true;

  /**
   * NumberConstraint.format
   */
  readonly format: NumberFormat | null;

  /**
   * NumberConstraint.minValue
   */
  readonly minValue: number | null;

  /**
   * NumberConstraint.maxValue
   */
  readonly maxValue: number | null;

  /**
   * NumberConstraint.stepValue
   */
  readonly stepValue: number | null;

  /**
   * NumberConstraint.precision
   */
  readonly precision: number | null;

  /**
   * NumberConstraint.scale
   */
  readonly scale: number | null;

  constructor(options: {
    format?: NumberFormat | null;
    minValue?: number | null;
    maxValue?: number | null;
    stepValue?: number | null;
    precision?: number | null;
    scale?: number | null;
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
    let _format = options.format ?? null;
    this.format = _format;
    let _minValue = options.minValue ?? null;
    this.minValue = _minValue;
    let _maxValue = options.maxValue ?? null;
    this.maxValue = _maxValue;
    let _stepValue = options.stepValue ?? null;
    this.stepValue = _stepValue;
    let _precision = options.precision ?? null;
    this.precision = _precision;
    let _scale = options.scale ?? null;
    this.scale = _scale;

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
/* ==== DESTACK_GENERATED_END:STRUCT:2502 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2504 ==== */
/**
 * The constraint of a collection.
 */
export class CollectionConstraint extends StructFrozen {
  static metatype: StructType = StructType.COLLECTION_CONSTRAINT;
  static __isFrozen__: boolean = true;

  /**
   * CollectionConstraint.minLength
   */
  readonly minLength: number | null;

  /**
   * CollectionConstraint.maxLength
   */
  readonly maxLength: number | null;

  constructor(options: {
    minLength?: number | null;
    maxLength?: number | null;
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
    let _minLength = options.minLength ?? null;
    this.minLength = _minLength;
    let _maxLength = options.maxLength ?? null;
    this.maxLength = _maxLength;

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
/* ==== DESTACK_GENERATED_END:STRUCT:2504 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2505 ==== */
/**
 * The constraint of a node.
 */
export class NodeConstraint extends StructFrozen {
  static metatype: StructType = StructType.NODE_CONSTRAINT;
  static __isFrozen__: boolean = true;

  /**
   * NodeConstraint.nodeTypes
   */
  readonly nodeTypes: Array<NodeType>;

  /**
   * NodeConstraint.nodeTraits
   */
  readonly nodeTraits: Array<TraitType>;

  constructor(options: {
    nodeTypes?: Array<NodeType>;
    nodeTraits?: Array<TraitType>;
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
    let _nodeTypes = options.nodeTypes ?? null;
    if (_nodeTypes === null) {
      throw new Error(`NodeConstraint.nodeTypes is required`);
    }
    this.nodeTypes = _nodeTypes;
    let _nodeTraits = options.nodeTraits ?? null;
    if (_nodeTraits === null) {
      throw new Error(`NodeConstraint.nodeTraits is required`);
    }
    this.nodeTraits = _nodeTraits;

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
/* ==== DESTACK_GENERATED_END:STRUCT:2505 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2501 ==== */
/**
 * A Type in the type system.
 */
export class Type extends StructFrozen {
  static metatype: StructType = StructType.TYPE;
  static __isFrozen__: boolean = true;

  /**
   * Type.cardinality
   */
  readonly cardinality: TypeCardinality;

  /**
   * Type.scalarType
   */
  readonly scalarType: ScalarType;

  /**
   * Type.primitiveType
   */
  readonly primitiveType: PrimitiveType | null;

  /**
   * Type.enumType
   */
  readonly enumType: EnumType | null;

  /**
   * Type.nodeType
   */
  readonly nodeType: NodeType | null;

  /**
   * node_definition
   */
  get nodeDefinition(): CustomEntityDefinition | null {
    const nodePtr: NodeReference | null = this.nodeDefinitionPtr;
    if (nodePtr !== null) {
      if (this._supergraph === null) {
        return null;
      }
      return this._supergraph.get(nodePtr.id) as CustomEntityDefinition | null;
    }
    return null;
  }
  readonly nodeDefinitionPtr: NodeReference | null;

  /**
   * Type.structType
   */
  readonly structType: StructType | null;

  /**
   * base_type
   */
  get baseType(): Node | null {
    const nodePtr: NodeReference | null = this.baseTypePtr;
    if (nodePtr !== null) {
      if (this._supergraph === null) {
        return null;
      }
      return this._supergraph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  readonly baseTypePtr: NodeReference | null;

  /**
   * Type.keyType
   */
  readonly keyType: Type | null;

  /**
   * Type.isRequired
   */
  readonly isRequired: boolean | null;

  /**
   * Type.isVariable
   */
  readonly isVariable: boolean | null;

  /**
   * Type.defaultValue
   */
  readonly defaultValue: Value | null;

  /**
   * Type.defaultFactory
   */
  readonly defaultFactory: DefaultFactory | null;

  /**
   * Type.collectionConstraint
   */
  readonly collectionConstraint: CollectionConstraint | null;

  /**
   * Type.stringConstraint
   */
  readonly stringConstraint: StringConstraint | null;

  /**
   * Type.numberConstraint
   */
  readonly numberConstraint: NumberConstraint | null;

  /**
   * Type.nodeConstraint
   */
  readonly nodeConstraint: NodeConstraint | null;

  constructor(options: {
    cardinality?: TypeCardinality;
    scalarType: ScalarType;
    primitiveType?: PrimitiveType | null;
    enumType?: EnumType | null;
    nodeType?: NodeType | null;
    nodeDefinition?: CustomEntityDefinition | NodeReference | null;
    structType?: StructType | null;
    baseType?: Node | NodeReference | null;
    keyType?: Type | null;
    isRequired?: boolean | null;
    isVariable?: boolean | null;
    defaultValue?: Value | null;
    defaultFactory?: DefaultFactory | null;
    collectionConstraint?: CollectionConstraint | null;
    stringConstraint?: StringConstraint | null;
    numberConstraint?: NumberConstraint | null;
    nodeConstraint?: NodeConstraint | null;
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
    let _cardinality = options.cardinality ?? null;
    if (_cardinality === null) {
      _cardinality = TypeCardinality.SCALAR;
    }
    if (_cardinality === null) {
      throw new Error(`Type.cardinality is required`);
    }
    this.cardinality = _cardinality;
    let _scalarType = options.scalarType;
    if (_scalarType === null) {
      throw new Error(`Type.scalarType is required`);
    }
    this.scalarType = _scalarType;
    let _primitiveType = options.primitiveType ?? null;
    this.primitiveType = _primitiveType;
    let _enumType = options.enumType ?? null;
    this.enumType = _enumType;
    let _nodeType = options.nodeType ?? null;
    this.nodeType = _nodeType;
    let _nodeDefinition = options.nodeDefinition ?? null;
    if (_nodeDefinition != null && _nodeDefinition instanceof Node) {
      _nodeDefinition = _nodeDefinition.toRef();
    }
    this.nodeDefinitionPtr = _nodeDefinition;
    let _structType = options.structType ?? null;
    this.structType = _structType;
    let _baseType = options.baseType ?? null;
    if (_baseType != null && _baseType instanceof Node) {
      _baseType = _baseType.toRef();
    }
    this.baseTypePtr = _baseType;
    let _keyType = options.keyType ?? null;
    this.keyType = _keyType;
    let _isRequired = options.isRequired ?? null;
    this.isRequired = _isRequired;
    let _isVariable = options.isVariable ?? null;
    this.isVariable = _isVariable;
    let _defaultValue = options.defaultValue ?? null;
    this.defaultValue = _defaultValue;
    let _defaultFactory = options.defaultFactory ?? null;
    this.defaultFactory = _defaultFactory;
    let _collectionConstraint = options.collectionConstraint ?? null;
    this.collectionConstraint = _collectionConstraint;
    let _stringConstraint = options.stringConstraint ?? null;
    this.stringConstraint = _stringConstraint;
    let _numberConstraint = options.numberConstraint ?? null;
    this.numberConstraint = _numberConstraint;
    let _nodeConstraint = options.nodeConstraint ?? null;
    this.nodeConstraint = _nodeConstraint;

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
/* ==== DESTACK_GENERATED_END:STRUCT:2501 ==== */
