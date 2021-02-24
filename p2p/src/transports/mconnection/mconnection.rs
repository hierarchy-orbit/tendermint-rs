//! `MConnection`: a Transport which multiplexes messages from different sources over a single TCP
//! stream.
//! Spec: https://github.com/tendermint/spec/blob/master/spec/p2p/connection.md#p2p-multiplex-connection

use std::net::{Incoming, Shutdown, SocketAddr, TcpStream};

use ed25519_dalek as ed25519;
use eyre::Result;

use super::super::super::secret_connection::{SecretConnection, Version};
use super::super::super::transport::*;

struct MConnectionTransport<'a> {
    incoming: MIncoming<'a>,
}

struct MConnection {
    public_key: ed25519::PublicKey,
    stream: TcpStream,
    secret_connection: SecretConnection<TcpStream>,
}

struct MEndpoint {}

struct MIncoming<'a> {
    tcp_incoming: Incoming<'a>,
}

impl MConnection {
    pub fn new(
        private_key: ed25519::Keypair,
        addr: SocketAddr,
        protocol_version: Version,
    ) -> Result<Self> {
        let mut stream = TcpStream::connect(addr)?;
        let public_key = private_key.public;
        let secret_connection = SecretConnection::new(stream, &private_key, protocol_version)?;

        Ok(Self {
            public_key,
            stream,
            secret_connection,
        })
    }
}

impl Connection for MConnection {
    type Error = std::io::Error;
    type Read = 
    type Write = 

    fn advertised_addrs(&self) -> Vec<SocketAddr> {}
    fn close(&self) -> Result<()> {
        self.stream.shutdown(Shutdown::Both)
    }
    fn local_addr(&self) -> SocketAddr {
        self.stream.local_addr().unwrap()
    }
    fn open_bidirectional(
        &self,
        stream_id: &StreamId,
    ) -> Result<(Self::Read, Self::Write), Self::Error> {
    }
    fn public_key(&self) -> tendermint::public_key::PublicKey {}
    fn remote_addr(&self) -> SocketAddr {}
}

impl Endpoint for MEndpoint {
    type Connection = MConnection;

    fn connect(&self, info: ConnectInfo) -> Result<Self::Connection> {}

    fn listen_addrs(&self) -> Vec<SocketAddr> {}
}

impl<'a> Iterator for MIncoming<'a> {
    type Item = Result<MConnection>;

    fn next(&mut self) -> Option<u32> {}
}

impl<'a> Transport for MConnectionTransport<'a> {
    type Connection = MConnection;
    type Endpoint = MEndpoint;
    type Incoming = MIncoming<'a>;

    fn bind(&self, bind_info: BindInfo) -> Result<(Self::Endpoint, Self::Incoming)> {}

    fn shutdown(&self) -> Result<()> {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_key_test() {}
}
