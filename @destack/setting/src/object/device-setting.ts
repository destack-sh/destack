import { through } from "@destack/access";
import { type Select } from "@destack/db";
import { defineObject, device, field } from "@destack/object";
import { PackageId } from "@destack/package";
import { schema, Version } from "@destack/schema";
import { SettingName } from "../setting/setting.ts";

/** A setting value one device stores for everyone using it, above every value placed in a scope. */
export const deviceSetting = defineObject({
    name: "device-setting",
    plural: "deviceSettings",
    scope: device,
    storage: "local",
    fields: {
        /** The package declaring the setting. */
        packageId: field.string(PackageId).immutable(),
        /** The setting's name in its declaring package. */
        name: field.string(SettingName).immutable(),
        /** The value. */
        value: field.json(schema.json()),
        /** The release of the setting's package the value was written against. */
        release: field.string(Version),
    },
    // let the people signed in on the device read and write them
    permissions: { read: through("device", "use"), write: through("device", "use") },
    constraints: {
        name: { kind: "index", on: ["packageId", "name"], unique: true },
    },
});

/** A device's setting value as its row stores it. */
export type DeviceSettingValue = Select<typeof deviceSetting.table>;
