/** A declaration constructor as the transform, the build and the linter see it. */
export interface DeclarationConstructor {
    /** The package defining the constructor. */
    readonly package: string;
    /** The manifest description kind, absent for constructors the build does not inspect. */
    readonly kind?: string;
    /** The argument count before the module the transform appends, absent when no module is passed. */
    readonly arguments?: number;
}

/** Every Destack declaration constructor, by name. */
export const DECLARATION_CONSTRUCTORS = {
    defineAccount: { package: "@destack/model", kind: "account" },
    defineAuditAction: { package: "@destack/audit", kind: "audit-action", arguments: 1 },
    defineBucket: { package: "@destack/bucket", kind: "resource", arguments: 1 },
    defineDatabase: { package: "@destack/db", kind: "resource", arguments: 1 },
    defineDatabaseSchema: { package: "@destack/db", kind: "database-schema" },
    defineObject: { package: "@destack/access", kind: "access", arguments: 1 },
    definePackage: { package: "@destack/package", arguments: 1 },
    defineSchedule: { package: "@destack/service", kind: "schedule" },
    defineSecret: { package: "@destack/vault", kind: "secret", arguments: 1 },
    defineService: { package: "@destack/service", kind: "service", arguments: 2 },
    defineServiceConnection: {
        package: "@destack/service",
        kind: "service-connection",
        arguments: 2,
    },
    defineSetting: { package: "@destack/setting", kind: "setting", arguments: 1 },
    defineSettingAssignment: { package: "@destack/setting", kind: "setting-assignment" },
    defineSettingPolicy: { package: "@destack/setting", kind: "setting-policy" },
    defineSpace: { package: "@destack/space", kind: "space" },
    defineVault: { package: "@destack/vault", kind: "resource", arguments: 1 },
} as const satisfies Record<string, DeclarationConstructor>;

/** The name of a Destack declaration constructor. */
export type DeclarationConstructorName = keyof typeof DECLARATION_CONSTRUCTORS;
