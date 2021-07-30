# rust-doh

Simple DNS over HTTP server written in Rust.
This is a basic implementation of the [RFC8484](https://datatracker.ietf.org/doc/html/rfc8484).

## Configuration

This is configured through a JSON file:

```json
{
    "listen": "127.0.0.1:3000",
    "dns_servers": [
        {
            "address": "192.168.1.254:53"
        },
        {
            "address": "8.8.8.8:53",
            "transport": "tcp"
        }
    ]
}
```
