//!
//!

pub(crate) mod client;
pub(crate) mod server;

use crate::error::{StreamError, TaskError};
use crate::util::watch;

use tokio::sync::{oneshot, mpsc};
use tokio::task::JoinHandle;

/// Used in Client and Server
pub(crate) enum SendBack<P> {
	None,
	Packet(P),
	Close,
	CloseWithPacket
}

/// A Handle to a background task, if this handle is dropped
/// the connection will be dropped.
pub(crate) struct TaskHandle {
	pub close: oneshot::Sender<()>,
	pub task: JoinHandle<Result<(), TaskError>>
}

impl TaskHandle {
	/// Wait until the connection has nothing more todo which will then close
	/// the connection.
	pub async fn wait(self) -> Result<(), TaskError> {
		self.task.await
			.map_err(TaskError::Join)?
	}

	/// Send a close signal to the background task and wait until it closes.
	pub async fn close(self) -> Result<(), TaskError> {
		let _ = self.close.send(());
		self.task.await
			.map_err(TaskError::Join)?
	}

	// used for testing
	#[cfg(test)]
	pub fn abort(self) {
		self.task.abort();
	}
}

/// A sender of packets to an open stream.
#[derive(Debug, Clone)]
pub struct StreamSender<P> {
	pub(crate) inner: mpsc::Sender<P>
}

impl<P> StreamSender<P> {
	pub(crate) fn new(inner: mpsc::Sender<P>) -> Self {
		Self { inner }
	}

	/// Sends a packet to the client or the server.
	pub async fn send(&self, packet: P) -> Result<(), StreamError> {
		self.inner.send(packet).await
			.map_err(|_| StreamError::StreamAlreadyClosed)
	}
}

/// A stream of packets which is inside of a connection.
#[derive(Debug)]
pub struct StreamReceiver<P> {
	pub(crate) inner: mpsc::Receiver<P>
}

impl<P> StreamReceiver<P> {
	pub(crate) fn new(inner: mpsc::Receiver<P>) -> Self {
		Self { inner }
	}

	/// If none is returned this can mean that the connection
	/// was closed or the other side is finished sending.
	pub async fn receive(&mut self) -> Option<P> {
		self.inner.recv().await
	}

	/// Marks the stream as closed but allows to receive the remaining
	/// messages.
	pub fn close(&mut self) {
		self.inner.close();
	}
}

#[derive(Debug, Clone)]
pub struct Configurator<C> {
	inner: watch::Sender<C>
}

impl<C> Configurator<C> {
	pub(crate) fn new(inner: watch::Sender<C>) -> Self {
		Self { inner }
	}

	/// It is possible that there are no receivers left.
	/// 
	/// This is not checked
	pub fn update(&self, cfg: C) {
		self.inner.send(cfg);
	}

	pub fn read(&self) -> C
	where C: Clone {
		self.inner.newest()
	}
}