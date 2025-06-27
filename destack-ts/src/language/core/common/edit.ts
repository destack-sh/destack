import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import { NodeReference, Session, Supergraph } from "@destack/language/core";
import {
  EnumType,
  IsSubject,
  Node,
  StructFrozen,
  StructType,
} from "@destack/language/core/builtin";
import { CustomProperty, PropertyReference, Value } from "@destack/language/core/common";
import { registerEnumClass, registerStructClass } from "@destack/language/registry";
import { Origin } from "@destack/language/space";
import {
  ChangeDebounceProto,
  ChangeProto,
  ChangeResultProto,
  ChangeStatusProto,
  EditOperationProto,
  EditProto,
  EditTypeProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashString } from "@destack/utils/hash";
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

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.EDIT_TYPE, EditType);
/* ==== DESTACK_GENERATED_END:ENUM:50050 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:50051 ==== */
/**
 * EditOperation
 */
export enum EditOperation {
  SET = 1,
  CLEAR = 2,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.EDIT_OPERATION, EditOperation);
/* ==== DESTACK_GENERATED_END:ENUM:50051 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:50052 ==== */
/**
 * ChangeStatus
 */
export enum ChangeStatus {
  COMPLETED = 10,
  FAILED = 12,
  REJECTED = 13,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.CHANGE_STATUS, ChangeStatus);
/* ==== DESTACK_GENERATED_END:ENUM:50052 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:50053 ==== */
/**
 * ChangeDebounce
 */
export enum ChangeDebounce {
  LAZY = 10,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.CHANGE_DEBOUNCE, ChangeDebounce);
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
  get field(): CustomProperty | null {
    const nodePtr: NodeReference | null = this.fieldPtr;
    if (nodePtr !== null) {
      if (this._supergraph === null) {
        return null;
      }
      return this._supergraph.get(nodePtr.id) as CustomProperty | null;
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
    field?: CustomProperty | NodeReference | null;
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
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.id === other.id)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (!(this.operation === other.operation)) {
      return false;
    }
    if (!(this.nodePtr.id === other.nodePtr.id)) {
      return false;
    }
    if (
      (this.propPtr == null) !== (other.propPtr == null) ||
      (this.propPtr != null && !this.propPtr.equals(other.propPtr))
    ) {
      return false;
    }
    if (!(this.fieldPtr?.id === other.fieldPtr?.id)) {
      return false;
    }
    if (
      (this.key == null) !== (other.key == null) ||
      (this.key != null && !this.key.equals(other.key))
    ) {
      return false;
    }
    if (
      (this.value == null) !== (other.value == null) ||
      (this.value != null && !this.value.equals(other.value))
    ) {
      return false;
    }
    if (
      (this.undo == null) !== (other.undo == null) ||
      (this.undo != null && !this.undo.equals(other.undo))
    ) {
      return false;
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`id=${this.id}`);
      propertyReprs.push(`type=${EditType[this.type]}`);
      if (this.operation !== null) {
        propertyReprs.push(`operation=${EditOperation[this.operation]}`);
      }
      propertyReprs.push(`node=${this.node.repr()}`);
      if (this.propPtr !== null) {
        propertyReprs.push(`propPtr=${this.propPtr.repr()}`);
      }
      if (this.field !== null) {
        propertyReprs.push(`field=${this.field.repr()}`);
      }
      if (this.key !== null) {
        propertyReprs.push(`key=${this.key.repr()}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<Edit ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    if (this.operation !== null) {
      h = (h * 31 + this.operation) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    if (this.propPtr !== null) {
      h = (h * 31 + this.propPtr.hash()) & 0xffffffff;
    }
    if (this.fieldPtr !== null) {
      h = (h * 31 + hashString(this.fieldPtr.id)) & 0xffffffff;
    }
    if (this.key !== null) {
      h = (h * 31 + this.key.hash()) & 0xffffffff;
    }
    if (this.value !== null) {
      h = (h * 31 + this.value.hash()) & 0xffffffff;
    }
    if (this.undo !== null) {
      h = (h * 31 + this.undo.hash()) & 0xffffffff;
    }
    return h;

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
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
    if (object.operation != null) {
      objectValue["31"] = object.operation;
    }
    objectValue["32"] = object.nodePtr.toValue();
    if (object.propPtr != null) {
      objectValue["33"] = object.propPtr.toValue();
    }
    if (object.fieldPtr != null) {
      objectValue["34"] = object.fieldPtr.toValue();
    }
    if (object.key != null) {
      objectValue["35"] = object.key.toValue();
    }
    if (object.value != null) {
      objectValue["40"] = object.value.toValue();
    }
    if (object.undo != null) {
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
    const unpackedOperation = operationValue != undefined ? Number(operationValue) : null;
    const propPtrValue = objectValue["33"];
    const unpackedPropPtr =
      propPtrValue != undefined
        ? PropertyReference.fromValue(propPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const fieldPtrValue = objectValue["34"];
    const unpackedFieldPtr =
      fieldPtrValue != undefined
        ? NodeReference.fromValue(fieldPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const keyValue = objectValue["35"];
    const unpackedKey =
      keyValue != undefined
        ? Value.fromValue(keyValue, _session, _supergraph, _graph, _connection)
        : null;
    const valueValue = objectValue["40"];
    const unpackedValue =
      valueValue != undefined
        ? Value.fromValue(valueValue, _session, _supergraph, _graph, _connection)
        : null;
    const undoValue = objectValue["50"];
    const unpackedUndo =
      undoValue != undefined
        ? Edit.fromValue(undoValue, _session, _supergraph, _graph, _connection)
        : null;
    return new Edit({
      id: String(objectValue["2"]),
      type: Number(objectValue["30"]),
      operation: unpackedOperation,
      node: NodeReference.fromValue(objectValue["32"], _session, _supergraph, _graph, _connection),
      propPtr: unpackedPropPtr,
      field: unpackedFieldPtr,
      key: unpackedKey,
      value: unpackedValue,
      undo: unpackedUndo,
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

  toProto(): EditProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Edit.__packProto__(this);
    }
    return this._proto as EditProto;
  }

  static __packProto__(object: Edit): EditProto {
    const objectProto: Partial<EditProto> = { metatype: 50020 };
    objectProto.id = String(object.id);
    objectProto.type = Number(object.type) as EditTypeProto;
    if (object.operation != null) {
      objectProto.operation = Number(object.operation) as EditOperationProto;
    }
    objectProto.nodePtr = object.nodePtr.toProto();
    if (object.propPtr != null) {
      objectProto.propPtr = object.propPtr.toProto();
    }
    if (object.fieldPtr != null) {
      objectProto.fieldPtr = object.fieldPtr.toProto();
    }
    if (object.key != null) {
      objectProto.key = object.key.toProto();
    }
    if (object.value != null) {
      objectProto.value = object.value.toProto();
    }
    if (object.undo != null) {
      objectProto.undo = object.undo.toProto();
    }
    return objectProto as EditProto;
  }

  static __unpackProto__(
    objectProto: EditProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Edit {
    return new Edit({
      id: String(objectProto.id),
      type: Number(objectProto.type) as EditType,
      operation:
        objectProto.operation != undefined
          ? (Number(objectProto.operation) as EditOperation)
          : null,
      node: NodeReference.fromProto(
        objectProto.nodePtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      propPtr:
        objectProto.propPtr != undefined
          ? PropertyReference.fromProto(
              objectProto.propPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      field:
        objectProto.fieldPtr != undefined
          ? NodeReference.fromProto(
              objectProto.fieldPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      key:
        objectProto.key != undefined
          ? Value.fromProto(objectProto.key!, _session, _supergraph, _graph, _connection)
          : null,
      value:
        objectProto.value != undefined
          ? Value.fromProto(objectProto.value!, _session, _supergraph, _graph, _connection)
          : null,
      undo:
        objectProto.undo != undefined
          ? Edit.fromProto(objectProto.undo!, _session, _supergraph, _graph, _connection)
          : null,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: EditProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Edit {
    return Edit.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Edit {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = EditProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.EDIT, Edit);
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
      _edits = [];
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
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.id === other.id)) {
      return false;
    }
    if (!(this.name === other.name)) {
      return false;
    }
    if (!(this.createdAt === other.createdAt)) {
      return false;
    }
    if (!(this.createdByPtr?.id === other.createdByPtr?.id)) {
      return false;
    }
    if (
      (this.origin == null) !== (other.origin == null) ||
      (this.origin != null && !this.origin.equals(other.origin))
    ) {
      return false;
    }
    if (!(this.debounce === other.debounce)) {
      return false;
    }
    if (this.edits.length !== other.edits.length) {
      return false;
    }
    for (let i = 0; i < this.edits.length; i++) {
      if (!this.edits[i].equals(other.edits[i])) {
        return false;
      }
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`id=${this.id}`);
      if (this.name !== null) {
        propertyReprs.push(`name=${this.name}`);
      }
      propertyReprs.push(`createdAt=${this.createdAt.toString()}`);
      if (this.createdBy !== null) {
        propertyReprs.push(`createdBy=${this.createdBy.repr()}`);
      }
      if (this.origin !== null) {
        propertyReprs.push(`origin=${this.origin.repr()}`);
      }
      if (this.debounce !== null) {
        propertyReprs.push(`debounce=${ChangeDebounce[this.debounce]}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<Change ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    if (this.name !== null) {
      h = (h * 31 + hashString(this.name)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.createdByPtr !== null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    if (this.origin !== null) {
      h = (h * 31 + this.origin.hash()) & 0xffffffff;
    }
    if (this.debounce !== null) {
      h = (h * 31 + this.debounce) & 0xffffffff;
    }
    if (this.edits && this.edits.length > 0) {
      for (const _item of this.edits) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    return h;

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
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
    if (object.name != null) {
      objectValue["31"] = object.name;
    }
    objectValue["32"] = object.createdAt.toString();
    if (object.createdByPtr != null) {
      objectValue["33"] = object.createdByPtr.toValue();
    }
    if (object.origin != null) {
      objectValue["34"] = object.origin.toValue();
    }
    if (object.debounce != null) {
      objectValue["35"] = object.debounce;
    }
    if (object.edits.length > 0) {
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
    const unpackedName = nameValue != undefined ? nameValue : null;
    const createdByPtrValue = objectValue["33"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? NodeReference.fromValue(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const originValue = objectValue["34"];
    const unpackedOrigin =
      originValue != undefined
        ? Origin.fromValue(originValue, _session, _supergraph, _graph, _connection)
        : null;
    const debounceValue = objectValue["35"];
    const unpackedDebounce = debounceValue != undefined ? Number(debounceValue) : null;
    const unpackedEdits: any[] = [];
    if (objectValue["40"] != undefined) {
      for (const item of objectValue["40"]) {
        unpackedEdits.push(Edit.fromValue(item, _session, _supergraph, _graph, _connection));
      }
    }
    return new Change({
      id: String(objectValue["2"]),
      name: unpackedName,
      createdAt: Temporal.ZonedDateTime.from(objectValue["32"]),
      createdBy: unpackedCreatedByPtr,
      origin: unpackedOrigin,
      debounce: unpackedDebounce,
      edits: unpackedEdits,
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

  toProto(): ChangeProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Change.__packProto__(this);
    }
    return this._proto as ChangeProto;
  }

  static __packProto__(object: Change): ChangeProto {
    const objectProto: Partial<ChangeProto> = { metatype: 50021 };
    objectProto.id = String(object.id);
    if (object.name != null) {
      objectProto.name = object.name;
    }
    objectProto.createdAt = packProtoTimestamp(object.createdAt);
    if (object.createdByPtr != null) {
      objectProto.createdByPtr = object.createdByPtr.toProto();
    }
    if (object.origin != null) {
      objectProto.origin = object.origin.toProto();
    }
    if (object.debounce != null) {
      objectProto.debounce = Number(object.debounce) as ChangeDebounceProto;
    }
    if (object.edits) {
      const packedEdits: any[] = [];
      for (const item of object.edits) {
        packedEdits.push(item.toProto());
      }
      objectProto.edits = packedEdits;
    }
    return objectProto as ChangeProto;
  }

  static __unpackProto__(
    objectProto: ChangeProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Change {
    const unpackedEdits: any[] = [];
    if (objectProto.edits) {
      for (const item of objectProto.edits) {
        unpackedEdits.push(Edit.fromProto(item!, _session, _supergraph, _graph, _connection));
      }
    }
    return new Change({
      id: String(objectProto.id),
      name: objectProto.name != undefined ? objectProto.name : null,
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
      origin:
        objectProto.origin != undefined
          ? Origin.fromProto(objectProto.origin!, _session, _supergraph, _graph, _connection)
          : null,
      debounce:
        objectProto.debounce != undefined ? (Number(objectProto.debounce) as ChangeDebounce) : null,
      edits: unpackedEdits,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: ChangeProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Change {
    return Change.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Change {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = ChangeProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.CHANGE, Change);
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
      _edits = [];
    }
    this.edits = _edits;
    let _cascadedEdits = options.cascadedEdits ?? null;
    if (_cascadedEdits === null) {
      _cascadedEdits = [];
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
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.id === other.id)) {
      return false;
    }
    if (!(this.createdAt === other.createdAt)) {
      return false;
    }
    if (!(this.debounce === other.debounce)) {
      return false;
    }
    if (!(this.status === other.status)) {
      return false;
    }
    if (this.edits.length !== other.edits.length) {
      return false;
    }
    for (let i = 0; i < this.edits.length; i++) {
      if (!this.edits[i].equals(other.edits[i])) {
        return false;
      }
    }
    if (this.cascadedEdits.length !== other.cascadedEdits.length) {
      return false;
    }
    for (let i = 0; i < this.cascadedEdits.length; i++) {
      if (!this.cascadedEdits[i].equals(other.cascadedEdits[i])) {
        return false;
      }
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`id=${this.id}`);
      propertyReprs.push(`createdAt=${this.createdAt.toString()}`);
      if (this.debounce !== null) {
        propertyReprs.push(`debounce=${ChangeDebounce[this.debounce]}`);
      }
      propertyReprs.push(`status=${ChangeStatus[this.status]}`);
      // @ts-expect-error(readonly)
      this._repr = `<ChangeResult ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.debounce !== null) {
      h = (h * 31 + this.debounce) & 0xffffffff;
    }
    h = (h * 31 + this.status) & 0xffffffff;
    if (this.edits && this.edits.length > 0) {
      for (const _item of this.edits) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    if (this.cascadedEdits && this.cascadedEdits.length > 0) {
      for (const _item of this.cascadedEdits) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    return h;

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
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
    if (object.debounce != null) {
      objectValue["35"] = object.debounce;
    }
    objectValue["40"] = object.status;
    if (object.edits.length > 0) {
      const packedEdits: any[] = [];
      for (const item of object.edits) {
        packedEdits.push(item.toValue());
      }
      objectValue["41"] = packedEdits;
    }
    if (object.cascadedEdits.length > 0) {
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
    const unpackedDebounce = debounceValue != undefined ? Number(debounceValue) : null;
    const unpackedEdits: any[] = [];
    if (objectValue["41"] != undefined) {
      for (const item of objectValue["41"]) {
        unpackedEdits.push(Edit.fromValue(item, _session, _supergraph, _graph, _connection));
      }
    }
    const unpackedCascadedEdits: any[] = [];
    if (objectValue["42"] != undefined) {
      for (const item of objectValue["42"]) {
        unpackedCascadedEdits.push(
          Edit.fromValue(item, _session, _supergraph, _graph, _connection),
        );
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

  toProto(): ChangeResultProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = ChangeResult.__packProto__(this);
    }
    return this._proto as ChangeResultProto;
  }

  static __packProto__(object: ChangeResult): ChangeResultProto {
    const objectProto: Partial<ChangeResultProto> = { metatype: 50022 };
    objectProto.id = String(object.id);
    objectProto.createdAt = packProtoTimestamp(object.createdAt);
    if (object.debounce != null) {
      objectProto.debounce = Number(object.debounce) as ChangeDebounceProto;
    }
    objectProto.status = Number(object.status) as ChangeStatusProto;
    if (object.edits) {
      const packedEdits: any[] = [];
      for (const item of object.edits) {
        packedEdits.push(item.toProto());
      }
      objectProto.edits = packedEdits;
    }
    if (object.cascadedEdits) {
      const packedCascadedEdits: any[] = [];
      for (const item of object.cascadedEdits) {
        packedCascadedEdits.push(item.toProto());
      }
      objectProto.cascadedEdits = packedCascadedEdits;
    }
    return objectProto as ChangeResultProto;
  }

  static __unpackProto__(
    objectProto: ChangeResultProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ChangeResult {
    const unpackedEdits: any[] = [];
    if (objectProto.edits) {
      for (const item of objectProto.edits) {
        unpackedEdits.push(Edit.fromProto(item!, _session, _supergraph, _graph, _connection));
      }
    }
    const unpackedCascadedEdits: any[] = [];
    if (objectProto.cascadedEdits) {
      for (const item of objectProto.cascadedEdits) {
        unpackedCascadedEdits.push(
          Edit.fromProto(item!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new ChangeResult({
      id: String(objectProto.id),
      createdAt: unpackProtoTimestamp(objectProto.createdAt!),
      debounce:
        objectProto.debounce != undefined ? (Number(objectProto.debounce) as ChangeDebounce) : null,
      status: Number(objectProto.status) as ChangeStatus,
      edits: unpackedEdits,
      cascadedEdits: unpackedCascadedEdits,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: ChangeResultProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ChangeResult {
    return ChangeResult.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): ChangeResult {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = ChangeResultProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.CHANGE_RESULT, ChangeResult);
/* ==== DESTACK_GENERATED_END:STRUCT:50022 ==== */
