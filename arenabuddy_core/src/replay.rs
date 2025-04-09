use std::{
    collections::{BTreeMap, BTreeSet},
    vec::IntoIter,
};

use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use tracing::{debug, info, warn};

use crate::{
    cards::CardsDatabase,
    events::{Event, EventRef},
    models::{
        deck::Deck,
        mulligan::{MulliganInfo, MulliganInfoBuilder},
    },
    mtga_events::{
        business::BusinessEvent,
        client::{
            ClientMessage, MulliganOption, MulliganRespWrapper,
            RequestTypeClientToMatchServiceMessage,
        },
        gre::{
            DeckMessage, GREToClientMessage, GameObjectType, GameStateMessage, MulliganReqWrapper,
            RequestTypeGREToClientEvent,
        },
        mgrsc::{FinalMatchResult, RequestTypeMGRSCEvent, StateType},
        primitives::ZoneType,
    },
    processor::ParseOutput,
};

const DEFAULT_HAND_SIZE: i32 = 7;

#[derive(Debug, Default)]
pub struct MatchReplay {
    pub match_id: String,
    controller_seat_id: i32,
    pub match_start_message: RequestTypeMGRSCEvent,
    pub match_end_message: RequestTypeMGRSCEvent,
    pub client_server_messages: Vec<Event>,
    pub business_messages: Vec<BusinessEvent>,
}

impl MatchReplay {
    fn gre_events_iter(&self) -> impl Iterator<Item = &RequestTypeGREToClientEvent> {
        self.client_server_messages
            .iter()
            .filter_map(|mre| match mre {
                Event::GRE(message) => Some(message),
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
                Event::Client(message) => Some(message),
                _ => None,
            })
    }

    /// # Errors
    ///
    /// Returns an error if the controller seat ID is not found
    pub(crate) fn get_controller_seat_id(&self) -> i32 {
        self.controller_seat_id
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

    fn get_opponent_cards(&self) -> Vec<i32> {
        self.game_state_messages_iter()
            .flat_map(|gsm| &gsm.game_objects)
            .filter(|game_object| {
                game_object.owner_seat_id != self.controller_seat_id
                    && matches!(
                        game_object.type_field,
                        GameObjectType::Card | GameObjectType::MDFCBack
                    )
            })
            .map(|game_object| game_object.grp_id)
            .collect()
    }

    /// # Errors
    ///
    /// Returns an error if the controller seat id is not found
    fn get_opponent_color_identity(&self, cards_db: &CardsDatabase) -> String {
        let opponent_cards = self.get_opponent_cards();
        let mut color_identity = BTreeSet::new();
        for card in opponent_cards {
            if let Some(card_db_entry) = cards_db.get(&card) {
                debug!(
                    "card: {}, colors: {:?}",
                    card_db_entry.name, card_db_entry.color_identity
                );
                color_identity.extend(card_db_entry.color_identity.clone());
            }
        }
        color_identity.into_iter().collect::<String>()
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
        let controller_id = self.get_controller_seat_id();

        let mut game_number = 1;
        let mut opening_hands = Vec::<(i32, Vec<i32>)>::new();
        let mut mulligan_requests = Vec::<(i32, &MulliganReqWrapper)>::new();
        let mut play_or_draw: BTreeMap<i32, String> = BTreeMap::new();
        let opponent_color_identity = self.get_opponent_color_identity(cards_db);

        self.gre_messages_iter().try_for_each(|gre| -> Result<()> {
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

                        opening_hands.push((game_number, game_objects_in_hand));
                    }
                    Ok(())
                }
                GREToClientMessage::MulliganReq(wrapper) => {
                    mulligan_requests.push((game_number, wrapper));
                    Ok(())
                }
                GREToClientMessage::IntermissionReq(_) => {
                    game_number += 1;
                    Ok(())
                }
                _ => Ok(()),
            }
        })?;

        let mulligan_responses: BTreeMap<i32, &MulliganRespWrapper> = self
            .client_messages_iter()
            .filter_map(|client_message| match &client_message.payload {
                ClientMessage::MulliganResp(wrapper) => {
                    Some((wrapper.meta.game_state_id?, wrapper))
                }
                _ => None,
            })
            .collect();

        if opening_hands.len() != mulligan_requests.len() {
            warn!(
                "Missing mulligan data for {}. # of hands: {}, number mulligan responses: {}",
                self.match_id,
                opening_hands.len(),
                mulligan_requests.len(),
            );
            return Err(anyhow!("missing mulligan data"));
        }

        Ok(opening_hands
            .into_iter()
            .zip(mulligan_requests)
            .filter_map(|((gn, hand), (gn2, mulligan_request))| {
                if gn != gn2 {
                    warn!("invalid mulilgan data for {}", self.match_id);
                    return None;
                }

                let play_draw = play_or_draw
                    .get(&game_number)
                    .cloned()
                    .unwrap_or("Unknown".to_string());

                let hand_string = hand
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect::<Vec<String>>()
                    .join(",");

                let Some(game_state_id) = mulligan_request.meta.game_state_id else {
                    warn!("No game state ID found for mulligan request");
                    return None;
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

                MulliganInfoBuilder::default()
                    .match_id(self.match_id.clone())
                    .game_number(game_number)
                    .number_to_keep(number_to_keep)
                    .hand(hand_string)
                    .play_draw(play_draw.clone())
                    .opponent_identity(opp_identity)
                    .decision(decision)
                    .build()
                    .ok()
            })
            .collect())
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

    pub fn iter(&self) -> impl Iterator<Item = EventRef> {
        self.into_iter()
    }
}

impl<'a> IntoIterator for &'a MatchReplay {
    type Item = EventRef<'a>;
    type IntoIter = IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        let mut events = Vec::new();
        events.push(EventRef::MGRSC(&self.match_start_message));
        self.client_server_messages.iter().for_each(|mre| {
            events.push(mre.as_ref());
        });
        events.push(EventRef::MGRSC(&self.match_end_message));
        self.business_messages
            .iter()
            .for_each(|bm| events.push(EventRef::Business(bm)));
        events.into_iter()
    }
}

#[derive(Debug, Default)]
pub struct MatchReplayBuilder {
    pub match_id: Option<String>,
    pub match_start_message: Option<RequestTypeMGRSCEvent>,
    pub match_end_message: Option<RequestTypeMGRSCEvent>,
    pub client_server_messages: Vec<Event>,
    pub business_messages: Vec<BusinessEvent>,
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
            ParseOutput::GREMessage(gre_message) => {
                self.client_server_messages.push(Event::GRE(gre_message));
            }
            ParseOutput::ClientMessage(client_message) => self
                .client_server_messages
                .push(Event::Client(client_message)),
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
        let Some(controller_seat_id) = self.client_server_messages.iter().find_map(|e| {
            if let Event::GRE(r) = e {
                r.gre_to_client_event
                    .gre_to_client_messages
                    .iter()
                    .find_map(|e| {
                        if let GREToClientMessage::ConnectResp(w) = e {
                            w.meta.system_seat_ids.first().copied()
                        } else {
                            None
                        }
                    })
            } else {
                None
            }
        }) else {
            return Err(anyhow!("no controller seat id found"));
        };

        let match_replay = MatchReplay {
            match_id,
            controller_seat_id,
            match_start_message,
            match_end_message,
            client_server_messages: self.client_server_messages,
            business_messages: self.business_messages,
        };
        Ok(match_replay)
    }
}
