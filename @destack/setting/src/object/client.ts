import { uniqueIndex, type Select } from "@destack/db";
import { client, defineObject, field } from "@destack/object";
import { PackageId } from "@destack/package";
import { schema, Version } from "@destack/schema";
import { SettingName } from "../setting/setting.ts";

/**
 * A setting value one client keeps for everyone using it, above every value placed in a scope.
 *
 * A browser may evict the client's database, after which resolutions fall back to the placed values and defaults.
 */
export const clientSetting = defineObject({
    name: "client-setting",
    plural: "clientSettings",
    scope: client,
    storage: "local",
    fields: {
        /** The package declaring the setting. */
        packageId: field.string(PackageId),
        /** The setting's name in its declaring package. */
        name: field.string(SettingName),
        /** The value. */
        value: field.json(schema.json()),
        /** The release of the setting's package the value was written against. */
        release: field.string(Version),
    },
    permissions: ["read", "write"],
    methods: (method) => ({
        list: method.list("read"),
        create: method.create("write"),
        update: method.update("write", { fields: ["value", "release"] }),
        delete: method.delete("write"),
    }),
    constraints: (value) => [uniqueIndex("client_setting_name").on(value.packageId, value.name)],
});

/** A client's setting value as its row holds it. */
export type ClientSettingValue = Select<typeof clientSetting.table>;
