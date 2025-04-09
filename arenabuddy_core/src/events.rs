use serde::{Serialize, Serializer};

use crate::mtga_events::{
    business::BusinessEvent, client::RequestTypeClientToMatchServiceMessage,
    gre::RequestTypeGREToClientEvent, mgrsc::RequestTypeMGRSCEvent,
};

#[derive(Debug, Clone)]
pub enum Event {
    GRE(RequestTypeGREToClientEvent),
    Client(RequestTypeClientToMatchServiceMessage),
    MGRSC(RequestTypeMGRSCEvent),
    Business(BusinessEvent),
}

impl Event {
    pub fn as_ref(&self) -> EventRef<'_> {
        match self {
            Event::GRE(r) => EventRef::GRE(r),
            Event::Client(r) => EventRef::Client(r),
            Event::MGRSC(r) => EventRef::MGRSC(r),
            Event::Business(b) => EventRef::Business(b),
        }
    }
}

pub enum EventRef<'a> {
    GRE(&'a RequestTypeGREToClientEvent),
    Client(&'a RequestTypeClientToMatchServiceMessage),
    MGRSC(&'a RequestTypeMGRSCEvent),
    Business(&'a BusinessEvent),
}

impl Serialize for EventRef<'_> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::MGRSC(event) => event.serialize(serializer),
            Self::GRE(event) => event.serialize(serializer),
            Self::Client(event) => event.serialize(serializer),
            Self::Business(event) => event.serialize(serializer),
        }
    }
}
