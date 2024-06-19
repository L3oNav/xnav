//! Message passing abstractions for sending notifications to Tokio tasks and
//! awaiting their acknowledgement, which is useful for graceful shutdowns.

use tokio::sync::{broadcast, mpsc};

/// Message that can be sent as a notification to Tokio tasks.
#[derive(Clone, Copy, Debug)]
pub enum Notification {
    Shutdown,
}

/// Notifier object that can send messages to its subscribers.
pub struct Notifier {
    /// Sender half of the notifications channel.
    notification_sender: broadcast::Sender<Notification>,
    /// Receiver part of the acknowledgements channel.
    acknowledge_receiver: mpsc::Receiver<()>,
    /// Sender part of the acknowledgements channel.
    acknowledge_sender: mpsc::Sender<()>,
}

/// Used by subscribers to obtain a notification from a [`Notifier`] and
/// acknowledge receipt when possible.
pub struct Subscription {
    /// Receiver half of the notifications channel.
    notification_receiver: broadcast::Receiver<Notification>,
    /// Sender half of the acknowledgements channel.
    acknowledge_sender: mpsc::Sender<()>,
}

impl Notifier {
    /// Creates a new [`Notifier`] with all the channels set up.
    pub fn new() -> Self {
        let (notification_sender, _) = broadcast::channel(1);
        let (acknowledge_sender, acknowledge_receiver) = mpsc::channel(1);

        Self {
            notification_sender,
            acknowledge_sender,
            acknowledge_receiver,
        }
    }

    /// By subscribing to this [`Notifier`] the caller obtains a
    /// [`Subscription`] object that can be used to receive a [`Notification`].
    pub fn subscribe(&self) -> Subscription {
        let notification_receiver = self.notification_sender.subscribe();
        let acknowledge_sender = self.acknowledge_sender.clone();

        Subscription::new(notification_receiver, acknowledge_sender)
    }

    /// Sends a [`Notification`] to all subscribers.
    pub fn send(
        &self,
        notification: Notification,
    ) -> Result<usize, broadcast::error::SendError<Notification>> {
        self.notification_sender.send(notification)
    }

    /// Waits for all the subscribers to acknowledge the last sent
    /// [`Notification`].
    pub async fn collect_acknowledgements(self) {
        let Self {
            notification_sender,
            mut acknowledge_receiver,
            acknowledge_sender,
        } = self;

        // Drop the acknowledge_sender sender to allow the channel to be closed
        drop(acknowledge_sender);

        // Wait for all acks one by one.
        while let Some(_) = acknowledge_receiver.recv().await {}

        drop(notification_sender);
    }
}

impl Subscription {
    /// Creates a new [`Subscription`] object.
    pub fn new(
        notification_receiver: broadcast::Receiver<Notification>,
        acknowledge_sender: mpsc::Sender<()>,
    ) -> Self {
        Self {
            notification_receiver,
            acknowledge_sender,
        }
    }

    /// Reads the notifications channel to check if a notification was sent.
    pub fn receive_notification(&mut self) -> Option<Notification> {
        self.notification_receiver.try_recv().ok()
    }

    /// Sends an acknowledgment on the acknowledgements channel.
    pub async fn acknowledge_notification(&self) {
        self.acknowledge_sender.send(()).await.unwrap();
    }
}
