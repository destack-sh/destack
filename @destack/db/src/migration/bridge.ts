import type { Triggers } from "./trigger.ts";
import type { Dialect } from "../dialect/dialect.ts";
import { assertNever } from "../error/error.ts";
import { quote } from "../dialect/quote.ts";
import { hashName } from "../table/namespace.ts";

/** A renamed column kept equal to its previous column while an older release still writes that one. */
export interface Bridge {
    /** The previous column. */
    readonly from: string;
    /** The renamed column. */
    readonly to: string;
}

/** Create the triggers copying each write of one bridged column into the other. */
function install(table: string, bridge: Bridge, dialect: Dialect): string[] {
    // name the triggers by a short digest
    const name = bridgeName(table, bridge);
    const target = quote(table);
    const from = quote(bridge.from);
    const to = quote(bridge.to);

    // fill the omitted column and follow a change of either column with SQLite triggers
    if (dialect === "sqlite") {
        return [
            `CREATE TRIGGER ${quote(`${name}_insert`)} AFTER INSERT ON ${target} BEGIN
                UPDATE ${target} SET ${to} = coalesce(NEW.${to}, NEW.${from}),
                    ${from} = coalesce(NEW.${from}, NEW.${to}) WHERE rowid = NEW.rowid;
            END`,
            `CREATE TRIGGER ${quote(`${name}_from`)} AFTER UPDATE OF ${from} ON ${target}
                WHEN NEW.${from} IS NOT OLD.${from} AND NEW.${from} IS NOT NEW.${to} BEGIN
                UPDATE ${target} SET ${to} = NEW.${from} WHERE rowid = NEW.rowid;
            END`,
            `CREATE TRIGGER ${quote(`${name}_to`)} AFTER UPDATE OF ${to} ON ${target}
                WHEN NEW.${to} IS NOT OLD.${to} AND NEW.${to} IS NOT NEW.${from} BEGIN
                UPDATE ${target} SET ${from} = NEW.${to} WHERE rowid = NEW.rowid;
            END`,
        ];
    }
    // do the same in one PostgreSQL row trigger
    else if (dialect === "postgresql") {
        return [
            `CREATE FUNCTION ${quote(name)}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
                IF TG_OP = 'INSERT' THEN
                    NEW.${to} := coalesce(NEW.${to}, NEW.${from});
                    NEW.${from} := coalesce(NEW.${from}, NEW.${to});
                ELSIF NEW.${from} IS DISTINCT FROM OLD.${from} THEN
                    NEW.${to} := NEW.${from};
                ELSIF NEW.${to} IS DISTINCT FROM OLD.${to} THEN
                    NEW.${from} := NEW.${to};
                END IF;
                RETURN NEW;
            END $$`,
            `CREATE TRIGGER ${quote(name)} BEFORE INSERT OR UPDATE ON ${target}
                FOR EACH ROW EXECUTE FUNCTION ${quote(name)}()`,
        ];
    }
    // reject other dialects
    else {
        return assertNever(dialect);
    }
}

/** Remove the triggers of one bridge. */
function remove(table: string, bridge: Bridge, dialect: Dialect): string[] {
    const name = bridgeName(table, bridge);

    // drop each SQLite trigger
    if (dialect === "sqlite") {
        return ["insert", "from", "to"].map(
            (suffix) => `DROP TRIGGER IF EXISTS ${quote(`${name}_${suffix}`)}`,
        );
    }
    // drop the PostgreSQL trigger function with its trigger
    else if (dialect === "postgresql") {
        return [`DROP FUNCTION IF EXISTS ${quote(name)}() CASCADE`];
    }
    // reject other dialects
    else {
        return assertNever(dialect);
    }
}

/** Name one bridge's triggers by a digest of its table and columns. */
function bridgeName(table: string, bridge: Bridge): string {
    return `destack_bridge_${hashName(`${table}\0${bridge.from}\0${bridge.to}`)}`;
}

/** The triggers keeping renamed columns equal to their previous names while an older release writes those. */
export const bridgeTriggers: Triggers = {
    install: (state, dialect) =>
        (state.bridges ?? []).flatMap((bridge) => install(state.table.name, bridge, dialect)),
    remove: (state, dialect) =>
        (state.bridges ?? []).flatMap((bridge) => remove(state.table.name, bridge, dialect)),
};
