import { greet, type Greeting } from "@destack/build-unit-library-fixture";

/** Greet the world, through the library. */
export function greetWorld(): Greeting {
    return greet("world");
}
