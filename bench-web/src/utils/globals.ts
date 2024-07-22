import { pretendReadonly } from "@/utils/ref";
import { ref } from "vue";

export const VERSION = "2024.07.22.0";

// global constants given from env :BenchWebEnv
export const COMMIT = import.meta.env.VITE_APP_COMMIT;
export const ENV = import.meta.env.VITE_APP_ENVIRONMENT;
export const SUPERVISOR_URL = import.meta.env.VITE_APP_SUPERVISOR_URL || "https://127.0.0.1:8080";
export const SENTRY_DSN = import.meta.env.VITE_APP_SENTRY_DSN;

export const IS_DEV = ENV == null || ENV == "dev";
export const DISCORD_URL = "https://discord.gg/HUUzkfBn2p";
// actually synced from local, but we want a global we can safely import
export const isDeveloperMode = pretendReadonly(ref(IS_DEV));
