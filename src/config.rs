#![cfg(not(target_arch = "wasm32"))]

pub fn print() {
    #[cfg(feature = "azure")]
    println!("{}", res_azure::ConnectOptions::requirements());
    #[cfg(all(feature = "kafka", not(feature = "nats")))]
    println!("{}", res_kafka::ConnectOptions::requirements());
    #[cfg(feature = "mongodb")]
    println!("{}", res_mongodb::ConnectOptions::requirements());
    #[cfg(feature = "nats")]
    println!("{}", res_nats::ConnectOptions::requirements());
    #[cfg(feature = "postgres")]
    println!("{}", res_postgres::ConnectOptions::requirements());
    #[cfg(feature = "redis")]
    println!("{}", res_redis::ConnectOptions::requirements());
}
