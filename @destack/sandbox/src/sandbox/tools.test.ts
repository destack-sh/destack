import { expect, test } from "@destack/test";
import { SANDBOX_TOOLS, SandboxTool } from "./tools.ts";

test("find a Linux release's sandbox tools beside its program, and none for a checkout or on macOS", () => {
    const release = {
        executable: "/opt/destack/helpers/destack-sandbox",
        runtime: { revision: "0", isStandalone: true },
    };
    const checkout = {
        executable: "/usr/bin/bun",
        runtime: { revision: "0", isStandalone: false },
    };

    expect(
        SANDBOX_TOOLS.map((tool) => [
            SandboxTool.path(tool, release, "linux"),
            SandboxTool.path(tool, checkout, "linux"),
            SandboxTool.path(tool, release, "darwin"),
        ]),
    ).toEqual([
        ["/opt/destack/helpers/rg", undefined, undefined],
        ["/opt/destack/helpers/socat", undefined, undefined],
    ]);
});
