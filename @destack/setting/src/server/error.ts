import { ServiceError } from "@destack/service/error";
import { SettingError } from "../error/index.ts";

/** Translate setting failures into explicit service outcomes without hiding unrelated errors. */
export function reportSettingError(error: unknown): never {
    if (!(error instanceof SettingError)) {
        throw error;
    }

    switch (error.code) {
        case "INVALID_TARGET":
            throw new ServiceError("BAD_REQUEST", { message: error.message, cause: error });
        case "INVALID_VALUE":
            throw new ServiceError("PRECONDITION_FAILED", { message: error.message, cause: error });
        case "CONFLICT":
            throw new ServiceError("CONFLICT", { message: error.message, cause: error });
        case "STALE_POLICY":
            throw new ServiceError("SERVICE_UNAVAILABLE", { message: error.message, cause: error });
    }
}
