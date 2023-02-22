import { ref } from "vue";

export const API_BASE_URL = import.meta.env.VITE_APP_API_BASE_URL || "localhost:8000";

// Can't define these in main because it would create a circular dependency.
export const WS_CONNECTED = ref(false); // auto-set in main.ts, read-only elsewhere
