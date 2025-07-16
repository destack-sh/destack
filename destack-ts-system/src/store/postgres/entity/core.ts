import {
  CustomProperty,
  IndexDefinition,
  NodeDefinitionReference,
  NodeReference,
  NodeType,
  PrimitiveType,
  PropertyDefinition,
  StoreKey,
} from "destack";

export const POSTGRES_BUILTIN_TABLE_PREFIX = "destack_";

export enum PostgresObjectKind {
  EXTENSION = "EXTENSION",
  TABLE = "TABLE",
  COLUMN = "COLUMN",
  CONSTRAINT = "CONSTRAINT",
  INDEX = "INDEX",
}

export enum PostgresColumnType {
  BIGINT = "bigint",
  BIGSERIAL = "bigserial",
  BIT = "bit",
  BIT_VARYING = "bit varying",
  BOOLEAN = "boolean",
  BOX = "box",
  BYTEA = "bytea",
  CHARACTER = "character",
  CHARACTER_VARYING = "varchar",
  CIDR = "cidr",
  CIRCLE = "circle",
  DATE = "date",
  DOUBLE_PRECISION = "double precision",
  INET = "inet",
  INTEGER = "integer",
  INTERVAL = "interval",
  JSON = "json",
  JSONB = "jsonb",
  LINE = "line",
  LSEG = "lseg",
  MACADDR = "macaddr",
  MONEY = "money",
  NUMERIC = "numeric",
  PATH = "path",
  POINT = "point",
  POLYGON = "polygon",
  REAL = "real",
  SMALLINT = "smallint",
  SMALLSERIAL = "smallserial",
  SERIAL = "serial",
  TEXT = "text",
  TIME = "time",
  TIMESTAMP = "timestamp",
  UUID = "uuid",
  XML = "xml",
}

/**
 * A Postgres cascade action.
 */
export enum PostgresCascadeAction {
  RESTRICT = "RESTRICT",
  CASCADE = "CASCADE",
  SET_NULL = "SET NULL",
  NO_ACTION = "NO ACTION",
  SET_DEFAULT = "SET DEFAULT",
}

/**
 * A Postgres constraint type.
 */
export enum PostgresConstraintType {
  PRIMARY_KEY = "PRIMARY KEY",
  FOREIGN_KEY = "FOREIGN KEY",
  UNIQUE = "UNIQUE",
  CHECK = "CHECK",
}

/**
 * A Postgres index type.
 */
export enum PostgresIndexType {
  BTREE = "BTREE",
  HASH = "HASH",
  GIN = "GIN",
  GIST = "GIST",
  BLOOM = "BLOOM",
}

export abstract class PostgresObject {
  abstract readonly kind: PostgresObjectKind;

  abstract get name(): string;
  abstract get qualifiedName(): string;
  abstract sql(): string;

  walk(): PostgresObject[] {
    return [this];
  }
}

/**
 * A Postgres extension.
 */
export class PostgresExtension extends PostgresObject {
  readonly kind = PostgresObjectKind.EXTENSION;

  constructor(public readonly name: string) {
    super();
  }

  get qualifiedName(): string {
    return this.name;
  }

  sql(): string {
    return `CREATE EXTENSION IF NOT EXISTS ${this.name}`;
  }

  toString(): string {
    return this.name;
  }
}

export abstract class PostgresTableObject extends PostgresObject {
  _table?: PostgresTable;

  get table(): PostgresTable {
    if (!this._table) {
      throw new Error(`${this} is not attached to a table`);
    }
    return this._table;
  }

  get tableName(): string {
    return this.table.name;
  }

  get qualifiedName(): string {
    if (this.kind === PostgresObjectKind.TABLE) {
      return (this as any).name;
    } else if (this.kind === PostgresObjectKind.INDEX) {
      return (this as any).name;
    } else {
      return `${this.tableName}.${(this as any).name}`;
    }
  }

  /**
   * Deep copy this table object without any external references.
   */
  clone(): this {
    const cloned = Object.create(Object.getPrototypeOf(this));
    Object.assign(cloned, this);
    cloned._table = undefined;
    return cloned;
  }
}

/**
 * A Postgres column.
 */
export class PostgresColumn extends PostgresTableObject {
  readonly kind = PostgresObjectKind.COLUMN;
  readonly name: string;
  readonly type: PrimitiveType;
  readonly prop: PropertyDefinition;
  readonly field?: CustomProperty;
  readonly isArray: boolean;
  readonly isPrimaryKey: boolean;
  readonly isForeignKeyTo?: string;
  readonly onDelete?: PostgresCascadeAction;
  readonly isUnique: boolean;
  readonly isNullable: boolean;
  readonly length?: number;
  readonly precision?: number;
  readonly scale?: number;
  readonly defaultValue?: string;

  constructor(options: {
    name: string;
    type: PrimitiveType;
    prop: PropertyDefinition;
    field?: CustomProperty;
    isArray?: boolean;
    isPrimaryKey?: boolean;
    isForeignKeyTo?: string;
    onDelete?: PostgresCascadeAction;
    isUnique?: boolean;
    isNullable?: boolean;
    length?: number;
    precision?: number;
    scale?: number;
    defaultValue?: string;
  }) {
    super();
    this.name = options.name;
    this.type = options.type;
    this.prop = options.prop;
    this.field = options.field;
    this.isArray = options.isArray ?? false;
    this.isPrimaryKey = options.isPrimaryKey ?? false;
    this.isForeignKeyTo = options.isForeignKeyTo;
    this.onDelete = options.onDelete;
    this.isUnique = options.isUnique ?? false;
    this.isNullable = options.isNullable ?? false;
    this.length = options.length;
    this.precision = options.precision;
    this.scale = options.scale;
    this.defaultValue = options.defaultValue;
  }

  typeSql(): string {
    let pgType: string;
    if (this.type === PrimitiveType.STRING && this.length !== undefined) {
      pgType = `VARCHAR(${this.length})`;
    } else if (this.type === PrimitiveType.DECIMAL) {
      pgType = `NUMERIC(${this.precision}, ${this.scale})`;
    } else {
      pgType = POSTGRES_TYPE_BY_PRIMITIVE_TYPE[this.type];
    }
    if (this.isArray) {
      pgType += "[]";
    }
    return pgType;
  }

  sql(): string {
    const parts = [`"${this.name}"`, this.typeSql()];
    if (!this.isNullable) {
      parts.push("NOT NULL");
    }
    if (this.isPrimaryKey) {
      parts.push("PRIMARY KEY");
    }
    // uniqueness is managed via constraints
    if (this.defaultValue !== undefined) {
      parts.push(`DEFAULT ${this.defaultValue}`);
    }
    if (this.isForeignKeyTo) {
      parts.push(`REFERENCES ${this.isForeignKeyTo}`);
      if (this.onDelete) {
        parts.push(`ON DELETE ${this.onDelete}`);
      }
    }
    return parts.join(" ");
  }

  toString(): string {
    const tableName = this._table?.name || "<detached>";
    const args: string[] = [];
    if (this.isArray) args.push("isArray");
    if (this.isUnique) args.push("isUnique");
    if (this.isNullable) args.push("isNullable");
    if (this.defaultValue) args.push(`default=${this.defaultValue}`);
    if (this.isPrimaryKey) args.push("isPrimaryKey");
    if (this.isForeignKeyTo) args.push(`isForeignKeyTo=${this.isForeignKeyTo}`);
    if (this.onDelete) args.push(`onDelete=${this.onDelete}`);

    const argsStr = args.length > 0 ? `, ${args.join(", ")}` : "";
    return `${tableName}.${this.name} (${this.type}${argsStr})`;
  }
}

/**
 * A Postgres constraint.
 */
export class PostgresConstraint extends PostgresTableObject {
  readonly kind = PostgresObjectKind.CONSTRAINT;
  readonly innerName: string;
  readonly type: PostgresConstraintType;
  readonly columns?: string[];
  readonly condition?: string;
  readonly index?: string;
  private _fullName?: string;

  constructor(options: {
    innerName: string;
    type: PostgresConstraintType;
    columns?: string[];
    condition?: string;
    index?: string;
    fullName?: string;
  }) {
    super();
    this.innerName = options.innerName;
    this.type = options.type;
    this.columns = options.columns ? [...options.columns].sort() : undefined;
    this.condition = options.condition;
    this.index = options.index;
    this._fullName = options.fullName;

    if (this.condition && (!this.condition.startsWith("(") || !this.condition.endsWith(")"))) {
      throw new Error(`invalid condition: ${this.condition}`);
    }
  }

  get name(): string {
    return this._fullName || `${this.tableName}_${this.innerName}`;
  }

  sql(): string {
    const parts = [`"${this.name}"`, this.type];
    if (this.type === PostgresConstraintType.CHECK) {
      parts.push(`(${this.condition})`);
    } else if (this.type === PostgresConstraintType.UNIQUE) {
      if (this.index) {
        parts.push(`USING INDEX "${this.tableName}_${this.index}"`);
      } else {
        const cols = this.columns?.map((col) => `"${col}"`).join(", ") || "";
        parts.push(`(${cols})`);
      }
    }
    return parts.join(" ");
  }

  toString(): string {
    const tableName = this._table?.name || "<detached>";
    return `${tableName}.${this.name} (${this.type}) [${this.columns}, condition=${this.condition}])`;
  }
}

/**
 * A Postgres index.
 */
export class PostgresIndex extends PostgresTableObject {
  readonly kind = PostgresObjectKind.INDEX;
  readonly innerName: string;
  readonly type: PostgresIndexType;
  readonly columns: string[];
  readonly cover: string[];
  readonly isUnique: boolean;
  readonly condition?: string;
  private _fullName?: string;

  constructor(options: {
    innerName: string;
    type: PostgresIndexType;
    columns: string[];
    cover?: string[];
    isUnique?: boolean;
    condition?: string;
    fullName?: string;
  }) {
    super();
    this.innerName = options.innerName;
    this.type = options.type;
    this.columns = options.columns;
    this.cover = options.cover ?? [];
    this.isUnique = options.isUnique ?? false;
    this.condition = options.condition;
    this._fullName = options.fullName;

    if (this.condition && (!this.condition.startsWith("(") || !this.condition.endsWith(")"))) {
      throw new Error(`invalid condition: ${this.condition}`);
    }
  }

  get name(): string {
    return this._fullName || `${this.tableName}_${this.innerName}`;
  }

  sql(): string {
    const parts = [
      `"${this.name}"`,
      `ON "${this.table.name}"`,
      `USING ${this.type}`,
      `(${this.columns.map((col) => `"${col}"`).join(", ")})`,
    ];
    if (this.cover.length > 0) {
      parts.push(`INCLUDE (${this.cover.join(", ")})`);
    }
    if (this.condition) {
      parts.push(`WHERE ${this.condition}`);
    }
    return parts.join(" ");
  }

  static fromIndex(index: IndexDefinition): PostgresIndex {
    return new PostgresIndex({
      innerName: index.name,
      type: PostgresIndexType.BTREE,
      columns: index.properties.map((p: any) => p.resolve().name),
      isUnique: false,
    });
  }

  toString(): string {
    const tableName = this._table?.name || "<detached>";
    return `${tableName}.${this.name} (${this.type}) [${this.columns}, condition=${this.condition}])`;
  }
}

/**
 * A Postgres table for a Node (builtin or custom).
 */
export class PostgresTable extends PostgresTableObject {
  readonly kind = PostgresObjectKind.TABLE;
  readonly name: string;
  readonly nodeType: NodeType;
  readonly columns: PostgresColumn[];
  readonly indexes: PostgresIndex[];
  readonly constraints: PostgresConstraint[];
  private _columnsByName: Map<string, PostgresColumn>;
  private _primaryKey?: PostgresColumn;

  constructor(options: {
    name: string;
    nodeType: NodeType;
    columns: PostgresColumn[];
    indexes?: PostgresIndex[];
    constraints?: PostgresConstraint[];
  }) {
    super();
    this.name = options.name;
    this.nodeType = options.nodeType;
    this.columns = options.columns;
    this.indexes = options.indexes ?? [];
    this.constraints = options.constraints ?? [];
    this._table = this;

    // attach all objects to this table
    for (const obj of [...this.columns, ...this.indexes, ...this.constraints]) {
      if (obj.table) {
        throw new Error(`${obj} is already attached to ${obj.table}`);
      }
      obj._table = this;
    }

    // build column lookup
    this._columnsByName = new Map();
    for (const column of this.columns) {
      if (this._columnsByName.has(column.name)) {
        throw new Error(`column ${column} is already defined in ${this}`);
      }
      this._columnsByName.set(column.name, column);
    }

    // find primary key
    this._primaryKey = this.columns.find((c) => c.isPrimaryKey);
  }

  get table(): PostgresTable {
    return this;
  }

  get primaryKey(): PostgresColumn | undefined {
    return this._primaryKey;
  }

  walk(): PostgresTableObject[] {
    return [this, ...this.columns, ...this.indexes, ...this.constraints];
  }

  sql(): string {
    // table creation would be implemented here
    throw new Error("Not implemented");
  }

  toString(): string {
    return `${this.name} (columns=${this.columns.length}, constraints=${this.constraints.length}, indexes=${this.indexes.length})`;
  }
}

export class PostgresSchema {
  readonly tables: readonly PostgresTable[];
  readonly extensions: readonly PostgresExtension[];
  private _tablesByName: Map<string, PostgresTable>;

  constructor(options: {
    extensions: readonly PostgresExtension[];
    tables: readonly PostgresTable[];
  }) {
    this.extensions = options.extensions;
    this.tables = options.tables;
    this._tablesByName = new Map(this.tables.map((table) => [table.name, table]));
  }

  walk(): PostgresObject[] {
    const result: PostgresObject[] = [];
    result.push(...this.extensions);
    for (const table of this.tables) {
      result.push(...table.walk());
    }
    return result;
  }

  toString(): string {
    return `extensions=${this.extensions.length}, tables=${this.tables.length}`;
  }
}

export class PostgresContext {
  private tablesByName: Map<string, PostgresTable> = new Map();

  constructor(public readonly storeKeys: StoreKey[]) {
    // would initialize builtin tables here
  }

  get(definition: NodeDefinitionReference | NodeReference): PostgresTable {
    const nodeType = definition instanceof NodeReference ? definition.type : definition.nodeType;
    const tableName = `${POSTGRES_BUILTIN_TABLE_PREFIX}${nodeType}`;
    const table = this.tablesByName.get(tableName);
    if (!table) {
      throw new Error(`no table for ${tableName} in ${this.storeKeys}`);
    }
    return table;
  }

  toString(): string {
    return `storeKeys=${this.storeKeys}, tables=[${Array.from(this.tablesByName.keys()).join(", ")}]`;
  }
}

export const EXTENSIONS = [
  new PostgresExtension("plpgsql"),
  new PostgresExtension("uuid-ossp"),
  new PostgresExtension("pgcrypto"),
];

// mapping from primitive types to postgres column types
export const POSTGRES_TYPE_BY_PRIMITIVE_TYPE: Record<PrimitiveType, PostgresColumnType> = {
  [PrimitiveType.STRING]: PostgresColumnType.CHARACTER_VARYING,
  [PrimitiveType.BOOLEAN]: PostgresColumnType.BOOLEAN,
  [PrimitiveType.INT16]: PostgresColumnType.SMALLINT,
  [PrimitiveType.INT32]: PostgresColumnType.INTEGER,
  [PrimitiveType.INT64]: PostgresColumnType.BIGINT,
  [PrimitiveType.FLOAT32]: PostgresColumnType.REAL,
  [PrimitiveType.FLOAT64]: PostgresColumnType.DOUBLE_PRECISION,
  [PrimitiveType.DATETIME]: PostgresColumnType.TIMESTAMP,
  [PrimitiveType.DATE]: PostgresColumnType.DATE,
  [PrimitiveType.TIME]: PostgresColumnType.TIME,
  [PrimitiveType.DURATION]: PostgresColumnType.INTERVAL,
  [PrimitiveType.JSON]: PostgresColumnType.JSONB,
  [PrimitiveType.UUID]: PostgresColumnType.UUID,
  [PrimitiveType.BYTES]: PostgresColumnType.BYTEA,
  [PrimitiveType.DECIMAL]: PostgresColumnType.NUMERIC,
};

export const PRIMITIVE_TYPE_BY_POSTGRES_TYPE: Record<PostgresColumnType, PrimitiveType> =
  Object.fromEntries(
    Object.entries(POSTGRES_TYPE_BY_PRIMITIVE_TYPE).map(([k, v]) => [v, k as any as PrimitiveType]),
  ) as Record<PostgresColumnType, PrimitiveType>;

