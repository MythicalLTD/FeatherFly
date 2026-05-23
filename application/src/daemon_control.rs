use std::{process::Command, time::Duration};

pub fn schedule_restart(delay_ms: u64) {
    tokio::spawn(async move {
        if delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        }

        tracing::info!("scheduling daemon restart");

        let Ok(exe) = std::env::current_exe() else {
            tracing::error!("restart failed: unable to locate current executable");
            return;
        };

        let args: Vec<String> = std::env::args().skip(1).collect();

        match Command::new(&exe).args(&args).spawn() {
            Ok(_) => {
                tracing::info!("replacement process spawned, exiting");
                std::process::exit(0);
            }
            Err(err) => {
                tracing::error!("restart failed: {err:#}");
            }
        }
    });
}
