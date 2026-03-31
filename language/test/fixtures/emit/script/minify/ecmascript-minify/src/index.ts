import "./runtime/side-effect.ts";

const inlined = 3;
const message = getMessage();
const sideEffectLabel = getSideEffectLabel();

console.log("Hello," + " world!", inlined, message, sideEffectLabel);
console.log(message, sideEffectLabel);

function getMessage() {
    return "Hello";
}

function getSideEffectLabel() {
    return "side-effect";
}
