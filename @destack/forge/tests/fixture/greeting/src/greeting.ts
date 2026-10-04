import { answer } from "@example/answer";

/** Greet someone with the answer. */
export function greet(name: string): string {
    return `hello ${name}, the answer is ${answer}`;
}
