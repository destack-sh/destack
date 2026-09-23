import { implement } from "@destack/service/server";
import { ServiceError } from "@destack/service/error";
import { login } from "../service/login.ts";

/** Typed local login procedures, awaiting credential storage and browser authentication. */
const implementation = implement(login);

/** Implement retained sign-ins and independent browser authentication operations. */
export const loginRouter = implementation.router({
    /** Read a retained sign-in by its local identity. */
    get: implementation.get.handler(() => {
        // TODO #Incomplete: read secure local login metadata by its independent identifier
        throw new ServiceError("NOT_IMPLEMENTED");
    }),
    /** List retained identity descriptions. */
    list: implementation.list.handler(() => {
        // TODO #Incomplete: list retained sign-ins without reading secret credential values
        throw new ServiceError("NOT_IMPLEMENTED");
    }),
    /** Observe changes to available identities. */
    watch: implementation.watch.handler(() => {
        // TODO #Incomplete: observe the retained login set through sign-in, sign-out and renewal
        throw new ServiceError("NOT_IMPLEMENTED");
    }),
    /** Remove the selected login and report remote revocation. */
    signOut: implementation.signOut.handler(() => {
        // TODO #Incomplete: remove local credentials and report whether issuer revocation was confirmed
        throw new ServiceError("NOT_IMPLEMENTED");
    }),
    /** Complete browser authentication independently for each operation. */
    signIn: {
        /** Start browser authentication. */
        start: implementation.signIn.start.handler(() => {
            // TODO #Incomplete: start issuer authentication with state and PKCE, retain independent operation progress
            throw new ServiceError("NOT_IMPLEMENTED");
        }),
        /** Read current authentication progress. */
        get: implementation.signIn.get.handler(() => {
            // TODO #Incomplete: read retained sign-in progress without exposing credentials
            throw new ServiceError("NOT_IMPLEMENTED");
        }),
        /** Observe authentication progress through completion. */
        watch: implementation.signIn.watch.handler(() => {
            // TODO #Incomplete: observe sign-in progress through completion or cancellation
            throw new ServiceError("NOT_IMPLEMENTED");
        }),
        /** Cancel a pending authentication attempt. */
        cancel: implementation.signIn.cancel.handler(() => {
            // TODO #Incomplete: cancel the selected sign-in and release its callback listener
            throw new ServiceError("NOT_IMPLEMENTED");
        }),
    },
});
