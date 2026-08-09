use super::arithmetic::{NumberRequest, NumberResponse};
use super::server::TestServer;
use crate::{Idempotency, MethodId, Request, RequestStream, Response, ResponseSender, Status};

/// Generated arithmetic service used to exercise the service declaration.
#[crate::service(name = "test.GeneratedArithmetic")]
trait GeneratedArithmeticService {
    /// Double one number.
    #[rpc(name = "Double", idempotency = "no_side_effects")]
    fn double(request: NumberRequest) -> NumberResponse;

    /// Accumulate caller values and return every intermediate total.
    #[rpc(name = "Sum", request_stream = u32, response_stream = u32)]
    fn sum(request: NumberRequest) -> NumberResponse;
}

/// Generated arithmetic service implementation.
#[derive(Debug)]
struct GeneratedArithmetic;

impl GeneratedArithmeticService for GeneratedArithmetic {
    /// Double one number.
    async fn double(
        &self,
        request: Request<NumberRequest>,
    ) -> Result<Response<NumberResponse>, Status> {
        let response = NumberResponse {
            value: request.value.value * 2,
        };

        Ok(Response::new(response))
    }

    /// Accumulate caller values and return every intermediate total.
    async fn sum(
        &self,
        request: Request<NumberRequest>,
        mut requests: RequestStream<u32>,
        mut responses: ResponseSender<u32>,
    ) -> Result<Response<NumberResponse>, Status> {
        let mut total = request.value.value;
        while let Some(value) = requests
            .receive()
            .await
            .map_err(|error| error.into_status())?
        {
            total += value;
            responses
                .send(&total)
                .await
                .map_err(|error| error.into_status())?;
        }

        Ok(Response::new(NumberResponse { value: total }))
    }
}

/// Generate and execute one typed service client and server.
#[test]
fn test_generate_service() {
    let service = GeneratedArithmeticServer::new(GeneratedArithmetic).expect("build server");
    let server = TestServer::new(service);
    let client =
        GeneratedArithmeticClient::new(server.connection().clone()).expect("create client");

    let response = client
        .double(NumberRequest { value: 21 })
        .expect("double number");

    assert_eq!(response.value, NumberResponse { value: 42 });
    assert_eq!(
        client
            .schema()
            .method(MethodId::for_name("test.GeneratedArithmetic", "Double",))
            .expect("registered double method")
            .idempotency(),
        Idempotency::NoSideEffects
    );

    // execute the generated bidirectional path through its cached descriptor
    let mut call = client
        .sum(NumberRequest { value: 1 })
        .expect("start generated sum");
    call.send(&2).expect("send first input");
    call.send(&3).expect("send second input");
    call.close_input().expect("close generated input");

    assert_eq!(call.receive().expect("receive first total"), Some(3));
    assert_eq!(call.receive().expect("receive second total"), Some(6));
    assert_eq!(call.receive().expect("finish generated output"), None);
    assert_eq!(
        call.response().expect("receive generated response").value,
        NumberResponse { value: 6 }
    );

    drop(client);
    server.close();
}
