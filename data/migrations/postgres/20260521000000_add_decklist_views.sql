-- Decklist exploration views for Grafana.
--
-- `deck.deck_cards`, `deck.sideboard_cards` and `opponent_deck.cards` are stored
-- as TEXT containing a flat JSON array of arena IDs, where repeated IDs encode
-- quantity (e.g. [101,101,101,202] == 3x card 101, 1x card 202).
--
-- These views expand those arrays and join the `card` table so dashboards can
-- query human-readable decklists with a simple WHERE on match + player.

-- One row per individual card copy in the controller's decks, joined to card
-- metadata. `source` distinguishes main deck vs sideboard.
CREATE OR REPLACE VIEW deck_card_expanded AS
SELECT
    d.match_id,
    d.game_number,
    'main'::text AS source,
    elem.value::bigint AS arena_id,
    c.name,
    c.mana_cost,
    c.cmc,
    c.type_line,
    c.layout,
    c.colors,
    c.color_identity,
    c.set_code,
    c.image_uri
FROM deck d
CROSS JOIN LATERAL jsonb_array_elements_text(
    COALESCE(NULLIF(d.deck_cards, ''), '[]')::jsonb
) AS elem(value)
LEFT JOIN card c ON c.arena_id = elem.value::bigint
UNION ALL
SELECT
    d.match_id,
    d.game_number,
    'sideboard'::text AS source,
    elem.value::bigint AS arena_id,
    c.name,
    c.mana_cost,
    c.cmc,
    c.type_line,
    c.layout,
    c.colors,
    c.color_identity,
    c.set_code,
    c.image_uri
FROM deck d
CROSS JOIN LATERAL jsonb_array_elements_text(
    COALESCE(NULLIF(d.sideboard_cards, ''), '[]')::jsonb
) AS elem(value)
LEFT JOIN card c ON c.arena_id = elem.value::bigint;

-- One row per individual card copy in the opponent's observed deck.
CREATE OR REPLACE VIEW opponent_deck_card_expanded AS
SELECT
    od.match_id,
    elem.value::bigint AS arena_id,
    c.name,
    c.mana_cost,
    c.cmc,
    c.type_line,
    c.layout,
    c.colors,
    c.color_identity,
    c.set_code,
    c.image_uri
FROM opponent_deck od
CROSS JOIN LATERAL jsonb_array_elements_text(
    COALESCE(NULLIF(od.cards, ''), '[]')::jsonb
) AS elem(value)
LEFT JOIN card c ON c.arena_id = elem.value::bigint;

-- Unified, quantity-aggregated decklist keyed by match_id + player_name.
-- Combines controller decks (per game, main + sideboard) and the opponent's
-- observed deck so a Grafana panel only needs WHERE match_id = ... AND
-- player_name = ...
CREATE OR REPLACE VIEW match_decklist AS
SELECT
    m.id AS match_id,
    m.controller_player_name AS player_name,
    'controller'::text AS player_role,
    dce.game_number,
    dce.source,
    dce.arena_id,
    dce.name,
    dce.mana_cost,
    dce.cmc,
    dce.type_line,
    dce.layout,
    dce.colors,
    dce.color_identity,
    dce.set_code,
    dce.image_uri,
    count(*)::int AS quantity
FROM match m
JOIN deck_card_expanded dce ON dce.match_id = m.id
GROUP BY
    m.id, m.controller_player_name, dce.game_number, dce.source, dce.arena_id,
    dce.name, dce.mana_cost, dce.cmc, dce.type_line, dce.layout,
    dce.colors, dce.color_identity, dce.set_code, dce.image_uri
UNION ALL
SELECT
    m.id AS match_id,
    m.opponent_player_name AS player_name,
    'opponent'::text AS player_role,
    NULL::int AS game_number,
    'main'::text AS source,
    oce.arena_id,
    oce.name,
    oce.mana_cost,
    oce.cmc,
    oce.type_line,
    oce.layout,
    oce.colors,
    oce.color_identity,
    oce.set_code,
    oce.image_uri,
    count(*)::int AS quantity
FROM match m
JOIN opponent_deck_card_expanded oce ON oce.match_id = m.id
GROUP BY
    m.id, m.opponent_player_name, oce.arena_id,
    oce.name, oce.mana_cost, oce.cmc, oce.type_line, oce.layout,
    oce.colors, oce.color_identity, oce.set_code, oce.image_uri;
