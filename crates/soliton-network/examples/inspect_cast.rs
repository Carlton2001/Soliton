//! Read-only receiver diagnostics: no app launch, media load or volume change.
fn main() -> anyhow::Result<()> {
    let address: std::net::SocketAddr = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("Usage: inspect_cast <ip:port>"))?
        .parse()?;
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async {
        let client = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            oxicast::CastClient::builder(address.ip().to_string(), address.port())
                .auto_reconnect(false)
                .request_timeout(std::time::Duration::from_secs(3))
                .connect(),
        )
        .await??;
        let status = client.receiver_status().await?;
        println!(
            "Standby: {}; active input: {}",
            status.is_stand_by, status.is_active_input
        );
        for app in status.applications {
            println!("{} | {}", app.app_id, app.display_name);
            if std::env::args().nth(2).as_deref() == Some("--close-test-receiver")
                && app.app_id == "CC1AD845"
            {
                client.stop_app(&app.session_id).await?;
                println!("Closed the Soliton test receiver");
            }
        }
        // Unlike disconnect(), dropping the client does not issue STOP_MEDIA.
        drop(client);
        anyhow::Ok(())
    })
}
