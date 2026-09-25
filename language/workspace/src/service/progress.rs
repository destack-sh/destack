use std::future::Future;

use futures::{FutureExt, pin_mut, select_biased};
use tspp_rpc::{Code, Response, ResponseSender, Status};

use crate::{CommandError, ProgressEvent, ProgressEvents};

impl ProgressEvents {
    /// Forward progress until one command completes or its RPC call is canceled.
    pub(crate) async fn forward<T>(
        mut self,
        mut responses: ResponseSender<ProgressEvent>,
        command: impl Future<Output = Result<T, CommandError>> + Send,
    ) -> Result<Response<T>, Status> {
        let command = command.fuse();
        pin_mut!(command);

        loop {
            // wait for cancellation, command completion, or the next progress event
            let event = {
                let canceled = responses.canceled().fuse();
                let event = self.receive().fuse();
                pin_mut!(canceled, event);

                select_biased! {
                    _ = canceled => {
                        return Err(Status::new(Code::Canceled, "RPC call was canceled"));
                    },
                    output = command => {
                        return output.map(Response::new).map_err(Status::from);
                    },
                    event = event => event,
                }
            };
            let Some(event) = event else {
                let canceled = responses.canceled().fuse();
                pin_mut!(canceled);

                return select_biased! {
                    _ = canceled => {
                        Err(Status::new(Code::Canceled, "RPC call was canceled"))
                    },
                    output = command => output.map(Response::new).map_err(Status::from),
                };
            };

            // preserve RPC stream backpressure between progress events
            responses
                .send(&event)
                .await
                .map_err(|error| error.into_status())?;
        }
    }
}
