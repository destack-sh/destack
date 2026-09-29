import type { AccountDeclaration, AccountDefinition } from "../declare/account.ts";

/** Describe a declared account configuration for the package manifest. */
export function describeAccount(account: AccountDeclaration): AccountDefinition {
    return account.definition;
}
