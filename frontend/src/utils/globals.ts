import { ref } from "vue";

export const VERSION = VITE_APP_VERSION;
export const COMMIT = VITE_APP_GIT_COMMIT || "dev";

export const API_BASE_URL = import.meta.env.VITE_APP_API_BASE_URL || "127.0.0.1:8000";
// only use SSL if not localhost
export const HTTP_API_BASE_URL = API_BASE_URL.includes("127.0.0.1")
  ? "http://127.0.0.1:8000"
  : "https://" + API_BASE_URL;
export const WS_API_BASE_URL = API_BASE_URL.includes("127.0.0.1") ? "ws://127.0.0.1:8000" : "wss://" + API_BASE_URL;

export const IS_LOCALHOST = HTTP_API_BASE_URL.includes("127.0.0.1");
export const IS_DEBUG = import.meta.env.MODE === "development";

// Can't define these in main because it would create a circular dependency.
export const WS_CONNECTED = ref(false); // auto-set in main.ts, read-only elsewhere

export const ACTIVE_SHARING_TOKEN = ref<string | null>(null);

export const INIT_MONACO = ref(false);
