import { ref } from "vue";

// Can't define these in main because it would create a circular dependency.
export const WS_CONNECTED = ref(false); // auto-set in main.ts, read-only elsewhere
