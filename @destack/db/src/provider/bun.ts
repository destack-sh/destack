import type { Provider } from "@destack/resource";

/** The providers opening databases on Bun: SQLite files beside a reference, loaded on first bind. */
export const providers: Readonly<Record<string, (reference: URL) => Promise<Provider>>> = {
    sqlite: async (reference) => {
        const { sqliteProvider } = await import("../sqlite/provider.ts");

        return sqliteProvider(new URL("./", reference));
    },
};
