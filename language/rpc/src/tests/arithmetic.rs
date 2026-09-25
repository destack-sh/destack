use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use serde::{Deserialize, Serialize};
use tspp_serde::{Reflect, Schema, Type};

use crate::{
    Code, MethodFingerprint, MethodId, MethodSchema, Response, ServerCall, Service, ServiceFuture,
    ServiceSchema, Status,
};

/// Request sent to the arithmetic test service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub(super) struct NumberRequest {
    /// Initial number.
    pub(super) value: u32,
}

/// Response returned by the arithmetic test service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub(super) struct NumberResponse {
    /// Computed number.
    pub(super) value: u32,
}

/// Large request exercising transparent deferred payload assembly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub(super) struct BlobRequest {
    /// Request bytes.
    pub(super) bytes: Vec<u8>,
}

/// Request used only by an added service method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
struct ExtraRequest {
    /// Extra request value.
    value: bool,
}

/// Arithmetic service exercising unary and bidirectional calls.
#[derive(Debug)]
pub(super) struct ArithmeticService {
    /// Canonical service schema.
    pub(super) schema: ServiceSchema,
    /// Unary method identifier.
    pub(super) double: MethodId,
    /// Bidirectional method identifier.
    pub(super) sum: MethodId,
    /// Client-streaming method that completes before reading its input.
    pub(super) early: MethodId,
    /// Large unary method identifier.
    pub(super) blob: MethodId,
}

impl ArithmeticService {
    /// Build the arithmetic service and its canonical schema.
    pub(super) fn new() -> Self {
        Self::build(false, false)
    }

    /// Build the arithmetic service with one unrelated added method.
    pub(super) fn with_extra_method() -> Self {
        Self::build(true, false)
    }

    /// Build the arithmetic service with one changed method contract.
    pub(super) fn with_changed_method() -> Self {
        Self::build(false, true)
    }

    /// Return one typed method's exact contract fingerprint.
    pub(super) fn fingerprint(&self, method: MethodId) -> MethodFingerprint {
        self.schema
            .method(method)
            .expect("registered method")
            .fingerprint()
    }

    /// Build one exact arithmetic service declaration.
    fn build(has_extra_method: bool, has_changed_method: bool) -> Self {
        let service = "test.Arithmetic";
        let double = MethodId::for_name(service, "double");
        let sum = MethodId::for_name(service, "sum");
        let early = MethodId::for_name(service, "early");
        let blob = MethodId::for_name(service, "blob");
        let mut types = Schema::default();
        let request = types.register::<NumberRequest>();
        let response = types.register::<NumberResponse>();
        let blob_request = types.register::<BlobRequest>();
        let extra_request = has_extra_method.then(|| types.register::<ExtraRequest>());
        let number = Type::Unsigned { bits: 32 };
        let double_response = if has_changed_method {
            Type::Bool
        } else {
            response.clone()
        };
        let mut methods = vec![
            MethodSchema::unary(service, "double", request.clone(), double_response, &types)
                .expect("valid unary method"),
            MethodSchema::bidirectional_streaming(
                service,
                "sum",
                request,
                response.clone(),
                number.clone(),
                number,
                &types,
            )
            .expect("valid bidirectional method"),
            MethodSchema::client_streaming(
                service,
                "early",
                Type::Unit,
                response.clone(),
                Type::Unsigned { bits: 32 },
                &types,
            )
            .expect("valid client-streaming method"),
            MethodSchema::unary(
                service,
                "blob",
                blob_request,
                Type::Unsigned { bits: 32 },
                &types,
            )
            .expect("valid unary method"),
        ];

        // add one unrelated method when requested
        if let Some(extra_request) = extra_request {
            methods.push(
                MethodSchema::unary(service, "extra", extra_request, Type::Bool, &types)
                    .expect("valid unary method"),
            );
        }
        let schema = ServiceSchema::new(service, methods, types).expect("valid service schema");

        Self {
            schema,
            double,
            sum,
            early,
            blob,
        }
    }
}

impl Service for ArithmeticService {
    /// Return the arithmetic service schema.
    fn schema(&self) -> &ServiceSchema {
        &self.schema
    }

    /// Execute one arithmetic request.
    fn call(&self, mut request: ServerCall) -> ServiceFuture<'_> {
        Box::pin(async move {
            match request.method() {
                // execute the unary method
                method if method == self.double => {
                    let initial: NumberRequest = request.decode()?;
                    YieldOnce::new().await;
                    let response = NumberResponse {
                        value: initial.value * 2,
                    };

                    request.respond(Response::new(response))
                }
                // execute the bidirectional method
                method if method == self.sum => {
                    let initial: NumberRequest = request.decode()?;
                    let mut outputs = request.response_sender::<u32>()?;
                    let mut inputs = request.request_stream::<u32>()?;
                    let mut total = initial.value;
                    while let Some(value) = inputs.receive().await? {
                        total += value;
                        outputs.send(&total).await?;
                    }

                    request.respond(Response::new(NumberResponse { value: total }))
                }
                // complete without consuming caller input
                method if method == self.early => {
                    request.respond(Response::new(NumberResponse { value: 42 }))
                }
                // execute the large unary method
                method if method == self.blob => {
                    let blob: BlobRequest = request.decode()?;

                    request.respond(Response::new(blob.bytes.len() as u32))
                }
                // reject unknown methods
                _ => Err(Status::new(Code::Unimplemented, "unknown method").into()),
            }
        })
    }
}

/// One future that requests and requires a second cooperative poll.
#[derive(Debug, Default)]
struct YieldOnce {
    /// Whether the first poll already yielded.
    has_yielded: bool,
}

impl YieldOnce {
    /// Create one future ready to yield.
    fn new() -> Self {
        Self::default()
    }
}

impl Future for YieldOnce {
    type Output = ();

    /// Yield once before completing.
    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        if self.has_yielded {
            Poll::Ready(())
        } else {
            self.has_yielded = true;
            context.waker().wake_by_ref();

            Poll::Pending
        }
    }
}
