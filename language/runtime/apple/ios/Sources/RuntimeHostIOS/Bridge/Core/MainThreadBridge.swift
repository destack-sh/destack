import Foundation

/// Run one bridge operation on the main thread and return its result.
func runOnMainThread<T: Sendable>(
  _ body: @escaping @MainActor () -> T
) -> T {
  if Thread.isMainThread {
    return MainActor.assumeIsolated {
      body()
    }
  }

  let semaphore = DispatchSemaphore(value: 0)
  var result: T?

  DispatchQueue.main.async {
    result = MainActor.assumeIsolated {
      body()
    }
    semaphore.signal()
  }

  semaphore.wait()

  guard let result else {
    preconditionFailure("runtime bridge main thread hop returned no result")
  }

  return result
}
