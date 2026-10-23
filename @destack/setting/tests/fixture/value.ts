import { principal } from "@destack/access";
import { schema } from "@destack/schema";
import type { DeviceSettingValue, SettingValue } from "../../src/object/index.ts";
import { ObjectReference, Scope } from "@destack/sync";
import { SettingPlacement, SettingSelection } from "../../src/setting/placement.ts";
import type { SettingSource } from "../../src/setting/resolution.ts";
import type { Setting } from "../../src/setting/setting.ts";
import { editor } from "./setting/index.ts";

/** Alice's personal scope. */
export const alice = "user-019f5530-8000-7000-8000-000000000003";

/** Alice, who owns her personal scope. */
export const aliceUser = principal.user.reference(Scope.universe.id, alice);

/** Bob's space, in which Alice acts. */
export const space = schema.identifier("space").parse("space-019f5530-8000-7000-8000-000000000003");

/** Bob's account, with his space. */
export const account = "account-019f5530-8000-7000-8000-000000000011";

/** The scopes above Alice's values in Bob's space, nearest first. */
export const chain = [space, account, Scope.universe.id];

/** Bob's installation, which Alice uses. */
export const installation = schema
    .identifier("installation")
    .parse("installation-019f5530-8000-7000-8000-000000000004");

/** Alice using Bob's installation. */
export const selection = SettingSelection.parse({
    scope: alice,
    space,
    installation,
});

/** Alice's personal editor mode. */
export const personal: SettingValue = {
    id: schema.identifier("setting").parse("setting-019f5530-8000-7000-8000-000000000006"),
    createdAt: 1000,
    ...ObjectReference.columns("createdBy", undefined),
    updatedAt: 1000,
    ...ObjectReference.columns("updatedBy", undefined),
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
    mode: "set",
    value: "vim",
    release: "2026.9.0",
};

/** The editor mode Alice's laptop stores. */
export const laptop: DeviceSettingValue = {
    id: schema
        .identifier("device-setting")
        .parse("device-setting-019f5530-8000-7000-8000-000000000007"),
    createdAt: 1000,
    ...ObjectReference.columns("createdBy", undefined),
    updatedAt: 1000,
    ...ObjectReference.columns("updatedBy", undefined),
    revision: 1,
    tags: {},
    scope: schema.identifier("device").parse("device-019f5530-8000-7000-8000-000000000009"),
    ...named(editor),
    value: "standard",
    release: "2026.9.0",
};

/** The editor mode Bob's space requires of the users acting in it. */
export const required: SettingValue = {
    ...personal,
    id: schema.identifier("setting").parse("setting-019f5530-8000-7000-8000-000000000008"),
    scope: space,
    mode: "require",
};

/** Read a setting's reference as the columns of its values store it. */
export function named(declared: Setting): Pick<SettingValue, "packageId" | "name"> {
    return { packageId: declared.reference.packageId, name: declared.reference.name };
}

/** Describe a placed value as the source it contributes to a resolution. */
export function source(value: SettingValue, kind: "value" | "invalid" = "value"): SettingSource {
    const placed = {
        kind: "value" as const,
        id: value.id,
        revision: value.revision,
        ...SettingPlacement.of(value),
    };

    return kind === "value" ? placed : { kind, source: placed };
}

/** Describe a device's value as the source it contributes to a resolution. */
export function deviceSource(
    value: DeviceSettingValue,
    kind: "device" | "invalid" = "device",
): SettingSource {
    const local = { kind: "device" as const, id: value.id, revision: value.revision };

    return kind === "device" ? local : { kind, source: local };
}
