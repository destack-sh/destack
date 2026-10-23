import { type Identifier, schema } from "@destack/schema";
import { PackageId } from "@destack/package";
import { check, type Column, uniqueIndex } from "@destack/db";
import { AccessError } from "../error/index.ts";

/** The schema of the stack declaration managing a record. */
const ManagerSchema = schema.object({
    /** The installation that applied the declaration. */
    installationId: schema.identifier("installation"),
    /** The immutable identity of the declaring package. */
    packageId: PackageId,
    /** The declaration's path within the package, such as installations/notes. */
    name: schema.string().regex(/^[a-z][a-z0-9-]*(?:\/[a-z][a-z0-9-]*)*$/u),
});
/** The stack declaration managing a record. */
export type Manager = schema.Infer<typeof ManagerSchema>;

/** The manager columns every durable record has, with the time its manager detached it. */
export interface ManagerColumns {
    /** The installation that applied the declaration. */
    readonly managerInstallationId: Identifier<"installation"> | null;
    /** The declaring package. */
    readonly managerPackageId: PackageId | null;
    /** The declaration's name. */
    readonly managerName: string | null;
    /** When the manager detached the record, null while it manages it. */
    readonly detachedAt: number | null;
}

/** The stack declaration managing a record. */
export const Manager = {
    /** The schema of a manager. */
    schema: ManagerSchema,

    /** Read a record's manager, or null for records managed at runtime. */
    read(columns: ManagerColumns): Manager | null {
        return columns.managerInstallationId === null
            ? null
            : ManagerSchema.parse({
                  installationId: columns.managerInstallationId,
                  packageId: columns.managerPackageId,
                  name: columns.managerName,
              });
    },

    /** Write a record's manager, or none, into its columns, as managing it now. */
    values(manager: Manager | null): ManagerColumns {
        return {
            managerInstallationId: manager?.installationId ?? null,
            managerPackageId: manager?.packageId ?? null,
            managerName: manager?.name ?? null,
            detachedAt: null,
        };
    },

    /** Report whether a manager manages a record now, without having detached it. */
    manages(columns: ManagerColumns, manager: Manager): boolean {
        const written = Manager.values(manager);

        return (
            Manager.isManaged(columns) &&
            columns.managerInstallationId === written.managerInstallationId &&
            columns.managerPackageId === written.managerPackageId &&
            columns.managerName === written.managerName
        );
    },

    /** Report whether any manager manages a record now: one is set and has not detached it. */
    isManaged(columns: ManagerColumns): boolean {
        return columns.managerInstallationId !== null && columns.detachedAt === null;
    },

    /** Select the records a manager manages now: its columns, and not detached. */
    condition(manager: Manager) {
        return {
            managerInstallationId: manager.installationId,
            managerPackageId: manager.packageId,
            managerName: manager.name,
            detachedAt: { isNull: true },
        } as const;
    },

    /** Declare a managed table's constraints: a full manager or none, one row per declaration. */
    constraints(
        name: string,
        columns: {
            /** The managing installation. */
            readonly managerInstallationId: Column;
            /** The managing package. */
            readonly managerPackageId: Column;
            /** The managing declaration's name. */
            readonly managerName: Column;
            /** The time the declaration stopped managing the row. */
            readonly detachedAt: Column;
        },
    ) {
        return [
            check(`${name}_manager`, {
                OR: [
                    {
                        managerInstallationId: { isNull: true },
                        managerPackageId: { isNull: true },
                        managerName: { isNull: true },
                    },
                    {
                        managerInstallationId: { isNotNull: true },
                        managerPackageId: { isNotNull: true },
                        managerName: { isNotNull: true },
                    },
                ],
            }),
            check(`${name}_detached`, {
                OR: [
                    { detachedAt: { isNull: true } },
                    { managerInstallationId: { isNotNull: true }, detachedAt: { gte: 0 } },
                ],
            }),
            uniqueIndex(`${name}_manager`)
                .on(columns.managerInstallationId, columns.managerPackageId, columns.managerName)
                .where({
                    managerInstallationId: { isNotNull: true },
                    detachedAt: { isNull: true },
                }),
        ];
    },

    /** Refuse changing a managed record from anything but its manager. */
    requireChangeable(columns: ManagerColumns, manager: Manager | null, name: string): void {
        if (
            Manager.isManaged(columns) &&
            (manager === null || !Manager.manages(columns, manager))
        ) {
            throw AccessError.of(
                "MANAGED",
                `${name} is managed by its declaration: detach it before changing it`,
            );
        }
    },
};
