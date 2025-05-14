use super::response::ConnectionResponse;
use crate::domains::cluster_actors::replication::ReplicationId;
use crate::domains::cluster_actors::replication::ReplicationState;
use crate::domains::peers::connected_peer_info::ConnectedPeerInfo;
use crate::domains::peers::identifier::PeerIdentifier;
use crate::domains::peers::identifier::TPeerAddress;

use crate::domains::query_parsers::QueryIO;

use crate::services::interface::TRead;
use crate::services::interface::TWrite;
use crate::write_array;
use anyhow::Context;
use std::sync::atomic::Ordering;
use tokio::net::TcpStream;
use tokio::net::tcp::OwnedReadHalf;
use tokio::net::tcp::OwnedWriteHalf;
use tracing::trace;

// The following is used only when the node is in follower mode
pub(crate) struct OutboundStream {
    pub(crate) r: OwnedReadHalf,
    pub(crate) w: OwnedWriteHalf,
    pub(crate) my_repl_info: ReplicationState,
    pub(crate) connected_node_info: Option<ConnectedPeerInfo>,
    pub(crate) connect_to: PeerIdentifier,
}

impl OutboundStream {
    pub(crate) async fn new(
        connect_to: PeerIdentifier,
        my_repl_info: ReplicationState,
    ) -> anyhow::Result<Self> {
        let stream = TcpStream::connect(&connect_to.cluster_bind_addr()).await?;
        let (read, write) = stream.into_split();
        Ok(OutboundStream {
            r: read,
            w: write,
            my_repl_info,
            connected_node_info: None,
            connect_to: connect_to.to_string().into(),
        })
    }
    pub async fn make_handshake(mut self, self_port: u16) -> anyhow::Result<Self> {
        // Trigger
        self.w.write(write_array!("PING")).await?;
        let mut ok_count = 0;
        let mut connection_info = ConnectedPeerInfo {
            id: Default::default(),
            replid: Default::default(),
            hwm: Default::default(),
            peer_list: Default::default(),
        };

        loop {
            trace!("Waiting for handshake response");
            let res = self.r.read_values().await?;
            trace!(?res, "Received handshake response");
            for query in res {
                match ConnectionResponse::try_from(query)? {
                    ConnectionResponse::Pong => {
                        let msg = write_array!("REPLCONF", "listening-port", self_port.to_string());
                        self.w.write(msg).await?
                    },
                    ConnectionResponse::Ok => {
                        ok_count += 1;
                        let msg = {
                            match ok_count {
                                1 => Ok(write_array!("REPLCONF", "capa", "psync2")),
                                // "?" here means the server is undecided about their leader. and -1 is the offset that follower is aware of
                                2 => Ok(write_array!(
                                    "PSYNC",
                                    self.my_repl_info.replid.clone(),
                                    self.my_repl_info.hwm.load(Ordering::Acquire).to_string()
                                )),
                                _ => Err(anyhow::anyhow!("Unexpected OK count")),
                            }
                        }?;
                        self.w.write(msg).await?
                    },
                    ConnectionResponse::FullResync { id, repl_id, offset } => {
                        connection_info.replid = ReplicationId::Key(repl_id);
                        connection_info.hwm = offset;
                        connection_info.id = id.into();
                        self.reply_with_ok().await?;
                    },
                    ConnectionResponse::Peers(peer_list) => {
                        connection_info.peer_list = peer_list;
                        self.connected_node_info = Some(connection_info);
                        self.reply_with_ok().await?;
                        return Ok(self);
                    },
                }
            }
        }
    }

    async fn reply_with_ok(&mut self) -> anyhow::Result<()> {
        self.w.write_io(QueryIO::SimpleString("OK".to_string())).await?;
        Ok(())
    }

    pub(crate) fn take_connection_info(&mut self) -> anyhow::Result<ConnectedPeerInfo> {
        Ok(self.connected_node_info.take().context("Connected node info not found")?)
    }
}
