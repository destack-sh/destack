import { expect, test } from "@destack/test";
import { Toolchain } from "./toolchain.ts";

test("find an executable's toolchain in its macOS application's resources, else beside it", () => {
    expect(
        [
            "/Applications/Destack.app/Contents/Helpers/destack-build",
            "/Applications/Destack.app/Contents/Helpers/arm64/destack-build",
            "/opt/destack/bin/destack-build",
            "/home/ada/Destack/helpers/destack-build",
            "/Users/ada/Contents/destack-build",
        ].map((executable) => Toolchain.of(executable)),
    ).toEqual([
        "/Applications/Destack.app/Contents/Resources/toolchain",
        "/Applications/Destack.app/Contents/Resources/toolchain",
        "/opt/destack/bin/toolchain",
        "/home/ada/Destack/helpers/toolchain",
        "/Users/ada/Contents/toolchain",
    ]);
});
