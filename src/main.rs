use chrono::{DateTime, Local};
use clap::Parser;
use solana_client::rpc_client::RpcClient;
use solana_sdk::signature::{Signature, read_keypair_file};
use solana_sdk::signer::Signer;
use solana_sdk::system_instruction;
use solana_sdk::transaction::Transaction;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::{Instant, SystemTime};

#[derive(Debug)]
struct BenchmarkResult {
    endpoint: String,
    start_time: Instant,
    start_system_time: SystemTime,
    end_time: Option<Instant>,
    end_system_time: Option<SystemTime>,
    block_height: Option<u64>,
    error: Option<String>,
    transaction_signature: Option<Signature>,
    transaction_block_height: Option<u64>,
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
            transaction_signature: None,
            transaction_block_height: None,
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

    fn set_transaction_signature(&mut self, signature: Signature) {
        self.transaction_signature = Some(signature);
    }

    fn set_transaction_block_height(&mut self, height: u64) {
        self.transaction_block_height = Some(height);
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

        let tx_signature = self
            .transaction_signature
            .map(|sig| sig.to_string())
            .unwrap_or_else(|| "No signature".to_string());

        let tx_block_height = self
            .transaction_block_height
            .map(|h| h.to_string())
            .unwrap_or_else(|| "N/A".to_string());

        let error_details = self
            .error
            .as_ref()
            .map(|err| format!("Error Details: {}\n", err))
            .unwrap_or_else(|| "".to_string());

        format!(
            "Endpoint: {}\nStart Time: {}\nEnd Time: {}\nStatus: {}\nTransaction Signature: {}\nTransaction Block Height: {}\n{}Duration: {}\n",
            self.endpoint,
            start_time,
            end_time,
            status,
            tx_signature,
            tx_block_height,
            error_details,
            duration
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

    let keypair = read_keypair_file(&args.keypair_path).unwrap();
    println!("Using Solana keypair at: {}", args.keypair_path.display());
    println!("Keypair public address: {}", keypair.pubkey());

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
    let keypair = Arc::new(keypair);
    for endpoint in endpoints {
        let keypair = Arc::clone(&keypair);
        let handle = thread::spawn(move || {
            let mut result = BenchmarkResult::new(endpoint.clone());

            // Create RPC client and fetch block height
            let rpc_client = RpcClient::new(endpoint.clone());

            println!("Connecting to {}", endpoint);

            match rpc_client.get_block_height() {
                Ok(height) => {
                    result.set_block_height(height);
                }
                Err(err) => {
                    result.set_error(err.to_string());
                    result.complete();
                    return result;
                }
            }

            // Create a simple transfer instruction
            let instruction = system_instruction::transfer(
                &keypair.pubkey(),
                &keypair.pubkey(),
                1, // Send 1 lamport to self
            );

            // Create and sign transaction - try multiple methods to get a blockhash
            let recent_blockhash = {
                // Method 1: Try get_latest_blockhash (newer method)
                if let Ok(blockhash) = rpc_client.get_latest_blockhash() {
                    println!("Got blockhash using get_latest_blockhash");
                    blockhash
                }
                // Method 2: Try get_latest_blockhash_with_commitment
                else if let Ok((blockhash, _)) =
                    rpc_client.get_latest_blockhash_with_commitment(rpc_client.commitment())
                {
                    println!("Got blockhash using get_latest_blockhash_with_commitment");
                    blockhash
                }
                // All methods failed
                else {
                    result.set_error(
                        "Failed to get blockhash: All available methods failed".to_string(),
                    );
                    result.complete();
                    return result;
                }
            };

            println!("Blockhash: {}", recent_blockhash);

            let transaction = Transaction::new_signed_with_payer(
                &[instruction],
                Some(&keypair.pubkey()),
                &[&keypair],
                recent_blockhash,
            );

            match rpc_client.send_and_confirm_transaction(&transaction) {
                Ok(signature) => {
                    println!("Transaction signature: {}", signature);

                    result.set_transaction_signature(signature);
                    // Get the block height for the confirmed transaction
                    match rpc_client.get_slot_with_commitment(rpc_client.commitment()) {
                        Ok(slot) => {
                            result.set_transaction_block_height(slot);
                        }
                        Err(err) => {
                            result.set_error(format!(
                                "Failed to get transaction block height: {}",
                                err
                            ));
                        }
                    }
                }
                Err(err) => {
                    result.set_error(format!("Transaction failed: {}", err));
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
