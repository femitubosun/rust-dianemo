use bytes::BytesMut;
use dianemo_rust::codec::decoder::decode_dia_value;
use dianemo_rust::config::AppSettings;
use dianemo_rust::state::PeerTable;
use dianemo_rust::types::{DiaReq, Peer, ResStatus};
use std::error::Error;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::sync::{mpsc, oneshot};

fn lan_ip() -> std::io::Result<IpAddr> {
    let s = std::net::UdpSocket::bind("0.0.0.0:0")?;
    s.connect("8.8.8.8:80")?;
    Ok(s.local_addr()?.ip())
}

fn bind_reusable_udp(port: u16) -> std::io::Result<UdpSocket> {
    let sock = socket2::Socket::new(socket2::Domain::IPV4, socket2::Type::DGRAM, None)?;
    sock.set_reuse_address(true)?;
    sock.set_reuse_port(true)?;
    sock.set_nonblocking(true)?;
    sock.bind(&std::net::SocketAddr::from(([0, 0, 0, 0], port)).into())?;
    UdpSocket::from_std(sock.into())
}

fn read_line_blocking() -> std::io::Result<String> {
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(input)
}

enum ConsoleMsg {
    ConfirmPair(String, oneshot::Sender<bool>),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let app_settings = AppSettings::default();
    let socket = Arc::new(bind_reusable_udp(app_settings.discovery_port)?);

    let my_id = cuid2::create_id();
    let peers = Arc::new(PeerTable::default());
    let tcp_listener = TcpListener::bind("0.0.0.0:0").await?;
    let tcp_port = tcp_listener.local_addr()?.port();

    let ip = lan_ip().unwrap_or(IpAddr::from([127, 0, 0, 1]));
    let name = hostname::get()
        .map(|h| h.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "unknown".into());
    let peer = Peer::new(&my_id, name, format!("{ip}:{tcp_port}"));

    let (console_tx, console_rx) = mpsc::channel(8);

    let announcer_socket = Arc::clone(&socket);
    let announcer_peer = peer.clone();
    tokio::spawn(async move {
        if let Err(e) = handle_announcement(announcer_socket, announcer_peer).await {
            eprintln!("announce failed: {e}")
        }
    });

    let handler_peer = peer.clone();
    let handler_tx = console_tx.clone();
    tokio::spawn(async move {
        loop {
            match tcp_listener.accept().await {
                Ok((stream, addr)) => {
                    dbg!(addr);
                    tokio::spawn(handle_connection(
                        stream,
                        handler_peer.clone(),
                        handler_tx.clone(),
                    ));
                }
                Err(e) => eprintln!("accept failed: {e}"),
            }
        }
    });

    tokio::spawn(console_task(Arc::clone(&peers), peer.clone(), console_rx));

    let mut buf = [0u8; 1024];
    loop {
        let (len, src) = socket.recv_from(&mut buf).await?;
        let mut bytes = BytesMut::from(&buf[..len]);
        match decode_dia_value(&mut bytes) {
            Ok(DiaReq::Announce(p)) if p.id != my_id.clone() => peers.upsert(p),
            Ok(_) => {}
            Err(e) => eprintln!("bad packet from {src}:{e}"),
        }
    }
}

async fn console_task(peers: Arc<PeerTable>, my_peer: Peer, mut rx: mpsc::Receiver<ConsoleMsg>) {
    let mut pending_confirm: Option<oneshot::Sender<bool>> = None;
    let mut awaiting_choice: Option<Vec<Peer>> = None;
    let mut stdin = tokio::task::spawn_blocking(read_line_blocking);
    println!("[Enter] list peers | [f] pick a file");

    loop {
        tokio::select! {
            Some(msg) = rx.recv() => match msg {
                ConsoleMsg::ConfirmPair(name, reply) => {
                    println!("--- {name} wants to pair [y/n]");
                    pending_confirm = Some(reply);
                }
            },
            line = &mut stdin => {
                let line = line.unwrap_or(Ok(String::new())).unwrap_or_default();
                stdin = tokio::task::spawn_blocking(read_line_blocking);
                let line = line.trim();

                if let Some(reply) = pending_confirm.take() {
                    let _ = reply.send(line.eq_ignore_ascii_case("y"));
                } else if line == "f" {
                    // rfd blocks while the dialog is open — blocking pool again
                    let path = tokio::task::spawn_blocking(|| {
                        rfd::FileDialog::new()
                            .set_title("Pick a file to send")
                            .pick_file()
                    })
                    .await
                    .unwrap_or(None);
                    match path {
                        Some(p) => println!("picked: {}", p.display()),
                        None => println!("cancelled"),
                    }
                    println!("[Enter] list peers | [f] pick a file");
                } else if let Some(list) = awaiting_choice.take() {
                    match line.parse::<usize>() {
                        Ok(i) if i < list.len() => pair_with(&list[i], &my_peer).await,
                        _ => println!("invalid choice"),
                    }
                    println!("[Enter] list peers | [f] pick a file");
                } else {
                    let known = peers.snapshot();
                    if known.is_empty() {
                        println!("no peers discovered yet");
                    } else {
                        for (i, p) in known.iter().enumerate() {
                            println!("  [{i}] {} @ {}", p.name, p.ip);
                        }
                        println!("pick a number:");
                        awaiting_choice = Some(known);
                    }
                }
            }
        }
    }
}

async fn pair_with(target: &Peer, my_peer: &Peer) {
    let mut stream = match TcpStream::connect(&target.ip).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("connect to {} failed: {e}", target.ip);
            return;
        }
    };

    let req = DiaReq::pair_req(my_peer.clone());
    match req.to_bytes() {
        Ok(b) => {
            if let Err(e) = stream.write_all(&b).await {
                eprintln!("send failed: {e}");
                return;
            }
        }
        Err(e) => {
            eprintln!("encode failed: {e}");
            return;
        }
    }

    let mut buf = [0u8; 4096];
    match tokio::time::timeout(Duration::from_secs(30), stream.read(&mut buf)).await {
        Ok(Ok(len)) => {
            let mut bytes = BytesMut::from(&buf[..len]);
            match decode_dia_value(&mut bytes) {
                Ok(DiaReq::PairRes(p, status)) => println!("{} answered: {:?}", p.name, status),
                other => println!("unexpected reply: {other:?}"),
            }
        }
        Ok(Err(e)) => eprintln!("read failed: {e}"),
        Err(_) => println!("{} didn't answer within 30s", target.name),
    }
}

async fn handle_connection(
    mut stream: TcpStream,
    my_peer: Peer,
    console: mpsc::Sender<ConsoleMsg>,
) {
    let mut buffer = [0; 4096];

    loop {
        match stream.read(&mut buffer).await {
            Ok(0) | Err(_) => return,
            Ok(len) => {
                let mut bytes = BytesMut::from(&buffer[..len]);
                match decode_dia_value(&mut bytes) {
                    Ok(DiaReq::PairReq(p)) => {
                        // ask the console; it owns stdin
                        let (reply_tx, reply_rx) = oneshot::channel();
                        if console
                            .send(ConsoleMsg::ConfirmPair(p.name.clone(), reply_tx))
                            .await
                            .is_err()
                        {
                            eprintln!("console task gone");
                            return;
                        }
                        let accepted = reply_rx.await.unwrap_or(false);
                        let status = if accepted {
                            ResStatus::Accept
                        } else {
                            ResStatus::Reject
                        };

                        let res = DiaReq::pair_res(my_peer.clone(), status);
                        match res.to_bytes() {
                            Ok(b) => {
                                let _ = stream.write_all(&b).await;
                            }
                            Err(e) => eprintln!("encode failed: {e}"),
                        }
                    }
                    Ok(other) => println!("got: {other}"),
                    Err(e) => eprintln!("bad packet: {e}"),
                }
            }
        }
    }
}

async fn handle_announcement(
    socket: Arc<UdpSocket>,
    peer: Peer,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    socket.set_broadcast(true)?;
    let accounce = DiaReq::announce(peer);

    let addr = format!("255.255.255.255:{}", AppSettings::default().discovery_port);

    loop {
        socket.send_to(&accounce.to_bytes()?, &addr).await?;
        tokio::time::sleep(Duration::from_secs(
            AppSettings::default().discovery_announce_interval_secs,
        ))
        .await;
    }
}
