use clap::{App, Arg};
use serde::Deserialize;

use std::error::Error as StdError;
use std::fmt;
use std::fs::File;
use std::io::BufReader;
use std::net::SocketAddr;

type Error = Box<dyn StdError>;

#[derive(Debug, Clone)]
struct EnumError {
    value: String,
}

impl EnumError {
    fn new(invalid_value: String) -> EnumError {
        EnumError {
            value: invalid_value,
        }
    }
}

impl fmt::Display for EnumError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid enum value: {}", self.value)
    }
}
impl StdError for EnumError {}

#[derive(Deserialize)]
struct JsonDnsServer {
    address: String,
    transport: Option<String>,
}

#[derive(Deserialize)]
struct JsonConfig {
    listen: String,
    dns_servers: Option<Vec<JsonDnsServer>>,
}

pub enum DnsTransport {
    UDP,
    TCP,
}

pub struct DnsServer {
    pub address: SocketAddr,
    pub transport: DnsTransport,
}

pub struct Arguments {
    pub listen: SocketAddr,
    pub dns_servers: Vec<DnsServer>,
}

impl Arguments {
    pub fn parse_cli() -> Result<Arguments, Error> {
        let matches = App::new("Rust DNS over HTTP")
            .version("0.0")
            .author("Alexandre Blazart <alexandre@blazart.fr>")
            .about("DNS over HTTP implementation")
            .arg(
                Arg::with_name("CONFIG_FILE")
                    .help("Sets the input file to use")
                    .required(true)
                    .index(1),
            )
            .get_matches();
        let file = File::open(matches.value_of("CONFIG_FILE").unwrap())?;
        let reader = BufReader::new(file);
        let json: JsonConfig = serde_json::from_reader(reader)?;
        let dns_servers = json
            .dns_servers
            .into_iter()
            .flatten()
            .map(|serv| {
                let address = serv.address.parse()?;
                let transport = match serv.transport {
                    None => Ok(DnsTransport::UDP),
                    Some(transp) => match &transp[..] {
                        "tcp" => Ok(DnsTransport::TCP),
                        "udp" => Ok(DnsTransport::UDP),
                        _ => Err(EnumError::new(transp)),
                    },
                }?;
                Ok(DnsServer {
                    address: address,
                    transport: transport,
                })
            })
            .collect::<Result<Vec<_>, Error>>();
        Ok(Arguments {
            listen: json.listen.parse()?,
            dns_servers: dns_servers?,
        })
    }
}
