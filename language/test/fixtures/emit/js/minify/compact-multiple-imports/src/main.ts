import "./external-1.ts";
import "./external-2.ts";
import { value as a } from "./external-3.ts";
import { value as b } from "./external-4.ts";
import { suffix } from "./external-4.ts";
import { describe } from "./external-5.ts";
import "./external-5.ts";

capture(a);
capture(b);
capture(describe(a, b, suffix));
