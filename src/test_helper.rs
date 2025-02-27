use std::sync::mpsc::{channel, Receiver};
use std::thread;
use std::time::Duration;
use tiny_http::{Response, Server};

/// Captures details about an incoming HTTP request.
#[derive(Debug)]
pub struct RequestDetails {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
}

/// A simple test server that listens on a given port, captures one request, and responds.
pub struct TestWebserver {
    rx: Receiver<RequestDetails>,
    /// The join handle of the server thread. Call `join()` when done.
    pub join_handle: thread::JoinHandle<()>,
}

impl TestWebserver {
    /// Starts the server on the given port.
    pub fn start(port: u16) -> Self {
        let (tx, rx) = channel();
        let address = format!("0.0.0.0:{}", port);

        let join_handle = thread::spawn(move || {
            let server = Server::http(&address).unwrap();
            if let Some(request) = server.incoming_requests().next() {
                // Capture request details.
                let details = RequestDetails {
                    method: request.method().to_string(),
                    url: request.url().to_string(),
                    headers: request
                        .headers()
                        .iter()
                        .map(|h| (h.field.as_str().to_string(), h.value.as_str().to_string()))
                        .collect(),
                };
                // Send the captured details to the test.
                let _ = tx.send(details);

                let response = Response::from_string("OK");
                let _ = request.respond(response);
            }
        });

        Self { rx, join_handle }
    }

    /// Attempts to receive the request details with the given timeout.
    pub fn get_request_details(&self, timeout: Duration) -> Option<RequestDetails> {
        self.rx.recv_timeout(timeout).ok()
    }
}
