import { installBaseline } from "./baseline.ts";

// install the baseline once, as the first module a browser entry evaluates
installBaseline();
