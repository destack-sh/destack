export {
    defineConfiguration,
    TEST_TIERS,
    TIERS_VARIABLE,
    RESULTS_VARIABLE,
    type TestTier,
    scenarioPlugin,
    type ProjectConfiguration,
    type TestConfiguration,
} from "./config.ts";
export type { TestMatrix } from "../project/index.ts";
export { configDefaults, mergeConfig } from "vitest/config";
export type {
    UserWorkspaceConfig as ProjectConfig,
    ViteUserConfigExport as Config,
} from "vitest/config";
