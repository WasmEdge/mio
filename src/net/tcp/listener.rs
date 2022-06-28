use crate::io_source::IoSource;
use crate::net::TcpStream;
#[cfg(unix)]
use crate::sys::tcp::set_reuseaddr;
#[cfg(not(feature = "wasmedge"))]
use crate::sys::{
    self,
    tcp::{bind, listen, new_for_addr},
};
use crate::{event, Interest, Registry, Token};
#[cfg(not(feature = "wasmedge"))]
use std::net;
use std::net::SocketAddr;
#[cfg(unix)]
use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};
#[cfg(target_os = "wasi")]
use std::os::wasi::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};
#[cfg(windows)]
use std::os::windows::io::{AsRawSocket, FromRawSocket, IntoRawSocket, RawSocket};
use std::{fmt, io};
/// A structure representing a socket server
///
/// # Examples
///
#[cfg_attr(feature = "os-poll", doc = "```")]
#[cfg_attr(not(feature = "os-poll"), doc = "```ignore")]
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use mio::{Events, Interest, Poll, Token};
/// use mio::net::TcpListener;
/// use std::time::Duration;
///
/// let mut listener = TcpListener::bind("127.0.0.1:34255".parse()?)?;
///
/// let mut poll = Poll::new()?;
/// let mut events = Events::with_capacity(128);
///
/// // Register the socket with `Poll`
/// poll.registry().register(&mut listener, Token(0), Interest::READABLE)?;
///
/// poll.poll(&mut events, Some(Duration::from_millis(100)))?;
///
/// // There may be a socket ready to be accepted
/// #     Ok(())
/// # }
/// ```
pub struct TcpListener {
    #[cfg(not(feature = "wasmedge"))]
    inner: IoSource<net::TcpListener>,
    #[cfg(feature = "wasmedge")]
    inner: IoSource<wasmedge_wasi_socket::TcpListener>,
}

impl TcpListener {
    /// Convenience method to bind a new TCP listener to the specified address
    /// to receive new connections.
    ///
    /// This function will take the following steps:
    ///
    /// 1. Create a new TCP socket.
    /// 2. Set the `SO_REUSEADDR` option on the socket on Unix.
    /// 3. Bind the socket to the specified address.
    /// 4. Calls `listen` on the socket to prepare it to receive new connections.
    // #[cfg(not(target_os = "wasi"))]
    #[cfg(not(feature = "wasmedge"))]
    pub fn bind(addr: SocketAddr) -> io::Result<TcpListener> {
        let socket = new_for_addr(addr)?;
        #[cfg(unix)]
        let listener = unsafe { TcpListener::from_raw_fd(socket) };
        #[cfg(windows)]
        let listener = unsafe { TcpListener::from_raw_socket(socket as _) };

        // On platforms with Berkeley-derived sockets, this allows to quickly
        // rebind a socket, without needing to wait for the OS to clean up the
        // previous one.
        //
        // On Windows, this allows rebinding sockets which are actively in use,
        // which allows “socket hijacking”, so we explicitly don't set it here.
        // https://docs.microsoft.com/en-us/windows/win32/winsock/using-so-reuseaddr-and-so-exclusiveaddruse
        #[cfg(not(windows))]
        set_reuseaddr(&listener.inner, true)?;

        bind(&listener.inner, addr)?;
        listen(&listener.inner, 1024)?;
        Ok(listener)
    }

    /// bind wasi
    #[cfg(feature = "wasmedge")]
    pub fn bind(addr: SocketAddr) -> io::Result<TcpListener> {
        let inner = wasmedge_wasi_socket::TcpListener::bind(addr, true)?;
        Ok(TcpListener {
            inner: IoSource::new(inner),
        })
    }

    /// Creates a new `TcpListener` from a standard `net::TcpListener`.
    ///
    /// This function is intended to be used to wrap a TCP listener from the
    /// standard library in the Mio equivalent. The conversion assumes nothing
    /// about the underlying listener; ; it is left up to the user to set it
    /// in non-blocking mode.
    #[cfg(not(feature = "wasmedge"))]
    pub fn from_std(listener: net::TcpListener) -> TcpListener {
        TcpListener {
            inner: IoSource::new(listener),
        }
    }

    /// fromstd wasi
    #[cfg(feature = "wasmedge")]
    pub fn from_std(listener: wasmedge_wasi_socket::TcpListener) -> TcpListener {
        TcpListener {
            inner: IoSource::new(listener),
        }
    }

    /// Accepts a new `TcpStream`.
    ///
    /// This may return an `Err(e)` where `e.kind()` is
    /// `io::ErrorKind::WouldBlock`. This means a stream may be ready at a later
    /// point and one should wait for an event before calling `accept` again.
    ///
    /// If an accepted stream is returned, the remote address of the peer is
    /// returned along with it.
    pub fn accept(&self) -> io::Result<(TcpStream, SocketAddr)> {
        #[cfg(not(feature = "wasmedge"))]
        return self.inner.do_io(|inner| {
            sys::tcp::accept(inner).map(|(stream, addr)| (TcpStream::from_std(stream), addr))
        });
        #[cfg(feature = "wasmedge")]
        return self.inner.do_io(|inner| {
            self.inner
                .accept(true)
                .map(|(stream, addr)| (TcpStream::from_std(stream), addr))
        });
    }

    /// Returns the local socket address of this listener.
    #[cfg(not(feature = "wasmedge"))]
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        return self.inner.local_addr();
    }

    /// Returns the local socket address of this listener.
    #[cfg(feature = "wasmedge")]
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        return self.inner.local_addr();
    }

    /// Sets the value for the `IP_TTL` option on this socket.
    ///
    /// This value sets the time-to-live field that is used in every packet sent
    /// from this socket.
    pub fn set_ttl(&self, ttl: u32) -> io::Result<()> {
        #[cfg(not(feature = "wasmedge"))]
        return self.inner.set_ttl(ttl);
        #[cfg(feature = "wasmedge")]
        Ok(())
    }

    /// Gets the value of the `IP_TTL` option for this socket.
    ///
    /// For more information about this option, see [`set_ttl`][link].
    ///
    /// [link]: #method.set_ttl
    pub fn ttl(&self) -> io::Result<u32> {
        #[cfg(not(feature = "wasmedge"))]
        return self.inner.ttl();
        #[cfg(feature = "wasmedge")]
        Ok(0)
    }

    /// Get the value of the `SO_ERROR` option on this socket.
    ///
    /// This will retrieve the stored error in the underlying socket, clearing
    /// the field in the process. This can be useful for checking errors between
    /// calls.
    pub fn take_error(&self) -> io::Result<Option<io::Error>> {
        #[cfg(not(feature = "wasmedge"))]
        return self.inner.take_error();
        #[cfg(feature = "wasmedge")]
        Ok(None)
    }
}

impl event::Source for TcpListener {
    fn register(
        &mut self,
        registry: &Registry,
        token: Token,
        interests: Interest,
    ) -> io::Result<()> {
        self.inner.register(registry, token, interests)
    }

    fn reregister(
        &mut self,
        registry: &Registry,
        token: Token,
        interests: Interest,
    ) -> io::Result<()> {
        self.inner.reregister(registry, token, interests)
    }

    fn deregister(&mut self, registry: &Registry) -> io::Result<()> {
        self.inner.deregister(registry)
    }
}

impl fmt::Debug for TcpListener {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

#[cfg(unix)]
impl IntoRawFd for TcpListener {
    fn into_raw_fd(self) -> RawFd {
        self.inner.into_inner().into_raw_fd()
    }
}

#[cfg(unix)]
impl AsRawFd for TcpListener {
    fn as_raw_fd(&self) -> RawFd {
        self.inner.as_raw_fd()
    }
}

#[cfg(unix)]
impl FromRawFd for TcpListener {
    /// Converts a `RawFd` to a `TcpListener`.
    ///
    /// # Notes
    ///
    /// The caller is responsible for ensuring that the socket is in
    /// non-blocking mode.
    unsafe fn from_raw_fd(fd: RawFd) -> TcpListener {
        TcpListener::from_std(FromRawFd::from_raw_fd(fd))
    }
}

#[cfg(windows)]
impl IntoRawSocket for TcpListener {
    fn into_raw_socket(self) -> RawSocket {
        self.inner.into_inner().into_raw_socket()
    }
}

#[cfg(windows)]
impl AsRawSocket for TcpListener {
    fn as_raw_socket(&self) -> RawSocket {
        self.inner.as_raw_socket()
    }
}

#[cfg(windows)]
impl FromRawSocket for TcpListener {
    /// Converts a `RawSocket` to a `TcpListener`.
    ///
    /// # Notes
    ///
    /// The caller is responsible for ensuring that the socket is in
    /// non-blocking mode.
    unsafe fn from_raw_socket(socket: RawSocket) -> TcpListener {
        TcpListener::from_std(FromRawSocket::from_raw_socket(socket))
    }
}

#[cfg(target_os = "wasi")]
impl IntoRawFd for TcpListener {
    fn into_raw_fd(self) -> RawFd {
        self.inner.into_inner().into_raw_fd()
    }
}

#[cfg(target_os = "wasi")]
impl AsRawFd for TcpListener {
    fn as_raw_fd(&self) -> RawFd {
        self.inner.as_raw_fd()
    }
}

#[cfg(target_os = "wasi")]
impl FromRawFd for TcpListener {
    /// Converts a `RawFd` to a `TcpListener`.
    ///
    /// # Notes
    ///
    /// The caller is responsible for ensuring that the socket is in
    /// non-blocking mode.
    unsafe fn from_raw_fd(fd: RawFd) -> TcpListener {
        TcpListener::from_std(FromRawFd::from_raw_fd(fd))
    }
}
