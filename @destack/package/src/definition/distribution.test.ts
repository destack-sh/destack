import { Digest } from "@destack/schema";
import { expect, test } from "@destack/test";
import { DistributionManifest, DistributionPlatform } from "./distribution.ts";
import { PackageError } from "../error/index.ts";
import { PackageId } from "./package.ts";

/** The vault's package. */
const VAULT = {
    id: PackageId.parse("package-01996ab0-0000-7000-8000-0000000000a1"),
    name: "@destack/vault",
    version: "1.0.0",
};

/** The start page's package. */
const START = {
    id: PackageId.parse("package-01996ab0-0000-7000-8000-0000000000a2"),
    name: "start",
    version: "2.0.0",
};

/** The locks each slice builds. */
const LOCKS = {
    vaultArm: Digest.parse("a".repeat(64)),
    vaultIntel: Digest.parse("b".repeat(64)),
    startArm: Digest.parse("c".repeat(64)),
    startIntel: Digest.parse("d".repeat(64)),
};

/** The arm64 slice's manifest. */
const ARM: DistributionManifest = {
    releases: [
        { package: VAULT, role: "builtin", locks: { "darwin-arm64": LOCKS.vaultArm } },
        { package: START, role: "app", locks: { "darwin-arm64": LOCKS.startArm } },
    ],
};

/** The x64 slice's vault release. */
const INTEL_VAULT: DistributionManifest["releases"][number] = {
    package: VAULT,
    role: "builtin",
    locks: { "darwin-x64": LOCKS.vaultIntel },
};

/** The x64 slice's start page release. */
const INTEL_START: DistributionManifest["releases"][number] = {
    package: START,
    role: "app",
    locks: { "darwin-x64": LOCKS.startIntel },
};

/** The x64 slice's manifest, in another order. */
const INTEL: DistributionManifest = { releases: [INTEL_START, INTEL_VAULT] };

test("merge two slices' manifests into each release's locks on both platforms", () => {
    // merge, then read each platform's releases and builtins back
    const merged = DistributionManifest.merge(ARM, INTEL);
    expect(merged).toEqual({
        releases: [
            {
                package: VAULT,
                role: "builtin",
                locks: { "darwin-arm64": LOCKS.vaultArm, "darwin-x64": LOCKS.vaultIntel },
            },
            {
                package: START,
                role: "app",
                locks: { "darwin-arm64": LOCKS.startArm, "darwin-x64": LOCKS.startIntel },
            },
        ],
    });
    expect(DistributionManifest.releases(merged, "darwin-x64")).toEqual([
        { package: VAULT, lock: LOCKS.vaultIntel },
        { package: START, lock: LOCKS.startIntel },
    ]);
    expect(DistributionManifest.builtins(merged, "darwin-arm64")).toEqual(
        new Map([[VAULT.id, { package: VAULT, lock: LOCKS.vaultArm }]]),
    );
});

test("merge a slice's manifest with itself unchanged", () => {
    expect(DistributionManifest.merge(ARM, ARM)).toEqual(ARM);
});

test("refuse to merge manifests of different packages, versions, roles or locks", () => {
    // ship the start page in one slice only
    expect(() => DistributionManifest.merge(ARM, { releases: [INTEL_VAULT] })).toThrow(
        PackageError.of(
            "DISTRIBUTION_INVALID",
            "distribution is invalid: only one platform ships 'start@2.0.0 (app)'",
        ),
    );

    // ship another version, then another role
    const newer = { ...INTEL_START, package: { ...START, version: "2.0.1" } };
    expect(() => DistributionManifest.merge(ARM, { releases: [INTEL_VAULT, newer] })).toThrow(
        PackageError.of(
            "DISTRIBUTION_INVALID",
            "distribution is invalid: its platforms ship 'start@2.0.0 (app)' and 'start@2.0.1 (app)'",
        ),
    );
    const library = { ...INTEL_START, role: "library" as const };
    expect(() => DistributionManifest.merge(ARM, { releases: [INTEL_VAULT, library] })).toThrow(
        PackageError.of(
            "DISTRIBUTION_INVALID",
            "distribution is invalid: its platforms ship 'start@2.0.0 (app)' and 'start@2.0.0 (library)'",
        ),
    );

    // ship another lock on the same platform
    const rebuilt = { ...INTEL_START, locks: { "darwin-arm64": LOCKS.startIntel } };
    expect(() => DistributionManifest.merge(ARM, { releases: [INTEL_VAULT, rebuilt] })).toThrow(
        PackageError.of(
            "DISTRIBUTION_INVALID",
            "distribution is invalid: it has two 'darwin-arm64' locks of 'start@2.0.0 (app)'",
        ),
    );
});

test("refuse to read a release without a lock on the platform", () => {
    expect(() => DistributionManifest.releases(ARM, "workerd")).toThrow(
        new TypeError("'the workerd lock of @destack/vault' is missing"),
    );
});

test("read a Bun host's platform", () => {
    expect(DistributionPlatform.of({ os: "darwin", architecture: "arm64" })).toBe("darwin-arm64");
});
