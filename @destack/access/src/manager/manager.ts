import { defineSchema, schema } from "@destack/schema";
import { PackageId } from "@destack/package";

/** The schema of the stack declaration managing a record. */
const ManagerSchema = defineSchema(
    schema.object({
        /** The installation that applied the declaration. */
        installationId: schema.identifier("installation"),
        /** The immutable identity of the declaring package. */
        packageId: PackageId,
        /** The declaration's path within the package, such as installations/notes. */
        name: schema.string().regex(/^[a-z][a-z0-9-]*(?:\/[a-z][a-z0-9-]*)*$/u),
    }),
);
/** The stack declaration managing a record. */
export type Manager = schema.Infer<typeof ManagerSchema>;

/** The stack declaration managing a record. */
export const Manager = {
    /** The schema of a manager. */
    schema: ManagerSchema,

    /** Read a record's manager, or null for records managed at runtime. */
    read(row: {
        /** The installation that applied the declaration. */
        readonly managerInstallationId: string | null;
        /** The declaring package. */
        readonly managerPackageId: string | null;
        /** The declaration's name. */
        readonly managerName: string | null;
    }): Manager | null {
        return row.managerInstallationId === null
            ? null
            : ManagerSchema.parse({
                  installationId: row.managerInstallationId,
                  packageId: row.managerPackageId,
                  name: row.managerName,
              });
    },

    /** Write a record's manager into its columns. */
    values(manager: Manager | null) {
        return {
            managerInstallationId: manager?.installationId ?? null,
            managerPackageId: manager?.packageId ?? null,
            managerName: manager?.name ?? null,
        };
    },
};
