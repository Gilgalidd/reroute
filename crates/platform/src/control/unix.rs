//! The local socket through which a new Reroute process hands its request
//! to the one running in the background (see [`reroute_core::control`]).
//!
//! The socket lives in `$XDG_RUNTIME_DIR/reroute/` (see
//! [`crate::paths::control_socket`]). The runtime directory belongs to the
//! user with mode 0700, and Reroute's directory inside it is 0700 too, so
//! only the user's own processes can connect; they could already run
//! `reroute <url>` themselves, and a request goes through the same checks.
//! A link is only ever sent into a directory that is the user's alone, so
//! that a session whose runtime directory is shared by mistake does not
//! hand the user's links to a socket someone else put there.
//!
//! The running Reroute is the one holding an exclusive lock on a file next
//! to the socket. The system releases that lock when the process ends, even
//! in a crash, so a socket left behind is never mistaken for a live one.

use std::fs::{File, TryLockError};
use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::time::Duration;

use reroute_core::control::{MAX_REQUEST, OK, Request};

/// How long either side waits for the other.
const TIMEOUT: Duration = Duration::from_secs(2);

/// Hand `request` to the Reroute listening on `socket`. False when none
/// listens or it did not accept the request; the caller then handles the
/// request itself, so a click is never lost.
pub fn deliver(socket: &Path, request: &Request) -> bool {
    if !socket.parent().is_some_and(is_private) {
        return false;
    }
    match ask(socket, request) {
        Some(answer) if answer == OK => true,
        // After an update, the Reroute still running until the next login
        // may be 0.1.12, which refuses a request with an activation token:
        // ask it again without, rather than start a second Reroute.
        Some(_) => request
            .without_activation()
            .is_some_and(|plain| ask(socket, &plain).as_deref() == Some(OK)),
        None => false,
    }
}

/// Send `request` and return the answer, or `None` when no Reroute answered.
fn ask(socket: &Path, request: &Request) -> Option<String> {
    let line = request.encode().ok()?;
    let stream = UnixStream::connect(socket).ok()?;
    match exchange(&stream, &line) {
        Ok(answer) => Some(answer.trim_end().to_owned()),
        Err(error) => {
            log::warn!("the running Reroute did not answer: {error}");
            None
        }
    }
}

fn exchange(mut stream: &UnixStream, line: &str) -> std::io::Result<String> {
    stream.set_read_timeout(Some(TIMEOUT))?;
    stream.set_write_timeout(Some(TIMEOUT))?;
    stream.write_all(line.as_bytes())?;
    let mut answer = String::new();
    BufReader::new(stream.take(16)).read_line(&mut answer)?;
    Ok(answer)
}

/// A Reroute that holds the lock and listens on the socket.
#[derive(Debug)]
pub struct Listener {
    listener: UnixListener,
    /// Held for the life of the process; see the module documentation.
    lock: File,
}

/// Become the Reroute that later processes hand their requests to. `None`
/// when another one already is.
pub fn claim(socket: &Path) -> std::io::Result<Option<Listener>> {
    if let Some(dir) = socket.parent() {
        create_private_dir(dir)?;
    }
    let lock = File::create(socket.with_extension("lock"))?;
    match lock.try_lock() {
        Ok(()) => {}
        Err(TryLockError::WouldBlock) => return Ok(None),
        Err(TryLockError::Error(error)) => return Err(error),
    }
    // Holding the lock, any socket file here is one a dead Reroute left.
    match std::fs::remove_file(socket) {
        Err(error) if error.kind() != ErrorKind::NotFound => return Err(error),
        _ => {}
    }
    let listener = UnixListener::bind(socket)?;
    Ok(Some(Listener { listener, lock }))
}

impl Listener {
    /// Answer requests on a thread of its own, passing each one that is
    /// understood to `handle`.
    pub fn serve<F>(self, handle: F)
    where
        F: Fn(Request) + Send + 'static,
    {
        let spawned = std::thread::Builder::new()
            .name("reroute-control".into())
            .spawn(move || {
                let Self { listener, lock } = self;
                for stream in listener.incoming() {
                    match stream {
                        Ok(stream) => {
                            if let Some(request) = answer(&stream) {
                                handle(request);
                            }
                        }
                        Err(error) => log::warn!("control socket: {error}"),
                    }
                }
                drop(lock);
            });
        if let Err(error) = spawned {
            log::error!("cannot listen for other Reroute processes: {error}");
        }
    }
}

/// Read one request, say whether it was understood, and return it if so.
fn answer(mut stream: &UnixStream) -> Option<Request> {
    stream.set_read_timeout(Some(TIMEOUT)).ok()?;
    stream.set_write_timeout(Some(TIMEOUT)).ok()?;
    let mut line = String::new();
    BufReader::new(stream.take(MAX_REQUEST as u64))
        .read_line(&mut line)
        .ok()?;
    let request = Request::decode(&line);
    let reply = if request.is_ok() { "ok\n" } else { "error\n" };
    stream.write_all(reply.as_bytes()).ok()?;
    request
        .map_err(|error| log::warn!("refused a request on the control socket: {error}"))
        .ok()
}

/// Is `dir` a real directory that belongs to this user and that nobody
/// else may open?
#[expect(
    clippy::verbose_bit_mask,
    reason = "`mode & 0o077 == 0` reads as \"no rights for the group or others\""
)]
fn is_private(dir: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    std::fs::symlink_metadata(dir).is_ok_and(|meta| {
        meta.is_dir()
            && meta.uid() == rustix::process::getuid().as_raw()
            && meta.mode() & 0o077 == 0
    })
}

fn create_private_dir(dir: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)?;
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    fn socket_in(dir: &tempfile::TempDir) -> std::path::PathBuf {
        dir.path().join("reroute").join("control.sock")
    }

    #[test]
    fn a_request_reaches_the_running_reroute() {
        let dir = tempfile::tempdir().unwrap();
        let socket = socket_in(&dir);
        let listener = claim(&socket).unwrap().expect("first to claim");
        let (sent, received) = mpsc::channel();
        listener.serve(move |request| sent.send(request).unwrap());

        let open = Request::Open {
            url: "https://example.com/".into(),
            activation: Some("kwin-12".into()),
        };
        assert!(deliver(&socket, &open));
        assert!(deliver(&socket, &Request::Settings));
        let timeout = Duration::from_secs(5);
        assert_eq!(received.recv_timeout(timeout).unwrap(), open);
        assert_eq!(received.recv_timeout(timeout).unwrap(), Request::Settings);
    }

    #[test]
    fn only_one_reroute_claims_the_socket() {
        let dir = tempfile::tempdir().unwrap();
        let socket = socket_in(&dir);
        let first = claim(&socket).unwrap();
        assert!(first.is_some());
        assert!(claim(&socket).unwrap().is_none(), "the lock is held");
        drop(first);
        assert!(claim(&socket).unwrap().is_some(), "free again");
    }

    #[test]
    fn a_socket_left_by_a_dead_reroute_is_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let socket = socket_in(&dir);
        std::fs::create_dir_all(socket.parent().unwrap()).unwrap();
        std::fs::write(&socket, b"").unwrap();
        assert!(!deliver(&socket, &Request::Settings));
        assert!(claim(&socket).unwrap().is_some());
    }

    #[test]
    fn nobody_listening_means_the_caller_does_the_work() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!deliver(&socket_in(&dir), &Request::Settings));
    }

    #[test]
    fn nonsense_is_answered_but_not_passed_on() {
        let dir = tempfile::tempdir().unwrap();
        let socket = socket_in(&dir);
        let (sent, received) = mpsc::channel();
        claim(&socket)
            .unwrap()
            .unwrap()
            .serve(move |request| sent.send(request).unwrap());
        let mut stream = UnixStream::connect(&socket).unwrap();
        stream.write_all(b"rm -rf /\n").unwrap();
        let mut reply = String::new();
        BufReader::new(&stream).read_line(&mut reply).unwrap();
        assert_eq!(reply, "error\n");
        assert!(received.recv_timeout(Duration::from_millis(200)).is_err());
    }

    /// A Reroute 0.1.12 knows only `open <url>`: it gets the request again
    /// without the token, and opens the link.
    #[test]
    fn a_reroute_from_before_tokens_still_gets_the_link() {
        let dir = tempfile::tempdir().unwrap();
        let socket = socket_in(&dir);
        create_private_dir(socket.parent().unwrap()).unwrap();
        let old = UnixListener::bind(&socket).unwrap();
        let (sent, received) = mpsc::channel();
        std::thread::spawn(move || {
            for stream in old.incoming().take(2) {
                let mut stream = stream.unwrap();
                let mut line = String::new();
                BufReader::new(&stream).read_line(&mut line).unwrap();
                let reply = if line.starts_with("open ") {
                    "ok\n"
                } else {
                    "error\n"
                };
                stream.write_all(reply.as_bytes()).unwrap();
                sent.send(line).unwrap();
            }
        });
        let open = Request::Open {
            url: "https://example.com/".into(),
            activation: Some("kwin-12".into()),
        };
        assert!(deliver(&socket, &open));
        let timeout = Duration::from_secs(5);
        assert!(
            received
                .recv_timeout(timeout)
                .unwrap()
                .starts_with("open-activated ")
        );
        assert_eq!(
            received.recv_timeout(timeout).unwrap(),
            "open https://example.com/\n"
        );
    }

    #[test]
    fn no_link_goes_into_a_directory_others_can_open() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let socket = socket_in(&dir);
        claim(&socket).unwrap().unwrap().serve(|_| {});
        assert!(deliver(&socket, &Request::Settings));
        std::fs::set_permissions(
            socket.parent().unwrap(),
            std::fs::Permissions::from_mode(0o755),
        )
        .unwrap();
        assert!(!deliver(&socket, &Request::Settings));
    }

    #[test]
    fn the_directory_is_private() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let socket = socket_in(&dir);
        let _listener = claim(&socket).unwrap();
        let mode = std::fs::metadata(socket.parent().unwrap())
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o700);
    }
}
