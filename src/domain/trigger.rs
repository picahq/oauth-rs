use crate::domain::Outcome;
use actix::prelude::*;
use integrationos_domain::Connection;

#[derive(Message, Debug, Clone, Eq, PartialEq)]
#[rtype(result = "Outcome")]
pub struct Trigger {
    connection: Connection,
}

impl Trigger {
    pub fn new(connection: Connection) -> Self {
        Self { connection }
    }

    pub fn connection(&self) -> &Connection {
        &self.connection
    }
}
