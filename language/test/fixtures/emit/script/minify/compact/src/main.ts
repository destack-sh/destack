import value from "./external.ts";
import * as self from "./main.ts";
import { renderCompactMessage } from "./render.ts";

const compactValues = [
    renderCompactMessage("alpha", value),
    renderCompactMessage("beta", value),
];

console.log(self, compactValues.join("|"));

export default function foo() {
    console.log(value);
}
