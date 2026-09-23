import { defineServiceConnection } from "@destack/service/declare";
import { PackageId } from "@destack/package";
import { settingService } from "@destack/setting/service";
import definition from "@destack/setting/destack.json" with { type: "json" };
import type {} from "@destack/package/import-meta";

/** Connection to the setting authority selected by the host. */
export const settings = defineServiceConnection(
    {
        packageId: import.meta.destack.package.id,
        name: "settings",
        service: { packageId: PackageId.parse(definition.id), name: "setting" },
    },
    settingService,
);
