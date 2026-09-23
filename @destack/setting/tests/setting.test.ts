import { expect, test } from "@destack/test";
import { identifier, schema } from "@destack/schema";
import { defineSetting, defineSettingAssignment } from "../src/declare/index.ts";
import { describeSetting, SettingDescription } from "../src/inspect/index.ts";
import { resolveSetting } from "../src/setting/resolution.ts";
import { SettingEdit } from "../src/setting/edit.ts";
import { SettingTarget, SettingSelection } from "../src/setting/target.ts";
import { target, personal, device, required, snapshot } from "./fixture/index.ts";
import { editor, notes } from "./fixture/settings/index.ts";
import { PackageId } from "@destack/package";

test("resolve personal, device, policy and reset values independently of input order", () => {
    // retain the complete explanation of personal and device precedence
    const ordinary = {
        setting: editor.reference,
        target,
        value: "standard",
        sources: [
            {
                kind: "assignment",
                id: device.id,
                revision: "019f5530-8000-7000-8000-000000000001",
                target: device.target,
            },
        ],
        overridden: [
            { kind: "default", package: notes },
            {
                kind: "assignment",
                id: personal.id,
                revision: "019f5530-8000-7000-8000-000000000001",
                target: personal.target,
            },
        ],
        enforcement: "ordinary",
        validUntil: 2000,
    };
    expect(resolveSetting(editor, snapshot, 1500)).toEqual(ordinary);
    expect(resolveSetting(editor, { ...snapshot, assignments: [personal, device] }, 1500)).toEqual(
        ordinary,
    );

    // remove the device override to expose the personal base again
    expect(
        resolveSetting(
            editor,
            {
                ...snapshot,
                assignments: [personal],
            },
            1500,
        ),
    ).toEqual({
        ...ordinary,
        value: "vim",
        sources: [
            {
                kind: "assignment",
                id: personal.id,
                revision: "019f5530-8000-7000-8000-000000000001",
                target: personal.target,
            },
        ],
        overridden: [{ kind: "default", package: notes }],
    });

    // apply mandatory policy without erasing lower-priority choices
    expect(resolveSetting(editor, { ...snapshot, policies: [required] }, 1500)).toEqual({
        ...ordinary,
        value: "vim",
        enforcement: "required",
        sources: [
            { kind: "policy", id: required.id, revision: "019f5530-8000-7000-8000-000000000001" },
        ],
        overridden: [...ordinary.overridden, ...ordinary.sources],
    });
});

test("separate identities and shared runtimes while rejecting invalid and stale selections", () => {
    // an unrelated person's settings do not affect Alice's view
    const other = {
        ...personal,
        target: {
            kind: "user" as const,
            user: { kind: "user" as const, authority: "global", id: "user-bob" },
        },
    };
    expect(resolveSetting(editor, { ...snapshot, assignments: [other] }, 1500)).toEqual({
        setting: editor.reference,
        target,
        value: "standard",
        sources: [{ kind: "default", package: notes }],
        overridden: [],
        enforcement: "ordinary",
        validUntil: 2000,
    });

    // anonymous visitors retain defaults without borrowing any signed-in user's assignment
    const anonymous = SettingSelection.parse({ kind: "user", user: null });
    expect(resolveSetting(editor, { ...snapshot, target: anonymous }, 1500)).toEqual({
        setting: editor.reference,
        target: anonymous,
        value: "standard",
        sources: [{ kind: "default", package: notes }],
        overridden: [],
        enforcement: "ordinary",
        validUntil: 2000,
    });

    // authority unavailability cannot erase an enforced setting
    expect(() => resolveSetting(editor, snapshot, 2000)).toThrowError(
        "setting policy selection has expired",
    );
    expect(() =>
        resolveSetting(
            editor,
            { ...snapshot, policies: [required, { ...required, value: "standard" }] },
            1500,
        ),
    ).toThrowError("required setting policies disagree");
    expect(() =>
        resolveSetting(editor, { ...snapshot, assignments: [{ ...personal, value: 42 }] }, 1500),
    ).toThrowError("stored value is incompatible with the setting declaration");

    // shared background work selects its installation without a human subject
    const shared = defineSetting({
        ...editor.declaration,
        name: "defaultTemplate",
        scope: "space",
        overrides: ["installation"],
    });
    const sharedTarget = SettingTarget.parse({
        kind: "space",
        location: { spaceId: "space-019f5530-8000-7000-8000-000000000003" },
    });
    expect(
        resolveSetting(shared, { ...snapshot, target: sharedTarget, assignments: [] }, 1500),
    ).toEqual({
        setting: shared.reference,
        target: sharedTarget,
        value: "standard",
        sources: [{ kind: "default", package: notes }],
        overridden: [],
        enforcement: "ordinary",
        validUntil: 2000,
    });
});

test("describe typed declarations and retain exact conditional edit preconditions", () => {
    // inspection retains the native schema's complete serializable description
    const description = describeSetting(editor);
    expect(SettingDescription.parse(JSON.parse(JSON.stringify(description)))).toEqual({
        package: notes,
        name: "editor.mode",
        title: "Editor mode",
        description: "Keyboard behavior in the note editor.",
        schema: {
            $schema: "https://json-schema.org/draft/2020-12/schema",
            type: "string",
            enum: ["standard", "vim"],
        },
        default: "standard",
        scope: "user",
        overrides: ["space", "installation", "device"],
        apply: "immediate",
    });

    // a reset retains the original write identity and observed revision across serialization
    const mutation = {
        requestId: "019f5530-8000-7000-8000-000000000009",
        setting: editor.reference,
        target,
        expectedRevision: "019f5530-8000-7000-8000-000000000001",
    };
    expect(SettingEdit.parse(JSON.parse(JSON.stringify(mutation)))).toEqual(mutation);

    // equal required structured values do not depend on object insertion order
    const structured = defineSetting({
        ...editor.declaration,
        name: "layout",
        schema: schema.object({ width: schema.number(), compact: schema.boolean() }),
        default: { width: 80, compact: false },
    });
    const policy = {
        ...required,
        setting: structured.reference,
        value: { width: 80, compact: true },
    };
    const equivalent = {
        ...policy,
        id: "setting-policy-019f5530-8000-7000-8000-000000000010" as typeof policy.id,
        value: { compact: true, width: 80 },
    };
    expect(
        resolveSetting(
            structured,
            { ...snapshot, assignments: [], policies: [equivalent, policy] },
            1500,
        ),
    ).toEqual({
        setting: structured.reference,
        target,
        value: { width: 80, compact: true },
        sources: [
            { kind: "policy", id: policy.id, revision: "019f5530-8000-7000-8000-000000000001" },
            { kind: "policy", id: equivalent.id, revision: "019f5530-8000-7000-8000-000000000001" },
        ],
        overridden: [{ kind: "default", package: notes }],
        enforcement: "required",
        validUntil: 2000,
    });
});

test("isolate shared installation configuration and host-local paths", () => {
    // two installations share a declaration while retaining independent values
    const shared = defineSetting({
        ...editor.declaration,
        name: "defaultTemplate",
        scope: "space",
        overrides: ["installation"],
    });
    const first = SettingTarget.parse({
        kind: "space",
        location: {
            spaceId: "space-019f5530-8000-7000-8000-000000000003",
            installationId: "installation-019f5530-8000-7000-8000-000000000004",
        },
    });
    const second = SettingTarget.parse({
        kind: "space",
        location: {
            spaceId: "space-019f5530-8000-7000-8000-000000000003",
            installationId: "installation-019f5530-8000-7000-8000-000000000014",
        },
    });
    const assignment = { ...personal, setting: shared.reference, target: first };
    expect(
        resolveSetting(shared, { ...snapshot, target: first, assignments: [assignment] }, 1500),
    ).toEqual({
        setting: shared.reference,
        target: first,
        value: "vim",
        sources: [
            {
                kind: "assignment",
                id: assignment.id,
                revision: "019f5530-8000-7000-8000-000000000001",
                target: assignment.target,
            },
        ],
        overridden: [{ kind: "default", package: notes }],
        enforcement: "ordinary",
        validUntil: 2000,
    });
    expect(
        resolveSetting(shared, { ...snapshot, target: second, assignments: [assignment] }, 1500),
    ).toEqual({
        setting: shared.reference,
        target: second,
        value: "standard",
        sources: [{ kind: "default", package: notes }],
        overridden: [],
        enforcement: "ordinary",
        validUntil: 2000,
    });

    // a cache path from one host never configures a different host
    const cache = defineSetting({
        ...editor.declaration,
        name: "cacheDirectory",
        scope: "host",
        overrides: [],
        schema: schema.string().min(1),
        default: "/cache",
        apply: "restart",
    });
    const local = SettingTarget.parse({
        kind: "host",
        hostId: "host-019f5530-8000-7000-8000-000000000015",
    });
    const remote = SettingTarget.parse({
        kind: "host",
        hostId: "host-019f5530-8000-7000-8000-000000000016",
    });
    const path = {
        ...personal,
        setting: cache.reference,
        target: local,
        value: "/Users/alice/cache",
    };
    expect(
        resolveSetting(cache, { ...snapshot, target: remote, assignments: [path] }, 1500),
    ).toEqual({
        setting: cache.reference,
        target: remote,
        value: "/cache",
        sources: [{ kind: "default", package: notes }],
        overridden: [],
        enforcement: "ordinary",
        validUntil: 2000,
    });
});

test("apply the same declaration restrictions to source assignments and retained values", () => {
    // source authoring rejects shared targets and unsupported personal refinements
    const personalOnly = defineSetting({ ...editor.declaration, overrides: [] });
    expect(() => defineSettingAssignment(personalOnly, target, "vim")).toThrowError(
        "assignment uses an unsupported setting override",
    );
    const shared = SettingTarget.parse({
        kind: "space",
        location: { spaceId: "space-019f5530-8000-7000-8000-000000000003" },
    });
    expect(() => defineSettingAssignment(editor, shared, "vim")).toThrowError(
        "assignment scope does not match the setting declaration",
    );
    expect(() => resolveSetting(personalOnly, snapshot, 1500)).toThrowError(
        "assignment uses an unsupported setting override",
    );

    // removing policy restores retained choices without rewriting their revisions
    expect(
        resolveSetting(
            editor,
            {
                ...snapshot,
                assignments: [personal],
                policies: [],
            },
            1500,
        ),
    ).toEqual({
        setting: editor.reference,
        target,
        value: "vim",
        sources: [
            {
                kind: "assignment",
                id: personal.id,
                revision: "019f5530-8000-7000-8000-000000000001",
                target: personal.target,
            },
        ],
        overridden: [{ kind: "default", package: notes }],
        enforcement: "ordinary",
        validUntil: 2000,
    });

    // changing a package schema reports incompatible retained values explicitly
    const upgraded = defineSetting({
        ...editor.declaration,
        package: { ...notes, version: "2026.10.0" },
        schema: schema.enum(["standard", "emacs"]),
        default: "standard",
    });
    expect(() =>
        resolveSetting(upgraded, { ...snapshot, assignments: [personal] }, 1500),
    ).toThrowError("stored value is incompatible with the setting declaration");
});

test("retain offline choices and qualify shared settings by the consuming package", () => {
    // reuse one declaration in two apps without conflating the declaring and consuming packages
    const shared = defineSetting({
        ...editor.declaration,
        scope: "user",
        overrides: ["package", "space", "installation", "device"],
    });
    const homeId = PackageId.parse("package-019f5530-8000-7000-8000-000000000020");
    const home = SettingTarget.parse({ ...target, packageId: homeId });
    const appAssignment = {
        ...personal,
        target: SettingTarget.parse({ ...personal.target, packageId: homeId }),
    };
    const local = { ...snapshot, target: home, assignments: [appAssignment], validUntil: null };
    const expected = {
        setting: shared.reference,
        target: home,
        value: "vim",
        sources: [
            {
                kind: "assignment",
                id: personal.id,
                revision: "019f5530-8000-7000-8000-000000000001",
                target: appAssignment.target,
            },
        ],
        overridden: [{ kind: "default", package: notes }],
        enforcement: "ordinary",
        validUntil: null,
    };
    expect(resolveSetting(shared, local, 100000)).toEqual(expected);

    // the same Home choice follows it to another installation, while another app retains defaults
    const elsewhere = SettingTarget.parse({
        ...home,
        location: { spaceId: "space-019f5530-8000-7000-8000-000000000021" },
    });
    expect(resolveSetting(shared, { ...local, target: elsewhere }, 100000)).toEqual({
        ...expected,
        target: elsewhere,
    });
    expect(resolveSetting(shared, { ...local, target }, 100000)).toEqual({
        ...expected,
        target,
        value: "standard",
        sources: [{ kind: "default", package: notes }],
        overridden: [],
    });

    // expired remote authority cannot become locally authoritative merely because its cache is empty
    expect(() => resolveSetting(shared, { ...local, validUntil: 2000 }, 100000)).toThrowError(
        "setting policy selection has expired",
    );

    // equally specific administrators cannot silently override one another
    const recommendation = { ...required, mode: "recommended" as const };
    const account = {
        ...recommendation,
        id: identifier("setting-policy").parse(
            "setting-policy-019f5530-8000-7000-8000-000000000022",
        ),
        authority: {
            kind: "account" as const,
            accountId: identifier("account").parse("account-019f5530-8000-7000-8000-000000000023"),
        },
        value: "standard",
    };
    expect(() =>
        resolveSetting(shared, { ...local, policies: [recommendation, account] }, 100000),
    ).toThrowError("multiple setting values have the same precedence");
});
