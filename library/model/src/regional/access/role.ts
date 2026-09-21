import {
    foreignKey,
    identifier,
    recordColumns,
    type Select,
    table,
    text,
    unique,
} from "@destack/db";
import { space } from "../space/space.ts";
import { provenanceChecks, provenanceColumns } from "../../source/index.ts";

/** A reusable collection of permissions within one space. */
export const role = table(
    "role",
    {
        ...recordColumns("role"),
        ...provenanceColumns(),
        /** The account defining this role. */
        accountId: identifier("account_id", "account").notNull(),
        /** The space defining this role. */
        spaceId: identifier("space_id", "space").notNull(),
        /** The space-local role name. */
        name: text("name").notNull(),
        /** The purpose shown when granting the role. */
        description: text("description").notNull(),
    },
    (role) => [
        ...provenanceChecks("role", role),
        unique("role_space_name").on(role.spaceId, role.name),
        unique("role_space_id").on(role.spaceId, role.id),
        unique("role_account_id").on(role.accountId, role.id),
        foreignKey({
            columns: [role.accountId, role.spaceId],
            foreignColumns: [space.accountId, space.id],
        }).onDelete("restrict"),
    ],
);

/** A space-defined role. */
export type Role = Select<typeof role>;
