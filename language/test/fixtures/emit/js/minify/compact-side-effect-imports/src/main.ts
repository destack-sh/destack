import "./external.ts";
import { describeSideEffect } from "./side-effect.ts";

console.log("entry side-effect import");
console.log(describeSideEffect("entry"));
