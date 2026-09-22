import { check, type Column, dialectSQL, identifier, sql, text } from "@destack/db";

/** Persist a qualified permission and its optional object restriction. */
export function permissionColumns() {
    return {
        /** The package declaring the permission. */
        packageId: identifier("package_id", "package").notNull(),
        /** The declared object type. */
        type: text("type").notNull(),
        /** The permission name within the object type. */
        name: text("name").notNull(),
        /** The selected object; null selects all objects in the enclosing scope. */
        objectId: text("object_id"),
    };
}

/** Constrain qualified permission names and object restrictions. */
export function permissionChecks(
    name: string,
    columns: {
        /** The declaring package. */
        packageId: Column;
        /** The declared object type. */
        type: Column;
        /** The declared permission. */
        name: Column;
        /** The optional selected object. */
        objectId: Column;
    },
) {
    return [
        ...[columns.type, columns.name].map((column, index) =>
            check(
                `${name}_name_${index}`,
                dialectSQL({
                    sqlite: sql`length(${column}) > 0 AND substr(${column}, 1, 1) GLOB '[a-z]' AND ${column} NOT GLOB '*[^a-z0-9-]*' AND ${column} NOT LIKE '%--%' AND ${column} NOT LIKE '%-'`,
                    postgresql: sql`(${column} COLLATE "C") ~ '^[a-z][a-z0-9]*(-[a-z0-9]+)*$'`,
                }),
            ),
        ),
        check(
            `${name}_object`,
            sql`${columns.objectId} IS NULL OR length(${columns.objectId}) > 0`,
        ),
    ];
}
