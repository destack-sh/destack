package dev.destack.runtime.android.bridge

import dev.destack.runtime.android.core.hostStatusNotFound
import dev.destack.runtime.android.core.hostStatusOk

/**
 * Validate one runtime ingress status for one bridge event delivery.
 */
internal fun validateRuntimeIngressStatus(
    status: RuntimeIngressStatus,
    operation: String,
) {
    if (status.code == hostStatusOk || status.code == hostStatusNotFound) {
        return
    }

    error(
        "runtime bridge could not deliver $operation: code ${status.code}, error ${status.errorId}",
    )
}
