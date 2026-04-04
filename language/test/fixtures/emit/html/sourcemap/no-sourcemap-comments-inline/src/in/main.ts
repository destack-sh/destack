import { greeting } from "./utils/strings.ts";
import { formatDate } from "./utils/date.ts";
import { renderSummary } from "./utils/summary.ts";

console.log(greeting("World"));
console.log(formatDate(new Date()));
console.log(renderSummary("World"));
