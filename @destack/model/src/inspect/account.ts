import { AccountDefinition } from "../declare/account.ts";

/** Describe a declared account configuration for the package manifest. */
export function describeAccount(account: AccountDefinition): AccountDefinition {
    return AccountDefinition.parse(account);
}
