import { Span, SpanStatusCode, Tracer, trace } from "@opentelemetry/api";

export function setupTelemetry() {
  // TODO :Telemetry: destack-ts telemetry
}

export function traceFunction<T extends (...args: any[]) => any>(
  tracer: Tracer,
  name: string,
  fn: T,
): T {
  return ((...args: Parameters<T>) => {
    return tracer.startActiveSpan(name, (span: Span) => {
      try {
        const result = fn(...args);
        // handle async functions
        if (result && typeof result.then === "function") {
          return result
            .catch((err: Error) => {
              span.recordException(err);
              span.setStatus({ code: SpanStatusCode.ERROR });
              throw err;
            })
            .finally(() => {
              span.end();
            });
        }
        // handle sync functions
        span.end();
        return result;
      } catch (err) {
        span.recordException(err as Error);
        span.setStatus({ code: SpanStatusCode.ERROR });
        span.end();
        throw err;
      }
    });
  }) as T;
}

export function getTracer(name: string) {
  return trace.getTracer(name);
}
