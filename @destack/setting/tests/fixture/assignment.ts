import { SettingTarget } from "../../src/setting/target.ts";
import { SettingAssignment } from "../../src/setting/assignment.ts";
import { SettingPolicy } from "../../src/setting/policy.ts";
import type { SettingSnapshot } from "../../src/setting/resolution.ts";
import { editor } from "./settings/index.ts";
import { editorAssignment } from "./stack/index.ts";

/** Alice using Bob's installation on her own device. */
export const target = SettingTarget.parse({
    kind: "user",
    user: { kind: "user", authority: "global", id: "user-019f5530-8000-7000-8000-000000000003" },
    location: {
        spaceId: "space-019f5530-8000-7000-8000-000000000003",
        installationId: "installation-019f5530-8000-7000-8000-000000000004",
    },
    deviceId: "device-019f5530-8000-7000-8000-000000000005",
});

/** An explicit personal base assignment. */
export const personal = SettingAssignment.parse({
    id: "setting-assignment-019f5530-8000-7000-8000-000000000006",
    setting: editorAssignment.setting,
    target: editorAssignment.target,
    value: editorAssignment.value,
    revision: "019f5530-8000-7000-8000-000000000001",
    provenance: null,
    detachedAt: null,
    createdAt: 1000,
    updatedAt: 1000,
});

/** A device-specific choice overriding the personal base. */
export const device = SettingAssignment.parse({
    ...personal,
    id: "setting-assignment-019f5530-8000-7000-8000-000000000007",
    target,
    value: "standard",
});

/** A required policy selected by the host for Bob's receiving space. */
export const required = SettingPolicy.parse({
    id: "setting-policy-019f5530-8000-7000-8000-000000000008",
    setting: editor.reference,
    authority: { kind: "space", spaceId: "space-019f5530-8000-7000-8000-000000000003" },
    mode: "required",
    value: "vim",
    revision: "019f5530-8000-7000-8000-000000000001",
    provenance: null,
    detachedAt: null,
    createdAt: 1000,
    updatedAt: 1000,
});

/** Complete host-filtered inputs for one invocation. */
export const snapshot: SettingSnapshot = {
    target,
    assignments: [device, personal],
    policies: [],
    validUntil: 2000,
};
