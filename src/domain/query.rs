use super::state::StatefulActor;
use actix::prelude::*;

#[derive(Message, Debug, Clone)]
#[rtype(result = "StatefulActor")]
pub struct Query;
