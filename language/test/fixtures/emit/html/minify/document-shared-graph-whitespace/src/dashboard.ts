import { largeModule } from "./large-module.ts";
import { sharedLabel, sharedUtil } from "./shared.ts";

console.log("About page");
console.log(sharedLabel);
console.log(largeModule());
sharedUtil();
