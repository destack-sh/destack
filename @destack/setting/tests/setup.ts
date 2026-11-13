import { openSettings } from "./fixture/store.ts";

/** A store opened and closed before a file's tests, so its first test pays no process warm-up. */
const warmed = await openSettings();
await warmed[Symbol.asyncDispose]();
