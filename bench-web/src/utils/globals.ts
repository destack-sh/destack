import { pretendReadonly } from "@/utils/ref";
import { ref } from "vue";

export const VERSION = "2025.05.04.2";

// global environment variables :BenchWebEnv
export const COMMIT = import.meta.env.VITE_COMMIT;
export const ENV = import.meta.env.VITE_ENVIRONMENT;
export const SUPERVISOR_URL = import.meta.env.VITE_SUPERVISOR_URL || "127.0.0.1:8080";
export const IP_API_KEY_B64 = import.meta.env.VITE_IP_API_KEY;
export const DISCORD_URL = import.meta.env.VITE_DISCORD_URL || "https://discord.gg/HUUzkfBn2p";

// other globals
export const GRPC_KEEPALIVE_INTERVAL_SECONDS = 60;
export const IP_API_KEY = IP_API_KEY_B64 ? atob(IP_API_KEY_B64) : null;
export const IS_DEV = ENV == null || ENV == "dev";
export const IS_DEVELOPER_MODE = pretendReadonly(ref(IS_DEV)); // hoisted for safe importing