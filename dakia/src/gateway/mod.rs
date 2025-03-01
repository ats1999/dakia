mod interceptor;
mod interceptors;

use super::Proxy;
use pingora::{server::configuration::ServerConf, services::listening::Service};
use pingora_proxy::{http_proxy_service_with_name, HttpProxy};
use std::sync::Arc;

use crate::{config::source_config::GatewayConfig, error::DakiaResult};

pub type HttpGateway = Service<HttpProxy<Proxy>>;

pub async fn build_http(
    gateway_config: &GatewayConfig,
    server_conf: &Arc<ServerConf>,
) -> DakiaResult<HttpGateway> {
    let proxy = Proxy::build(gateway_config).await?;
    let mut http_proxy_service =
        http_proxy_service_with_name(&server_conf, proxy, "Dakia HTTP Proxy");

    for inet_address in &gateway_config.bind_addresses {
        let addr = inet_address.get_formatted_address();
        http_proxy_service.add_tcp(&addr);
    }

    Ok(http_proxy_service)
}
