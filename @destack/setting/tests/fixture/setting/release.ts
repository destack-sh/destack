import { type BuildReader } from "@destack/package/manifest";
import { MemoryBuild } from "@destack/package/test";
import { describeSetting } from "../../../src/inspect/index.ts";
import { setting } from "../../../src/object/index.ts";
import type { Setting } from "../../../src/setting/index.ts";

/** Open a release of the setting package whose graph declares some settings. */
export async function release(settings: readonly Setting[]): Promise<BuildReader> {
    const declarations = settings.map((declared) => ({
        kind: "setting",
        package: setting.package.id,
        name: declared.name,
        description: describeSetting(declared),
    }));

    return await MemoryBuild.declaring(setting.package, declarations);
}
