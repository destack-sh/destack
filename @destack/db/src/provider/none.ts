import type { Provider } from "@destack/resource";

// TODO #Incomplete: open databases on workerd through a provider of its own
/** The providers opening databases on runtimes without a local provider. */
export const providers: Readonly<Record<string, (reference: URL) => Promise<Provider>>> = {};
