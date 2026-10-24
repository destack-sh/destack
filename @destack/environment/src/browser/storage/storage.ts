/** A browser's storage: Web Storage's key-value pairs and the Storage Standard's persistence. */
export interface Storage {
    /** Read a key's value, null without one. */
    readonly getItem: (key: string) => string | null;
    /** Write a key's value. */
    readonly setItem: (key: string, value: string) => void;
    /** Ask the browser to keep the origin's storage under pressure, answering whether it will. */
    readonly persist: () => Promise<boolean>;
}

/** Key-value storages. */
export const Storage: { readonly system: Storage } = {
    /** The browser's local storage and storage manager. */
    system: {
        getItem: (key) => localStorage.getItem(key),
        setItem: (key, value) => localStorage.setItem(key, value),
        persist: () => navigator.storage.persist(),
    },
};
