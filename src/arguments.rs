use clap::{App, Arg};
use serde::Deserialize;
use uriparse::{Host, URI};

use std::convert::TryFrom;
use std::error::Error as StdError;
use std::fmt;
use std::fs::File;
use std::io::BufReader;
use std::net::{IpAddr, SocketAddr};

type Error = Box<dyn StdError>;

#[derive(Debug, Clone)]
struct InvalidValue {
    value_type: &'static str,
    invalid_value: String,
}

impl InvalidValue {
    fn new(value_type: &'static str, invalid_value: String) -> InvalidValue {
        InvalidValue {
            value_type: value_type,
            invalid_value: invalid_value,
        }
    }
}

impl fmt::Display for InvalidValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid {}: {}", self.value_type, self.invalid_value)
    }
}
impl StdError for InvalidValue {}

#[derive(Deserialize)]
struct JsonConfig {
    listen: String,
    dns_servers: Option<Vec<String>>,
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
                let parsed_uri = URI::try_from(serv.as_str())?;
                match parsed_uri.path().to_string().as_str() {
                    "" | "/" => Ok(()),
                    _ => Err(InvalidValue::new(
                        "The URI should not contain any path",
                        serv.to_string(),
                    )),
                }?;
                match parsed_uri.query() {
                    Some(_) => Err(InvalidValue::new(
                        "The URI should not contain any query",
                        serv.to_string(),
                    )),
                    None => Ok(()),
                }?;
                match parsed_uri.fragment() {
                    Some(_) => Err(InvalidValue::new(
                        "The URI should not contain any fragment",
                        serv.to_string(),
                    )),
                    None => Ok(()),
                }?;

                let host = match parsed_uri.host() {
                    Some(h) => match h {
                        Host::IPv4Address(ip4) => Ok(IpAddr::V4(*ip4)),
                        Host::IPv6Address(ip6) => Ok(IpAddr::V6(*ip6)),
                        _ => Err(InvalidValue::new("IP", serv.to_string())),
                    },
                    None => Err(InvalidValue::new("IP", "<none>".into())),
                }?;
                let port = parsed_uri.port().unwrap_or(53);
                let transport = match parsed_uri.scheme().as_str() {
                    "tcp" => Ok(DnsTransport::TCP),
                    "udp" | "" => Ok(DnsTransport::UDP),
                    _ => Err(InvalidValue::new("scheme", serv.to_string())),
                }?;
                Ok(DnsServer {
                    address: SocketAddr::new(host, port),
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
