import { through } from "@destack/access";
import { type Select } from "@destack/db";
import { client, defineObject, field } from "@destack/object";
import { PackageId } from "@destack/package";
import { schema, Version } from "@destack/schema";
import { SettingName } from "../setting/setting.ts";

/** A setting value one client keeps for everyone using it, above every value placed in a scope. */
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
    // let the people signed in on the client read and write them
    permissions: { read: through("client", "use"), write: through("client", "use") },
    constraints: {
        name: { kind: "index", on: ["packageId", "name"], unique: true },
    },
}).methods((method) => ({
    create: method.create("write"),
    update: method.update("write", { fields: ["value", "release"] }),
    delete: method.delete("write"),
}));

/** A client's setting value as its row holds it. */
export type ClientSettingValue = Select<typeof clientSetting.table>;
