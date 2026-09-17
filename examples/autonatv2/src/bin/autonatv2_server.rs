use std::{error::Error, net::Ipv4Addr, time::Duration};

use cfg_if::cfg_if;
use clap::Parser;
use libp2p::{
    autonat,
    futures::StreamExt,
    identify, identity,
    multiaddr::Protocol,
    noise,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux, Multiaddr, SwarmBuilder,
};
use rand::rngs::OsRng;

#[derive(Debug, Parser)]
#[clap(name = "libp2p autonatv2 server")]
struct Opt {
    #[clap(short, long, default_value_t = 0)]
    listen_port: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::parse();
    cfg_if! {
        if #[cfg(feature = "jaeger")] {
            use opentelemetry::trace::TracerProvider;
            use tracing_subscriber::layer::SubscriberExt;
            let provider = telemetry_provider(None)?;
            let telemetry = tracing_opentelemetry::layer()
                .with_tracer(provider.tracer("autonatv2"));
            let subscriber = tracing_subscriber::Registry::default()
                .with(telemetry);
        } else {
            let subscriber = tracing_subscriber::fmt()
                .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
                .finish();
        }
    }
    let result = async {
        tracing::subscriber::set_global_default(subscriber)?;
        run(opt).await
    }
    .await;

    #[cfg(feature = "jaeger")]
    {
        // Flush ended spans before Tokio is torn down. The blocking exporter and
        // its shutdown wait must not occupy an async executor worker.
        let shutdown = tokio::task::spawn_blocking(move || {
            provider.shutdown_with_timeout(Duration::from_secs(5))
        })
        .await?;
        if let Err(shutdown) = shutdown {
            return Err(match result {
                Ok(()) => Box::<dyn Error>::from(shutdown),
                Err(error) => format!("{error}; telemetry shutdown failed: {shutdown}").into(),
            });
        }
    }
    result
}

#[cfg(feature = "jaeger")]
fn telemetry_provider(
    endpoint: Option<&str>,
) -> Result<opentelemetry_sdk::trace::SdkTracerProvider, Box<dyn Error>> {
    use opentelemetry_otlp::{Protocol as OtlpProtocol, SpanExporter, WithExportConfig};
    use opentelemetry_sdk::{trace::SdkTracerProvider, Resource};

    // Use the standard OTEL_EXPORTER_OTLP[_TRACES]_ENDPOINT configuration.
    let mut exporter = SpanExporter::builder()
        .with_http()
        .with_protocol(OtlpProtocol::HttpBinary)
        .with_timeout(Duration::from_secs(3));
    if let Some(endpoint) = endpoint {
        exporter = exporter.with_endpoint(endpoint);
    }
    let exporter = exporter.build()?;
    Ok(SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(Resource::builder().with_service_name("autonatv2").build())
        .build())
}

async fn run(opt: Opt) -> Result<(), Box<dyn Error>> {
    let mut swarm = SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_quic()
        .with_dns()?
        .with_behaviour(|key| Behaviour::new(key.public()))?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    swarm.listen_on(
        Multiaddr::empty()
            .with(Protocol::Ip4(Ipv4Addr::UNSPECIFIED))
            .with(Protocol::Tcp(opt.listen_port)),
    )?;

    let shutdown = shutdown_signal();
    tokio::pin!(shutdown);
    loop {
        let event = tokio::select! {
            stopped = &mut shutdown => {
                stopped?;
                return Ok(());
            }
            event = swarm.select_next_some() => event,
        };
        match event {
            SwarmEvent::NewListenAddr { address, .. } => println!("Listening on {address:?}"),
            SwarmEvent::Behaviour(event) => println!("{event:?}"),
            e => println!("{e:?}"),
        }
    }
}

async fn shutdown_signal() -> std::io::Result<()> {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        tokio::select! {
            stopped = tokio::signal::ctrl_c() => stopped,
            _ = terminate.recv() => Ok(()),
        }
    }
    #[cfg(not(unix))]
    tokio::signal::ctrl_c().await
}

#[derive(NetworkBehaviour)]
pub struct Behaviour {
    autonat: autonat::v2::server::Behaviour,
    identify: identify::Behaviour,
}

impl Behaviour {
    pub fn new(key: identity::PublicKey) -> Self {
        Self {
            autonat: autonat::v2::server::Behaviour::new(OsRng),
            identify: identify::Behaviour::new(identify::Config::new("/ipfs/0.1.0".into(), key)),
        }
    }
}

#[cfg(all(test, feature = "jaeger"))]
mod tests {
    use std::sync::Arc;

    use axum::{
        body::Bytes,
        http::{HeaderMap, StatusCode},
        routing::post,
        Router,
    };
    use opentelemetry::trace::TracerProvider;
    use opentelemetry_proto::tonic::{
        collector::trace::v1::ExportTraceServiceRequest, common::v1::any_value::Value,
    };
    use prost::Message;
    use tokio::{
        net::TcpListener,
        sync::{oneshot, Mutex},
    };
    use tracing_subscriber::layer::SubscriberExt;

    use super::*;

    #[tokio::test]
    async fn exports_service_and_span_to_otlp_http() -> Result<(), Box<dyn Error>> {
        check_collector(StatusCode::OK).await
    }

    #[tokio::test]
    async fn collector_rejection_is_reported_by_flush() -> Result<(), Box<dyn Error>> {
        check_collector(StatusCode::BAD_REQUEST).await
    }

    async fn check_collector(status: StatusCode) -> Result<(), Box<dyn Error>> {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let captured = requests.clone();
        let app = Router::new().route(
            "/v1/traces",
            post(move |headers: HeaderMap, body: Bytes| {
                let captured = captured.clone();
                async move {
                    captured.lock().await.push((headers, body));
                    status
                }
            }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let endpoint = format!("http://{}/v1/traces", listener.local_addr()?);
        let provider = telemetry_provider(Some(&endpoint))?;
        let subscriber = tracing_subscriber::registry()
            .with(tracing_opentelemetry::layer().with_tracer(provider.tracer("autonatv2")));
        tracing::subscriber::with_default(subscriber, || {
            let span = tracing::info_span!("collector_contract", answer = 42_i64);
            let _entered = span.enter();
        });

        let (shutdown, stopped) = oneshot::channel::<()>();
        let server = axum::serve(listener, app).with_graceful_shutdown(async {
            let _ = stopped.await;
        });
        let client = async move {
            let _shutdown = shutdown;
            tokio::task::spawn_blocking(move || {
                let flushed = provider.force_flush();
                let stopped = provider.shutdown_with_timeout(Duration::from_secs(5));
                (flushed, stopped)
            })
            .await
        };
        let (served, exported) = tokio::time::timeout(Duration::from_secs(15), async {
            tokio::join!(server, client)
        })
        .await?;
        served?;
        let (flushed, stopped) = exported?;
        stopped?;
        assert_eq!(flushed.is_ok(), status.is_success());

        let requests = requests.lock().await;
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].0["content-type"], "application/x-protobuf");
        let exported = ExportTraceServiceRequest::decode(requests[0].1.clone())?;
        assert_eq!(exported.resource_spans.len(), 1);
        let resource = &exported.resource_spans[0];
        assert!(resource.resource.as_ref().is_some_and(|resource| {
            resource.attributes.iter().any(|attribute| {
                attribute.key == "service.name"
                    && attribute
                        .value
                        .as_ref()
                        .and_then(|value| value.value.as_ref())
                        == Some(&Value::StringValue("autonatv2".to_owned()))
            })
        }));
        let spans: Vec<_> = resource
            .scope_spans
            .iter()
            .flat_map(|scope| &scope.spans)
            .collect();
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].name, "collector_contract");
        assert!(spans[0].attributes.iter().any(|attribute| {
            attribute.key == "answer"
                && attribute
                    .value
                    .as_ref()
                    .and_then(|value| value.value.as_ref())
                    == Some(&Value::IntValue(42))
        }));
        Ok(())
    }
}
