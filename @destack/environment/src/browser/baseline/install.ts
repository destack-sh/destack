import { BrowserContext } from "../context/context.ts";
import { installBaseline } from "./baseline.ts";

// install the baseline and the realm's context once, as the first module of every browser entry
installBaseline();
BrowserContext.install();
