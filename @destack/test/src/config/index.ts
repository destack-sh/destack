export {
    defineConfiguration,
    defineProject,
    BENCHMARK_MODULES,
    SCENARIO_MODULES,
    scenarioPlugin,
    type DriverReference,
    type ProjectConfiguration,
    type TestConfiguration,
} from "./config.ts";
export type { TestMatrix } from "../project/index.ts";
export { configDefaults, mergeConfig } from "vitest/config";
export type {
    UserWorkspaceConfig as ProjectConfig,
    ViteUserConfigExport as Config,
} from "vitest/config";
