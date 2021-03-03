//! `MConnection`: a Transport which multiplexes messages from different sources over a single TCP
//! stream.
//! Spec: https://github.com/tendermint/spec/blob/master/spec/p2p/connection.md#p2p-multiplex-connection

use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::time::Duration;

use ed25519_dalek as ed25519;
use eyre::{Result, WrapErr};

use super::super::super::secret_connection::{SecretConnection, Version};
use super::super::super::transport::*;

struct MConnectionTransport {}

struct MConnection {
    public_key: ed25519::PublicKey,
    secret_connection: SecretConnection<TcpStream>,
    stream: TcpStream,
}

struct MEndpoint {
    private_key: Arc<ed25519::Keypair>,
    protocol_version: Version,
}

struct MIncoming {
    tcp_listener: TcpListener,
    private_key: Arc<ed25519::Keypair>,
    protocol_version: Version,
}

impl MConnection {
    /// Opens a TCP connection to a remote host and establishes a secret connection
    /// [`SecretConnection`].
    pub fn connect(
        addr: &SocketAddr,
        private_key: &ed25519::Keypair,
        protocol_version: Version,
    ) -> Result<Self> {
        let stream = TcpStream::connect(addr)?;
        let stream_clone = stream.try_clone()?;

        let public_key = private_key.public.clone();
        let secret_connection = SecretConnection::new(stream, &private_key, protocol_version)?;

        Ok(Self {
            public_key,
            secret_connection,
            stream: stream_clone,
        })
    }

    /// Opens a TCP connection to a remote host with a timeout and establishes a secret connection
    /// [`SecretConnection`].
    pub fn connect_timeout(
        addr: &SocketAddr,
        timeout: Duration,
        private_key: &ed25519::Keypair,
        protocol_version: Version,
    ) -> Result<Self> {
        let stream = TcpStream::connect_timeout(addr, timeout)?;
        let stream_clone = stream.try_clone()?;

        let public_key = private_key.public.clone();
        let secret_connection = SecretConnection::new(stream, &private_key, protocol_version)?;

        Ok(Self {
            public_key,
            secret_connection,
            stream: stream_clone,
        })
    }
}

impl Connection for MConnection {
    type Error = std::io::Error;
    type Read = SecretConnection<TcpStream>;
    type Write = SecretConnection<TcpStream>;

    fn advertised_addrs(&self) -> Vec<SocketAddr> {
        vec![]
    }

    fn close(&self) -> Result<()> {
        self.stream
            .shutdown(Shutdown::Both)
            .wrap_err("failed to shutdown")
    }

    fn local_addr(&self) -> SocketAddr {
        self.stream.local_addr().unwrap()
    }

    fn remote_addr(&self) -> SocketAddr {
        self.stream.peer_addr().unwrap()
    }

    fn open_bidirectional(
        &self,
        _stream_id: &StreamId,
    ) -> Result<(&Self::Read, &Self::Write), Self::Error> {
        Ok((&self.secret_connection, &self.secret_connection))
    }

    fn public_key(&self) -> tendermint::public_key::PublicKey {
        tendermint::PublicKey::Ed25519(self.public_key)
    }
}

impl Endpoint for MEndpoint {
    type Connection = MConnection;

    /// Connects to the specified address using either `MConnection::connect` or
    /// `MConnection::connect_timeout` depending on whenever `ConnectInfo.timeout` is zero or not.
    fn connect(&self, info: ConnectInfo) -> Result<Self::Connection> {
        if info.timeout > Duration::new(0, 0) {
            MConnection::connect_timeout(
                &info.addr,
                info.timeout,
                &self.private_key,
                self.protocol_version,
            )
        } else {
            MConnection::connect(&info.addr, &self.private_key, self.protocol_version)
        }
    }

    fn listen_addrs(&self) -> Vec<SocketAddr> {
        vec![]
    }
}

impl Iterator for MIncoming {
    type Item = Result<MConnection>;

    /// Advances the iterator and returns the next `MConnection`.
    fn next(&mut self) -> Option<Result<MConnection>> {
        let public_key = self.private_key.public;

        match self
            .tcp_listener
            .incoming()
            .next()
            .unwrap() // it's safe to unwrap here because Incoming never returns None
            .wrap_err("failed to accept conn")
        {
            Ok(stream) => {
                let stream_clone = stream.try_clone().wrap_err("failed to clone stream");
                match (
                    SecretConnection::new(stream, &self.private_key, self.protocol_version),
                    stream_clone,
                ) {
                    (Ok(secret_connection), Ok(stream_clone)) => Some(Ok(MConnection {
                        public_key,
                        secret_connection,
                        stream: stream_clone,
                    })),
                    (_, Err(e)) => Some(Err(e)),
                    (Err(e), _) => Some(Err(e)),
                }
            }
            Err(e) => Some(Err(e)),
        }
    }
}

impl Transport for MConnectionTransport {
    type Connection = MConnection;
    type Endpoint = MEndpoint;
    type Incoming = MIncoming;

    /// Creates a new `TcpListener` which will be bound to the specified address.
    /// The private key will be used to establish a `SecretConnection` each time you connect or
    /// accept a connection.
    ///
    /// See `TcpListener::bind`
    ///
    /// # Examples
    ///
    /// Creates a TCP listener bound to `127.0.0.1:80`:
    ///
    /// ```no_run
    ///use rand::rngs::OsRng;
    /// use ed25519_dalek::Keypair;
    /// use ed25519_dalek::Signature;
    ///
    /// let mut csprng = OsRng{};
    ///
    /// let t = MConnectionTransport{}
    /// let (endpoint, incoming) = t.bind(BindInfo{
    ///     addr: "127.0.0.1:80",
    ///     private_key: Keypair::generate(&mut csprng),
    /// }).unwrap();
    /// ```
    fn bind(&self, bind_info: BindInfo) -> Result<(MEndpoint, MIncoming)> {
        let listener = TcpListener::bind(bind_info.addr)?;
        let pk = Arc::new(bind_info.private_key);
        Ok((
            MEndpoint {
                private_key: pk.clone(),
                protocol_version: bind_info.protocol_version,
            },
            MIncoming {
                tcp_listener: listener,
                private_key: pk.clone(),
                protocol_version: bind_info.protocol_version,
            },
        ))
    }

    /// Noop.
    fn shutdown(&self) -> Result<()> {
        // The socket will be closed when the MConnectionTransport is dropped.
        Ok(())
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn bind_and_connect() {}

//     #[test]
//     fn bind_and_connect_timeout() {}

//     #[test]
//     fn bind_and_accept() {}
// }
