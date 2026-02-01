use std::{
    fmt::{self, Display, Formatter},
    iter,
    net::IpAddr,
};

use color_eyre::eyre::eyre;
use compose_spec::{
    service::ports::{Port, Protocol, ShortPort},
    ShortOrLong,
};
use indexmap::IndexSet;
use serde::Serialize;

#[derive(Serialize, Debug, Default, Clone, PartialEq)]
pub struct Socket {
    /// Ports for socket activation.
    #[serde(rename = "ListenStream")]
    pub listen_stream: Vec<String>,
}

impl Display for Socket {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let image = crate::serde::quadlet::to_string(self).map_err(|_| fmt::Error)?;
        f.write_str(&image)
    }
}

fn create_listen_stream(host_ip: Option<&IpAddr>, port: &u16, suffix: Option<&str>) -> String {
    let suffix = suffix.unwrap_or("");
    match host_ip {
        Some(IpAddr::V4(host_ip)) => format!("{host_ip}:{port}{suffix}"),
        Some(IpAddr::V6(host_ip)) => format!("[{host_ip}]:{port}{suffix}"),
        None => format!("{port}{suffix}"),
    }
}

fn port_to_listen_stream_iter<'a>(
    protocol: Option<&'a Protocol>,
    published: Option<&'a compose_spec::service::ports::Range>,
    host_ip: Option<&'a IpAddr>,
    target: &u16,
) -> impl IntoIterator<Item = Result<String, color_eyre::Report>> + 'a {
    let protocol_suffix = match &protocol {
        Some(Protocol::Tcp) => None,
        None => None,
        Some(Protocol::Udp) => Some("/udp"),
        Some(Protocol::Other(protocol)) => {
            return None.into_iter().flatten().chain(None.into_iter()).chain(
                Some(iter::once(Err(eyre!(
                    "protocol '{protocol}' is not supported for systemd .socket file generation",
                ))))
                .into_iter()
                .flatten(),
            );
        }
    };

    if let Some(published) = &published {
        let start = published.start();
        let streams = (start..=published.end().unwrap_or(start))
            .map(move |port_num| create_listen_stream(host_ip, &port_num, protocol_suffix))
            .map(Ok);
        Some(streams)
            .into_iter()
            .flatten()
            .chain(None.into_iter())
            .chain(None.into_iter().flatten())
    } else {
        let stream = create_listen_stream(host_ip, target, protocol_suffix);
        None.into_iter()
            .flatten()
            .chain(Some(Ok(stream)).into_iter())
            .chain(None.into_iter().flatten())
    }
}

fn ports_to_socket<'a>(
    ports: &mut impl Iterator<Item = &'a ShortOrLong<ShortPort, Port>>,
) -> Result<Socket, color_eyre::Report> {
    let mut socket = Socket::default();

    ports.try_for_each(|port| {
        match port {
            ShortOrLong::Short(port) => {
                let host_ip = port.host_ip.as_ref();
                let protocol = port.protocol.as_ref();
                port.ranges.into_iter().try_for_each(|(host, container)| {
                    let published = host.map(Into::into);
                    for listen_stream in port_to_listen_stream_iter(
                        protocol,
                        published.as_ref(),
                        host_ip,
                        &container,
                    ) {
                        socket.listen_stream.push(listen_stream?);
                    }
                    Ok::<(), color_eyre::Report>(())
                })?;
            }
            ShortOrLong::Long(ref port) => {
                for listen_stream in port_to_listen_stream_iter(
                    port.protocol.as_ref(),
                    port.published.as_ref(),
                    port.host_ip.as_ref(),
                    &port.target,
                ) {
                    socket.listen_stream.push(listen_stream?);
                }
            }
        }
        Ok::<(), color_eyre::Report>(())
    })?;

    Ok(socket)
}

impl TryFrom<&IndexSet<ShortOrLong<ShortPort, Port>>> for Socket {
    type Error = color_eyre::Report;

    fn try_from(value: &IndexSet<ShortOrLong<ShortPort, Port>>) -> Result<Self, Self::Error> {
        ports_to_socket(&mut value.iter())
    }
}
