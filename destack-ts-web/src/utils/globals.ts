export const VERSION = "2025.07.11.1";

// global environment variables :DestackWebEnv
export const COMMIT = import.meta.env.VITE_COMMIT;
export const ENV = import.meta.env.VITE_ENVIRONMENT;
export const UNIVERSE_URL = import.meta.env.VITE_UNIVERSE_URL || "127.0.0.1:8080";
export const IP_API_KEY_B64 = import.meta.env.VITE_IP_API_KEY;
export const DISCORD_URL = import.meta.env.VITE_DISCORD_URL || "https://discord.gg/HUUzkfBn2p";

// other globals
export const GRPC_KEEPALIVE_INTERVAL_SECONDS = 60;
export const IP_API_KEY = IP_API_KEY_B64 ? atob(IP_API_KEY_B64) : null;
export const IS_DEV = ENV == null || ENV == "dev";
export const TELEMETRY = !IS_DEV;