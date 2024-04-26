use crate::domain::Unit;
use actix::prelude::*;
use integrationos_domain::error::IntegrationOSError as Error;

#[derive(Message, Debug, Clone)]
#[rtype(result = "Result<Unit, Error>")]
pub struct Refresh {
    refresh_before_in_minutes: i64,
}

impl Refresh {
    pub fn new(refresh_before_in_minutes: i64) -> Self {
        Self {
            refresh_before_in_minutes,
        }
    }

    pub fn refresh_before_in_minutes(&self) -> i64 {
        self.refresh_before_in_minutes
    }
}
