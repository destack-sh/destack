import { pretendReadonly } from "@/utils/ref";
import { ref } from "vue";

export const VERSION = "2024.06.27.1";
export const COMMIT = import.meta.env.VITE_VERCEL_GIT_COMMIT_SHA;

export const IS_DEV = import.meta.env.DEV;
export const SUPERVISOR_URL = import.meta.env.VITE_APP_SUPERVISOR_URL || "https://127.0.0.1:8080";
export const DISCORD_URL = "https://discord.gg/HUUzkfBn2p";

// actually synced from local, but we want a global we can safely import
export const isDeveloperMode = pretendReadonly(ref(IS_DEV));