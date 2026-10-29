use super::*;
mod records;
mod requests;
mod wire;

fn spec() -> Spec {
    Spec {
        name: "fixture".into(),
        placement: Placement {
            zone: "ap-hongkong-2".into(),
            vpc: "vpc-fixture".into(),
            subnet: "subnet-fixture".into(),
            project: 0,
        },
        hardware: Hardware {
            flavor: "S5.SMALL2".into(),
            image: "img-fixture".into(),
            disk: Disk {
                class: Storage::Premium,
                size: 50,
            },
        },
        connection: Connection {
            groups: vec!["sg-fixture".into()],
            key: "skey-fixture".into(),
            public: true,
            bandwidth: 1,
        },
    }
}

fn request(operation: Operation) -> Request {
    Request {
        region: "ap-hongkong".into(),
        operation,
    }
}

fn rsa() -> String {
    sized(2048)
}

fn sized(bits: usize) -> String {
    use base64::Engine;
    let mut bytes = Vec::new();
    let mut modulus = vec![0u8; bits / 8 + 1];
    modulus[1] = 0x80;
    let end = modulus.len() - 1;
    modulus[end] = 1;
    for field in [b"ssh-rsa".as_slice(), &[1, 0, 1], &modulus] {
        bytes.extend_from_slice(&(field.len() as u32).to_be_bytes());
        bytes.extend_from_slice(field);
    }
    format!(
        "ssh-rsa {} fixture",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )
}
