import { TABLE } from "../table/table.ts";
import type { Tree } from "./tree.ts";
import type { Dialect } from "../dialect/dialect.ts";
import { assertNever } from "../error/error.ts";
import type { TreeDescription } from "../inspect/tree.ts";

/** Generated tree SQL surrounding a table migration. */
export class TreeMigration {
    /** Remove generated triggers before changing their tables. */
    readonly before: readonly string[];
    /** Install generated triggers after changing their tables. */
    readonly after: readonly string[];
    /** The tree declarations retained with this migration. */
    readonly trees: readonly TreeDescription[];
    /** Whether the declared trees differ from the previous migration. */
    readonly changed: boolean;
    /** Historical descriptions requiring a generated row backfill. */
    readonly rebuild: readonly TreeDescription[];

    /** Plan tree changes around the SQL produced by Drizzle. */
    constructor(trees: readonly Tree[], previous: readonly TreeDescription[], dialect: Dialect) {
        // compare declarations independently of their source order
        this.trees = trees
            .map((tree) => tree.describe())
            .sort((left, right) => left.ancestors.localeCompare(right.ancestors));
        const ordered = [...previous].sort((left, right) =>
            left.ancestors.localeCompare(right.ancestors),
        );
        this.changed = JSON.stringify(this.trees) !== JSON.stringify(ordered);

        // remove old triggers before Drizzle rebuilds or drops their source tables
        this.before = previous.flatMap((tree) => [
            ...dropTreeTriggers(tree, dialect),
            `DROP INDEX IF EXISTS ${quote(`${tree.ancestors}_parent`)};`,
        ]);
        const after: string[] = [];
        const rebuild: TreeDescription[] = [];
        for (const tree of trees) {
            const description = tree.describe();
            const old = previous.find((entry) => entry.ancestors === description.ancestors);

            // reconstruct derived paths when introducing or changing a parent relationship
            if (!old || JSON.stringify(old) !== JSON.stringify(description)) {
                rebuild.push(description);
            }
            after.push(...treeMigration(tree, dialect));
        }
        this.after = after;
        this.rebuild = rebuild;
    }
}

/** Generate indexes and triggers for a declared tree. */
export function treeMigration(tree: Tree, dialect: Dialect): readonly string[] {
    // derive every identifier from the declared table and column names
    const definition = tree.definition;
    const source = quote(definition.table[TABLE].name);
    const closure = quote(tree.ancestors[TABLE].name);
    const columns = definition.table[TABLE].columns;
    const id = quote(columns[definition.id].definition.name);
    const scope = quote(columns[definition.scope].definition.name);
    const parent = quote(columns[definition.parent].definition.name);
    const prefix = tree.ancestors[TABLE].name;

    // index direct children for parent validation and deletion checks
    const prepare = [
        `CREATE INDEX ${quote(`${prefix}_parent`)} ON ${source}(${scope}, ${parent});`,
    ];

    // retain paths within the moved subtree and replace only external ancestor paths
    const insert = `INSERT INTO ${closure}(scope, ancestor, descendant, depth)
        VALUES (NEW.${scope}, NEW.${id}, NEW.${id}, 0);
        INSERT INTO ${closure}(scope, ancestor, descendant, depth)
        SELECT scope, ancestor, NEW.${id}, depth + 1 FROM ${closure}
        WHERE scope = NEW.${scope} AND descendant = NEW.${parent};`;
    const move = `DELETE FROM ${closure} WHERE scope = OLD.${scope}
        AND descendant IN (SELECT descendant FROM ${closure} WHERE scope = OLD.${scope} AND ancestor = OLD.${id})
        AND ancestor IN (SELECT ancestor FROM ${closure} WHERE scope = OLD.${scope} AND descendant = OLD.${id} AND ancestor <> OLD.${id});
        INSERT INTO ${closure}(scope, ancestor, descendant, depth)
        SELECT above.scope, above.ancestor, below.descendant, above.depth + below.depth + 1
        FROM ${closure} above CROSS JOIN ${closure} below
        WHERE above.scope = NEW.${scope} AND below.scope = NEW.${scope}
          AND above.descendant = NEW.${parent} AND below.ancestor = NEW.${id};`;
    const remove = `DELETE FROM ${closure} WHERE scope = OLD.${scope} AND (ancestor = OLD.${id} OR descendant = OLD.${id});`;
    const missingParent = `NEW.${parent} IS NOT NULL AND NOT EXISTS (SELECT 1 FROM ${source} WHERE ${scope} = NEW.${scope} AND ${id} = NEW.${parent})`;
    const cycle = `EXISTS (SELECT 1 FROM ${closure} WHERE scope = OLD.${scope} AND ancestor = OLD.${id} AND descendant = NEW.${parent})`;
    const children = `EXISTS (SELECT 1 FROM ${source} WHERE ${scope} = OLD.${scope} AND ${parent} = OLD.${id})`;

    // emit SQLite triggers
    if (dialect === "sqlite") {
        return [
            ...prepare,
            `CREATE TRIGGER ${quote(`${prefix}_insert_check`)} BEFORE INSERT ON ${source} BEGIN
                SELECT RAISE(ABORT, 'tree parent is missing') WHERE ${missingParent};
                SELECT RAISE(ABORT, 'tree identity already exists') WHERE EXISTS (SELECT 1 FROM ${closure} WHERE scope = NEW.${scope} AND descendant = NEW.${id});
            END;`,
            `CREATE TRIGGER ${quote(`${prefix}_insert`)} AFTER INSERT ON ${source} BEGIN ${insert} END;`,
            `CREATE TRIGGER ${quote(`${prefix}_move_check`)} BEFORE UPDATE OF ${id}, ${scope}, ${parent} ON ${source} BEGIN
                SELECT RAISE(ABORT, 'tree identity is immutable') WHERE NEW.${id} IS NOT OLD.${id} OR NEW.${scope} IS NOT OLD.${scope};
                SELECT RAISE(ABORT, 'tree parent is missing') WHERE ${missingParent};
                SELECT RAISE(ABORT, 'tree move creates a cycle') WHERE ${cycle};
            END;`,
            `CREATE TRIGGER ${quote(`${prefix}_move`)} AFTER UPDATE OF ${parent} ON ${source}
                WHEN NEW.${parent} IS NOT OLD.${parent} BEGIN ${move} END;`,
            `CREATE TRIGGER ${quote(`${prefix}_delete_check`)} BEFORE DELETE ON ${source} BEGIN
                SELECT RAISE(ABORT, 'tree node has children') WHERE ${children};
            END;`,
            `CREATE TRIGGER ${quote(`${prefix}_delete`)} AFTER DELETE ON ${source} BEGIN ${remove} END;`,
        ];
    }
    if (dialect !== "postgresql") {
        return assertNever(dialect);
    }

    // serialize hierarchy mutations before reading ancestry; stale serializable writers abort
    const lock = quote(tree.revision[TABLE].name);
    const before = quote(`${prefix}_before`);
    const after = quote(`${prefix}_after`);

    return [
        ...prepare,
        `CREATE FUNCTION ${before}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
            INSERT INTO ${lock}(scope, revision) VALUES (CASE WHEN TG_OP = 'DELETE' THEN OLD.${scope} ELSE NEW.${scope} END, 1)
            ON CONFLICT (scope) DO UPDATE SET revision = ${lock}.revision + 1;
            IF TG_OP = 'DELETE' THEN
                IF ${children} THEN RAISE EXCEPTION 'tree node has children'; END IF;
                RETURN OLD;
            END IF;
            IF TG_OP = 'UPDATE' THEN
                IF NEW.${id} IS DISTINCT FROM OLD.${id} OR NEW.${scope} IS DISTINCT FROM OLD.${scope} THEN RAISE EXCEPTION 'tree identity is immutable'; END IF;
                IF ${cycle} THEN RAISE EXCEPTION 'tree move creates a cycle'; END IF;
            END IF;
            IF ${missingParent} THEN RAISE EXCEPTION 'tree parent is missing'; END IF;
            RETURN NEW;
        END $$;`,
        `CREATE FUNCTION ${after}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
            IF TG_OP = 'INSERT' THEN ${insert}
            ELSIF TG_OP = 'DELETE' THEN ${remove}
            ELSIF NEW.${parent} IS DISTINCT FROM OLD.${parent} THEN ${move}
            END IF;
            RETURN NULL;
        END $$;`,
        `CREATE TRIGGER ${quote(`${prefix}_check`)} BEFORE INSERT OR UPDATE OF ${id}, ${scope}, ${parent} OR DELETE ON ${source} FOR EACH ROW EXECUTE FUNCTION ${before}();`,
        `CREATE TRIGGER ${quote(`${prefix}_maintain`)} AFTER INSERT OR UPDATE OF ${parent} OR DELETE ON ${source} FOR EACH ROW EXECUTE FUNCTION ${after}();`,
    ];
}

/** Remove generated triggers and functions while retaining application records and index tables. */
export function dropTreeTriggers(tree: TreeDescription, dialect: Dialect): string[] {
    const statements: string[] = [];
    if (dialect === "sqlite") {
        for (const suffix of [
            "insert_check",
            "insert",
            "move_check",
            "move",
            "delete_check",
            "delete",
        ]) {
            statements.push(`DROP TRIGGER IF EXISTS ${quote(`${tree.ancestors}_${suffix}`)};`);
        }
    } else if (dialect === "postgresql") {
        statements.push(
            `DROP FUNCTION IF EXISTS ${quote(`${tree.ancestors}_before`)}() CASCADE;`,
            `DROP FUNCTION IF EXISTS ${quote(`${tree.ancestors}_after`)}() CASCADE;`,
        );
    } else {
        return assertNever(dialect);
    }

    return statements;
}

/** Quote a declared SQL identifier without accepting executable SQL. */
function quote(name: string): string {
    return `"${name.replaceAll('"', '""')}"`;
}
