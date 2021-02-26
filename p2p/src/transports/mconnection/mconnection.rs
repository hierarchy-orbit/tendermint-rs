//! `MConnection`: a Transport which multiplexes messages from different sources over a single TCP
//! stream.
//! Spec: https://github.com/tendermint/spec/blob/master/spec/p2p/connection.md#p2p-multiplex-connection

use std::net::{Incoming, Shutdown, SocketAddr, TcpListener, TcpStream};
use std::time::Duration;

use ed25519_dalek as ed25519;
use eyre::{Result, WrapErr};

use super::super::super::secret_connection::{SecretConnection, Version};
use super::super::super::transport::*;

struct MConnectionTransport<'a> {
    incoming: MIncoming<'a>,
    endpoint: MEndpoint,
}

struct MConnection {
    public_key: ed25519::PublicKey,
    secret_connection: SecretConnection<TcpStream>,
    stream: TcpStream,
}

struct MEndpoint {
    private_key: ed25519::Keypair,
    protocol_version: Version,
}

struct MIncoming<'a> {
    tcp_incoming: Incoming<'a>,
    // TODO: duplicate fields between MEndpoint and MIncoming
    private_key: ed25519::Keypair,
    protocol_version: Version,
}

impl MConnection {
    /// Opens a TCP connection to a remote host and establishes a secret connection
    /// [`SecretConnection`].
    pub fn connect(
        addr: &SocketAddr,
        private_key: ed25519::Keypair,
        protocol_version: Version,
    ) -> Result<Self> {
        let mut stream = TcpStream::connect(addr)?;

        let public_key = private_key.public;
        let secret_connection = SecretConnection::new(stream, &private_key, protocol_version)?;

        Ok(Self {
            public_key,
            secret_connection,
            stream,
        })
    }

    /// Opens a TCP connection to a remote host with a timeout and establishes a secret connection
    /// [`SecretConnection`].
    pub fn connect_timeout(
        addr: &SocketAddr,
        timeout: Duration,
        private_key: ed25519::Keypair,
        protocol_version: Version,
    ) -> Result<Self> {
        let mut stream = TcpStream::connect_timeout(addr, timeout)?;
        let public_key = private_key.public;
        let secret_connection = SecretConnection::new(stream, &private_key, protocol_version)?;

        Ok(Self {
            public_key,
            secret_connection,
            stream,
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
        stream_id: &StreamId,
    ) -> Result<(Self::Read, Self::Write), Self::Error> {
        Ok((self.secret_connection, self.secret_connection))
    }

    fn public_key(&self) -> tendermint::public_key::PublicKey {
        tendermint::PublicKey::Ed25519(self.public_key)
    }
}

impl Endpoint for MEndpoint {
    type Connection = MConnection;

    fn connect(&self, info: ConnectInfo) -> Result<Self::Connection> {
        if info.timeout > Duration::new(0, 0) {
            MConnection::connect_timeout(
                &info.addr,
                info.timeout,
                self.private_key,
                self.protocol_version,
            )
        } else {
            MConnection::connect(&info.addr, self.private_key, self.protocol_version)
        }
    }

    fn listen_addrs(&self) -> Vec<SocketAddr> {
        vec![]
    }
}

impl<'a> Iterator for MIncoming<'a> {
    type Item = Result<MConnection>;

    fn next(&mut self) -> Option<Result<MConnection>> {
        let public_key = self.private_key.public;

        match self
            .tcp_incoming
            .next()
            .unwrap() // it's safe to unwrap here because Incoming never returns None
            .wrap_err("failed to accept conn")
        {
            Ok(stream) => {
                match SecretConnection::new(stream, &self.private_key, self.protocol_version) {
                    Ok(secret_connection) => Some(Ok(MConnection {
                        public_key,
                        secret_connection,
                        stream,
                    })),
                    Err(e) => Some(Err(e)),
                }
            }
            Err(e) => Some(Err(e)),
        }
    }
}

impl<'a> Transport for MConnectionTransport<'a> {
    type Connection = MConnection;
    type Endpoint = MEndpoint;
    type Incoming = MIncoming<'a>;

    fn bind(&self, bind_info: BindInfo) -> Result<(MEndpoint, MIncoming<'a>)> {
        let listener = TcpListener::bind(bind_info.addr)?;
        Ok((
            MEndpoint {
                private_key: bind_info.private_key,
                protocol_version: bind_info.protocol_version,
            },
            MIncoming {
                tcp_incoming: listener.incoming(),
                private_key: bind_info.private_key,
                protocol_version: bind_info.protocol_version,
            },
        ))
    }

    fn shutdown(&self) -> Result<()> {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_key_test() {}
}
