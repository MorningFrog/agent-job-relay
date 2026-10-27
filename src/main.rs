mod cli;

#[tokio::main(worker_threads = 2)]
async fn main() {
    agent_job_relay::platform::initialize();
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    if let Err(e) = cli::run().await {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}
