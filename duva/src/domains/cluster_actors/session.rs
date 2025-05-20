use chrono::{DateTime, Utc};
use std::collections::HashMap;
use uuid::Uuid;

use crate::make_smart_pointer;

#[derive(Default, Debug)]
pub(crate) struct ClientSessions(HashMap<Uuid, Session>);

#[derive(Default, Debug)]
pub(crate) struct Session {
    last_accessed: DateTime<Utc>,
    processed_req_id: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SessionRequest {
    pub(crate) request_id: u64,
    pub(crate) client_id: Uuid,
}
impl SessionRequest {
    pub(crate) fn new(request_id: u64, client_id: Uuid) -> Self {
        Self { request_id, client_id }
    }
}

make_smart_pointer!(ClientSessions,HashMap<Uuid, Session>);

impl ClientSessions {
    pub(crate) fn is_processed(&self, client_req: &Option<SessionRequest>) -> bool {
        let Some(client_req) = client_req else {
            return false;
        };
        if self.get(&client_req.client_id).is_none() {
            return false;
        }
        let session = self.get(&client_req.client_id).unwrap();
        let Some(res) = session.processed_req_id.as_ref() else {
            return false;
        };

        *res == client_req.request_id
    }
    pub(crate) fn set_response(&mut self, session_req: Option<SessionRequest>) {
        let Some(session_req) = session_req else { return };

        let entry = self
            .entry(session_req.client_id)
            .or_insert(Session { last_accessed: Default::default(), processed_req_id: None });
        entry.last_accessed = Utc::now();
        entry.processed_req_id = Some(session_req.request_id);
    }
}
