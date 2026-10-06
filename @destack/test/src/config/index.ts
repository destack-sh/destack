export {
    defineConfiguration,
    defineProject,
    SCENARIO_MODULES,
    scenarioPlugin,
    type DriverReference,
} from "./config.ts";
export { configDefaults, mergeConfig } from "vitest/config";
export type {
    UserWorkspaceConfig as ProjectConfig,
    ViteUserConfigExport as Config,
} from "vitest/config";
