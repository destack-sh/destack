import { AccountHandle } from "@destack/account/object";
import { HostName } from "../object/index.ts";

/** The parent domains of view origins and host addresses. */
export interface Domains {
    /** The domain of spaces' views, such as `destack.space`. */
    readonly space: string;
    /** The domain of hosts, such as `destack.computer`. */
    readonly host: string;
}

/** The parent domains of Destack's names. */
export const DOMAINS: Domains = { space: "destack.space", host: "destack.computer" };

/** A host of an account: `<host>.<handle>.destack.computer`. */
export interface HostAddress {
    /** The host's name within its account. */
    readonly host: string;
    /** The handle of the account the host belongs to. */
    readonly handle: string;
}

/** Read host addresses from server names. */
export const HostAddress = {
    /** Read the host address of a server name under a domain, absent for any other name. */
    parse(serverName: string, domain: string): HostAddress | undefined {
        // take the two labels below the domain
        const suffix = `.${domain}`;
        const labels = serverName.endsWith(suffix)
            ? serverName.slice(0, -suffix.length).split(".")
            : [];
        const [host, handle] = labels as [string, string];
        const isValid =
            labels.length === 2 &&
            HostName.safeParse(host).success &&
            AccountHandle.safeParse(handle).success;

        return isValid ? { host, handle } : undefined;
    },
};
