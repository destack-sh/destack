import type { Condition } from "../query/condition.ts";
import type { SQL } from "../sql/index.ts";
import type { Column, ReferenceAction } from "./column.ts";

/** A table constraint or index. */
export type TableConstraint = Check | Index | Unique | ForeignKey;

/** A named row predicate. */
export interface Check {
    /** The constraint category. */
    readonly kind: "check";
    /** The SQL constraint name. */
    readonly name: string;
    /** The condition every row meets. */
    readonly condition: Condition;
}

/** A named ordered index, optionally partial. */
export class Index {
    /** The constraint category. */
    readonly kind = "index";
    /** The SQL index name. */
    readonly name: string;
    /** Whether duplicate keys are rejected. */
    readonly isUnique: boolean;
    /** The indexed columns or expressions. */
    readonly columns: readonly (Column | SQL)[];
    /** The condition selecting indexed rows, all rows when absent. */
    readonly predicate: Condition | undefined;
    /** Whether a unique index treats nulls as equal values. */
    readonly isNullsNotDistinct: boolean;

    /** Declare an index. */
    constructor(
        name: string,
        isUnique: boolean,
        columns: readonly (Column | SQL)[] = [],
        predicate?: Condition,
        isNullsNotDistinct = false,
    ) {
        // keep the index definition
        this.name = name;
        this.isUnique = isUnique;
        this.columns = columns;
        this.predicate = predicate;
        this.isNullsNotDistinct = isNullsNotDistinct;
    }

    /** Select indexed columns in order. */
    on(...columns: [Column | SQL, ...(Column | SQL)[]]): Index {
        return new Index(
            this.name,
            this.isUnique,
            columns,
            this.predicate,
            this.isNullsNotDistinct,
        );
    }

    /** Restrict the index to rows meeting a condition. */
    where(predicate: Condition): Index {
        return new Index(
            this.name,
            this.isUnique,
            this.columns,
            predicate,
            this.isNullsNotDistinct,
        );
    }

    /** Treat nulls as equal values in a unique index. */
    nullsNotDistinct(): Index {
        return new Index(this.name, this.isUnique, this.columns, this.predicate, true);
    }
}

/** Columns whose non-null values are distinct together. */
export class Unique {
    /** The constraint category. */
    readonly kind = "unique";
    /** The SQL constraint name, derived when absent. */
    readonly name: string | undefined;
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

/** A reference from some columns to others. */
export class ForeignKey {
    /** The constraint category. */
    readonly kind = "foreignKey";
    /** The SQL constraint name, derived when absent. */
    readonly name: string | undefined;
    /** The referencing columns in order. */
    readonly columns: readonly Column[];
    /** The referenced columns in matching order. */
    readonly foreignColumns: readonly Column[];
    /** The referenced table's SQL name. */
    readonly references: string;
    /** The referential actions. */
    readonly actions: ReferenceAction;

    /** Declare a foreign key. */
    constructor(
        definition: {
            readonly name?: string | undefined;
            readonly columns: readonly Column[];
            readonly foreignColumns: readonly Column[];
        },
        actions: ReferenceAction = {},
    ) {
        // require referenced columns of one table
        const [first] = definition.foreignColumns;
        if (
            first === undefined ||
            definition.foreignColumns.some((column) => column.table !== first.table)
        ) {
            throw new TypeError("a foreign key must reference columns of one table");
        }

        // keep the columns and actions
        this.name = definition.name;
        this.columns = definition.columns;
        this.foreignColumns = definition.foreignColumns;
        this.references = first.table;
        this.actions = actions;
    }

    /** Select the action on referenced row deletion. */
    onDelete(action: NonNullable<ReferenceAction["onDelete"]>): ForeignKey {
        return new ForeignKey(this, { ...this.actions, onDelete: action });
    }

    /** Select the action on referenced key updates. */
    onUpdate(action: NonNullable<ReferenceAction["onUpdate"]>): ForeignKey {
        return new ForeignKey(this, { ...this.actions, onUpdate: action });
    }
}

/** Declare a named condition every row meets. */
export function check(name: string, condition: Condition): Check {
    return { kind: "check", name, condition };
}

/** Declare a non-unique index. */
export function index(name: string): Index {
    return new Index(name, false);
}

/** Declare a unique index. */
export function uniqueIndex(name: string): Index {
    return new Index(name, true);
}

/** Declare a compound unique constraint. */
export function unique(name?: string): Unique {
    return new Unique(name);
}

/** Declare a compound foreign key. */
export function foreignKey(definition: ConstructorParameters<typeof ForeignKey>[0]): ForeignKey {
    return new ForeignKey(definition);
}
