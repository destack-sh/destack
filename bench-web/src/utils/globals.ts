export const VERSION = "2024.03.23.0";
export const COMMIT = import.meta.env.VITE_VERCEL_GIT_COMMIT_SHA;

export const IS_DEBUG = import.meta.env.DEV;
export const SUPERVISOR_URL = import.meta.env.VITE_APP_SUPERVISOR_URL || "http://127.0.0.1:8080";
export const DISCORD_URL = "https://discord.gg/pSBdq6XC";
