import { expect, test } from "@destack/test";
import { DependencyName } from "./package.ts";

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
