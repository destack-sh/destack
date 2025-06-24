import {
  Field,
  IsSubject,
  Node,
  NodeReference,
  PropertyReference,
  Session,
  StructFrozen,
  StructType,
  Supergraph,
  Value,
} from "@destack/language/core";
import { Origin } from "@destack/language/space";
import { Temporal } from "temporal-polyfill";
import { v4 as uuid4 } from "uuid";

/* ==== DESTACK_GENERATED_START:ENUM:50050 ==== */
/**
 * EditType
 */
export enum EditType {
  CREATE = 1,
  UPSERT = 2,
  UPDATE = 3,
  MOVE = 4,
  ARCHIVE = 5,
  UNARCHIVE = 6,
  DELETE = 7,
  RESTORE = 8,
  ERASE = 9,
}
/* ==== DESTACK_GENERATED_END:ENUM:50050 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:50051 ==== */
/**
 * EditOperation
 */
export enum EditOperation {
  SET = 1,
  CLEAR = 2,
}
/* ==== DESTACK_GENERATED_END:ENUM:50051 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:50052 ==== */
/**
 * ChangeStatus
 */
export enum ChangeStatus {
  COMPLETED = 10,
  FAILED = 12,
  REJECTED = 13,
}
/* ==== DESTACK_GENERATED_END:ENUM:50052 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:50053 ==== */
/**
 * ChangeDebounce
 */
export enum ChangeDebounce {
  LAZY = 10,
}
/* ==== DESTACK_GENERATED_END:ENUM:50053 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50020 ==== */
/**
 * An Edit to a Node.
 */
export class Edit extends StructFrozen {
  static metatype: StructType = StructType.EDIT;
  static __isFrozen__: boolean = true;

  /**
   * Edit.id
   */
  readonly id: string;

  /**
   * Edit.type
   */
  readonly type: EditType;

  /**
   * Edit.operation
   */
  readonly operation: EditOperation | null;

  /**
   * node
   */
  get node(): Node | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
      if (this._supergraph === null) {
        return null;
      }
      return this._supergraph.get(nodePtr.id) as Node;
    }
    return null;
  }
  readonly nodePtr: NodeReference;

  /**
   * Edit.propPtr
   */
  readonly propPtr: PropertyReference | null;

  /**
   * field
   */
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

  /**
   * Edit.key
   */
  readonly key: Value | null;

  /**
   * Edit.value
   */
  readonly value: Value | null;

  /**
   * The inverse Edit (if it cannot be derived from the Edit itself).
   */
  readonly undo: Edit | null;

  constructor(options: {
    id?: string;
    type: EditType;
    operation?: EditOperation | null;
    node: Node | NodeReference;
    propPtr?: PropertyReference | null;
    field?: Field | NodeReference | null;
    key?: Value | null;
    value?: Value | null;
    undo?: Edit | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _value?: { [key: string]: any } | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties
    let _id = options.id ?? null;
    if (_id === null) {
      _id = uuid4();
    }
    if (_id === null) {
      throw new Error(`Edit.id is required`);
    }
    this.id = _id;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`Edit.type is required`);
    }
    this.type = _type;
    let _operation = options.operation ?? null;
    this.operation = _operation;
    let _node = options.node;
    if (_node != null && _node instanceof Node) {
      _node = _node.toRef();
    }
    if (_node === null) {
      throw new Error(`Edit.node is required`);
    }
    this.nodePtr = _node;
    let _propPtr = options.propPtr ?? null;
    this.propPtr = _propPtr;
    let _field = options.field ?? null;
    if (_field != null && _field instanceof Node) {
      _field = _field.toRef();
    }
    this.fieldPtr = _field;
    let _key = options.key ?? null;
    this.key = _key;
    let _value = options.value ?? null;
    this.value = _value;
    let _undo = options.undo ?? null;
    this.undo = _undo;

    // identity
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._value = options._value ?? null;
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

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = Edit.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Edit): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 50020;
    objectValue["2"] = String(object.id);
    objectValue["30"] = object.type;
    if (object.operation !== null) {
      objectValue["31"] = object.operation;
    }
    objectValue["32"] = object.nodePtr.toValue();
    if (object.propPtr !== null) {
      objectValue["33"] = object.propPtr.toValue();
    }
    if (object.fieldPtr !== null) {
      objectValue["34"] = object.fieldPtr.toValue();
    }
    if (object.key !== null) {
      objectValue["35"] = object.key.toValue();
    }
    if (object.value !== null) {
      objectValue["40"] = object.value.toValue();
    }
    if (object.undo !== null) {
      objectValue["50"] = object.undo.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Edit {
    const operationValue = objectValue["31"];
    const unpackedOperation = operationValue !== undefined ? Number(operationValue) : null;
    const propPtrValue = objectValue["33"];
    const unpackedPropPtr =
      propPtrValue !== undefined
        ? PropertyReference.fromValue(propPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const keyValue = objectValue["35"];
    const unpackedKey =
      keyValue !== undefined ? Value.fromValue(keyValue, _session, _supergraph, _graph, _connection) : null;
    const valueValue = objectValue["40"];
    const unpackedValue =
      valueValue !== undefined ? Value.fromValue(valueValue, _session, _supergraph, _graph, _connection) : null;
    const undoValue = objectValue["50"];
    const unpackedUndo =
      undoValue !== undefined ? Edit.fromValue(undoValue, _session, _supergraph, _graph, _connection) : null;
    const fieldValue = objectValue["34"];
    const unpackedField =
      fieldValue !== undefined ? NodeReference.fromValue(fieldValue, _session, _supergraph, _graph, _connection) : null;
    return new Edit({
      id: String(objectValue["2"]),
      type: Number(objectValue["30"]),
      operation: unpackedOperation,
      propPtr: unpackedPropPtr,
      key: unpackedKey,
      value: unpackedValue,
      undo: unpackedUndo,
      node: NodeReference.fromValue(objectValue["32"], _session, _supergraph, _graph, _connection),
      field: unpackedField,
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Edit {
    return Edit.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:50020 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50021 ==== */
/**
 * A Change is an atomic sequence of Edits.
 */
export class Change extends StructFrozen {
  static metatype: StructType = StructType.CHANGE;
  static __isFrozen__: boolean = true;

  /**
   * Change.id
   */
  readonly id: string;

  /**
   * Change.name
   */
  readonly name: string | null;

  /**
   * Change.createdAt
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * created_by
   */
  get createdBy(): (Node & IsSubject) | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr !== null) {
      if (this._supergraph === null) {
        return null;
      }
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;

  /**
   * Change.origin
   */
  readonly origin: Origin | null;

  /**
   * Change.debounce
   */
  readonly debounce: ChangeDebounce | null;

  /**
   * Change.edits
   */
  readonly edits: Array<Edit>;

  constructor(options: {
    id?: string;
    name?: string | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    origin?: Origin | null;
    debounce?: ChangeDebounce | null;
    edits?: Array<Edit>;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _value?: { [key: string]: any } | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties
    let _id = options.id ?? null;
    if (_id === null) {
      _id = uuid4();
    }
    if (_id === null) {
      throw new Error(`Change.id is required`);
    }
    this.id = _id;
    let _name = options.name ?? null;
    this.name = _name;
    let _createdAt = options.createdAt ?? null;
    if (_createdAt === null) {
      _createdAt = Temporal.Now.zonedDateTimeISO();
    }
    if (_createdAt === null) {
      throw new Error(`Change.createdAt is required`);
    }
    this.createdAt = _createdAt;
    let _createdBy = options.createdBy ?? null;
    if (_createdBy != null && _createdBy instanceof Node) {
      _createdBy = _createdBy.toRef();
    }
    this.createdByPtr = _createdBy;
    let _origin = options.origin ?? null;
    this.origin = _origin;
    let _debounce = options.debounce ?? null;
    this.debounce = _debounce;
    let _edits = options.edits ?? null;
    if (_edits === null) {
      throw new Error(`Change.edits is required`);
    }
    this.edits = _edits;

    // identity
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._value = options._value ?? null;
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

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = Change.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Change): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 50021;
    objectValue["2"] = String(object.id);
    if (object.name !== null) {
      objectValue["31"] = object.name;
    }
    objectValue["32"] = object.createdAt.toString();
    if (object.createdByPtr !== null) {
      objectValue["33"] = object.createdByPtr.toValue();
    }
    if (object.origin !== null) {
      objectValue["34"] = object.origin.toValue();
    }
    if (object.debounce !== null) {
      objectValue["35"] = object.debounce;
    }
    if (object.edits) {
      const packedEdits: any[] = [];
      for (const item of object.edits) {
        packedEdits.push(item.toValue());
      }
      objectValue["40"] = packedEdits;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Change {
    const nameValue = objectValue["31"];
    const unpackedName = nameValue !== undefined ? nameValue : null;
    const originValue = objectValue["34"];
    const unpackedOrigin =
      originValue !== undefined ? Origin.fromValue(originValue, _session, _supergraph, _graph, _connection) : null;
    const debounceValue = objectValue["35"];
    const unpackedDebounce = debounceValue !== undefined ? Number(debounceValue) : null;
    const unpackedEdits: any[] = [];
    if (objectValue["40"] !== undefined) {
      for (const item of objectValue["40"]) {
        unpackedEdits.push(Edit.fromValue(item, _session, _supergraph, _graph, _connection));
      }
    }
    const createdByValue = objectValue["33"];
    const unpackedCreatedBy =
      createdByValue !== undefined
        ? NodeReference.fromValue(createdByValue, _session, _supergraph, _graph, _connection)
        : null;
    return new Change({
      id: String(objectValue["2"]),
      name: unpackedName,
      createdAt: Temporal.ZonedDateTime.from(objectValue["32"]),
      origin: unpackedOrigin,
      debounce: unpackedDebounce,
      edits: unpackedEdits,
      createdBy: unpackedCreatedBy,
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Change {
    return Change.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:50021 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50022 ==== */
/**
 * The result of a Change. If rejected, edits/cascaded_edits are empty.
 */
export class ChangeResult extends StructFrozen {
  static metatype: StructType = StructType.CHANGE_RESULT;
  static __isFrozen__: boolean = true;

  /**
   * The id of the Change.
   */
  readonly id: string;

  /**
   * The time the ChangeResult was created.
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * ChangeResult.debounce
   */
  readonly debounce: ChangeDebounce | null;

  /**
   * ChangeResult.status
   */
  readonly status: ChangeStatus;

  /**
   * The applied Edits (may differ).
   */
  readonly edits: Array<Edit>;

  /**
   * The Edits cascaded from the applied Edits.
   */
  readonly cascadedEdits: Array<Edit>;

  constructor(options: {
    id?: string;
    createdAt?: Temporal.ZonedDateTime;
    debounce?: ChangeDebounce | null;
    status: ChangeStatus;
    edits?: Array<Edit>;
    cascadedEdits?: Array<Edit>;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _value?: { [key: string]: any } | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties
    let _id = options.id ?? null;
    if (_id === null) {
      _id = uuid4();
    }
    if (_id === null) {
      throw new Error(`ChangeResult.id is required`);
    }
    this.id = _id;
    let _createdAt = options.createdAt ?? null;
    if (_createdAt === null) {
      _createdAt = Temporal.Now.zonedDateTimeISO();
    }
    if (_createdAt === null) {
      throw new Error(`ChangeResult.createdAt is required`);
    }
    this.createdAt = _createdAt;
    let _debounce = options.debounce ?? null;
    this.debounce = _debounce;
    let _status = options.status;
    if (_status === null) {
      throw new Error(`ChangeResult.status is required`);
    }
    this.status = _status;
    let _edits = options.edits ?? null;
    if (_edits === null) {
      throw new Error(`ChangeResult.edits is required`);
    }
    this.edits = _edits;
    let _cascadedEdits = options.cascadedEdits ?? null;
    if (_cascadedEdits === null) {
      throw new Error(`ChangeResult.cascadedEdits is required`);
    }
    this.cascadedEdits = _cascadedEdits;

    // identity
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._value = options._value ?? null;
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

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = ChangeResult.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: ChangeResult): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 50022;
    objectValue["2"] = String(object.id);
    objectValue["10"] = object.createdAt.toString();
    if (object.debounce !== null) {
      objectValue["35"] = object.debounce;
    }
    objectValue["40"] = object.status;
    if (object.edits) {
      const packedEdits: any[] = [];
      for (const item of object.edits) {
        packedEdits.push(item.toValue());
      }
      objectValue["41"] = packedEdits;
    }
    if (object.cascadedEdits) {
      const packedCascadedEdits: any[] = [];
      for (const item of object.cascadedEdits) {
        packedCascadedEdits.push(item.toValue());
      }
      objectValue["42"] = packedCascadedEdits;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ChangeResult {
    const debounceValue = objectValue["35"];
    const unpackedDebounce = debounceValue !== undefined ? Number(debounceValue) : null;
    const unpackedEdits: any[] = [];
    if (objectValue["41"] !== undefined) {
      for (const item of objectValue["41"]) {
        unpackedEdits.push(Edit.fromValue(item, _session, _supergraph, _graph, _connection));
      }
    }
    const unpackedCascadedEdits: any[] = [];
    if (objectValue["42"] !== undefined) {
      for (const item of objectValue["42"]) {
        unpackedCascadedEdits.push(Edit.fromValue(item, _session, _supergraph, _graph, _connection));
      }
    }
    return new ChangeResult({
      id: String(objectValue["2"]),
      createdAt: Temporal.ZonedDateTime.from(objectValue["10"]),
      debounce: unpackedDebounce,
      status: Number(objectValue["40"]),
      edits: unpackedEdits,
      cascadedEdits: unpackedCascadedEdits,
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ChangeResult {
    return ChangeResult.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:50022 ==== */
