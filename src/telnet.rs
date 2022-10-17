use std::sync::Arc;

use tokio::{io::AsyncBufReadExt, net::UdpSocket};
use webrtc_util::Conn;

use crate::client::{binding_request, binding_response};

#[derive(clap::Parser, Debug)]
pub struct Args {
    server: String,
    /// Turn only
    username: Option<String>,
    /// Turn only
    password: Option<String>,
}
impl Args {
    pub fn main(self) {
        println!("{self:?}");
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(self.main_rt())
    }

    async fn main_rt(self) {
        let server = self.server;
        let credentials = self.username.and_then(|u| self.password.map(|p| (u, p)));
        let socket = UdpSocket::bind("0.0.0.0:0").await.unwrap();
        socket.set_ttl(128).unwrap();

        let addr;
        let conn: Box<dyn Conn>;
        match credentials {
            Some((username, password)) => {
                let client = turn::client::Client::new(turn::client::ClientConfig {
                    stun_serv_addr: server.to_string(),
                    turn_serv_addr: server.to_string(),
                    username,
                    password,
                    realm: Default::default(),
                    software: Default::default(),
                    rto_in_ms: Default::default(),
                    conn: Arc::new(socket),
                    vnet: Default::default(),
                })
                .await
                .unwrap();

                client.listen().await.unwrap();

                let turn_conn = client.allocate().await.unwrap();

                addr = turn_conn.local_addr().await.unwrap();
                conn = Box::new(turn_conn);
            }
            None => {
                socket.send_to(&binding_request(), &server).await.unwrap();
                let mut buf = vec![0; 4096];
                let (n, _) = socket.recv_from(&mut buf).await.unwrap();
                buf.truncate(n);

                addr = binding_response(&buf).unwrap();
                conn = Box::new(socket);
            }
        };

        println!("Local ip:");
        println!("{addr:?}");

        let stdin = tokio::io::stdin();
        let mut stdin = tokio::io::BufReader::new(stdin);

        let mut buf = vec![0; 4096];
        loop {
            let mut line = Default::default();
            buf.resize(4096, 0);
            tokio::select! {
                r = stdin.read_line(&mut line) => {
                    r.unwrap();
                    let peer = line.trim().parse().unwrap();
                    conn.send_to(line.as_bytes(), peer).await.unwrap();
                }
                r = conn.recv_from(&mut buf) => {
                    let (n, peer) = r.unwrap();
                    buf.truncate(n);

                    println!("{peer:?}: {buf:?}");
                }
            }
        }
    }
}

trait CommunicationTrait {}
