import Foundation
import RuntimeHostAppleCore

/// Validate one runtime ingress status for one bridge event delivery.
func validateRuntimeIngressStatus(
  _ status: RuntimeIngressStatus,
  operation: String
) {
  if status.code == hostStatusOk || status.code == hostStatusNotFound {
    return
  }

  preconditionFailure(
    "runtime bridge could not deliver \(operation): code \(status.code), error \(status.errorID)"
  )
}
