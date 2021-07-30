use crate::arguments::{DnsServer, DnsTransport};

use domain::base::octets::ParseError;
use domain::base::Message;
use domain::resolv::stub::conf::{ResolvConf, ResolvOptions, ServerConf, Transport};
use domain::resolv::stub::StubResolver;
use log::info;

use std::error::Error as StdError;
use std::fmt;
use std::result::Result as StdResult;

type GenericError = Box<dyn StdError>;

#[derive(Debug, Clone)]
struct ArraySizeError {
    len: usize,
}

impl ArraySizeError {
    fn new(len: usize) -> ArraySizeError {
        ArraySizeError { len: len }
    }
}

impl fmt::Display for ArraySizeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "wrong size for array: {}", self.len)
    }
}
impl StdError for ArraySizeError {}

pub struct DnsClient {
    resolver: StubResolver,
}

impl DnsClient {
    pub fn new(servers: &[DnsServer]) -> StdResult<DnsClient, GenericError> {
        let mut config = ResolvConf {
            servers: servers
                .iter()
                .map(|srv| {
                    ServerConf::new(
                        srv.address,
                        match srv.transport {
                            DnsTransport::UDP => Transport::Udp,
                            DnsTransport::TCP => Transport::Tcp,
                        },
                    )
                })
                .collect(),
            options: ResolvOptions::default(),
        };
        config.finalize();
        Ok(DnsClient {
            resolver: StubResolver::from_conf(config),
        })
    }

    pub fn clone(&self) -> DnsClient {
        DnsClient {
            resolver: self.resolver.clone(),
        }
    }

    pub async fn call(&self, request: &[u8]) -> StdResult<Vec<u8>, GenericError> {
        let message = Message::from_octets(request)?;
        let questions = message.question().collect::<Result<Vec<_>, ParseError>>()?;
        if questions.len() != 1 {
            return Err(Box::new(ArraySizeError::new(questions.len())));
        }
        let parsed_question = questions[0];
        info!(
            "DNS query: {} {} {}",
            parsed_question.qname(),
            parsed_question.qtype(),
            parsed_question.qclass()
        );
        let answer = self.resolver.query(parsed_question).await?;
        Ok(answer.as_slice().to_vec())
    }
}
