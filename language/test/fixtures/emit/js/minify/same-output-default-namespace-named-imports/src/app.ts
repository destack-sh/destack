import runtime, * as runtimeNamespace from "./runtime.ts";
import { embeddedFiles } from "./runtime.ts";

console.log(typeof embeddedFiles);
console.log(typeof runtime.argv);
console.log(typeof runtimeNamespace.argv);
