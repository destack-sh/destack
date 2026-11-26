import { createContext, useContext } from "solid-js";

/** An example or scenario a page shows, by its name and its kind. */
export interface IndexEntry {
    /** The name, as `<shown>.<member>`. */
    readonly name: string;
    /** Whether it is an example or a scenario. */
    readonly kind: "example" | "scenario";
}

/** The examples and scenarios of the example page an example renders on. */
export const ExampleIndexContext = createContext<readonly IndexEntry[]>([]);

/** Read the examples and scenarios of the example page an example renders on. */
export function useExampleIndex(): readonly IndexEntry[] {
    return useContext(ExampleIndexContext);
}
