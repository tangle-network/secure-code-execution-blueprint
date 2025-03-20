use clap::Parser;
use code_exec::ResourceLimits;
use code_exec_server::{create_app, run_server};
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Server address to listen on
    #[arg(short, long, default_value = "0.0.0.0:3000")]
    addr: SocketAddr,

    /// Maximum number of concurrent executions
    #[arg(short, long, default_value = "10")]
    max_concurrent: usize,

    /// Memory limit in bytes
    #[arg(long, default_value = "104857600")] // 100MB
    memory_limit: u64,

    /// CPU time limit in seconds
    #[arg(long, default_value = "5")]
    cpu_time_limit: u32,

    /// Maximum number of processes
    #[arg(long, default_value = "10")]
    max_processes: u32,

    /// File size limit in bytes
    #[arg(long, default_value = "10485760")] // 10MB
    file_size_limit: u64,

    /// Disk space limit in bytes
    #[arg(long, default_value = "104857600")] // 100MB
    disk_space_limit: u64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args = Args::parse();

    let resource_limits = ResourceLimits {
        memory: args.memory_limit,
        cpu_time: args.cpu_time_limit,
        processes: args.max_processes,
        file_size: args.file_size_limit,
        disk_space: args.disk_space_limit,
    };

    let app = create_app(args.max_concurrent, resource_limits).await?;
    run_server(app, args.addr).await?;

    Ok(())
}
