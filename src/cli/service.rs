use std::fmt::{self, Display, Formatter};

use clap::{Args, ValueEnum};
use compose_spec::service::Restart;

#[derive(Args, Default, Debug, Clone, PartialEq, Eq)]
pub struct Service {
    /// Configure if and when the service should be restarted
    #[arg(long, value_name = "POLICY")]
    restart: Option<RestartConfig>,
    /// Whether the service should be treated as a one-time task
    #[arg(long, value_name = "ONESHOT")]
    oneshot: bool,
}

impl Service {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

impl Display for Service {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        writeln!(f, "[Service]")?;
        if let Some(restart) = self.restart.and_then(|restart| restart.to_possible_value()) {
            writeln!(f, "Restart={}", restart.get_name())?;
        }
        if self.oneshot {
            writeln!(f, "Type=oneshot")?;
            writeln!(f, "RemainAfterExit=true")?;
        }
        Ok(())
    }
}

/// Possible service restart configurations
///
/// From [systemd.service](https://www.freedesktop.org/software/systemd/man/systemd.service.html#Restart=)
#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
enum RestartConfig {
    No,
    OnSuccess,
    OnFailure,
    OnAbnormal,
    OnWatchdog,
    OnAbort,
    #[value(alias = "unless-stopped")]
    Always,
}

impl From<Restart> for RestartConfig {
    fn from(value: Restart) -> Self {
        match value {
            Restart::No => Self::No,
            Restart::Always | Restart::UnlessStopped => Self::Always,
            Restart::OnFailure => Self::OnFailure,
        }
    }
}

pub struct ServiceBuilder {
    restart: Option<Restart>,
    oneshot: bool,
}

impl ServiceBuilder {
    pub fn new() -> Self {
        Self {
            restart: None,
            oneshot: false,
        }
    }

    pub fn with_restart(&mut self, restart: Restart) -> &mut Self {
        self.restart = Some(restart);
        self
    }

    pub fn oneshot(&mut self) -> &mut Self {
        self.oneshot = true;
        self
    }

    pub fn build(self) -> Option<Service> {
        if self.restart.is_none() && !self.oneshot {
            None
        } else {
            Some(Service {
                restart: self.restart.map(Into::into),
                oneshot: self.oneshot,
            })
        }
    }
}
