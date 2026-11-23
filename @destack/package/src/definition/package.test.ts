import { Digest, Version } from "@destack/schema";
import { expect, test } from "@destack/test";
import { DependencyName, PackageId, PackageRelease } from "./package.ts";

test("read a bare import specifier's dependency and export subpath, scoped or not", () => {
    // split each specifier into its dependency and the subpath of its exports
    const specifiers = ["@destack/object", "@destack/object/client", "hyparquet", "jose/jwt/sign"];
    expect(
        specifiers.map((specifier) => [
            DependencyName.of(specifier),
            DependencyName.subpath(specifier),
        ]),
    ).toEqual([
        ["@destack/object", "."],
        ["@destack/object", "./client"],
        ["hyparquet", "."],
        ["jose", "./jwt/sign"],
    ]);
});

test("select a release by its exact version, dev among them", () => {
    // ship two releases and a dev build of one package
    const id = PackageId.parse("package-01996ab0-0000-7000-8000-0000000000a1");
    const release = (version: string, byte: string): PackageRelease => ({
        package: { id, name: "@example/vault", version },
        image: Digest.parse(byte.repeat(64)),
    });
    const shipped = [
        release("2026.10.0", "a"),
        release(Version.development, "b"),
        release("2026.9.0", "c"),
    ];
    const select = (version: string) => PackageRelease.select(shipped, id, version)?.image;

    // select each version
    expect([select("2026.10.0"), select(Version.development), select("2026.8.0")]).toEqual([
        "a".repeat(64),
        "b".repeat(64),
        undefined,
    ]);
});
