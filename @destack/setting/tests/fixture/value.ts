import { schema } from "@destack/schema";
import type { SettingValue } from "../../src/object/index.ts";
import { Scope } from "@destack/sync";
import { SettingPlacement, SettingSelection } from "../../src/setting/placement.ts";
import type { SettingSource } from "../../src/setting/resolution.ts";
import type { Setting } from "../../src/setting/setting.ts";
import { editor } from "./setting/index.ts";

/** Alice's personal scope. */
export const alice = "user-019f5530-8000-7000-8000-000000000003";

/** Bob's space, in which Alice acts. */
export const space = schema.identifier("space").parse("space-019f5530-8000-7000-8000-000000000003");

/** Bob's account, holding his space. */
export const account = "account-019f5530-8000-7000-8000-000000000011";

/** The scopes above Alice's values in Bob's space, nearest first. */
export const chain = [space, account, Scope.universe.id];

/** Bob's installation, which Alice uses. */
export const installation = schema
    .identifier("installation")
    .parse("installation-019f5530-8000-7000-8000-000000000004");

/** Alice's own device. */
export const device = schema
    .identifier("device")
    .parse("device-019f5530-8000-7000-8000-000000000005");

/** Alice using Bob's installation on her own device. */
export const selection = SettingSelection.parse({
    scope: alice,
    space,
    installation,
    deviceId: device,
});

/** Alice's personal editor mode. */
export const personal: SettingValue = {
    id: schema.identifier("setting").parse("setting-019f5530-8000-7000-8000-000000000006"),
    createdAt: 1000,
    createdBy: null,
    updatedAt: 1000,
    updatedBy: null,
    revision: 1,
    tags: {},
    scope: alice,
    managerInstallationId: null,
    managerPackageId: null,
    managerName: null,
    detachedAt: null,
    ...named(editor),
    package: null,
    space: null,
    installation: null,
    deviceId: null,
    mode: "set",
    value: "vim",
    release: "2026.9.0",
};

/** Alice's editor mode in Bob's installation on her device. */
export const override: SettingValue = {
    ...personal,
    id: schema.identifier("setting").parse("setting-019f5530-8000-7000-8000-000000000007"),
    installation,
    deviceId: device,
    value: "standard",
};

/** The editor mode Bob's space requires of the users acting in it. */
export const required: SettingValue = {
    ...personal,
    id: schema.identifier("setting").parse("setting-019f5530-8000-7000-8000-000000000008"),
    scope: space,
    mode: "require",
};

/** Read a setting's reference as the columns of its values hold it. */
export function named(declared: Setting): Pick<SettingValue, "packageId" | "name"> {
    return { packageId: declared.reference.packageId, name: declared.reference.name };
}

/** Describe a value as the source it contributes to a resolution, valid or skipped as invalid. */
export function source(value: SettingValue, kind: "value" | "invalid" = "value"): SettingSource {
    return {
        kind,
        id: value.id,
        revision: value.revision,
        ...SettingPlacement.of(value),
    };
}
