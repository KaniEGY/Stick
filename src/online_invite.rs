//! Shareable Stick Studio room invitations. Links contain only a network
//! endpoint, room code and mode; never include a password or user token.
//! The links can be pasted into Stick, they are not registered OS URI handlers.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoomInvite {
    pub address: String,
    pub code: String,
    pub animation: bool,
}

pub fn validate_endpoint(value: &str) -> Result<String, &'static str> {
    let value = value.trim();
    if value.is_empty() || value.len() > 253
        || !value.is_ascii()
        || value.chars().any(|c| c.is_ascii_whitespace()
            || c.is_ascii_control() || "&?#/@\\\\%".contains(c)) {
        return Err("Enter a valid host address, IP or DNS name");
    }
    let host = if let Some(rest) = value.strip_prefix('[') {
        let (inside, after) = rest.split_once(']').ok_or("Invalid IPv6 brackets")?;
        if inside.parse::<std::net::Ipv6Addr>().is_err() { return Err("Invalid IPv6 address"); }
        if !after.is_empty() {
            let port = after.strip_prefix(':').ok_or("Invalid address port")?;
            validate_port(port)?;
        }
        return Ok(value.to_string());
    } else if let Some((host, port)) = value.rsplit_once(':') {
        validate_port(port)?;
        host
    } else {
        value
    };
    if host.is_empty() || host.split('.').any(|part| part.is_empty())
        || !host.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-' || b == b'_') {
        return Err("Invalid host or DNS name");
    }
    Ok(value.to_string())
}

fn validate_port(value: &str) -> Result<(), &'static str> {
    match value.parse::<u16>() { Ok(port) if port > 0 => Ok(()), _ => Err("Invalid TCP port") }
}

pub fn invitation_endpoint<'a>(custom: &'a str, remote: Option<&'a str>, lan: &'a str) -> &'a str {
    let custom = custom.trim();
    if custom.is_empty() { remote.unwrap_or(lan) } else { custom }
}

pub fn make_invite(address: &str, code: &str, animation: bool) -> Result<String, &'static str> {
    let address = validate_endpoint(address)?;
    if code.len() != 8 || !code.bytes().all(|c| c.is_ascii_digit()) {
        return Err("The room code must contain eight digits");
    }
    Ok(format!("stick://join?address={}&code={}&mode={}", address, code,
        if animation { "animation" } else { "drawing" }))
}

pub fn parse_invite(value: &str) -> Result<RoomInvite, &'static str> {
    let value = value.trim();
    if value.len() > 512 { return Err("Invitation is too long"); }
    let query = value.strip_prefix("stick://join?").ok_or("Not a Stick Studio invitation")?;
    let mut address = None;
    let mut code = None;
    let mut animation = None;
    for part in query.split('&') {
        let (key,val) = part.split_once('=').ok_or("Malformed invitation")?;
        match key {
            "address" if address.is_none() => address = Some(validate_endpoint(val)?),
            "code" if code.is_none() && val.len()==8
                && val.bytes().all(|b| b.is_ascii_digit()) => code = Some(val.to_string()),
            "mode" if animation.is_none() && (val=="animation" || val=="drawing") => {
                animation=Some(val=="animation");
            },
            _ => return Err("Invalid invitation field"),
        }
    }
    Ok(RoomInvite { address: address.ok_or("Missing host address")?,
        code: code.ok_or("Invalid room code")?, animation: animation.ok_or("Missing room mode")? })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invitations_prefer_explicit_then_remote_then_local() {
        let lan = "192.168.1.4:47554";
        let remote = "100.75.1.3:47554";
        assert_eq!(invitation_endpoint(" ", Some(remote), lan), remote);
        assert_eq!(invitation_endpoint("", None, lan), lan);
        assert_eq!(invitation_endpoint(" example.org:4000 ", Some(remote), lan), "example.org:4000");
    }

    #[test]
    fn round_trip_without_secret() {
        let link=make_invite("draw.example.org:47554","00128394",true).unwrap();
        assert_eq!(parse_invite(&link).unwrap(),RoomInvite{
            address:"draw.example.org:47554".into(),code:"00128394".into(),animation:true});
        assert!(!link.contains("password"));
    }

    #[test]
    fn accepts_private_addresses_and_ipv6_brackets() {
        assert!(validate_endpoint("192.168.1.22:47554").is_ok());
        assert!(validate_endpoint("[2001:db8::15]:47554").is_ok());
        assert!(validate_endpoint("10.1.1.5").is_ok());
        assert!(validate_endpoint("my-host.local:47554").is_ok());
    }

    #[test]
    fn rejects_modified_and_unsafe_invites() {
        for text in [
            "stick://join?address=bad.com:0&code=12345678&mode=drawing",
            "stick://join?address=bad.com&code=1234&mode=drawing",
            "stick://join?address=bad.com&code=12345678&mode=drawing&password=secret",
            "stick://join?address=bad.com&address=example.com&code=12345678&mode=drawing",
            "stick://join?address=bad.com/@other&code=12345678&mode=drawing",
        ] {assert!(parse_invite(text).is_err(), "{text}");}
    }
}
