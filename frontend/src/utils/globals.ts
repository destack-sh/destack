import { ref } from "vue";

export const HTTP_API_BASE_URL = import.meta.env.VITE_APP_API_BASE_URL?.includes("127.0.0.1")
  ? "https://" + import.meta.env.VITE_APP_API_BASE_URL
  : "http://127.0.0.1:8000";

export const IS_LOCALHOST = HTTP_API_BASE_URL.includes("127.0.0.1");

// Can't define these in main because it would create a circular dependency.
export const WS_CONNECTED = ref(false); // auto-set in main.ts, read-only elsewhere
