use integrationos_domain::Connection;

#[derive(Debug, Clone, Eq, PartialEq)]
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
