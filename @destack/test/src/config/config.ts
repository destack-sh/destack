import * as vitest from "vitest/config";
import type { UserWorkspaceConfig, ViteUserConfig } from "vitest/config";
import { modulePlugin } from "@destack/package/transform/vite";

/** Define a test configuration whose package sources receive Destack module metadata. */
/* oxlint-disable-next-line destack/prevent-abbreviations -- mirrors the vitest defineConfig export */
export function defineConfig(configuration: ViteUserConfig): ViteUserConfig {
    return vitest.defineConfig({
        ...configuration,
        plugins: [modulePlugin(), ...(configuration.plugins ?? [])],
    });
}

/** Define a test project whose package sources receive Destack module metadata. */
export function defineProject(configuration: UserWorkspaceConfig): UserWorkspaceConfig {
    return vitest.defineProject({
        ...configuration,
        plugins: [modulePlugin(), ...(configuration.plugins ?? [])],
    });
}
