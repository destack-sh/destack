import { check, index, sql, uniqueIndex, type Select, Condition } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { identifier, schema, Version } from "@destack/schema";
import { PackageId } from "@destack/package";
import { SpaceSetting } from "../declare/space.ts";
import { SETTING_MODES } from "../setting/mode.ts";
import { SettingName } from "../setting/setting.ts";
import { account, device, organisation, user } from "@destack/account/object";
import { host } from "@destack/account/object";
import { space } from "@destack/space/object";

/** A setting value placed in a scope. */
export const setting = defineObject({
    name: "setting",
    plural: "settings",
    scope: [user, space, account, organisation, host],
    declarable: { schema: SpaceSetting },
    // hand recommendations and requirements down to the scopes inside
    inherited: { where: Condition.ne("mode", "set") },
    fields: {
        /** The package declaring the setting. */
        packageId: field.string(PackageId),
        /** The setting's name in its declaring package. */
        name: field.string(SettingName),

        // hold the package, space and installation overrides as identifiers, since they live in other databases
        /** The consuming package the value applies to. */
        package: field.string(PackageId).optional(),
        /** The space the value applies in. */
        space: field.string(identifier("space")).optional(),
        /** The installation the value applies to. */
        installation: field.string(identifier("installation")).optional(),
        // reference the device, which lives in the same user scope as the values set for it
        /** The user's device the value applies on. */
        device: field.reference(device, { delete: "cascade" }).optional(),

        /** How the value applies. */
        mode: field.enum(SETTING_MODES),
        /** The value. */
        value: field.json(schema.json()),
        /** The release of the setting's package the value was written against. */
        release: field.string(Version),
    },
    permissions: ["read", "write"],
    detachable: { by: "write" },
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("write"),
        update: method.update("write", { fields: ["mode", "value", "release"] }),
        delete: method.delete("write"),
    }),
    constraints: (value) => [
        uniqueIndex("setting_placement").on(
            value.packageId,
            value.name,
            value.scope,
            sql`coalesce(${value.package}, '')`,
            sql`coalesce(${value.space}, '')`,
            sql`coalesce(${value.installation}, '')`,
            sql`coalesce(${value.device}, '')`,
        ),
        index("setting_scope").on(value.scope, value.id),
        check(
            "setting_override",
            sql`(${value.installation} IS NULL OR ${value.space} IS NULL) AND (${value.mode} = 'set' OR (${value.package} IS NULL AND ${value.space} IS NULL AND ${value.device} IS NULL))`,
        ),
    ],
});

/** A setting value as its row holds it. */
export type SettingValue = Select<typeof setting.table>;
