use crate::{ReplicationState, domains::query_parsers::QueryIO, prelude::PeerIdentifier};

pub(crate) use peer_messages::*;

#[derive(Debug)]
pub(crate) enum PeerListenerCommand {
    AppendEntriesRPC(HeartBeat),
    ClusterHeartBeat(HeartBeat),
    Acks(ReplicationResponse),
    RequestVote(RequestVote),
    RequestVoteReply(RequestVoteReply),
}

impl TryFrom<QueryIO> for PeerListenerCommand {
    type Error = anyhow::Error;
    fn try_from(query: QueryIO) -> anyhow::Result<Self> {
        match query {
            QueryIO::AppendEntriesRPC(peer_state) => Ok(Self::AppendEntriesRPC(peer_state)),
            QueryIO::ClusterHeartBeat(heartbeat) => Ok(Self::ClusterHeartBeat(heartbeat)),
            QueryIO::ConsensusFollowerResponse(acks) => Ok(PeerListenerCommand::Acks(acks)),
            QueryIO::RequestVote(vote) => Ok(PeerListenerCommand::RequestVote(vote)),
            QueryIO::RequestVoteReply(reply) => Ok(PeerListenerCommand::RequestVoteReply(reply)),
            _ => Err(anyhow::anyhow!("Invalid data")),
        }
    }
}

mod peer_messages {
    use crate::domains::{
        cluster_actors::replication::ReplicationId, operation_logs::WriteOperation,
        peers::peer::PeerState,
    };

    use super::*;
    #[derive(Clone, Debug, PartialEq, bincode::Encode, bincode::Decode)]
    pub struct RequestVote {
        pub(crate) term: u64, // current term of the candidate. Without it, the old leader wouldn't be able to step down gracefully.
        pub(crate) candidate_id: PeerIdentifier,
        pub(crate) last_log_index: u64,
        pub(crate) last_log_term: u64, //the term of the last log entry, used for election restrictions. If the term is low, it won’t win the election.
    }
    impl RequestVote {
        pub(crate) fn new(
            repl: &ReplicationState,
            last_log_index: u64,
            last_log_term: u64,
        ) -> Self {
            Self {
                term: repl.term,
                candidate_id: repl.self_identifier(),
                last_log_index,
                last_log_term,
            }
        }
    }

    #[derive(Clone, Debug, PartialEq, bincode::Encode, bincode::Decode)]
    pub struct RequestVoteReply {
        pub(crate) term: u64,
        pub(crate) vote_granted: bool,
    }

    #[derive(Debug, Clone, PartialEq, bincode::Decode, bincode::Encode)]
    pub struct ReplicationResponse {
        pub(crate) log_idx: u64,
        pub(crate) term: u64,
        pub(crate) rej_reason: RejectionReason,
        pub(crate) from: PeerIdentifier,
    }

    #[derive(Debug, Clone, PartialEq, bincode::Decode, bincode::Encode)]
    pub(crate) enum RejectionReason {
        ReceiverHasHigherTerm,
        LogInconsistency,
        None,
    }

    impl ReplicationResponse {
        pub(crate) fn new(
            log_idx: u64,
            rej_reason: RejectionReason,
            repl_state: &ReplicationState,
        ) -> Self {
            Self { log_idx, term: repl_state.term, rej_reason, from: repl_state.self_identifier() }
        }

        pub(crate) fn is_granted(&self) -> bool {
            self.rej_reason == RejectionReason::None
        }

        #[cfg(test)]
        pub(crate) fn set_from(self, from: &str) -> Self {
            Self { from: PeerIdentifier(from.to_string()), ..self }
        }
    }

    #[derive(Debug, Clone, PartialEq, bincode::Encode, bincode::Decode)]

    pub struct HeartBeat {
        pub(crate) from: PeerIdentifier,
        pub(crate) term: u64,
        pub(crate) hwm: u64,
        pub(crate) replid: ReplicationId,
        pub(crate) hop_count: u8,
        pub(crate) ban_list: Vec<BannedPeer>,
        pub(crate) append_entries: Vec<WriteOperation>,
        pub(crate) cluster_nodes: Vec<PeerState>,
        pub(crate) prev_log_index: u64, //index of log entry immediately preceding new ones
        pub(crate) prev_log_term: u64,  //term of prev_log_index entry
    }
    impl HeartBeat {
        pub(crate) fn set_append_entries(mut self, entries: Vec<WriteOperation>) -> Self {
            self.append_entries = entries;
            self
        }

        pub(crate) fn set_cluster_nodes(mut self, cluster_nodes: Vec<PeerState>) -> Self {
            self.cluster_nodes = cluster_nodes;
            self
        }
    }

    #[derive(Debug, Clone, Eq, PartialOrd, Ord, bincode::Encode, bincode::Decode, Hash)]
    pub struct BannedPeer {
        pub(crate) p_id: PeerIdentifier,
        pub(crate) ban_time: u64,
    }
    impl PartialEq for BannedPeer {
        fn eq(&self, other: &Self) -> bool {
            self.p_id == other.p_id
        }
    }

    impl std::borrow::Borrow<PeerIdentifier> for BannedPeer {
        fn borrow(&self) -> &PeerIdentifier {
            &self.p_id
        }
    }
}
