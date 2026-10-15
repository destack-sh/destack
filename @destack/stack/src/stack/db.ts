import { TABLE, type Table } from "@destack/db";
import { settingTables } from "@destack/setting/stack";
import {
    account,
    assignment,
    client,
    hostname,
    key,
    machine,
    placement,
    residency,
    space,
    user,
} from "@destack/account/object";
import { bucket } from "../object/bucket.ts";
import { dependency } from "../object/dependency.ts";
import { database } from "../object/database.ts";
import { https } from "../object/https.ts";
import stackPackage from "../package.ts";

/** The account package's rows every space's database replicates. */
export const spaceReplicated: readonly Table[] = [
    user.table,
    client.table,
    key.table,
    account.table,
    machine.table,
    hostname.table,
    space.table,
    placement.table,
    assignment.table,
    residency.table,
];

/** The tables of a space's database, with its settings and the installation assertions it spent. */
export const stackTables: readonly Table[] = [
    ...new Set([
        ...[
            ...Object.values(stackPackage.objects).filter(
                (object) => object.storage === "durable" || object.storage === "events",
            ),
            database,
            bucket,
            dependency,
            https,
        ].flatMap((object) => object.tables),
        ...settingTables,
    ]),
].filter(
    (table) => !spaceReplicated.some((replica) => replica[TABLE].sqlName === table[TABLE].sqlName),
);
