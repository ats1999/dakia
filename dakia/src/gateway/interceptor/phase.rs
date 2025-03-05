use std::fmt;

pub type PhaseMask = u8;

#[derive(PartialEq, Clone, Debug, Eq)]
pub enum Phase {
    RequestFilter = 0x01,
    UpstreamProxyFilter = 0x02,
    PreUpstreamRequest = 0x03,
    PostUpstreamResponse = 0x04,
    PreDownstreamResponse = 0x05,
}

impl Phase {
    pub fn mask(&self) -> PhaseMask {
        self.clone() as PhaseMask
    }

    pub fn all_phase_mask() -> PhaseMask {
        let bits = Phase::RequestFilter.mask()
            | Phase::UpstreamProxyFilter.mask()
            | Phase::PreUpstreamRequest.mask()
            | Phase::PostUpstreamResponse.mask();

        bits as PhaseMask
    }
}

impl Ord for Phase {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.clone() as PhaseMask).cmp(&(other.clone() as PhaseMask))
    }
}

impl PartialOrd for Phase {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Phase {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let phase_str = match self {
            Phase::RequestFilter => "request_filter",
            Phase::UpstreamProxyFilter => "upstream_proxy_filter",
            Phase::PreUpstreamRequest => "pre_upstream_request",
            Phase::PostUpstreamResponse => "post_upstream_response",
            Phase::PreDownstreamResponse => "pre_downstream_response",
        };
        write!(f, "{}", phase_str)
    }
}

pub fn is_phase_enabled(phase_mask: PhaseMask, phase: &Phase) -> bool {
    (phase_mask & phase.mask()) == phase.mask()
}
