use std::collections::{BTreeMap, BTreeSet};
use std::vec::IntoIter;

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Serialize, Serializer};
use tracing::{debug, info, warn};

use crate::cards::CardsDatabase;
use crate::models::deck::Deck;
use crate::models::mulligan::MulliganInfo;
use crate::models::mulligan::MulliganInfoBuilder;
use crate::mtga_events::business::BusinessEventRequest;
use crate::mtga_events::client::{
    ClientMessage, MulliganOption, MulliganRespWrapper, RequestTypeClientToMatchServiceMessage,
};
use crate::mtga_events::gre::{
    DeckMessage, GREToClientMessage, GameObjectType, GameStateMessage, MulliganReqWrapper,
    RequestTypeGREToClientEvent,
};
use crate::mtga_events::mgrsc::{FinalMatchResult, RequestTypeMGRSCEvent, StateType};
use crate::mtga_events::primitives::ZoneType;
use crate::processor::ParseOutput;

const DEFAULT_HAND_SIZE: i32 = 7;

#[derive(Debug, Default)]
pub struct MatchReplay {
    pub match_id: String,
    pub match_start_message: RequestTypeMGRSCEvent,
    pub match_end_message: RequestTypeMGRSCEvent,
    pub client_server_messages: Vec<MatchReplayEvent>,
    pub business_messages: Vec<BusinessEventRequest>,
}

#[derive(Debug, Clone)]
pub enum MatchReplayEvent {
    GRE(RequestTypeGREToClientEvent),
    Client(RequestTypeClientToMatchServiceMessage),
    MGRSC(RequestTypeMGRSCEvent),
}

pub enum MatchReplayEventRef<'a> {
    GRE(&'a RequestTypeGREToClientEvent),
    Client(&'a RequestTypeClientToMatchServiceMessage),
    MGRSC(&'a RequestTypeMGRSCEvent),
    Business(&'a BusinessEventRequest),
}

impl Serialize for MatchReplayEventRef<'_> {
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

impl MatchReplay {
    fn gre_events_iter(&self) -> impl Iterator<Item = &RequestTypeGREToClientEvent> {
        self.client_server_messages
            .iter()
            .filter_map(|mre| match mre {
                MatchReplayEvent::GRE(message) => Some(message),
                _ => None,
            })
    }

    fn gre_messages_iter(&self) -> impl Iterator<Item = &GREToClientMessage> {
        self.gre_events_iter()
            .flat_map(|gre| &gre.gre_to_client_event.gre_to_client_messages)
    }

    fn game_state_messages_iter(&self) -> impl Iterator<Item = &GameStateMessage> {
        self.gre_messages_iter()
            .filter_map(|gre_message| match gre_message {
                GREToClientMessage::GameStateMessage(wrapper) => Some(&wrapper.game_state_message),
                _ => None,
            })
    }

    fn client_messages_iter(
        &self,
    ) -> impl Iterator<Item = &RequestTypeClientToMatchServiceMessage> {
        self.client_server_messages
            .iter()
            .filter_map(|mre| match mre {
                MatchReplayEvent::Client(message) => Some(message),
                _ => None,
            })
    }

    /// # Errors
    ///
    /// Returns an error if the controller seat ID is not found
    pub(crate) fn get_controller_seat_id(&self) -> Result<i32> {
        for gre_message in self.gre_messages_iter() {
            if let GREToClientMessage::ConnectResp(wrapper) = gre_message {
                return Ok(wrapper.meta.system_seat_ids[0]);
            }
        }
        Err(anyhow!("Controller seat ID not found"))
    }

    /// # Errors
    ///
    /// Returns an error if the player names are not found
    pub(crate) fn get_player_names(&self, seat_id: i32) -> Result<(String, String)> {
        if let Some(players) = &self.match_start_message.mgrsc_event.game_room_info.players {
            let controller = players
                .iter()
                .find(|player| player.system_seat_id == seat_id);
            let opponent = players
                .iter()
                .find(|player| player.system_seat_id != seat_id);
            if let Some(controller) = controller {
                if let Some(opponent) = opponent {
                    return Ok((controller.player_name.clone(), opponent.player_name.clone()));
                }
            }
        }
        Err(anyhow!("player names not found"))
    }

    /// # Errors
    ///
    /// Returns an error if the controller seat ID is not found
    fn get_opponent_cards(&self) -> Result<Vec<i32>> {
        let controller_id = self.get_controller_seat_id()?;
        let opponent_cards = self
            .game_state_messages_iter()
            .flat_map(|gsm| &gsm.game_objects)
            .filter(|game_object| {
                game_object.owner_seat_id != controller_id
                    && (game_object.type_field == GameObjectType::Card
                        || game_object.type_field == GameObjectType::MDFCBack)
            })
            .map(|game_object| game_object.grp_id)
            .collect();
        Ok(opponent_cards)
    }

    /// # Errors
    ///
    /// Returns an error if the controller seat id is not found
    fn get_opponent_color_identity(&self, cards_db: &CardsDatabase) -> Result<String> {
        let opponent_cards = self.get_opponent_cards()?;
        let mut color_identity = BTreeSet::new();
        for card in opponent_cards {
            if let Some(card_db_entry) = cards_db.get(&card) {
                let colors = if card_db_entry.name == "jegantha_the_wellspring" {
                    vec!["R".to_string(), "G".to_string()]
                } else {
                    card_db_entry.color_identity.clone()
                };
                debug!("card: {}, colors: {:?}", card_db_entry.name, colors);
                color_identity.extend(colors);
            }
        }
        Ok(color_identity.into_iter().collect::<String>())
    }

    /// # Errors
    ///
    /// Returns an error if the match results are not found
    pub fn get_match_results(&self) -> Result<FinalMatchResult> {
        self.match_end_message
            .mgrsc_event
            .game_room_info
            .final_match_result
            .clone()
            .ok_or(anyhow!("Match results not found"))
    }

    /// # Errors
    ///
    /// Returns an error if there is no `ConnectResp` in the GRE events
    fn get_initial_decklist(&self) -> Result<DeckMessage> {
        for gre_message in self.gre_messages_iter() {
            if let GREToClientMessage::ConnectResp(wrapper) = gre_message {
                return Ok(wrapper.connect_resp.deck_message.clone());
            }
        }
        Err(anyhow!("Initial decklist not found"))
    }

    fn get_sideboarded_decklists(&self) -> Vec<DeckMessage> {
        self.client_messages_iter()
            .filter_map(|message| {
                if let ClientMessage::SubmitDeckResp(submit_deck_resp) = &message.payload {
                    Some(submit_deck_resp.submit_deck_resp.deck.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// # Errors
    ///
    /// Returns an Error if the initial decklist is not found
    pub fn get_decklists(&self) -> Result<Vec<Deck>> {
        let mut decklists = vec![self.get_initial_decklist()?];
        decklists.append(&mut self.get_sideboarded_decklists());
        Ok(decklists
            .iter()
            .map(|deck| -> Deck { deck.into() })
            .enumerate()
            .map(|(i, mut deck)| {
                deck.game_number = i32::try_from(i).unwrap_or_else(|e| {
                    warn!("Error converting usize to i32: {}", e);
                    0
                }) + 1;
                deck
            })
            .collect())
    }

    /// # Errors
    ///
    /// Returns an error if the controller seat ID is not found among other things
    #[allow(clippy::too_many_lines)]
    pub fn get_mulligan_infos(&self, cards_db: &CardsDatabase) -> Result<Vec<MulliganInfo>> {
        let controller_id = self.get_controller_seat_id()?;

        let mut game_number = 1;
        let mut opening_hands = BTreeMap::<i32, Vec<Vec<i32>>>::new();
        let mut mulligan_requests = BTreeMap::<i32, Vec<&MulliganReqWrapper>>::new();
        let mut play_or_draw: BTreeMap<i32, String> = BTreeMap::new();
        let opponent_color_identity = self.get_opponent_color_identity(cards_db)?;

        for gre in self.gre_messages_iter() {
            match gre {
                GREToClientMessage::GameStateMessage(wrapper) => {
                    let gsm = &wrapper.game_state_message;

                    if gsm.players.len() == 2
                        && gsm.players.iter().all(|player| {
                            player.pending_message_type
                                == Some("ClientMessageType_MulliganResp".to_string())
                        })
                    {
                        if let Some(turn_info) = &gsm.turn_info {
                            if let Some(decision_player) = turn_info.decision_player {
                                let pd = if decision_player == controller_id {
                                    "Play"
                                } else {
                                    "Draw"
                                };
                                info!("game_number: {}, play_or_draw: {}", game_number, pd);
                                play_or_draw.insert(game_number, pd.to_string());
                            }
                        }
                    }

                    if gsm.players.iter().any(|player| {
                        player.controller_seat_id == controller_id
                            && player.pending_message_type
                                == Some("ClientMessageType_MulliganResp".to_string())
                    }) {
                        let controller_hand_zone_id = gsm
                            .zones
                            .iter()
                            .find(|zone| {
                                zone.type_field == ZoneType::Hand
                                    && zone.owner_seat_id == Some(controller_id)
                            })
                            .ok_or(anyhow!("Controller hand zone not found"))?
                            .zone_id;
                        let game_objects_in_hand: Vec<i32> = gsm
                            .game_objects
                            .iter()
                            .filter(|go| {
                                let Some(zone_id) = go.zone_id else {
                                    return false;
                                };
                                zone_id == controller_hand_zone_id
                                    && go.type_field == GameObjectType::Card
                            })
                            .map(|go| go.grp_id)
                            .collect();
                        opening_hands
                            .entry(game_number)
                            .or_default()
                            .push(game_objects_in_hand);
                    }
                }
                GREToClientMessage::MulliganReq(wrapper) => {
                    mulligan_requests
                        .entry(game_number)
                        .or_default()
                        .push(wrapper);
                }
                GREToClientMessage::IntermissionReq(_) => {
                    game_number += 1;
                }
                _ => {}
            }
        }

        let mulligan_responses: BTreeMap<i32, &MulliganRespWrapper> = self
            .client_messages_iter()
            .filter_map(|client_message| match &client_message.payload {
                ClientMessage::MulliganResp(wrapper) => {
                    Some((wrapper.meta.game_state_id?, wrapper))
                }
                _ => None,
            })
            .collect();

        let mut mulligan_infos = Vec::new();
        for (game_number, hands) in opening_hands {
            let mut mulligan_requests_iter = mulligan_requests
                .get(&game_number)
                .ok_or(anyhow!(
                    "No mulligan requests found for game {}",
                    game_number
                ))?
                .iter();
            let play_draw = play_or_draw.get(&game_number).ok_or(anyhow!(
                "No play/draw decision found for game {}",
                game_number
            ))?;
            for hand in hands {
                let hand_string = hand
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect::<Vec<String>>()
                    .join(",");
                let Some(mulligan_request) = mulligan_requests_iter.next() else {
                    warn!("No mulligan request found for game {}", game_number);
                    continue;
                };
                let Some(game_state_id) = mulligan_request.meta.game_state_id else {
                    warn!("No game state ID found for mulligan request");
                    continue;
                };
                let number_to_keep =
                    DEFAULT_HAND_SIZE - mulligan_request.mulligan_req.mulligan_count;
                let decision = match mulligan_responses.get(&game_state_id) {
                    Some(mulligan_response) => match mulligan_response.mulligan_resp.decision {
                        MulliganOption::AcceptHand => "Keep",
                        MulliganOption::Mulligan => "Mulligan",
                    },
                    None => "Match Ended",
                }
                .to_string();
                let opp_identity = if game_number == 1 {
                    "Unknown"
                } else {
                    &opponent_color_identity
                }
                .to_string();

                let mulligan = MulliganInfoBuilder::default()
                    .match_id(self.match_id.clone())
                    .game_number(game_number)
                    .number_to_keep(number_to_keep)
                    .hand(hand_string)
                    .play_draw(play_draw.clone())
                    .opponent_identity(opp_identity)
                    .decision(decision)
                    .build()?;

                mulligan_infos.push(mulligan);
            }
        }

        Ok(mulligan_infos)
    }

    pub fn match_start_time(&self) -> Option<DateTime<Utc>> {
        self.business_messages.iter().find_map(|bm| bm.event_time)
    }

    /// Gets the format for this match if found (e.g. "`Traditional_Explorer_Ranked`")
    /// MTGA usually underscore-spaces format names
    pub fn match_format(&self) -> Option<String> {
        self.business_messages
            .iter()
            .find(|message| message.event_id.is_some())
            .and_then(|message| message.event_id.clone())
    }

    pub fn iter(&self) -> impl Iterator<Item = MatchReplayEventRef> {
        self.into_iter()
    }
}

impl<'a> IntoIterator for &'a MatchReplay {
    type Item = MatchReplayEventRef<'a>;
    type IntoIter = IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        let mut events = Vec::new();
        events.push(MatchReplayEventRef::MGRSC(&self.match_start_message));
        self.client_server_messages.iter().for_each(|mre| {
            let mre_ref = match mre {
                MatchReplayEvent::GRE(event) => MatchReplayEventRef::GRE(event),
                MatchReplayEvent::Client(event) => MatchReplayEventRef::Client(event),
                MatchReplayEvent::MGRSC(event) => MatchReplayEventRef::MGRSC(event),
            };
            events.push(mre_ref);
        });
        events.push(MatchReplayEventRef::MGRSC(&self.match_end_message));
        self.business_messages.iter().for_each(|bm| {
            events.push(MatchReplayEventRef::Business(bm));
        });
        events.into_iter()
    }
}

#[derive(Debug, Default)]
pub struct MatchReplayBuilder {
    pub match_id: Option<String>,
    pub match_start_message: Option<RequestTypeMGRSCEvent>,
    pub match_end_message: Option<RequestTypeMGRSCEvent>,
    pub client_server_messages: Vec<MatchReplayEvent>,
    pub business_messages: Vec<BusinessEventRequest>,
}

#[derive(Debug)]
pub enum MatchReplayBuilderError {
    MissingMatchId,
    MissingMatchStartMessage,
    MissingMatchEndMessage,
}

impl std::fmt::Display for MatchReplayBuilderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::MissingMatchId => "Missing Match Id",
                Self::MissingMatchStartMessage => "Missing Match Start Message",
                Self::MissingMatchEndMessage => "Missing Match End Message",
            }
        )
    }
}

impl std::error::Error for MatchReplayBuilderError {}

impl MatchReplayBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ingest_event(&mut self, event: ParseOutput) -> bool {
        match event {
            ParseOutput::GREMessage(gre_message) => self
                .client_server_messages
                .push(MatchReplayEvent::GRE(gre_message)),
            ParseOutput::ClientMessage(client_message) => self
                .client_server_messages
                .push(MatchReplayEvent::Client(client_message)),
            ParseOutput::MGRSCMessage(mgrsc_event) => {
                return self.ingest_mgrc_event(mgrsc_event);
            }
            ParseOutput::BusinessMessage(business_message) => {
                if business_message.is_relevant() {
                    debug!("Business message: {:?}", business_message);
                    self.business_messages.push(business_message.request);
                }
            }
            ParseOutput::NoEvent => {}
        }
        false
    }

    pub fn ingest_mgrc_event(&mut self, mgrsc_event: RequestTypeMGRSCEvent) -> bool {
        let state_type = mgrsc_event.mgrsc_event.game_room_info.state_type.clone();
        let match_id = mgrsc_event
            .mgrsc_event
            .game_room_info
            .game_room_config
            .match_id
            .clone();
        match state_type {
            StateType::MatchCompleted => {
                // match is over
                self.match_end_message = Some(mgrsc_event);
                return true;
            }
            StateType::Playing => {
                info!("found match: {}", match_id);
                self.match_id = Some(match_id);
                self.match_start_message = Some(mgrsc_event);
            }
        }
        false
    }

    /// # Errors
    ///
    /// Returns an error if the builder is missing key information
    /// except it doesn't right now, so don't worry about it
    pub fn build(self) -> Result<MatchReplay> {
        let match_id = self
            .match_id
            .ok_or(MatchReplayBuilderError::MissingMatchId)?;
        let match_start_message = self
            .match_start_message
            .ok_or(MatchReplayBuilderError::MissingMatchStartMessage)?;
        let match_end_message = self
            .match_end_message
            .ok_or(MatchReplayBuilderError::MissingMatchEndMessage)?;

        let match_replay = MatchReplay {
            match_id,
            match_start_message,
            match_end_message,
            client_server_messages: self.client_server_messages,
            business_messages: self.business_messages,
        };
        Ok(match_replay)
    }
}
