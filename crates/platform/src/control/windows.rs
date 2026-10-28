//! Windows: the named pipe through which a new Reroute hands its request to
//! the one running in the background.
//!
//! The pipe is `\\.\pipe\reroute-<session>-<hash>` (see
//! [`crate::paths::control_socket`]): one per signed-in session and
//! configuration directory. Named pipes share one namespace for the whole
//! computer, so a name proves nothing by itself:
//!
//! * The running Reroute creates the pipe with an access list that admits
//!   only its own user and the system, refuses clients from other computers,
//!   and claims the name with `FILE_FLAG_FIRST_PIPE_INSTANCE`. The name lives
//!   exactly as long as that process, crash included, so nothing it leaves
//!   behind is ever mistaken for a live Reroute.
//! * A new Reroute sends a link only to a pipe whose server runs as the same
//!   user, in case another user of the computer created a pipe of that name
//!   first. Such a pipe can at worst make each click start its own Reroute.
//!
//! Windows lets a program bring its window to the front only when the user
//! is working with it, or when a program they are working with allows it. A
//! click starts a new Reroute, which has that right; before sending its
//! request, it passes the right on to the running one
//! (`AllowSetForegroundWindow`), as the activation token does on Wayland.

// The pipe, the access list and the check of the server's user have no safe
// binding in the standard library or `winreg`. Every call says why it is
// sound in a `SAFETY:` comment; see docs/security.md.
#![allow(unsafe_code)]

use std::ffi::OsStr;
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::path::Path;
use std::sync::{Arc, mpsc};
use std::time::Duration;

use reroute_core::control::{MAX_REQUEST, OK, Request};
use windows_sys::Win32::Foundation::{
    ERROR_ACCESS_DENIED, ERROR_PIPE_CONNECTED, HANDLE, INVALID_HANDLE_VALUE, LocalFree,
};
use windows_sys::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows_sys::Win32::Security::{
    GetTokenInformation, PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES, TOKEN_QUERY, TOKEN_USER,
    TokenUser,
};
use windows_sys::Win32::Storage::FileSystem::{FILE_FLAG_FIRST_PIPE_INSTANCE, PIPE_ACCESS_DUPLEX};
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, GetNamedPipeServerProcessId, PIPE_READMODE_BYTE,
    PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
};
use windows_sys::Win32::System::RemoteDesktop::ProcessIdToSessionId;
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows_sys::Win32::UI::WindowsAndMessaging::AllowSetForegroundWindow;

/// How long a new Reroute waits for the running one to answer.
const TIMEOUT: Duration = Duration::from_secs(2);

/// Size of the pipe's buffers: a request is at most [`MAX_REQUEST`] bytes,
/// and Windows grows the buffer when needed.
const BUFFER: u32 = 4096;

/// The signed-in session this process runs in, which names the pipe: a
/// user signed in twice (a remote desktop beside the console) gets the
/// picker in the session they clicked in.
pub fn session() -> Option<u32> {
    let mut session = 0;
    // SAFETY: takes a process id by value and writes one `u32` through a
    // pointer to a local that outlives the call.
    let ok = unsafe { ProcessIdToSessionId(std::process::id(), &raw mut session) };
    (ok != 0).then_some(session)
}

/// Hand `request` to the Reroute serving `pipe`. False when none serves it
/// or it did not accept the request; the caller then handles the request
/// itself, so a click is never lost.
pub fn deliver(pipe: &Path, request: &Request) -> bool {
    ask(pipe, request).as_deref() == Some(OK)
}

/// Send `request` and return the answer, or `None` when no Reroute of this
/// user answered in time.
fn ask(pipe: &Path, request: &Request) -> Option<String> {
    let line = request.encode().ok()?;
    let stream = OpenOptions::new().read(true).write(true).open(pipe).ok()?;
    let server = server_process(&stream)?;
    if !runs_as_this_user(server) {
        log::warn!(
            "{} is served by another user's program; not sending it the link",
            pipe.display()
        );
        return None;
    }
    // SAFETY: takes a process id by value. Failing is harmless: the picker
    // then opens without coming to the front.
    unsafe { AllowSetForegroundWindow(server) };
    exchange_within_timeout(stream, line)
}

/// Write the request and read the answer on a thread of its own: reading a
/// pipe cannot time out, and a running Reroute that stopped responding (a
/// suspended process, for one) must not hold the click for longer than
/// [`TIMEOUT`].
fn exchange_within_timeout(stream: File, line: String) -> Option<String> {
    let (sent, received) = mpsc::channel();
    let spawned = std::thread::Builder::new()
        .name("reroute-hand-over".into())
        .spawn(move || {
            // The receiver is gone after a timeout; nothing to tell it then.
            let _ = sent.send(exchange(&stream, &line));
        });
    if let Err(error) = spawned {
        log::warn!("cannot reach the running Reroute: {error}");
        return None;
    }
    match received.recv_timeout(TIMEOUT) {
        Ok(Ok(answer)) => Some(answer.trim_end().to_owned()),
        Ok(Err(error)) => {
            log::warn!("the running Reroute did not answer: {error}");
            None
        }
        Err(_) => {
            log::warn!("the running Reroute did not answer within {TIMEOUT:?}");
            None
        }
    }
}

fn exchange(mut stream: &File, line: &str) -> io::Result<String> {
    stream.write_all(line.as_bytes())?;
    let mut answer = String::new();
    BufReader::new(stream.take(16)).read_line(&mut answer)?;
    Ok(answer)
}

/// The process at the other end of a pipe opened as a client.
fn server_process(stream: &File) -> Option<u32> {
    let mut pid = 0;
    // SAFETY: the handle is the open pipe that `stream` owns, valid for the
    // whole call; the id is written to a local that outlives it.
    let ok = unsafe { GetNamedPipeServerProcessId(stream.as_raw_handle(), &raw mut pid) };
    (ok != 0).then_some(pid)
}

/// Does process `pid` run as the user running this one? False when that
/// cannot be told, as for another user's process, which this one may not
/// inspect.
fn runs_as_this_user(pid: u32) -> bool {
    // SAFETY: takes an access mask, a flag and a process id by value. The
    // handle it returns, if any, is owned below.
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if process.is_null() {
        return false;
    }
    // SAFETY: `OpenProcess` succeeded, so `process` is a handle that nothing
    // else owns; `OwnedHandle` closes it once.
    let process = unsafe { OwnedHandle::from_raw_handle(process) };
    match (user_sid(process.as_raw_handle()), own_user_sid()) {
        (Some(theirs), Some(ours)) => theirs == ours,
        _ => false,
    }
}

/// The security identifier, as text (`S-1-5-21-…`), of the user running
/// this process.
fn own_user_sid() -> Option<String> {
    // SAFETY: returns a constant that stands for this process and need not
    // be closed.
    user_sid(unsafe { GetCurrentProcess() })
}

/// The security identifier, as text, of the user `process` runs as.
/// `process` must be a valid handle with query access.
fn user_sid(process: HANDLE) -> Option<String> {
    let mut token: HANDLE = std::ptr::null_mut();
    // SAFETY: `process` is valid for the call (see above); the new handle is
    // written to a local that outlives it.
    if unsafe { OpenProcessToken(process, TOKEN_QUERY, &raw mut token) } == 0 {
        return None;
    }
    // SAFETY: `OpenProcessToken` succeeded, so `token` is a handle that
    // nothing else owns; `OwnedHandle` closes it once.
    let token = unsafe { OwnedHandle::from_raw_handle(token) };

    // A first call says how large the answer is.
    let mut size = 0;
    // SAFETY: a null buffer of length zero asks only for the size, which is
    // written to a local that outlives the call. The call fails, as it must
    // without a buffer; the size tells whether it worked.
    unsafe {
        GetTokenInformation(
            token.as_raw_handle(),
            TokenUser,
            std::ptr::null_mut(),
            0,
            &raw mut size,
        )
    };
    if size == 0 {
        return None;
    }
    // `u64`s, so that the buffer is aligned for the `TOKEN_USER` at its start.
    let mut buffer = vec![0_u64; (size as usize).div_ceil(8)];
    // SAFETY: the buffer holds at least `size` writable bytes and lives
    // until the end of this function; the call writes no more than `size`.
    let ok = unsafe {
        GetTokenInformation(
            token.as_raw_handle(),
            TokenUser,
            buffer.as_mut_ptr().cast(),
            size,
            &raw mut size,
        )
    };
    if ok == 0 {
        return None;
    }
    // SAFETY: on success the buffer starts with a `TOKEN_USER`, suitably
    // aligned (see above), whose `Sid` points into the same buffer.
    let sid = unsafe { (*buffer.as_ptr().cast::<TOKEN_USER>()).User.Sid };
    let mut text: *mut u16 = std::ptr::null_mut();
    // SAFETY: `sid` is valid while `buffer` lives; the call allocates the
    // text and writes its address to a local that outlives the call.
    if unsafe { ConvertSidToStringSidW(sid, &raw mut text) } == 0 {
        return None;
    }
    // SAFETY: on success `text` is a NUL-terminated UTF-16 string, read up
    // to its NUL and then freed once with `LocalFree`, as the API requires.
    unsafe {
        let mut len = 0;
        while *text.add(len) != 0 {
            len += 1;
        }
        let sid = String::from_utf16_lossy(std::slice::from_raw_parts(text, len));
        LocalFree(text.cast());
        Some(sid)
    }
}

/// A security descriptor built from its text form, freed when dropped.
struct SecurityDescriptor(PSECURITY_DESCRIPTOR);

impl SecurityDescriptor {
    /// The access list of the pipe: full access for `user` and for the
    /// system, nobody else. `P` protects it from inherited entries.
    fn for_user(user: &str) -> io::Result<Self> {
        let text = wide(OsStr::new(&format!("D:P(A;;GA;;;{user})(A;;GA;;;SY)")));
        let mut descriptor: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
        // SAFETY: `text` is NUL-terminated UTF-16 that lives until the call
        // returns; the descriptor's address is written to a local that
        // outlives it. The size is not wanted, which a null pointer says.
        let ok = unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                text.as_ptr(),
                SDDL_REVISION_1,
                &raw mut descriptor,
                std::ptr::null_mut(),
            )
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self(descriptor))
    }
}

impl Drop for SecurityDescriptor {
    fn drop(&mut self) {
        // SAFETY: the descriptor was allocated by
        // `ConvertStringSecurityDescriptorToSecurityDescriptorW`, which says
        // to free it with `LocalFree`, and is freed only here.
        unsafe { LocalFree(self.0) };
    }
}

/// A Reroute that owns the pipe's name and answers on it.
#[derive(Debug)]
pub struct Listener {
    /// The pipe's name, NUL-terminated UTF-16.
    name: Vec<u16>,
    /// The user the pipe admits, as text.
    user: String,
    /// The first instance of the pipe, which holds the name.
    first: OwnedHandle,
}

/// Become the Reroute that later processes hand their requests to. `None`
/// when another one already is: the name is taken.
pub fn claim(pipe: &Path) -> io::Result<Option<Listener>> {
    let name = wide(pipe.as_os_str());
    let user =
        own_user_sid().ok_or_else(|| io::Error::other("cannot tell which user runs Reroute"))?;
    match create_instance(&name, &user, true) {
        Ok(first) => Ok(Some(Listener { name, user, first })),
        Err(error) if is(&error, ERROR_ACCESS_DENIED) => Ok(None),
        Err(error) => Err(error),
    }
}

/// One instance of the pipe, waiting for a client. `first` claims the name,
/// and fails if the name already exists.
fn create_instance(name: &[u16], user: &str, first: bool) -> io::Result<OwnedHandle> {
    let descriptor = SecurityDescriptor::for_user(user)?;
    let attributes = SECURITY_ATTRIBUTES {
        nLength: SECURITY_ATTRIBUTES_SIZE,
        lpSecurityDescriptor: descriptor.0,
        bInheritHandle: 0,
    };
    let mut open_mode = PIPE_ACCESS_DUPLEX;
    if first {
        open_mode |= FILE_FLAG_FIRST_PIPE_INSTANCE;
    }
    // SAFETY: `name` is NUL-terminated UTF-16 and `attributes` points to a
    // live descriptor; both outlive the call, which keeps neither.
    let handle = unsafe {
        CreateNamedPipeW(
            name.as_ptr(),
            open_mode,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
            PIPE_UNLIMITED_INSTANCES,
            BUFFER,
            BUFFER,
            0,
            &raw const attributes,
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `CreateNamedPipeW` succeeded, so `handle` is a handle that
    // nothing else owns; `OwnedHandle` closes it once.
    Ok(unsafe { OwnedHandle::from_raw_handle(handle) })
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "SECURITY_ATTRIBUTES is a few bytes long"
)]
const SECURITY_ATTRIBUTES_SIZE: u32 = std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32;

/// Wait until a client opens `pipe`.
fn wait_for_client(pipe: &OwnedHandle) -> io::Result<()> {
    // SAFETY: the handle is a pipe instance that `pipe` owns, valid for the
    // whole call; no `OVERLAPPED`, so the call waits.
    if unsafe { ConnectNamedPipe(pipe.as_raw_handle(), std::ptr::null_mut()) } != 0 {
        return Ok(());
    }
    let error = io::Error::last_os_error();
    // A client that opened the pipe between its creation and this call.
    if is(&error, ERROR_PIPE_CONNECTED) {
        Ok(())
    } else {
        Err(error)
    }
}

impl Listener {
    /// Answer requests on a thread of its own, passing each one that is
    /// understood to `handle`. Each client gets a thread, so that one that
    /// stops halfway does not keep the others waiting.
    pub fn serve<F>(self, handle: F)
    where
        F: Fn(Request) + Send + Sync + 'static,
    {
        let spawned = std::thread::Builder::new()
            .name("reroute-control".into())
            .spawn(move || self.accept(&Arc::new(handle)));
        if let Err(error) = spawned {
            log::error!("cannot listen for other Reroute processes: {error}");
        }
    }

    fn accept<F>(self, handle: &Arc<F>)
    where
        F: Fn(Request) + Send + Sync + 'static,
    {
        let Self { name, user, first } = self;
        let mut waiting = first;
        loop {
            let connected = wait_for_client(&waiting);
            // The next instance exists before this one is passed on or
            // closed, so that the name never lapses and no second Reroute
            // can claim it.
            let next = match create_instance(&name, &user, false) {
                Ok(next) => next,
                Err(error) => {
                    log::error!("cannot listen for other Reroute processes: {error}");
                    return;
                }
            };
            let client = File::from(std::mem::replace(&mut waiting, next));
            if let Err(error) = connected {
                // A client that left at once, for one: this instance goes.
                log::warn!("control pipe: {error}");
                continue;
            }
            let handle = Arc::clone(handle);
            let spawned = std::thread::Builder::new()
                .name("reroute-request".into())
                .spawn(move || {
                    if let Some(request) = answer(&client) {
                        handle(request);
                    }
                });
            if let Err(error) = spawned {
                log::warn!("cannot answer another Reroute: {error}");
            }
        }
    }
}

/// Read one request, say whether it was understood, and return it if so.
fn answer(mut stream: &File) -> Option<Request> {
    let mut line = String::new();
    BufReader::new(stream.take(MAX_REQUEST as u64))
        .read_line(&mut line)
        .ok()?;
    let request = Request::decode(&line);
    let reply = if request.is_ok() { "ok\n" } else { "error\n" };
    stream.write_all(reply.as_bytes()).ok()?;
    request
        .map_err(|error| log::warn!("refused a request on the control pipe: {error}"))
        .ok()
}

/// `text` as NUL-terminated UTF-16, as the Windows API takes it.
fn wide(text: &OsStr) -> Vec<u16> {
    text.encode_wide().chain([0]).collect()
}

/// Is `error` the Windows error `code`?
fn is(error: &io::Error, code: u32) -> bool {
    error.raw_os_error().and_then(|raw| u32::try_from(raw).ok()) == Some(code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A pipe name of its own for each test: tests run in parallel, and
    /// names are shared by the whole computer.
    fn pipe(test: &str) -> PathBuf {
        PathBuf::from(format!(
            r"\\.\pipe\reroute-test-{}-{test}",
            std::process::id()
        ))
    }

    #[test]
    fn a_request_reaches_the_running_reroute() {
        let pipe = pipe("reaches");
        let listener = claim(&pipe).unwrap().expect("first to claim");
        let (sent, received) = mpsc::channel();
        listener.serve(move |request| sent.send(request).unwrap());

        let open = Request::Open {
            url: "https://example.com/".into(),
            activation: None,
        };
        assert!(deliver(&pipe, &open));
        let timeout = Duration::from_secs(5);
        assert_eq!(received.recv_timeout(timeout).unwrap(), open);
        assert!(deliver(&pipe, &Request::Settings));
        assert_eq!(received.recv_timeout(timeout).unwrap(), Request::Settings);
    }

    #[test]
    fn only_one_reroute_claims_the_name() {
        let pipe = pipe("once");
        let first = claim(&pipe).unwrap();
        assert!(first.is_some());
        assert!(claim(&pipe).unwrap().is_none(), "the name is taken");
        drop(first);
        assert!(claim(&pipe).unwrap().is_some(), "free again");
    }

    #[test]
    fn the_name_stays_taken_while_serving() {
        let pipe = pipe("serving");
        claim(&pipe).unwrap().unwrap().serve(|_| {});
        for _ in 0..3 {
            assert!(deliver(&pipe, &Request::Settings));
            assert!(claim(&pipe).unwrap().is_none());
        }
    }

    #[test]
    fn nobody_listening_means_the_caller_does_the_work() {
        assert!(!deliver(&pipe("nobody"), &Request::Settings));
    }

    #[test]
    fn nonsense_is_answered_but_not_passed_on() {
        let pipe = pipe("nonsense");
        let (sent, received) = mpsc::channel();
        claim(&pipe)
            .unwrap()
            .unwrap()
            .serve(move |request| sent.send(request).unwrap());
        let stream = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&pipe)
            .unwrap();
        (&stream).write_all(b"del /s /q C:\\\n").unwrap();
        let mut reply = String::new();
        BufReader::new(&stream).read_line(&mut reply).unwrap();
        assert_eq!(reply, "error\n");
        assert!(received.recv_timeout(Duration::from_millis(200)).is_err());
    }

    #[test]
    fn a_server_of_this_user_is_recognised() {
        assert!(own_user_sid().unwrap().starts_with("S-1-"));
        assert!(runs_as_this_user(std::process::id()));
        // The System Idle Process cannot be inspected: it does not count.
        assert!(!runs_as_this_user(0));
    }

    #[test]
    fn this_process_has_a_session() {
        assert!(session().is_some());
    }
}
