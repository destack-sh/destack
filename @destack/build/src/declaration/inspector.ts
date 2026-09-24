import { describeAuditAction } from "@destack/audit/inspect";
import { describeBucket } from "@destack/bucket/inspect";
import { describeDatabase, describeDatabaseSchema } from "@destack/db/inspect";
import { describeSecret, describeVault } from "@destack/vault/inspect";
import {
    describeSchedule,
    describeService,
    describeServiceConnection,
    describeWorkload,
} from "@destack/service/inspect";
import { describeSpace } from "@destack/space/inspect";
import { describeAccount } from "@destack/model/inspect";
import { describeObject } from "@destack/access/inspect";
import {
    describeSetting,
    describeSettingAssignment,
    describeSettingPolicy,
} from "@destack/setting/inspect";
import type { Package } from "@destack/package";
import {
    DECLARATION_CONSTRUCTORS,
    type Declaration,
    type DeclarationConstructorName,
} from "@destack/package/declare";
import { BuildError } from "../error/index.ts";

/** Declaration constructors and the domain functions describing their values. */
export const INSPECTORS = {
    defineAccount: describeAccount,
    defineAuditAction: describeAuditAction,
    defineBucket: describeBucket,
    defineDatabase: describeDatabase,
    defineDatabaseSchema: describeDatabaseSchema,
    defineObject: describeObject,
    defineSchedule: describeSchedule,
    defineSecret: describeSecret,
    defineService: describeService,
    defineServiceConnection: describeServiceConnection,
    defineSetting: describeSetting,
    defineSettingAssignment: describeSettingAssignment,
    defineSettingPolicy: describeSettingPolicy,
    defineSpace: describeSpace,
    defineVault: describeVault,
    defineWorkload: describeWorkload,
} satisfies { [Name in InspectedConstructor]: (value: never) => object };

/** A declaration constructor the build inspects. */
export type InspectorName = keyof typeof INSPECTORS;

/** Registered constructors with a manifest kind. */
type InspectedConstructor = {
    [Name in DeclarationConstructorName]: (typeof DECLARATION_CONSTRUCTORS)[Name] extends {
        kind: string;
    }
        ? Name
        : never;
}[DeclarationConstructorName];

/** Describe an evaluated declaration, requiring stamped declarations to belong to the package. */
export function describeDeclaration(name: InspectorName, value: unknown, owner: Package): object {
    // compare the stamped package with the inspected source package
    if ("arguments" in DECLARATION_CONSTRUCTORS[name]) {
        const declaring = (value as Declaration).package;
        if (
            declaring.id !== owner.id ||
            declaring.name !== owner.name ||
            declaring.version !== owner.version
        ) {
            throw new BuildError(
                "INSPECTION_FAILED",
                `${name} declaration belongs to a different package`,
            );
        }
    }
    const describe = INSPECTORS[name] as (value: unknown) => object;

    return describe(value);
}
