import {
  Agent,
  Field,
  Node,
  NodeReference,
  Origin,
  PropertyReference,
  Session,
  StructFrozen,
  StructType,
  Supergraph,
  User,
  Value,
} from "@/language";
import { Temporal } from "temporal-polyfill";
import { v4 as uuid4 } from "uuid";

/* ==== DESTACK_GENERATED_START:ENUM:50050 ==== */
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
export enum EditOperation {
  SET = 1,
  CLEAR = 2,
}
/* ==== DESTACK_GENERATED_END:ENUM:50051 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:50052 ==== */
export enum ChangeStatus {
  COMPLETED = 10,
  FAILED = 12,
  REJECTED = 13,
}
/* ==== DESTACK_GENERATED_END:ENUM:50052 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:50053 ==== */
export enum ChangeDebounce {
  LAZY = 10,
}
/* ==== DESTACK_GENERATED_END:ENUM:50053 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50020 ==== */
export class Edit extends StructFrozen {
  static metatype: StructType = StructType.EDIT;
  static __isFrozen__: boolean = true;

  readonly id: string;
  readonly type: EditType;
  readonly operation: EditOperation | null;
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
  readonly key: Value | null;
  readonly value: Value | null;
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
/* ==== DESTACK_GENERATED_END:STRUCT:50020 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50021 ==== */
export class Change extends StructFrozen {
  static metatype: StructType = StructType.CHANGE;
  static __isFrozen__: boolean = true;

  readonly id: string;
  readonly name: string | null;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): Agent | User | null | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr !== null) {
      if (this._supergraph === null) {
        return null;
      }
      return this._supergraph.get(nodePtr.id) as Agent | User | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;
  readonly origin: Origin | null;
  readonly debounce: ChangeDebounce | null;
  readonly edits: Array<Edit>;

  constructor(options: {
    id?: string;
    name?: string | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: Agent | User | NodeReference | null;
    origin?: Origin | null;
    debounce?: ChangeDebounce | null;
    edits?: Array<Edit>;
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
/* ==== DESTACK_GENERATED_END:STRUCT:50021 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50022 ==== */
export class ChangeResult extends StructFrozen {
  static metatype: StructType = StructType.CHANGE_RESULT;
  static __isFrozen__: boolean = true;

  readonly id: string;
  readonly createdAt: Temporal.ZonedDateTime;
  readonly debounce: ChangeDebounce | null;
  readonly status: ChangeStatus;
  readonly edits: Array<Edit>;
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
/* ==== DESTACK_GENERATED_END:STRUCT:50022 ==== */
