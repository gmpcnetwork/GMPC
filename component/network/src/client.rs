use std::error::Error as StdError;
use std::net::SocketAddr;
use std::result::Result as StdResult;

use bytes::{BufMut, BytesMut};
use futures::sync::mpsc;
use futures::{Async, Future, Poll, Stream};
use serde_json::Value as JsonValue;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{ConnectFuture, TcpStream};

use super::dispatch::{Dispatch, NewDispatch};
use super::error::Error;

type Tx = mpsc::UnboundedSender<JsonValue>;
type Rx = mpsc::UnboundedReceiver<JsonValue>;


#[derive(Debug)]
pub struct Client<D> {
    serve: Serve<D>,
}

#[derive(Debug)]
pub struct Builder {
    id: String,
    pwd: String,
    connect: Option<ConnectFuture>,
}

impl Client<()> {
    pub fn bind(addr: &SocketAddr, id: String, pwd: String) -> Builder {
        let connect = TcpStream::connect(addr);
        Client::builder(id, pwd, connect)
    }

    fn builder(id: String, pwd: String, connect: ConnectFuture) -> Builder {
        Builder {
            id,
            pwd,
            connect: Some(connect),
        }
    }
}

impl<D> Client<D> {
    pub fn execute<F>(&self, fut: F) -> StdResult<(), Error>
        where
            F: Future<Item = (), Error = ()> + Send + 'static, {
        use tokio_executor::Executor;
        ::tokio_executor::DefaultExecutor::current().spawn(Box::new(fut)).map_err(|_e| Error::new_execute())
    }
}

impl<D> Future for Client<D>
    where
        D: NewDispatch + Send + 'static,
        D::Error: Into<Box<StdError + Send + Sync>>,
        D::Future: Send,
        <D::Dispatch as Dispatch>::Future: Send + 'static,
{
    type Item = ();
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            if let Some(fut) = try_ready!(self.serve.poll()) {
                self.execute(fut)?
            } else {
                return Ok(Async::Ready(()))
            }
        }
    }
}

#[derive(Debug)]
enum State {
    Connecting,
    Authenticating,
    Working,
}

#[derive(Debug)]
pub struct Serve<D> {
    id: String,
    pwd: String,
    connect: ConnectFuture,
    socket: Option<TcpStream>,
    rd: BytesMut,
    wr: BytesMut,
    rx: Rx,
    tx: Tx,
    new_dispatch: D,
    state: State,
}

impl Builder {
    pub fn serve<D>(mut self, new_dispatch: D) -> Client<D> {
        let connect = self.connect.take().unwrap();
        let (tx, rx) = mpsc::unbounded();
        let serve = Serve {
            id: self.id,
            pwd: self.pwd,
            connect,
            socket: None,
            rd: BytesMut::new(),
            wr: BytesMut::new(),
            rx,
            tx,
            new_dispatch,
            state: State::Connecting,
        };

        Client {
            serve,
        }
    }
}

impl<D> Serve<D> {
    fn connected(&mut self) -> StdResult<bool, Error> {
        if let Async::Ready(s) = self.connect.poll().map_err(|e| Error::new_connect(e))? {
            self.socket = Some(s);
            info!("Successfully connected");
            return Ok(true)
        }

        Ok(false)
    }

    fn authenticate(&mut self) -> Poll<(), Error> {
        let auth_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "mining.authorize",
            "params": [self.id, self.pwd]
        });
        debug!("Send an authentication message");
        self.write(auth_request)
    }

    fn authenticated(&mut self) -> StdResult<bool, Error> {
        if let Async::Ready(res) = self.read()? {
            if res["id"] == 1 && res["result"] == true {
                info!("Successfully authenticated");
                return Ok(true)
            } else {
                return Err(Error::new_authenticate())
            }
        }

        Ok(false)
    }

    fn working(&mut self) -> Poll<Dispatcher<D::Future>, Error>
        where
            D: NewDispatch, {
        if let Async::Ready(Some(json_value)) = self.rx.poll().unwrap() {
            debug!("Send a message: {:?}", json_value);
            self.write(json_value)?;
        }

        if let Async::Ready(req) = self.read()? {
            return Ok(Async::Ready(Dispatcher {
                future: self.new_dispatch.new_dispatch(),
                req,
                tx: self.tx.clone(),
            }))
        }

        Ok(Async::NotReady)
    }

    fn read(&mut self) -> Poll<JsonValue, Error> {
        self.rd.reserve(1024);
        let socket = self.socket.as_mut().unwrap();
        let n = try_ready!(socket.read_buf(&mut self.rd).map_err(|e| Error::new_io(e)));

        if n == 0 {
            return Err(Error::new_closed())
        }

        let pos = self.rd.windows(1).enumerate().find(|&(_, bytes)| bytes == b"\n").map(|(i, _)| i);
        if let Some(pos) = pos {
            let mut line = self.rd.split_to(pos + 1);
            line.split_off(pos);

            let req = String::from_utf8(line.to_vec()).expect("Response should be utf-8");
            let ret = ::serde_json::from_str(&req).map_err(|_e| Error::new_incomplete())?;
            return Ok(Async::Ready(ret))
        }

        Ok(Async::NotReady)
    }

    fn write(&mut self, value: JsonValue) -> Poll<(), Error> {
        let mut req = ::serde_json::ser::to_vec(&value).map_err(|_e| Error::new_incomplete())?;
        req.extend(b"\n");

        self.wr.reserve(1024);
        self.wr.put(req);

        let socket = self.socket.as_mut().unwrap();
        while !self.wr.is_empty() {
            let n = try_ready!(socket.poll_write(&self.wr).map_err(|e| Error::new_io(e)));
            assert!(n > 0);
            let _ = self.wr.split_to(n);
        }

        Ok(Async::Ready(()))
    }
}

impl<D> Stream for Serve<D>
    where
        D: NewDispatch,
{
    type Item = Dispatcher<D::Future>;
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        loop {
            let next = match self.state {
                State::Connecting => {
                    if !self.connected()? {
                        return Ok(Async::NotReady)
                    }

                    self.authenticate()?;
                    State::Authenticating
                }
                State::Authenticating => {
                    if !self.authenticated()? {
                        return Ok(Async::NotReady)
                    }

                    State::Working
                }
                State::Working => {
                    if let Async::Ready(fut) = self.working()? {
                        return Ok(Async::Ready(Some(fut)))
                    }

                    return Ok(Async::NotReady)
                }
            };
            self.state = next;
        }
    }
}

pub struct Dispatcher<F> {
    future: F,
    req: JsonValue,
    tx: Tx,
}

impl<F, D> Future for Dispatcher<F>
    where
        F: Future<Item = D>,
        D: Dispatch,
        D::Future: Send + 'static,
        D::Error: Into<Box<StdError + Send + Sync>>,
        D::Future: Future<Item = Option<JsonValue>, Error = D::Error> + Send + 'static,
{
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Ok(Async::Ready(ref mut dispatcher)) = self.future.poll() {
            let mut fut = dispatcher.call(self.req.clone());
            if let Ok(Async::Ready(Some(res))) = fut.poll() {
                let _ = self.tx.unbounded_send(res);
            }
        }
        Ok(Async::Ready(()))
    }
}