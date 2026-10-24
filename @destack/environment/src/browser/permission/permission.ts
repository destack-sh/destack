/** A browser's permission states, as the Permissions API has them. */
export interface Permissions {
    /** Read a permission's status, which follows its changes. */
    readonly query: (descriptor: PermissionDescriptor) => Promise<PermissionStatus>;
}

/** Permission sources. */
export const Permissions: { readonly system: Permissions } = {
    /** The browser's permissions. */
    system: {
        query: (descriptor) => navigator.permissions.query(descriptor),
    },
};
