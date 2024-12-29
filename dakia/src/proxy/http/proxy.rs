use crate::{
    config::source_config::GatewayConfig,
    shared::{config_store, pattern_registry::PatternRegistryType},
};

use super::{
    builder,
    helpers::{self, is_valid_ds_host},
    DakiaHttpGatewayCtx,
};
use async_trait::async_trait;
use pingora::{
    prelude::HttpPeer,
    proxy::{ProxyHttp, Session},
    Error,
};

#[derive(Clone)]
pub struct Proxy {
    name: String,
    ds_host_pattern_registry: PatternRegistryType,
}

impl Proxy {
    pub async fn build(
        gateway_config: &GatewayConfig,
    ) -> Result<Proxy, Box<dyn std::error::Error>> {
        let ds_host_pattern_registry =
            builder::build_ds_host_pattern_registry(gateway_config).await?;
        let proxy = Proxy {
            name: gateway_config.name.clone(),
            ds_host_pattern_registry,
        };
        Ok(proxy)
    }
}

#[async_trait]
impl ProxyHttp for Proxy {
    type CTX = DakiaHttpGatewayCtx;
    fn new_ctx(&self) -> Self::CTX {
        DakiaHttpGatewayCtx::new()
    }

    async fn early_request_filter(
        &self,
        _session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> Result<(), Box<Error>> {
        // update config into context
        let c = config_store::get().await;
        _ctx.config = c;

        Ok(())
    }

    async fn request_filter(
        &self,
        _session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> Result<bool, Box<Error>> {
        let host = helpers::get_header(_session, "host");

        match host {
            None => {
                // TODO: add option to customize http response status and body
                helpers::write_response_ds(_session, 400, None).await?;
                return Ok(true);
            }

            Some(x) => {
                let is_valid_ds_host =
                    is_valid_ds_host(&_ctx.config, &self.name, &self.ds_host_pattern_registry, x)
                        .await
                        .map_err(|e| e.to_pingora_error())?;

                if !is_valid_ds_host {
                    // TODO: add option to customize http response status and body
                    helpers::write_response_ds(_session, 403, None).await?;
                    return Ok(true);
                }
            }
        };

        Ok(false)
    }

    async fn upstream_peer(
        &self,
        _session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>, Box<Error>> {
        let addr = ("127.0.0.1", 3000);

        let peer = Box::new(HttpPeer::new(addr, false, "one.one.one.one".to_string()));
        Ok(peer)
    }
}
