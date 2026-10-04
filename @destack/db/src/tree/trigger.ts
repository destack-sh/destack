import type { Triggers } from "../migration/trigger.ts";
import type { Dialect } from "../dialect/dialect.ts";
import { assertNever } from "../error/error.ts";
import type { TreeDescription } from "../inspect/tree.ts";
import { quote } from "../dialect/quote.ts";

/** Generate a tree's indexes and triggers. */
function install(tree: TreeDescription, dialect: Dialect): readonly string[] {
    // index direct children
    const prepare = `CREATE INDEX ${quote(`${tree.ancestors}_parent`)} ON ${quote(tree.table)} (${quote(tree.scope)}, ${quote(tree.parent)})`;

    // emit SQLite triggers
    if (dialect === "sqlite") {
        return [prepare, ...sqliteTriggers(tree)];
    }
    // emit PostgreSQL trigger functions and triggers
    else if (dialect === "postgresql") {
        return [prepare, ...postgresTriggers(tree)];
    }
    // reject other dialects
    else {
        return assertNever(dialect);
    }
}

/** Generate a tree's SQLite triggers. */
function sqliteTriggers(tree: TreeDescription): string[] {
    // derive the identifiers
    const source = quote(tree.table);
    const id = quote(tree.id);
    const scope = quote(tree.scope);
    const parent = quote(tree.parent);
    const prefix = tree.ancestors;

    return [
        `CREATE TRIGGER ${quote(`${prefix}_insert_check`)} BEFORE INSERT ON ${source} BEGIN
                SELECT RAISE(ABORT, 'tree parent is missing') WHERE ${missingParent(tree)};
                SELECT RAISE(ABORT, 'tree identity already exists') WHERE ${duplicate(tree)};
            END`,
        `CREATE TRIGGER ${quote(`${prefix}_insert`)} AFTER INSERT ON ${source} BEGIN ${insertPaths(tree)} END`,
        `CREATE TRIGGER ${quote(`${prefix}_move_check`)}
            BEFORE UPDATE OF ${id}, ${scope}, ${parent} ON ${source} BEGIN
                SELECT RAISE(ABORT, 'tree identity is immutable')
                WHERE NEW.${id} IS NOT OLD.${id} OR NEW.${scope} IS NOT OLD.${scope};
                SELECT RAISE(ABORT, 'tree parent is missing') WHERE ${missingParent(tree)};
                SELECT RAISE(ABORT, 'tree move creates a cycle') WHERE ${cycle(tree)};
            END`,
        `CREATE TRIGGER ${quote(`${prefix}_move`)} AFTER UPDATE OF ${parent} ON ${source}
                WHEN NEW.${parent} IS NOT OLD.${parent} BEGIN ${movePaths(tree)} END`,
        `CREATE TRIGGER ${quote(`${prefix}_delete_check`)} BEFORE DELETE ON ${source} BEGIN
                SELECT RAISE(ABORT, 'tree node has children') WHERE ${children(tree)};
            END`,
        `CREATE TRIGGER ${quote(`${prefix}_delete`)} AFTER DELETE ON ${source} BEGIN ${removePaths(tree)} END`,
    ];
}

/** Generate a tree's PostgreSQL trigger functions and triggers. */
function postgresTriggers(tree: TreeDescription): string[] {
    // serialize hierarchy writes before reading ancestry
    const source = quote(tree.table);
    const id = quote(tree.id);
    const scope = quote(tree.scope);
    const parent = quote(tree.parent);
    const lock = quote(tree.revision);
    const before = quote(`${tree.ancestors}_before`);
    const after = quote(`${tree.ancestors}_after`);

    return [
        `CREATE FUNCTION ${before}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
            INSERT INTO ${lock} (scope, revision)
            VALUES (CASE WHEN TG_OP = 'DELETE' THEN OLD.${scope} ELSE NEW.${scope} END, 1)
            ON CONFLICT (scope) DO UPDATE SET revision = ${lock}.revision + 1;
            IF TG_OP = 'DELETE' THEN
                IF ${children(tree)} THEN RAISE EXCEPTION 'tree node has children'; END IF;
                RETURN OLD;
            END IF;
            IF TG_OP = 'UPDATE' THEN
                IF NEW.${id} IS DISTINCT FROM OLD.${id} OR NEW.${scope} IS DISTINCT FROM OLD.${scope} THEN
                    RAISE EXCEPTION 'tree identity is immutable';
                END IF;
                IF ${cycle(tree)} THEN RAISE EXCEPTION 'tree move creates a cycle'; END IF;
            END IF;
            IF ${missingParent(tree)} THEN RAISE EXCEPTION 'tree parent is missing'; END IF;
            RETURN NEW;
        END $$`,
        `CREATE FUNCTION ${after}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
            IF TG_OP = 'INSERT' THEN ${insertPaths(tree)}
            ELSIF TG_OP = 'DELETE' THEN ${removePaths(tree)}
            ELSIF NEW.${parent} IS DISTINCT FROM OLD.${parent} THEN ${movePaths(tree)}
            END IF;
            RETURN NULL;
        END $$`,
        `CREATE TRIGGER ${quote(`${tree.ancestors}_check`)}
            BEFORE INSERT OR UPDATE OF ${id}, ${scope}, ${parent} OR DELETE ON ${source}
            FOR EACH ROW
            EXECUTE FUNCTION ${before}()`,
        `CREATE TRIGGER ${quote(`${tree.ancestors}_maintain`)}
            AFTER INSERT OR UPDATE OF ${parent} OR DELETE ON ${source}
            FOR EACH ROW
            EXECUTE FUNCTION ${after}()`,
    ];
}

/** Add an inserted node's paths to itself and its parent's ancestors. */
function insertPaths(tree: TreeDescription): string {
    // derive the identifiers
    const closure = quote(tree.ancestors);
    const id = quote(tree.id);
    const scope = quote(tree.scope);

    return `
        INSERT INTO ${closure} (scope, ancestor, descendant, depth)
        VALUES (NEW.${scope}, NEW.${id}, NEW.${id}, 0);
        INSERT INTO ${closure} (scope, ancestor, descendant, depth)
        SELECT scope, ancestor, NEW.${id}, depth + 1
        FROM ${closure}
        WHERE scope = NEW.${scope}
            AND descendant = NEW.${quote(tree.parent)};`;
}

/** Keep paths within a moved subtree and replace the external ones. */
function movePaths(tree: TreeDescription): string {
    // derive the identifiers
    const closure = quote(tree.ancestors);
    const id = quote(tree.id);
    const scope = quote(tree.scope);

    return `
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
            AND above.descendant = NEW.${quote(tree.parent)}
            AND below.ancestor = NEW.${id};`;
}

/** Remove a deleted node's paths. */
function removePaths(tree: TreeDescription): string {
    const id = quote(tree.id);

    return `
        DELETE FROM ${quote(tree.ancestors)}
        WHERE scope = OLD.${quote(tree.scope)}
            AND (ancestor = OLD.${id} OR descendant = OLD.${id});`;
}

/** Match a new row whose parent is missing. */
function missingParent(tree: TreeDescription): string {
    const scope = quote(tree.scope);
    const parent = quote(tree.parent);

    return `
        NEW.${parent} IS NOT NULL
        AND NOT EXISTS (
            SELECT 1
            FROM ${quote(tree.table)}
            WHERE ${scope} = NEW.${scope}
                AND ${quote(tree.id)} = NEW.${parent}
        )`;
}

/** Match a move under one of the node's descendants. */
function cycle(tree: TreeDescription): string {
    return `
        EXISTS (
            SELECT 1
            FROM ${quote(tree.ancestors)}
            WHERE scope = OLD.${quote(tree.scope)}
                AND ancestor = OLD.${quote(tree.id)}
                AND descendant = NEW.${quote(tree.parent)}
        )`;
}

/** Match an old row with children. */
function children(tree: TreeDescription): string {
    const scope = quote(tree.scope);

    return `
        EXISTS (
            SELECT 1
            FROM ${quote(tree.table)}
            WHERE ${scope} = OLD.${scope}
                AND ${quote(tree.parent)} = OLD.${quote(tree.id)}
        )`;
}

/** Match a new row whose identity the tree already has. */
function duplicate(tree: TreeDescription): string {
    return `
        EXISTS (
            SELECT 1
            FROM ${quote(tree.ancestors)}
            WHERE scope = NEW.${quote(tree.scope)}
                AND descendant = NEW.${quote(tree.id)}
        )`;
}

/** Remove a tree's triggers, functions and parent index. */
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

/** The triggers keeping tree indexes. */
export const treeTriggers: Triggers = {
    install: (state, dialect) => (state.tree ? install(state.tree, dialect) : []),
    remove: (state, dialect) => (state.tree ? remove(state.tree, dialect) : []),
};
