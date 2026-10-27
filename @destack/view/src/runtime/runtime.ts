import { installBaseline } from "./baseline/index.ts";

export * from "@solidjs/web";

// install the web-standard APIs the Destack browser baseline lacks before any view code runs
installBaseline();
