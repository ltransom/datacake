use std::io;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use hyper::client::conn::http2::SendRequest;
use http_body_util::Full;
use bytes::Bytes;
use tokio::sync::{Mutex, OnceCell};
use tokio::time::timeout;

use crate::net::Error;

#[derive(Clone)]
/// A client used for simulation testing via turmoil.
///
/// This is not a production grade client and is only really meant for testing not
/// performance.
pub struct LazyClient {
    addr: SocketAddr,
    client: Arc<OnceCell<Mutex<SendRequest<Full<Bytes>>>>>,
}

impl LazyClient {
    /// Creates a new lazy client.
    pub fn connect(socket: SocketAddr) -> Self {
        Self {
            addr: socket,
            client: Arc::new(OnceCell::new()),
        }
    }

    /// Ensures the connection is initialised and ready to handle events.
    pub async fn get_or_init(&self) -> Result<&Mutex<SendRequest<Full<Bytes>>>, Error> {
        if let Some(existing) = self.client.get() {
            return Ok(existing);
        }

        let io = timeout(
            Duration::from_secs(2),
            turmoil::net::TcpStream::connect(self.addr),
        )
        .await
        .map_err(|_| {
            Error::Io(io::Error::new(
                ErrorKind::TimedOut,
                "Failed to connect within deadline",
            ))
        })??;

        let (sender, connection) = hyper::client::conn::http2::Builder::new(hyper_util::rt::TokioExecutor::new())
            .keep_alive_while_idle(true)
            .adaptive_window(true)
            .handshake(hyper_util::rt::TokioIo::new(io))
            .await?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                error!(error = ?e, "Error in client connection");
            }
        });

        self.client.set(Mutex::new(sender)).unwrap();
        Ok(self.client.get().unwrap())
    }
}
