use std::net::{SocketAddr, UdpSocket};

use stun::{
    message::{Message, CLASS_REQUEST, METHOD_BINDING},
    xoraddr::XorMappedAddress,
};

#[derive(clap::Parser)]
pub struct Args {
    servers: Vec<String>,
}
impl Args {
    pub fn main(self) {
        let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
        for stun in self.servers {
            let data = binding_request();

            socket.send_to(&data, stun).unwrap();

            let mut buf = vec![0; 4096];
            let (n, _) = socket.recv_from(&mut buf).unwrap();
            buf.truncate(n);

            let ip = binding_response(&buf).unwrap();
            println!("{ip:?}");
        }

        println!("{:?}", socket.local_addr().unwrap());
    }
}

pub fn binding_request() -> Vec<u8> {
    let mut msg = Message::new();
    msg.typ.method = METHOD_BINDING;
    msg.typ.class = CLASS_REQUEST;
    msg.new_transaction_id().unwrap();
    msg.encode();

    msg.marshal_binary().unwrap()
}

pub fn binding_response(buf: &[u8]) -> Result<SocketAddr, stun::Error> {
    let mut msg = Message::new();
    msg.unmarshal_binary(buf)?;
    let mut xor = [XorMappedAddress::default()];
    msg.parse(&mut xor)?;

    Ok(SocketAddr::new(xor[0].ip, xor[0].port))
}
