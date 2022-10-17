use std::net::{SocketAddr, UdpSocket};

use stun::{message::Message, xoraddr::XorMappedAddress};

use crate::Error;

#[derive(clap::Parser)]
pub struct Args {
    listen: String,
}
impl Args {
    pub fn main(self) {
        let socket = UdpSocket::bind(self.listen).unwrap();

        let mut buf = Vec::new();
        loop {
            buf.resize(4096, 0);
            let (n, from) = socket.recv_from(&mut buf).unwrap();
            buf.truncate(n);
            if let Err(e) = Self::handle_one(&buf, from, &socket) {
                eprintln!("{e:?}");
            }
        }
    }

    fn handle_one(buf: &[u8], from: SocketAddr, socket: &UdpSocket) -> Result<(), Error> {
        let mut msg = Message::default();
        msg.unmarshal_binary(buf)?;
        println!("{msg:?}");
        msg.attributes.0.clear();
        msg.build(&[Box::new(XorMappedAddress {
            ip: from.ip(),
            port: from.port(),
        })])?;
        msg.encode();
        println!("answer {from:?} {msg:?}");
        let buf = msg.marshal_binary()?;
        socket.send_to(&buf, from)?;

        Ok(())
    }
}
