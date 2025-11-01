use std::pin::Pin;

use futures::Stream;
use futures::task::{Context, Poll};
use tokio::sync::mpsc;
use wasi_messaging::Message;

#[derive(Debug)]
pub struct Subscriber {
    sid: u64,
    receiver: mpsc::Receiver<Message>,
    sender: mpsc::Sender<Message>,
}

impl Stream for Subscriber {
    type Item = Message;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.receiver.poll_recv(cx)
    }
}

// /// `Command` represents all commands that a [`Client`] can handle
// #[derive(Debug)]
// pub(crate) enum Command {
//     Publish(OutboundMessage),
//     Request {
//         subject: Subject,
//         payload: Bytes,
//         respond: Subject,
//         headers: Option<HeaderMap>,
//         sender: oneshot::Sender<Message>,
//     },
//     Subscribe {
//         sid: u64,
//         subject: Subject,
//         queue_group: Option<String>,
//         sender: mpsc::Sender<Message>,
//     },
//     Unsubscribe {
//         sid: u64,
//         max: Option<u64>,
//     },
//     Flush {
//         observer: oneshot::Sender<()>,
//     },
//     Drain {
//         sid: Option<u64>,
//     },
//     Reconnect,
// }
