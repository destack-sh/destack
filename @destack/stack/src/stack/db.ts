import type { Table } from "@destack/db";
import { type Follow, type PackageHandle, tablesOf } from "@destack/object";
import settingPackage from "@destack/setting/package";
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
import workflowPackage from "@destack/workflow/package";
import objectPackage from "@destack/object/package";
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

/** The resource types the stack provisions in a space in addition to its package's objects. */
const PROVISIONED = { database, bucket, dependency, https };

/** List a space database's tables: object, stack, settings, workflows and its host's own. */
export function spaceTables(host: {
    /** The packages the host serves in addition to the platform's. */
    readonly packages?: readonly Pick<PackageHandle, "objects" | "tables">[];
    /** The shapes the host follows in addition to the space's own. */
    readonly follows?: readonly Follow[];
    /** The host's own tables. */
    readonly tables?: readonly Table[];
}): { readonly tables: readonly Table[]; readonly replicated?: readonly Table[] } {
    return tablesOf({
        packages: [
            objectPackage,
            stackPackage,
            settingPackage,
            workflowPackage,
            { objects: PROVISIONED, tables: [] },
            ...(host.packages ?? []),
        ],
        follows: host.follows ?? [],
        tables: host.tables ?? [],
    });
}
