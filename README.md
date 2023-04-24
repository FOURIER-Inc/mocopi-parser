# mocopi-parser

mocopi-parser is a parser library of streamed data from [mocopi](https://www.sony.net/Products/mocopi-dev/jp/).

## Example

This example using [local-ip-address](https://crates.io/crates/local-ip-address) crate.

```rust
use std::net::UdpSocket;
use local_ip_address::local_ip;

fn main() {
    let ip = local_ip().unwrap();
    let port = 12351;
    let addr = format!("{}:{}", ip, port);

    let socket = UdpSocket::bind(&addr).unwrap();

    let mut buff = [0u8; 2048];

    loop {
        socket.recv_from(buff).unwrap();

        let packet = mocopi_parser::parse(&buff).unwrap();
        
        println!("{:?}", packet);
    }
}
```

## References

1. [mocopi receiver](https://github.com/seagetch/mcp-receiver/blob/main/doc/Protocol.md)
2. [技術仕様](https://www.sony.net/Products/mocopi-dev/jp/documents/Home/TechSpec.html)
