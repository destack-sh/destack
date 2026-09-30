import { describeFile } from "@destack/package/file";
import { BuildReader, type PackageManifest } from "@destack/package/manifest";
import { describeSetting } from "../../../src/inspect/index.ts";
import { setting } from "../../../src/object/index.ts";
import type { Setting } from "../../../src/setting/index.ts";
import type { Package } from "@destack/package";

/** A constructor of another package describing its declarations as settings too. */
export interface SettingConstructor {
    /** The constructor's package. */
    readonly package: Package;
    /** The constructor's name. */
    readonly name: string;
}

/** Open a release with a manifest that declares some settings through a constructor. */
export async function release(
    settings: readonly Setting[],
    constructor: SettingConstructor = { package: setting.package, name: "defineSetting" },
): Promise<BuildReader> {
    // record each setting as the build records a declaration
    const declarations = settings.map((setting) => ({
        name: setting.name,
        kind: "setting",
        package: setting.package,
        constructor: {
            package: constructor.package,
            symbol: { module: "src/declare/index.ts", name: constructor.name },
        },
        symbol: { package: setting.package, symbol: { module: "src/setting.ts", name: "setting" } },
        source: { file: "src/setting.ts", line: 0, column: 0 },
        description: describeSetting(setting),
    }));

    // hold them in the setting package's description collection
    const bytes = new TextEncoder().encode(JSON.stringify(declarations));
    const file = await describeFile("manifest/setting.json", "application/json", bytes);
    const descriptions = { setting: { package: setting.package, file } };

    return new BuildReader({ descriptions } as unknown as PackageManifest, async () => bytes);
}
