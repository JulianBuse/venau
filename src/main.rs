use clap::command;
use libtailscale::Tailscale;
use tokio::net::TcpStream;
use tokio::task;
use tokio::io;
use tokio::select;

async fn handle_stream(mut incoming: TcpStream) {
    let proxy_host = TcpStream::connect("localhost:1337").await.expect("Unable to connect to proxy host.");

    let (mut inread, mut inwrite) = incoming.into_split();
    let (mut pread, mut pwrite) = proxy_host.into_split();

    let incoming_to_proxy = tokio::spawn(async move { io::copy(&mut inread, &mut pwrite).await });
    let proxy_to_incoming = tokio::spawn(async move { io::copy(&mut pread, &mut inwrite).await });

    select! {
        _ = incoming_to_proxy => println!("copied data from incoming to proxy"),
        _ = proxy_to_incoming => println!("copied data from proxy to incoming")
    }
}

#[tokio::main]
async fn main() {

    let mut ts = Tailscale::new();
    ts.set_ephemeral(true).expect("There was an error setting the node ephemeral state");
    ts.set_hostname("proxy").expect("There was an error setting the node hostname");
    ts.set_authkey("authkey").expect("There was an error setting the node authkey");
    ts.set_logfd(-1).expect("There was an error setting node loglevel");
    ts.up().expect("There was an error getting the node connected to the tailscale network");

    let ts_listener = ts.listen("tcp", "1337")?;

    for stream in ts_listener.incoming() {
        match stream {
            Ok(stream) => {
                stream.set_nonblocking(true).unwrap();
                let tokio_stream = TcpStream::from_std(stream).unwrap();
                task::spawn(async move {
                    handle_stream(tokio_stream)
                });
            }
            Err(error) => println!("Error {}", error)
        }
    }
}
