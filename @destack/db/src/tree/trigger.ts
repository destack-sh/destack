import type { Triggers } from "../migration/trigger.ts";
import type { Dialect } from "../dialect/dialect.ts";
import { assertNever } from "../error/error.ts";
import type { TreeDescription } from "../inspect/tree.ts";
import { quote } from "../dialect/quote.ts";

/** Generate indexes and triggers for a described tree. */
function install(tree: TreeDescription, dialect: Dialect): readonly string[] {
    // derive every identifier from the described table and column names
    const source = quote(tree.table);
    const closure = quote(tree.ancestors);
    const id = quote(tree.id);
    const scope = quote(tree.scope);
    const parent = quote(tree.parent);
    const prefix = tree.ancestors;

    // index direct children for parent validation and deletion checks
    const prepare = [
        `CREATE INDEX ${quote(`${prefix}_parent`)} ON ${source} (${scope}, ${parent})`,
    ];

    // retain paths within the moved subtree and replace only external ancestor paths
    const insert = `
        INSERT INTO ${closure} (scope, ancestor, descendant, depth)
        VALUES (NEW.${scope}, NEW.${id}, NEW.${id}, 0);
        INSERT INTO ${closure} (scope, ancestor, descendant, depth)
        SELECT scope, ancestor, NEW.${id}, depth + 1
        FROM ${closure}
        WHERE scope = NEW.${scope}
            AND descendant = NEW.${parent};`;
    const move = `
        DELETE FROM ${closure}
        WHERE scope = OLD.${scope}
            AND descendant IN (
                SELECT descendant
                FROM ${closure}
                WHERE scope = OLD.${scope}
                    AND ancestor = OLD.${id}
            )
            AND ancestor IN (
                SELECT ancestor
                FROM ${closure}
                WHERE scope = OLD.${scope}
                    AND descendant = OLD.${id}
                    AND ancestor <> OLD.${id}
            );
        INSERT INTO ${closure} (scope, ancestor, descendant, depth)
        SELECT above.scope, above.ancestor, below.descendant, above.depth + below.depth + 1
        FROM ${closure} above
        CROSS JOIN ${closure} below
        WHERE above.scope = NEW.${scope}
            AND below.scope = NEW.${scope}
            AND above.descendant = NEW.${parent}
            AND below.ancestor = NEW.${id};`;
    const remove = `
        DELETE FROM ${closure}
        WHERE scope = OLD.${scope}
            AND (ancestor = OLD.${id} OR descendant = OLD.${id});`;
    const missingParent = `
        NEW.${parent} IS NOT NULL
        AND NOT EXISTS (
            SELECT 1
            FROM ${source}
            WHERE ${scope} = NEW.${scope}
                AND ${id} = NEW.${parent}
        )`;
    const cycle = `
        EXISTS (
            SELECT 1
            FROM ${closure}
            WHERE scope = OLD.${scope}
                AND ancestor = OLD.${id}
                AND descendant = NEW.${parent}
        )`;
    const children = `
        EXISTS (
            SELECT 1
            FROM ${source}
            WHERE ${scope} = OLD.${scope}
                AND ${parent} = OLD.${id}
        )`;
    const duplicate = `
        EXISTS (
            SELECT 1
            FROM ${closure}
            WHERE scope = NEW.${scope}
                AND descendant = NEW.${id}
        )`;

    // emit SQLite triggers
    if (dialect === "sqlite") {
        return [
            ...prepare,
            `CREATE TRIGGER ${quote(`${prefix}_insert_check`)} BEFORE INSERT ON ${source} BEGIN
                SELECT RAISE(ABORT, 'tree parent is missing') WHERE ${missingParent};
                SELECT RAISE(ABORT, 'tree identity already exists') WHERE ${duplicate};
            END`,
            `CREATE TRIGGER ${quote(`${prefix}_insert`)} AFTER INSERT ON ${source} BEGIN ${insert} END`,
            `CREATE TRIGGER ${quote(`${prefix}_move_check`)}
            BEFORE UPDATE OF ${id}, ${scope}, ${parent} ON ${source} BEGIN
                SELECT RAISE(ABORT, 'tree identity is immutable')
                WHERE NEW.${id} IS NOT OLD.${id} OR NEW.${scope} IS NOT OLD.${scope};
                SELECT RAISE(ABORT, 'tree parent is missing') WHERE ${missingParent};
                SELECT RAISE(ABORT, 'tree move creates a cycle') WHERE ${cycle};
            END`,
            `CREATE TRIGGER ${quote(`${prefix}_move`)} AFTER UPDATE OF ${parent} ON ${source}
                WHEN NEW.${parent} IS NOT OLD.${parent} BEGIN ${move} END`,
            `CREATE TRIGGER ${quote(`${prefix}_delete_check`)} BEFORE DELETE ON ${source} BEGIN
                SELECT RAISE(ABORT, 'tree node has children') WHERE ${children};
            END`,
            `CREATE TRIGGER ${quote(`${prefix}_delete`)} AFTER DELETE ON ${source} BEGIN ${remove} END`,
        ];
    }
    if (dialect !== "postgresql") {
        return assertNever(dialect);
    }

    // serialize hierarchy mutations before reading ancestry; stale serializable writers abort
    const lock = quote(tree.revision);
    const before = quote(`${prefix}_before`);
    const after = quote(`${prefix}_after`);

    return [
        ...prepare,
        `CREATE FUNCTION ${before}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
            INSERT INTO ${lock} (scope, revision)
            VALUES (CASE WHEN TG_OP = 'DELETE' THEN OLD.${scope} ELSE NEW.${scope} END, 1)
            ON CONFLICT (scope) DO UPDATE SET revision = ${lock}.revision + 1;
            IF TG_OP = 'DELETE' THEN
                IF ${children} THEN RAISE EXCEPTION 'tree node has children'; END IF;
                RETURN OLD;
            END IF;
            IF TG_OP = 'UPDATE' THEN
                IF NEW.${id} IS DISTINCT FROM OLD.${id} OR NEW.${scope} IS DISTINCT FROM OLD.${scope} THEN
                    RAISE EXCEPTION 'tree identity is immutable';
                END IF;
                IF ${cycle} THEN RAISE EXCEPTION 'tree move creates a cycle'; END IF;
            END IF;
            IF ${missingParent} THEN RAISE EXCEPTION 'tree parent is missing'; END IF;
            RETURN NEW;
        END $$`,
        `CREATE FUNCTION ${after}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
            IF TG_OP = 'INSERT' THEN ${insert}
            ELSIF TG_OP = 'DELETE' THEN ${remove}
            ELSIF NEW.${parent} IS DISTINCT FROM OLD.${parent} THEN ${move}
            END IF;
            RETURN NULL;
        END $$`,
        `CREATE TRIGGER ${quote(`${prefix}_check`)}
            BEFORE INSERT OR UPDATE OF ${id}, ${scope}, ${parent} OR DELETE ON ${source}
            FOR EACH ROW
            EXECUTE FUNCTION ${before}()`,
        `CREATE TRIGGER ${quote(`${prefix}_maintain`)}
            AFTER INSERT OR UPDATE OF ${parent} OR DELETE ON ${source}
            FOR EACH ROW
            EXECUTE FUNCTION ${after}()`,
    ];
}

/** Remove generated triggers, functions and the parent index while retaining records and index tables. */
function remove(tree: TreeDescription, dialect: Dialect): string[] {
    const statements = [`DROP INDEX IF EXISTS ${quote(`${tree.ancestors}_parent`)}`];

    // drop each SQLite trigger
    if (dialect === "sqlite") {
        for (const suffix of [
            "insert_check",
            "insert",
            "move_check",
            "move",
            "delete_check",
            "delete",
        ]) {
            statements.push(`DROP TRIGGER IF EXISTS ${quote(`${tree.ancestors}_${suffix}`)}`);
        }
    }
    // drop the PostgreSQL trigger functions with their triggers
    else if (dialect === "postgresql") {
        statements.push(
            `DROP FUNCTION IF EXISTS ${quote(`${tree.ancestors}_before`)}() CASCADE`,
            `DROP FUNCTION IF EXISTS ${quote(`${tree.ancestors}_after`)}() CASCADE`,
        );
    }
    // reject other dialects
    else {
        return assertNever(dialect);
    }

    return statements;
}

/** The triggers keeping a tree table's ancestor index. */
export const treeTriggers: Triggers = {
    install: (state, dialect) => (state.tree ? install(state.tree, dialect) : []),
    remove: (state, dialect) => (state.tree ? remove(state.tree, dialect) : []),
};
