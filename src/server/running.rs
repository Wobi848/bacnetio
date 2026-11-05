//! One bacnetio per data folder: a second start opens the running one in the browser instead of
//! starting a second server, which would write the same files and, without BACnet port 47808,
//! find fewer devices.

use std::net::{Ipv4Addr, SocketAddr};
use std::path::Path;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Holds the web port of the bacnetio serving the folder.
const PORT_FILE: &str = "web-port";
/// A bacnetio on this computer answers at once; a stale port must not hold up the start.
const ASK_TIMEOUT: Duration = Duration::from_secs(2);

/// The address of a bacnetio that already serves `data_dir`.
pub async fn already_running(data_dir: &Path) -> Option<SocketAddr> {
    let text = std::fs::read_to_string(data_dir.join(PORT_FILE)).ok()?;
    let port: u16 = text.trim().parse().ok()?;
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let answers = tokio::time::timeout(ASK_TIMEOUT, is_bacnetio(address)).await;
    matches!(answers, Ok(true)).then_some(address)
}

/// Notes the port for the next start; after a crash the note stays, and the next start finds
/// nobody answering there.
pub fn remember(data_dir: &Path, address: SocketAddr) -> std::io::Result<()> {
    std::fs::create_dir_all(data_dir)?;
    std::fs::write(data_dir.join(PORT_FILE), address.port().to_string())
}

/// Another program may have taken the port since: only bacnetio's own answer counts.
async fn is_bacnetio(address: SocketAddr) -> bool {
    let Ok(mut stream) = TcpStream::connect(address).await else {
        return false;
    };
    let request = format!(
        "GET /api/info HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nConnection: close\r\n\r\n",
        address.port()
    );
    let mut answer = Vec::new();
    if stream.write_all(request.as_bytes()).await.is_err()
        || stream.read_to_end(&mut answer).await.is_err()
    {
        return false;
    }
    let answer = String::from_utf8_lossy(&answer);
    answer.split_whitespace().nth(1) == Some("200") && answer.contains("\"version\"")
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use tokio::net::TcpListener;

    use super::*;
    use crate::server::{router, session::AppState};

    fn folder(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("bacnetio-{name}-{}", std::process::id()));
        // left over from an aborted run
        let _ = std::fs::remove_dir_all(&path);
        path
    }

    #[tokio::test]
    async fn a_running_bacnetio_is_found_by_its_folder() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
        let address = listener.local_addr().unwrap();
        let mut state = AppState::default();
        state.web_address = Some(address);
        tokio::spawn(axum::serve(listener, router(Arc::new(state))).into_future());
        let dir = folder("running");
        remember(&dir, address).unwrap();
        assert_eq!(already_running(&dir).await, Some(address));
    }

    #[tokio::test]
    async fn a_note_left_by_a_crash_or_another_program_is_ignored() {
        let dir = folder("stale");
        assert_eq!(already_running(&dir).await, None);
        // a port nobody listens on any more
        let closed = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
        let address = closed.local_addr().unwrap();
        drop(closed);
        remember(&dir, address).unwrap();
        assert_eq!(already_running(&dir).await, None);
        // another program on the port
        let other = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
        let address = other.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut stream, _) = other.accept().await.unwrap();
            let mut request = [0; 512];
            let _ = stream.read(&mut request).await;
            let _ = stream.write_all(b"HTTP/1.1 200 OK\r\n\r\nhello").await;
        });
        remember(&dir, address).unwrap();
        assert_eq!(already_running(&dir).await, None);
        std::fs::write(dir.join(PORT_FILE), "not a port").unwrap();
        assert_eq!(already_running(&dir).await, None);
    }
}
