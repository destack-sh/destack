import { pretendReadonly } from "@/utils/ref";
import { ref } from "vue";

export const VERSION = "2024.08.05.0";

// global constants given from env :BenchWebEnv

export const COMMIT = import.meta.env.VITE_APP_COMMIT;
export const ENV = import.meta.env.VITE_APP_ENVIRONMENT;
export const SUPERVISOR_URL = import.meta.env.VITE_APP_SUPERVISOR_URL || "127.0.0.1:8080";
export const SENTRY_DSN = import.meta.env.VITE_APP_SENTRY_DSN;
export const GRPC_KEEPALIVE_INTERVAL = import.meta.env.VITE_APP_GRPC_KEEPALIVE_INTERVAL ?? 60;

export const IS_DEV = ENV == null || ENV == "dev";
// actually synced from local, but we want a global we can safely import
export const isDeveloperMode = pretendReadonly(ref(IS_DEV));

export const DISCORD_URL = "https://discord.gg/HUUzkfBn2p";
const IP_API_KEY_B64 = import.meta.env.VITE_APP_IP_API_KEY;
export const IP_API_KEY = IP_API_KEY_B64 ? atob(IP_API_KEY_B64) : null;
