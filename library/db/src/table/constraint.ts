import type { SQL } from "drizzle-orm";
import type { Column, ReferenceActions } from "./column.ts";

/** A table constraint or index. */
export type TableConstraint = Check | Index | PrimaryKey | Unique | ForeignKey;

/** A named SQL predicate required for every row. */
export interface Check {
    /** The constraint category. */
    readonly kind: "check";
    /** The SQL constraint name. */
    readonly name: string;
    /** The row predicate. */
    readonly expression: SQL;
}

/** A named ordered index, optionally restricted to selected rows. */
export class Index {
    /** The constraint category. */
    readonly kind = "index";
    /** The SQL index name. */
    readonly name: string;
    /** Whether duplicate keys are rejected. */
    readonly unique: boolean;
    /** The indexed columns or expressions. */
    readonly columns: readonly (Column | SQL)[];
    /** The predicate selecting indexed rows. */
    readonly predicate?: SQL;

    /** Declare an index. */
    constructor(
        name: string,
        unique: boolean,
        columns: readonly (Column | SQL)[] = [],
        predicate?: SQL,
    ) {
        this.name = name;
        this.unique = unique;
        this.columns = columns;
        this.predicate = predicate;
    }

    /** Select indexed columns in order. */
    on(...columns: [Column | SQL, ...(Column | SQL)[]]): Index {
        return new Index(this.name, this.unique, columns, this.predicate);
    }

    /** Restrict the index to rows matching a predicate. */
    where(predicate: SQL): Index {
        return new Index(this.name, this.unique, this.columns, predicate);
    }
}

/** The columns identifying a row. */
export interface PrimaryKey {
    /** The constraint category. */
    readonly kind: "primaryKey";
    /** The SQL constraint name. */
    readonly name?: string;
    /** The identifying columns in order. */
    readonly columns: readonly [Column, ...Column[]];
}

/** A group of columns whose non-null values must be distinct. */
export class Unique {
    /** The constraint category. */
    readonly kind = "unique";
    /** The SQL constraint name. */
    readonly name?: string;
    /** The constrained columns. */
    readonly columns: readonly Column[];

    /** Declare a unique constraint. */
    constructor(name?: string, columns: readonly Column[] = []) {
        this.name = name;
        this.columns = columns;
    }

    /** Select the columns constrained together. */
    on(...columns: [Column, ...Column[]]): Unique {
        return new Unique(this.name, columns);
    }
}

/** A reference from one group of columns to another. */
export class ForeignKey {
    /** The constraint category. */
    readonly kind = "foreignKey";
    /** The SQL constraint name. */
    readonly name?: string;
    /** The referencing columns in order. */
    readonly columns: readonly Column[];
    /** The referenced columns in matching order. */
    readonly foreignColumns: readonly Column[];
    /** The referential actions. */
    readonly actions: ReferenceActions;

    /** Declare a foreign key. */
    constructor(
        definition: {
            readonly name?: string;
            readonly columns: readonly Column[];
            readonly foreignColumns: readonly Column[];
        },
        actions: ReferenceActions = {},
    ) {
        this.name = definition.name;
        this.columns = definition.columns;
        this.foreignColumns = definition.foreignColumns;
        this.actions = actions;
    }

    /** Select the action for referenced row deletion. */
    onDelete(action: NonNullable<ReferenceActions["onDelete"]>): ForeignKey {
        return new ForeignKey(this, { ...this.actions, onDelete: action });
    }

    /** Select the action for referenced key updates. */
    onUpdate(action: NonNullable<ReferenceActions["onUpdate"]>): ForeignKey {
        return new ForeignKey(this, { ...this.actions, onUpdate: action });
    }
}

/** Declare a named row predicate. */
export function check(name: string, expression: SQL): Check {
    return { kind: "check", name, expression };
}

/** Declare a non-unique index. */
export function index(name: string): Index {
    return new Index(name, false);
}

/** Declare a unique index. */
export function uniqueIndex(name: string): Index {
    return new Index(name, true);
}

/** Declare a compound primary key. */
export function primaryKey(definition: Omit<PrimaryKey, "kind">): PrimaryKey {
    return { kind: "primaryKey", ...definition };
}

/** Declare a compound unique constraint. */
export function unique(name?: string): Unique {
    return new Unique(name);
}

/** Declare a compound foreign key. */
export function foreignKey(definition: ConstructorParameters<typeof ForeignKey>[0]): ForeignKey {
    return new ForeignKey(definition);
}
