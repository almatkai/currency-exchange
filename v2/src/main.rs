#[tokio::main(worker_threads = 2)]
async fn main() {
    match tezbar_currency_exchange::Adapter::new() {
        Ok(adapter) => {
            if tezbar_extension_sdk::serve(adapter).await.is_err() {
                eprintln!("Currency Exchange protocol stopped");
                std::process::exit(1);
            }
        }
        Err(_) => {
            eprintln!("Currency Exchange initialization failed");
            std::process::exit(1);
        }
    }
}
