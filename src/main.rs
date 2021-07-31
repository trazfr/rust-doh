// https://datatracker.ietf.org/doc/html/rfc8484

mod arguments;
mod dns_client;
mod http_server;

use arguments::Arguments;
use dns_client::DnsClient;
use log::info;

#[tokio::main]
async fn main() {
    env_logger::init();
    let args = Arguments::parse_cli().unwrap();
    info!("listen on {}", args.listen);
    info!(
        "dns servers: {}",
        args.dns_servers
            .iter()
            .map(|s| {
                let transport: String = match s.transport {
                    arguments::DnsTransport::UDP => "udp://",
                    arguments::DnsTransport::TCP => "tcp://",
                }
                .to_owned();
                transport + &s.address.to_string()
            })
            .collect::<Vec<String>>()
            .join(",")
    );

    let client = DnsClient::new(&args.dns_servers).unwrap();
    http_server::run(&args.listen, client).await.unwrap()
}
