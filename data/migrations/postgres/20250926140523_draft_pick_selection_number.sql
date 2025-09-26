-- Add selection_number to draft_pack table and update unique constraint

-- Add selection_number column to draft_pack table
ALTER TABLE draft_pack ADD COLUMN selection_number INTEGER NOT NULL DEFAULT 0;

-- Drop the existing unique constraint
ALTER TABLE draft_pack DROP CONSTRAINT draft_pack_draft_id_pack_number_pick_number_key;

-- Add new unique constraint including selection_number
ALTER TABLE draft_pack ADD CONSTRAINT draft_pack_unique_selection
    UNIQUE(draft_id, pack_number, pick_number, selection_number);
