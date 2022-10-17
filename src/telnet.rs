use crate::{
    client::{binding_request, binding_response},
    Error,
};
use std::{net::SocketAddr, sync::Arc};
use tokio::{
    io::AsyncBufReadExt,
    net::{lookup_host, UdpSocket},
};
use turn::client::Client;
use webrtc_util::Conn;

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
        log::info!("{self:?}");
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        if let Err(e) = rt.block_on(self.main_rt()) {
            log::error!("{e:?}");
        }
    }

    async fn main_rt(self) -> Result<(), Error> {
        let server = self.server;
        let credentials = self.username.and_then(|u| self.password.map(|p| (u, p)));
        let socket = UdpSocket::bind("0.0.0.0:0").await.unwrap();
        socket.set_ttl(128).unwrap();

        let addr;
        let mut conn: Arc<dyn Conn + Send + Sync>;
        match credentials {
            Some((username, password)) => {
                let client =
                    connect_to_turn(Arc::new(socket), server.as_str(), username, password).await?;
                let turn_conn = client.allocate().await?;

                addr = turn_conn.local_addr().await?;
                conn = Arc::new(turn_conn);
            }
            None => {
                socket.send_to(&binding_request(), &server).await?;
                let mut buf = vec![0; 4096];
                let (n, _) = socket.recv_from(&mut buf).await?;
                buf.truncate(n);

                addr = binding_response(&buf)?;
                conn = Arc::new(socket);
            }
        };

        println!("Local ip:");
        println!("{addr:?}");

        let stdin = tokio::io::stdin();
        let mut stdin = tokio::io::BufReader::new(stdin);

        let mut buf = vec![0; 4096];

        loop {
            let mut line = Default::default();
            let r = tokio::select! {
                r = stdin.read_line(&mut line) => {
                    handle_line(&mut conn, r.map(|_| line.as_str())).await
                }
                r = conn.recv_from(&mut buf) => {
                    handle_recv(r.map(|(n, peer)| (&buf[0..n], peer))).await
                }
            };

            if let Err(e) = r {
                log::error!("{e:?}");
            }
        }
    }
}

async fn connect_to_turn(
    socket: Arc<dyn Conn + Send + Sync>,
    server: &str,
    username: String,
    password: String,
) -> Result<Client, Error> {
    log::debug!("{server:?} {username:?} {password:?}");
    let client = turn::client::Client::new(turn::client::ClientConfig {
        stun_serv_addr: server.to_string(),
        turn_serv_addr: server.to_string(),
        username,
        password,
        realm: Default::default(),
        software: Default::default(),
        rto_in_ms: Default::default(),
        conn: socket,
        vnet: Default::default(),
    })
    .await?;

    client.listen().await?;

    Ok(client)
}

async fn handle_line<E>(conn: &mut Arc<dyn Conn + Send + Sync>, line: Result<&str, E>) -> Result<(), Error>
where
    Error: From<E>,
{
    let line = line?;

    if let Some(addr) = line.strip_prefix("stun:") {
        let server = lookup_one(addr).await?;
        conn.send_to(&binding_request(), server).await?;
        return Ok(());
    }

    if let Some(turn) = line.strip_prefix("turn:") {
        let mut comps = turn.trim().split(' ');
        let server = comps.next().ok_or(Error::TurnParse)?;
        let username = comps.next().ok_or(Error::TurnParse)?.to_string();
        let password = comps.next().ok_or(Error::TurnParse)?.to_string();

        let client = connect_to_turn(conn.clone(), server, username, password).await?;
        *conn = Arc::new(client.allocate().await?);
        return Ok(());
    }

    let peer = lookup_one(line).await?;
    conn.send_to(line.as_bytes(), peer).await?;

    Ok(())
}

async fn handle_recv<E>(data: Result<(&[u8], SocketAddr), E>) -> Result<(), Error>
where
    Error: From<E>,
{
    let (buf, peer) = data?;

    if let Ok(addr) = binding_response(buf) {
        println!("{addr:?}");
        return Ok(());
    }

    println!("{peer:?}: {buf:?}");

    Ok(())
}

async fn lookup_one(addr: &str) -> Result<SocketAddr, Error> {
    Ok(lookup_host(addr.trim()).await?.next().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Address not found".to_string(),
        )
    })?)
}
