use chrono::{DateTime, Local};
use clap::Parser;
use solana_client::rpc_client::RpcClient;
use std::path::PathBuf;
use std::thread;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug)]
struct BenchmarkResult {
    endpoint: String,
    start_time: Instant,
    start_system_time: SystemTime,
    end_time: Option<Instant>,
    end_system_time: Option<SystemTime>,
    block_height: Option<u64>,
    error: Option<String>,
}

impl BenchmarkResult {
    fn new(endpoint: String) -> Self {
        Self {
            endpoint,
            start_time: Instant::now(),
            start_system_time: SystemTime::now(),
            end_time: None,
            end_system_time: None,
            block_height: None,
            error: None,
        }
    }

    fn complete(&mut self) {
        self.end_time = Some(Instant::now());
        self.end_system_time = Some(SystemTime::now());
    }

    fn duration(&self) -> Option<std::time::Duration> {
        self.end_time.map(|end| end.duration_since(self.start_time))
    }

    fn set_error(&mut self, error: String) {
        self.error = Some(error);
    }

    fn set_block_height(&mut self, height: u64) {
        self.block_height = Some(height);
    }

    fn format_system_time(time: SystemTime) -> String {
        let datetime: DateTime<Local> = time.into();
        datetime.format("%Y-%m-%d %H:%M:%S.%3f %Z").to_string()
    }

    fn display(&self) -> String {
        let duration = self
            .duration()
            .map(|d| format!("{:.2?}", d))
            .unwrap_or_else(|| "N/A".to_string());

        let status = if let Some(height) = self.block_height {
            format!("Success (Block Height: {})", height)
        } else if let Some(ref error) = self.error {
            format!("Error: {}", error)
        } else {
            "Unknown Status".to_string()
        };

        let start_time = Self::format_system_time(self.start_system_time);
        let end_time = self
            .end_system_time
            .map(Self::format_system_time)
            .unwrap_or_else(|| "N/A".to_string());

        format!(
            "Endpoint: {}\nStart Time: {}\nEnd Time: {}\nStatus: {}\nDuration: {}\n",
            self.endpoint, start_time, end_time, status, duration
        )
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Comma-separated list of Solana RPC endpoints
    #[arg(short, long)]
    endpoints: String,

    /// Path to the Solana keypair JSON file
    #[arg(short = 'k', long = "keypair")]
    keypair_path: PathBuf,
}

fn main() {
    let args = Args::parse();

    println!("Using Solana keypair at: {}", args.keypair_path.display());

    let endpoints: Vec<String> = args
        .endpoints
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();

    println!(
        "\nStarting benchmark for {} endpoints...\n",
        endpoints.len()
    );

    let mut handles = vec![];

    // Spawn a thread for each endpoint
    for endpoint in endpoints {
        let handle = thread::spawn(move || {
            let mut result = BenchmarkResult::new(endpoint.clone());

            // Create RPC client and fetch block height
            let rpc_client = RpcClient::new(endpoint);
            match rpc_client.get_block_height() {
                Ok(height) => {
                    result.set_block_height(height);
                }
                Err(err) => {
                    result.set_error(err.to_string());
                }
            }

            result.complete();
            result
        });
        handles.push(handle);
    }

    // Collect all results
    let results: Vec<BenchmarkResult> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();

    // Display results
    println!("\nBenchmark Results:");
    println!("=================");
    for (i, result) in results.iter().enumerate() {
        println!("\nEndpoint #{}", i + 1);
        println!("-----------");
        print!("{}", result.display());
    }
}
